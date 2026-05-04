use super::module_artifacts::*;
use super::*;
use std::time::Instant;

pub(crate) fn run_source_analysis_and_canonicalization(
    ui: &CliLog,
    entry_path: &str,
    entry_input: &str,
    total_steps: usize,
    output_opts: CompileOutputOptions,
) -> crate::error::RR<(SourceAnalysisOutput, SourceAnalysisMetrics)> {
    let mut loaded_paths: FxHashSet<PathBuf> = FxHashSet::default();
    let mut queue = std::collections::VecDeque::new();
    let module_cache_root = module_artifact_cache_root(entry_path);

    let entry_abs = normalize_module_path(Path::new(entry_path));
    loaded_paths.insert(entry_abs.clone());
    queue.push_back(ModuleLoadJob {
        path: entry_abs,
        content: Some(entry_input.to_string()),
        ast: None,
        mod_id: 0,
        is_entry: true,
        imports_preloaded: false,
    });

    let mut next_mod_id = 1;

    let _step_load = ui.step_start(
        1,
        total_steps,
        "Source Analysis",
        "parse + scope resolution",
    );
    let mut hir_modules = Vec::new();
    let mut hir_lowerer =
        crate::hir::lower::Lowerer::with_policy(crate::hir::lower::LoweringPolicy {
            strict_let: output_opts.strict_let,
            warn_implicit_decl: output_opts.warn_implicit_decl,
        });
    let mut load_errors: Vec<crate::error::RRException> = Vec::new();
    let mut source_analysis_elapsed_ns = 0u128;
    let mut canonicalization_elapsed_ns = 0u128;
    let mut parsed_modules = 0usize;
    let mut cached_modules = 0usize;

    while let Some(job) = queue.pop_front() {
        let ModuleLoadJob {
            path: curr_path,
            content,
            ast,
            mod_id,
            is_entry,
            imports_preloaded,
        } = job;
        let curr_path_str = curr_path.to_string_lossy().to_string();
        ui.trace(&format!("module#{}", mod_id), &curr_path_str);

        if !is_entry && ast.is_none() {
            let artifact_started = Instant::now();
            if let Some(module) = load_module_artifact(
                &module_cache_root,
                &curr_path,
                crate::hir::def::ModuleId(mod_id),
                &mut hir_lowerer,
            )? {
                source_analysis_elapsed_ns += artifact_started.elapsed().as_nanos();
                cached_modules += 1;
                ui.trace("artifact", &curr_path_str);
                enqueue_module_imports(
                    &module,
                    &curr_path,
                    &mut loaded_paths,
                    &mut queue,
                    &mut next_mod_id,
                )?;
                hir_modules.push(module);
                continue;
            }
            source_analysis_elapsed_ns += artifact_started.elapsed().as_nanos();
        }

        let source_started = Instant::now();
        let ast_prog = if let Some(ast_prog) = ast {
            ast_prog
        } else {
            let content = if let Some(content) = content {
                content
            } else {
                match fs::read_to_string(&curr_path) {
                    Ok(content) => content,
                    Err(e) => {
                        return Err(crate::error::RRException::new(
                            "RR.ParseError",
                            crate::error::RRCode::E0001,
                            crate::error::Stage::Parse,
                            format!("failed to load imported module '{}': {}", curr_path_str, e),
                        ));
                    }
                }
            };
            parsed_modules += 1;
            let mut parser = Parser::new(&content);
            match parser.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    load_errors.push(e);
                    continue;
                }
            }
        };
        if !imports_preloaded {
            let import_count = enqueue_ast_module_imports(
                &ast_prog.stmts,
                &curr_path,
                &mut loaded_paths,
                &mut queue,
                &mut next_mod_id,
            )?;
            if import_count > 0 {
                queue.insert(
                    import_count,
                    ModuleLoadJob {
                        path: curr_path,
                        content: None,
                        ast: Some(ast_prog),
                        mod_id,
                        is_entry,
                        imports_preloaded: true,
                    },
                );
                source_analysis_elapsed_ns += source_started.elapsed().as_nanos();
                continue;
            }
        }

        let source_metadata = ast_prog.clone();
        let hir_mod = match hir_lowerer.lower_module(ast_prog, crate::hir::def::ModuleId(mod_id)) {
            Ok(v) => v,
            Err(e) => {
                load_errors.push(e);
                continue;
            }
        };
        source_analysis_elapsed_ns += source_started.elapsed().as_nanos();
        for w in hir_lowerer.take_warnings() {
            ui.warn(&format!("{}: {}", curr_path_str, w));
        }

        enqueue_module_imports(
            &hir_mod,
            &curr_path,
            &mut loaded_paths,
            &mut queue,
            &mut next_mod_id,
        )?;

        let canonicalize_started = Instant::now();
        let desugared_module = desugar_single_module(hir_mod)?;
        canonicalization_elapsed_ns += canonicalize_started.elapsed().as_nanos();
        if !is_entry {
            store_module_artifact(
                &module_cache_root,
                &curr_path,
                &desugared_module,
                &hir_lowerer,
                source_metadata.clone(),
            )?;
            hir_lowerer.prune_private_module_metadata(&source_metadata);
        }
        hir_modules.push(desugared_module);
    }
    if !load_errors.is_empty() {
        if load_errors.len() == 1 {
            return Err(load_errors.remove(0));
        }
        return Err(crate::error::RRException::aggregate(
            "RR.ParseError",
            crate::error::RRCode::E0001,
            crate::error::Stage::Parse,
            format!("source analysis failed: {} error(s)", load_errors.len()),
            load_errors,
        ));
    }
    ui.step_line_ok(&format!(
        "Loaded {} module(s) in {}",
        hir_modules.len(),
        format_duration(duration_from_nanos(source_analysis_elapsed_ns))
    ));

    hir_modules.sort_by_key(|module| module.id.0);
    let hir_prog = crate::hir::def::HirProgram {
        modules: hir_modules,
    };
    let global_symbols = hir_lowerer.into_symbols();

    let step_desugar = ui.step_start(
        2,
        total_steps,
        "Canonicalization",
        "normalize HIR structure",
    );
    ui.step_line_ok(&format!(
        "Desugared {} source-read module(s) in {}",
        parsed_modules,
        format_duration(duration_from_nanos(canonicalization_elapsed_ns))
    ));
    let _ = step_desugar;

    Ok((
        SourceAnalysisOutput {
            desugared_hir: hir_prog,
            global_symbols,
        },
        SourceAnalysisMetrics {
            source_analysis_elapsed_ns,
            canonicalization_elapsed_ns,
            parsed_modules,
            cached_modules,
        },
    ))
}
