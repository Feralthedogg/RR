//! Source-loading, MIR synthesis, and emitted-R assembly helpers for the
//! compiler pipeline.
//!
//! The functions in this module prepare stable per-function jobs, lower HIR to
//! MIR, and concatenate emitted function fragments into the final artifact.

use super::super::*;
use crate::error::{RRCode, RRException};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedModuleArtifact {
    schema: String,
    compiler_version: String,
    canonical_path: String,
    source_len: u64,
    source_mtime_ns: u128,
    public_symbols: Vec<String>,
    public_function_arities: Vec<(String, usize)>,
    emit_roots: Vec<String>,
    module_fingerprint: u64,
    symbols: Vec<(u32, String)>,
    module: crate::hir::def::HirModule,
}

struct ModuleLoadJob {
    path: PathBuf,
    content: Option<String>,
    mod_id: u32,
    is_entry: bool,
}

/// Owned lowering job for a user-declared function.
pub(crate) struct MirLowerJob {
    pub(crate) fn_name: String,
    pub(crate) is_public: bool,
    pub(crate) params: Vec<String>,
    pub(crate) var_names: FxHashMap<crate::hir::def::LocalId, String>,
    pub(crate) hir_fn: crate::hir::def::HirFn,
}

/// Synthetic lowering job used for module top-level entry shims.
pub(crate) struct TopLevelMirLowerJob {
    pub(crate) fn_name: String,
    pub(crate) hir_fn: crate::hir::def::HirFn,
}

/// One parallel emission result before the final artifact is concatenated.
pub(crate) struct EmittedFnFragment {
    pub(crate) code: String,
    pub(crate) map: Vec<MapEntry>,
    pub(crate) cache_hit: bool,
    pub(crate) optimized_code: Option<String>,
    pub(crate) optimized_map: Option<Vec<MapEntry>>,
    pub(crate) optimized_cache_hit: bool,
}

/// Cheap work estimate used by the scheduler for HIR-lowering jobs.
pub(crate) fn hir_fn_work_size(f: &crate::hir::def::HirFn) -> usize {
    let stmt_count = f.body.stmts.len();
    let local_count = f.local_names.len();
    (stmt_count * 8)
        .saturating_add(local_count)
        .saturating_add(f.params.len())
}

/// Collect user-defined callees that are reachable from a single MIR function.
pub(crate) fn called_user_fns(
    fn_ir: &crate::mir::def::FnIR,
    program: &ProgramIR,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for value in &fn_ir.values {
        match &value.kind {
            crate::mir::def::ValueKind::Call { callee, .. } => {
                let canonical = callee.strip_suffix("_fresh").unwrap_or(callee);
                if program.contains_name(canonical) {
                    out.insert(canonical.to_string());
                }
            }
            crate::mir::def::ValueKind::Load { var } => {
                if program.contains_name(var) {
                    out.insert(var.clone());
                }
            }
            crate::mir::def::ValueKind::RSymbol { name } => {
                if program.contains_name(name) {
                    out.insert(name.clone());
                }
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn collect_seq_len_param_end_slots_by_fn(
    program: &ProgramIR,
) -> FxHashMap<String, FxHashMap<usize, usize>> {
    fn unique_assign_source(
        fn_ir: &crate::mir::def::FnIR,
        var: &str,
    ) -> Option<crate::mir::def::ValueId> {
        let mut src: Option<crate::mir::def::ValueId> = None;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                let crate::mir::def::Instr::Assign {
                    dst, src: value, ..
                } = instr
                else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                match src {
                    None => src = Some(*value),
                    Some(prev) if prev == *value => {}
                    Some(_) => return None,
                }
            }
        }
        src
    }

    fn resolve_load_alias_value(
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
    ) -> crate::mir::def::ValueId {
        let mut cur = vid;
        let mut seen = FxHashSet::default();
        while seen.insert(cur) {
            let crate::mir::def::ValueKind::Load { var } = &fn_ir.values[cur].kind else {
                break;
            };
            let Some(src) = unique_assign_source(fn_ir, var) else {
                break;
            };
            cur = src;
        }
        cur
    }

    fn seq_len_base_arg_slot(
        fn_ir: &crate::mir::def::FnIR,
        call_args: &[crate::mir::def::ValueId],
        arg: crate::mir::def::ValueId,
    ) -> Option<usize> {
        let resolved = resolve_load_alias_value(fn_ir, arg);
        let crate::mir::def::ValueKind::Call {
            callee,
            args,
            names,
        } = &fn_ir.values[resolved].kind
        else {
            return None;
        };
        if callee != "seq_len" || args.len() != 1 || names.iter().any(|name| name.is_some()) {
            return None;
        }
        let base = resolve_load_alias_value(fn_ir, args[0]);
        let mut matches = call_args
            .iter()
            .enumerate()
            .filter_map(|(slot, candidate)| {
                (resolve_load_alias_value(fn_ir, *candidate) == base).then_some(slot)
            });
        let slot = matches.next()?;
        matches.next().is_none().then_some(slot)
    }

    let mut summaries: FxHashMap<String, FxHashMap<usize, usize>> = FxHashMap::default();

    for slot in program.all_slots() {
        let Some(fn_ir) = program.get_slot(slot) else {
            continue;
        };
        for value in &fn_ir.values {
            let crate::mir::def::ValueKind::Call {
                callee,
                args,
                names,
            } = &value.kind
            else {
                continue;
            };
            if names.iter().any(|name| name.is_some()) {
                continue;
            }
            let canonical = callee.strip_suffix("_fresh").unwrap_or(callee.as_str());
            let Some(callee_ir) = program.get(canonical) else {
                continue;
            };
            if args.len() != callee_ir.params.len() {
                continue;
            }
            let local: FxHashMap<usize, usize> = args
                .iter()
                .enumerate()
                .filter_map(|(slot, arg)| {
                    seq_len_base_arg_slot(fn_ir, args, *arg).map(|end_slot| (slot, end_slot))
                })
                .collect();
            let entry = summaries
                .entry(canonical.to_string())
                .or_insert(local.clone());
            if entry != &local {
                entry.retain(|slot, end_slot| local.get(slot) == Some(end_slot));
            }
        }
    }

    summaries.retain(|_, slots| !slots.is_empty());
    summaries
}

pub(crate) fn reachable_emit_order_slots(program: &ProgramIR) -> Vec<FnSlot> {
    if program.emit_roots.is_empty() {
        return program.emit_order.clone();
    }

    let mut reachable = FxHashSet::default();
    let mut worklist: Vec<String> = program
        .emit_root_names()
        .iter()
        .filter(|name| program.contains_name(name.as_str()))
        .cloned()
        .collect();

    while let Some(name) = worklist.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(fn_ir) = program.get(&name) else {
            continue;
        };
        for callee in called_user_fns(fn_ir, program) {
            if !reachable.contains(&callee) {
                worklist.push(callee);
            }
        }
    }

    if reachable.iter().all(|name| name.starts_with("Sym_top_")) {
        return program.emit_order.clone();
    }

    program
        .emit_order
        .iter()
        .copied()
        .filter(|slot| {
            program
                .fns
                .get(*slot)
                .is_some_and(|unit| reachable.contains(unit.name.as_str()))
        })
        .collect()
}

fn duration_from_nanos(ns: u128) -> Duration {
    Duration::from_nanos(ns.min(u64::MAX as u128) as u64)
}

fn module_artifact_cache_root(entry_path: &str) -> PathBuf {
    if let Some(v) = std::env::var_os("RR_INCREMENTAL_CACHE_DIR")
        && !v.is_empty()
    {
        return PathBuf::from(v).join("modules");
    }

    let normalized_entry = normalize_module_path(Path::new(entry_path));
    let mut cur = normalized_entry
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        let managed_root = cur.join("rr.mod").is_file()
            || cur.join("src").join("main.rr").is_file()
            || cur.join("src").join("lib.rr").is_file();
        let legacy_root = cur.file_name().and_then(|name| name.to_str()) != Some("src")
            && cur.join("main.rr").is_file();
        if managed_root || legacy_root {
            return cur.join("Build").join("incremental").join("modules");
        }
        let Some(parent) = cur.parent() else {
            return normalized_entry
                .parent()
                .unwrap_or(Path::new("."))
                .join(".rr-cache")
                .join("modules");
        };
        cur = parent.to_path_buf();
    }
}

fn module_artifact_path(cache_root: &Path, canonical_path: &Path) -> PathBuf {
    let path_hash = stable_hash_bytes(canonical_path.to_string_lossy().as_bytes());
    cache_root.join(format!("{:016x}.json", path_hash))
}

fn source_file_signature(path: &Path) -> std::io::Result<(u64, u128)> {
    let meta = fs::metadata(path)?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_nanos())
        .unwrap_or(0);
    Ok((meta.len(), modified))
}

fn desugar_single_module(
    module: crate::hir::def::HirModule,
) -> crate::error::RR<crate::hir::def::HirModule> {
    let mut desugarer = crate::hir::desugar::Desugarer::new();
    let mut program = desugarer.desugar_program(crate::hir::def::HirProgram {
        modules: vec![module],
    })?;
    program.modules.pop().ok_or_else(|| {
        InternalCompilerError::new(Stage::Lower, "desugarer produced no module").into_exception()
    })
}

fn collect_public_symbols_from_module(
    module: &crate::hir::def::HirModule,
    symbol_map: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> Vec<String> {
    let mut names = Vec::new();
    for item in &module.items {
        if let crate::hir::def::HirItem::Fn(f) = item
            && f.public
        {
            if let Some(name) = symbol_map.get(&f.name) {
                names.push(name.clone());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn collect_public_function_arities(
    module: &crate::hir::def::HirModule,
    symbol_map: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for item in &module.items {
        if let crate::hir::def::HirItem::Fn(f) = item
            && f.public
            && let Some(name) = symbol_map.get(&f.name)
        {
            out.push((name.clone(), f.params.len()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect_emit_roots(
    module: &crate::hir::def::HirModule,
    symbol_map: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> Vec<String> {
    let mut out = collect_public_symbols_from_module(module, symbol_map);
    if module
        .items
        .iter()
        .any(|item| matches!(item, crate::hir::def::HirItem::Stmt(_)))
    {
        out.push(format!("Sym_top_{}", module.id.0));
    }
    out.sort();
    out.dedup();
    out
}

fn build_cached_module_artifact(
    module_path: &Path,
    module: &crate::hir::def::HirModule,
    lowerer: &crate::hir::lower::Lowerer,
) -> crate::error::RR<CachedModuleArtifact> {
    let (source_len, source_mtime_ns) = source_file_signature(module_path).map_err(|e| {
        RRException::new(
            "RR.ParseError",
            RRCode::E0001,
            Stage::Parse,
            format!(
                "failed to read module metadata '{}': {}",
                module_path.display(),
                e
            ),
        )
    })?;
    let symbol_entries = lowerer.symbols_snapshot();
    let mut symbol_map = FxHashMap::default();
    for (id, name) in &symbol_entries {
        symbol_map.insert(*id, name.clone());
    }
    let public_symbols = collect_public_symbols_from_module(module, &symbol_map);
    let public_function_arities = collect_public_function_arities(module, &symbol_map);
    let emit_roots = collect_emit_roots(module, &symbol_map);
    let module_fingerprint = serde_json::to_vec(module)
        .map(|bytes| stable_hash_bytes(&bytes))
        .unwrap_or(0);
    Ok(CachedModuleArtifact {
        schema: "rr-module-artifact".to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        canonical_path: module_path.to_string_lossy().to_string(),
        source_len,
        source_mtime_ns,
        public_symbols,
        public_function_arities,
        emit_roots,
        module_fingerprint,
        symbols: symbol_entries
            .into_iter()
            .map(|(id, name)| (id.0, name))
            .collect(),
        module: module.clone(),
    })
}

fn store_module_artifact(
    cache_root: &Path,
    module_path: &Path,
    module: &crate::hir::def::HirModule,
    lowerer: &crate::hir::lower::Lowerer,
) -> crate::error::RR<()> {
    fs::create_dir_all(cache_root).map_err(|e| {
        RRException::new(
            "RR.CompilerError",
            RRCode::ICE9001,
            Stage::Codegen,
            format!(
                "failed to create module artifact cache dir '{}': {}",
                cache_root.display(),
                e
            ),
        )
    })?;
    let artifact = build_cached_module_artifact(module_path, module, lowerer)?;
    let payload = serde_json::to_vec_pretty(&artifact).map_err(|e| {
        InternalCompilerError::new(
            Stage::Codegen,
            format!(
                "failed to serialize module artifact '{}': {}",
                module_path.display(),
                e
            ),
        )
        .into_exception()
    })?;
    fs::write(module_artifact_path(cache_root, module_path), payload).map_err(|e| {
        RRException::new(
            "RR.CompilerError",
            RRCode::ICE9001,
            Stage::Codegen,
            format!(
                "failed to write module artifact '{}': {}",
                module_path.display(),
                e
            ),
        )
    })?;
    Ok(())
}

fn load_module_artifact(
    cache_root: &Path,
    module_path: &Path,
    mod_id: crate::hir::def::ModuleId,
    lowerer: &mut crate::hir::lower::Lowerer,
) -> crate::error::RR<Option<crate::hir::def::HirModule>> {
    let artifact_path = module_artifact_path(cache_root, module_path);
    if !artifact_path.is_file() {
        return Ok(None);
    }
    let payload = match fs::read(&artifact_path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let artifact: CachedModuleArtifact = match serde_json::from_slice(&payload) {
        Ok(artifact) => artifact,
        Err(_) => return Ok(None),
    };
    if artifact.schema != "rr-module-artifact"
        || artifact.compiler_version != env!("CARGO_PKG_VERSION")
        || artifact.canonical_path != module_path.to_string_lossy()
    {
        return Ok(None);
    }
    let (source_len, source_mtime_ns) = match source_file_signature(module_path) {
        Ok(sig) => sig,
        Err(_) => return Ok(None),
    };
    if artifact.source_len != source_len || artifact.source_mtime_ns != source_mtime_ns {
        return Ok(None);
    }
    let symbols: Vec<(crate::hir::def::SymbolId, String)> = artifact
        .symbols
        .into_iter()
        .map(|(id, name)| (crate::hir::def::SymbolId(id), name))
        .collect();
    if !lowerer.try_preload_symbols(&symbols) {
        return Ok(None);
    }
    let mut module = artifact.module;
    module.id = mod_id;
    Ok(Some(module))
}

fn enqueue_module_imports(
    module: &crate::hir::def::HirModule,
    curr_path: &Path,
    loaded_paths: &mut FxHashSet<PathBuf>,
    queue: &mut std::collections::VecDeque<ModuleLoadJob>,
    next_mod_id: &mut u32,
) -> crate::error::RR<()> {
    for item in &module.items {
        if let crate::hir::def::HirItem::Import(imp) = item {
            let target = crate::pkg::resolve_import_path(curr_path, &imp.module)?;
            if !loaded_paths.contains(&target) {
                if !target.is_absolute() {
                    return Err(crate::error::RRException::new(
                        "RR.ParseError",
                        crate::error::RRCode::E0001,
                        crate::error::Stage::Parse,
                        format!(
                            "relative import resolution requires an absolute entry path; normalize '{}' before compiling",
                            curr_path.display()
                        ),
                    ));
                }
                loaded_paths.insert(target.clone());
                queue.push_back(ModuleLoadJob {
                    path: target,
                    content: None,
                    mod_id: *next_mod_id,
                    is_entry: false,
                });
                *next_mod_id += 1;
            }
        }
    }
    Ok(())
}

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
        mod_id: 0,
        is_entry: true,
    });

    let mut next_mod_id = 1;

    let _step_load = ui.step_start(
        1,
        total_steps,
        "Source Analysis",
        "parse + scope resolution",
    );
    let mut hir_modules = Vec::new();
    let mut hir_lowerer = crate::hir::lower::Lowerer::with_policy(
        output_opts.strict_let,
        output_opts.warn_implicit_decl,
    );
    let mut load_errors: Vec<crate::error::RRException> = Vec::new();
    let mut source_analysis_elapsed_ns = 0u128;
    let mut canonicalization_elapsed_ns = 0u128;
    let mut parsed_modules = 0usize;
    let mut cached_modules = 0usize;

    while let Some(job) = queue.pop_front() {
        let curr_path = job.path;
        let curr_path_str = curr_path.to_string_lossy().to_string();
        ui.trace(&format!("module#{}", job.mod_id), &curr_path_str);

        if !job.is_entry {
            let artifact_started = Instant::now();
            if let Some(module) = load_module_artifact(
                &module_cache_root,
                &curr_path,
                crate::hir::def::ModuleId(job.mod_id),
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

        let content = if let Some(content) = job.content {
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

        let source_started = Instant::now();
        let mut parser = Parser::new(&content);
        let ast_prog = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                load_errors.push(e);
                continue;
            }
        };

        let hir_mod =
            match hir_lowerer.lower_module(ast_prog, crate::hir::def::ModuleId(job.mod_id)) {
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
        if !job.is_entry {
            store_module_artifact(
                &module_cache_root,
                &curr_path,
                &desugared_module,
                &hir_lowerer,
            )?;
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

fn emit_r_functions(
    ui: &CliLog,
    total_steps: usize,
    program: &ProgramIR,
    emit_order: &[FnSlot],
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
    let (out, map, _, _, _) = emit_r_functions_cached(
        ui,
        total_steps,
        program,
        emit_order,
        &[],
        OptLevel::O0,
        TypeConfig::default(),
        ParallelConfig::default(),
        &scheduler,
        CompileOutputOptions::default(),
        None,
    )?;
    Ok((out, map))
}

fn trivial_zero_arg_entry_callee(
    fn_ir: &crate::mir::def::FnIR,
    program: &ProgramIR,
) -> Option<String> {
    if !fn_ir.params.is_empty() {
        return None;
    }
    let mut returned = None;
    for block in &fn_ir.blocks {
        if let crate::mir::def::Terminator::Return(Some(val)) = block.term
            && returned.replace(val).is_some()
        {
            return None;
        }
    }
    let ret = returned?;
    match &fn_ir.values.get(ret)?.kind {
        crate::mir::def::ValueKind::Call {
            callee,
            args,
            names,
        } if args.is_empty() && names.is_empty() => {
            let target = program.get(callee)?;
            if target.params.is_empty() && !callee.starts_with("Sym_top_") {
                Some(callee.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn quoted_body_entry_targets(program: &ProgramIR, top_level_calls: &[FnSlot]) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for top_slot in top_level_calls {
        let Some(top_fn) = program.get_slot(*top_slot) else {
            continue;
        };
        let Some(callee) = trivial_zero_arg_entry_callee(top_fn, program) else {
            continue;
        };
        out.insert(callee);
    }
    out
}

fn wrap_zero_arg_function_body_in_quote(code: &str, fn_name: &str) -> Option<String> {
    const MIN_LINES_FOR_ENTRY_QUOTE_WRAP: usize = 20;
    if code.lines().count() < MIN_LINES_FOR_ENTRY_QUOTE_WRAP {
        return None;
    }

    let header = format!("{fn_name} <- function() \n{{\n");
    let footer = "}\n";
    if !code.starts_with(&header) || !code.ends_with(footer) {
        return None;
    }

    let body = &code[header.len()..code.len() - footer.len()];
    let body_name = format!(".__rr_body_{}", fn_name);
    let mut wrapped = String::new();
    wrapped.push_str(&format!("{body_name} <- quote({{\n"));
    wrapped.push_str(body);
    if !body.ends_with('\n') {
        wrapped.push('\n');
    }
    wrapped.push_str("})\n");
    wrapped.push_str(&header);
    wrapped.push_str(&format!("  eval({body_name}, envir = environment())\n"));
    wrapped.push_str(footer);
    Some(wrapped)
}

fn apply_raw_rewrites_to_fragment(
    mut output: String,
    pure_user_calls: &FxHashSet<String>,
    _output_opts: CompileOutputOptions,
) -> String {
    let _ = pure_user_calls;
    output = rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&output);
    output = rewrite_mountain_dx_temp_in_raw_emitted_r(&output);
    output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&output);
    output = strip_unused_helper_params_in_raw_emitted_r(&output);
    output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&output);
    output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&output);
    output = rewrite_helper_expr_reuse_calls_in_raw_emitted_r(&output);
    output = rewrite_dot_product_helper_calls_in_raw_emitted_r(&output);
    output = rewrite_sym119_helper_calls_in_raw_emitted_r(&output);
    output = rewrite_trivial_fill_helper_calls_in_raw_emitted_r(&output);
    output = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&output);
    output = rewrite_duplicate_sym183_calls_in_raw_emitted_r(&output);
    output = restore_particle_state_rebinds_in_raw_emitted_r(&output);
    output = collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(&output);
    output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&output);
    output = restore_cg_loop_carried_updates_in_raw_emitted_r(&output);
    output = restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(&output);
    output = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&output);
    output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&output);
    output = rewrite_mountain_dx_temp_in_raw_emitted_r(&output);
    output = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&output);
    output = strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(&output);
    output = restore_cg_loop_carried_updates_in_raw_emitted_r(&output);
    output = strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(&output);
    output
}

fn optimize_emitted_fragment(
    code: &str,
    map: &[MapEntry],
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
) -> (String, Vec<MapEntry>) {
    let mut output = code.to_string();
    let skip_generated_poly_loop_rewrites = contains_generated_poly_loop_controls(&output);
    if !skip_generated_poly_loop_rewrites {
        output = apply_raw_rewrites_to_fragment(output, pure_user_calls, output_opts);
    }
    let (optimized_output, line_map) = if skip_generated_poly_loop_rewrites {
        let line_map = (1..=output.lines().count() as u32).collect::<Vec<_>>();
        (output, line_map)
    } else {
        crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_options(
            &output,
            direct_builtin_call_map,
            pure_user_calls,
            fresh_user_calls,
            output_opts.preserve_all_defs,
        )
    };
    let optimized_map = remap_source_map_lines(map.to_vec(), &line_map);
    (optimized_output, optimized_map)
}

fn apply_full_raw_rewrites(
    mut output: String,
    pure_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
) -> String {
    let _ = pure_user_calls;
    output = rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&output);
    output = rewrite_mountain_dx_temp_in_raw_emitted_r(&output);
    output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&output);
    output = strip_unused_helper_params_in_raw_emitted_r(&output);
    output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&output);
    output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&output);
    output = rewrite_helper_expr_reuse_calls_in_raw_emitted_r(&output);
    output = rewrite_dot_product_helper_calls_in_raw_emitted_r(&output);
    output = rewrite_sym119_helper_calls_in_raw_emitted_r(&output);
    output = rewrite_trivial_fill_helper_calls_in_raw_emitted_r(&output);
    output = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&output);
    output = rewrite_duplicate_sym183_calls_in_raw_emitted_r(&output);
    output = restore_particle_state_rebinds_in_raw_emitted_r(&output);
    output = collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(&output);
    output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&output);
    output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&output);
    output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&output);
    output = restore_cg_loop_carried_updates_in_raw_emitted_r(&output);
    output = restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(&output);
    output = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&output);
    output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&output);
    output = rewrite_mountain_dx_temp_in_raw_emitted_r(&output);
    output = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&output);
    output = strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(&output);
    if !output_opts.preserve_all_defs {
        output = prune_unreachable_raw_helper_definitions(&output);
    }
    output = restore_cg_loop_carried_updates_in_raw_emitted_r(&output);
    if !output_opts.preserve_all_defs {
        output = prune_unreachable_raw_helper_definitions(&output);
    }
    output = strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(&output);
    output
}

fn apply_post_assembly_finalize_rewrites(output: String) -> String {
    let mut kept = Vec::new();
    let mut prev_blank = false;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed == "# rr-cse-pruned" {
            continue;
        }
        if trimmed.is_empty() {
            if prev_blank {
                continue;
            }
            prev_blank = true;
            kept.push(String::new());
            continue;
        }
        prev_blank = false;
        kept.push(line.to_string());
    }
    let mut rewritten = kept.join("\n");
    if !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten
}

fn maybe_emit_raw_debug_output(
    assembled_output: &str,
    pure_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
    cache: Option<&dyn EmitFunctionCache>,
) -> crate::error::RR<u128> {
    let Some(path) = std::env::var_os("RR_DEBUG_RAW_R_PATH") else {
        return Ok(0);
    };
    let started = Instant::now();
    let raw_output = if contains_generated_poly_loop_controls(assembled_output) {
        assembled_output.to_string()
    } else {
        let raw_rewrite_cache_key = cache.map(|_| {
            crate::compiler::pipeline::raw_rewrite_output_cache_key(
                assembled_output,
                pure_user_calls,
                output_opts.preserve_all_defs,
                output_opts.compile_mode,
            )
        });
        if let (Some(cache), Some(cache_key)) = (cache, raw_rewrite_cache_key.as_deref()) {
            if let Some(cached_output) = cache.load_raw_rewrite(cache_key)? {
                cached_output
            } else {
                let rewritten =
                    apply_full_raw_rewrites(assembled_output.to_string(), pure_user_calls, output_opts);
                cache.store_raw_rewrite(cache_key, &rewritten)?;
                rewritten
            }
        } else {
            apply_full_raw_rewrites(assembled_output.to_string(), pure_user_calls, output_opts)
        }
    };
    let _ = std::fs::write(path, &raw_output);
    Ok(started.elapsed().as_nanos())
}

fn apply_full_peephole_to_output(
    output: &str,
    map: &[MapEntry],
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
) -> (String, Vec<MapEntry>) {
    let (optimized_output, line_map) =
        crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_options(
            output,
            direct_builtin_call_map,
            pure_user_calls,
            fresh_user_calls,
            output_opts.preserve_all_defs,
        );
    let optimized_map = remap_source_map_lines(map.to_vec(), &line_map);
    (optimized_output, optimized_map)
}

fn assemble_emitted_fragments(
    fragments: &[EmittedFnFragment],
    use_optimized: bool,
) -> (String, Vec<MapEntry>) {
    let mut final_output = String::new();
    let mut final_source_map = Vec::new();
    let mut line_offset = 0u32;

    for fragment in fragments {
        let (code, map) = if use_optimized {
            (
                fragment.optimized_code.as_ref().unwrap_or(&fragment.code),
                fragment.optimized_map.as_ref().unwrap_or(&fragment.map),
            )
        } else {
            (&fragment.code, &fragment.map)
        };
        let mut shifted_map = map.clone();
        for entry in &mut shifted_map {
            entry.r_line = entry.r_line.saturating_add(line_offset);
        }
        line_offset = line_offset.saturating_add(emitted_segment_line_count(code));
        final_output.push_str(code);
        final_output.push('\n');
        final_source_map.extend(shifted_map);
    }

    (final_output, final_source_map)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_r_functions_cached(
    ui: &CliLog,
    total_steps: usize,
    program: &ProgramIR,
    emit_order: &[FnSlot],
    top_level_calls: &[FnSlot],
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    scheduler: &CompilerScheduler,
    output_opts: CompileOutputOptions,
    cache: Option<&dyn EmitFunctionCache>,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize, EmitMetrics)> {
    let step_emit = ui.step_start(
        5,
        total_steps,
        "R Code Emission",
        "reconstruct control flow",
    );
    let quoted_entry_targets = quoted_body_entry_targets(program, top_level_calls);
    let pure_user_calls = collect_referentially_pure_user_functions(program);
    let fresh_user_calls = collect_fresh_returning_user_functions(program);
    let seq_len_param_end_slots_by_fn = collect_seq_len_param_end_slots_by_fn(program);
    let shared_pure_user_calls = Arc::new(pure_user_calls.clone());
    let shared_fresh_user_calls = Arc::new(fresh_user_calls.clone());
    let shared_seq_len_param_end_slots_by_fn = Arc::new(seq_len_param_end_slots_by_fn.clone());
    let direct_builtin_call_map =
        matches!(type_cfg.native_backend, crate::typeck::NativeBackend::Off)
            && matches!(parallel_cfg.mode, ParallelMode::Off);
    let emit_total_ir = emit_order
        .iter()
        .filter_map(|slot| program.get_slot(*slot))
        .map(fn_ir_work_size)
        .sum::<usize>();
    let cache_load_elapsed_ns = Arc::new(AtomicU64::new(0));
    let emitter_elapsed_ns = Arc::new(AtomicU64::new(0));
    let cache_store_elapsed_ns = Arc::new(AtomicU64::new(0));
    let quote_wrap_elapsed_ns = Arc::new(AtomicU64::new(0));
    let quoted_wrapped_functions = Arc::new(AtomicUsize::new(0));
    let fragment_build_started = Instant::now();
    let emitted_fragments = scheduler.map_try_stage(
        crate::compiler::scheduler::CompilerParallelStage::Emit,
        emit_order.to_vec(),
        emit_total_ir,
        |slot| {
            let cache_load_elapsed_ns = Arc::clone(&cache_load_elapsed_ns);
            let emitter_elapsed_ns = Arc::clone(&emitter_elapsed_ns);
            let cache_store_elapsed_ns = Arc::clone(&cache_store_elapsed_ns);
            let quote_wrap_elapsed_ns = Arc::clone(&quote_wrap_elapsed_ns);
            let quoted_wrapped_functions = Arc::clone(&quoted_wrapped_functions);
            let shared_pure_user_calls = Arc::clone(&shared_pure_user_calls);
            let shared_fresh_user_calls = Arc::clone(&shared_fresh_user_calls);
            let shared_seq_len_param_end_slots_by_fn =
                Arc::clone(&shared_seq_len_param_end_slots_by_fn);
            let Some(unit) = program.fns.get(slot) else {
                return Err(crate::error::InternalCompilerError::new(
                    crate::error::Stage::Codegen,
                    format!("emit order references missing function slot '{}'", slot),
                )
                .into_exception());
            };
            let Some(fn_ir) = unit.ir.as_ref() else {
                return Err(crate::error::InternalCompilerError::new(
                crate::error::Stage::Codegen,
                format!(
                    "emit order references missing function '{}': MIR synthesis invariant violated",
                    unit.name
                ),
            )
            .into_exception());
            };

            let key = fn_emit_cache_key(
                fn_ir,
                opt_level,
                type_cfg,
                parallel_cfg,
                output_opts.compile_mode,
                seq_len_param_end_slots_by_fn.get(unit.name.as_str()),
            );
            let maybe_hit = if let Some(c) = cache {
                let started = Instant::now();
                let loaded = c.load(&key)?;
                cache_load_elapsed_ns
                    .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                loaded
            } else {
                None
            };

            let (code, map, cache_hit) = if let Some((code, map)) = maybe_hit {
                (code, map, true)
            } else {
                let mut emitter =
                    crate::codegen::mir_emit::MirEmitter::with_shared_analysis_options(
                        Arc::clone(&shared_fresh_user_calls),
                        Arc::clone(&shared_pure_user_calls),
                        shared_seq_len_param_end_slots_by_fn,
                        direct_builtin_call_map,
                    );
                let emit_started = Instant::now();
                let (code, map) = emitter.emit(fn_ir)?;
                emitter_elapsed_ns
                    .fetch_add(emit_started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                if let Some(c) = cache {
                    let store_started = Instant::now();
                    c.store(&key, &code, &map)?;
                    cache_store_elapsed_ns
                        .fetch_add(store_started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                }
                (code, map, false)
            };

            let mut code = code;
            let mut map = map;
            let wrap_started = Instant::now();
            let maybe_wrapped = if quoted_entry_targets.contains(unit.name.as_str()) {
                wrap_zero_arg_function_body_in_quote(&code, unit.name.as_str())
            } else {
                None
            };
            if let Some(wrapped) = maybe_wrapped {
                code = wrapped;
                for entry in &mut map {
                    entry.r_line = entry.r_line.saturating_sub(1);
                }
                quote_wrap_elapsed_ns
                    .fetch_add(wrap_started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                quoted_wrapped_functions.fetch_add(1, Ordering::Relaxed);
            }
            let optimized_key = cache.map(|_| {
                crate::compiler::pipeline::optimized_fragment_output_cache_key(
                    &code,
                    direct_builtin_call_map,
                    shared_pure_user_calls.as_ref(),
                    shared_fresh_user_calls.as_ref(),
                    output_opts.preserve_all_defs,
                    output_opts.compile_mode,
                )
            });
            let maybe_optimized_hit =
                if let (Some(c), Some(key)) = (cache, optimized_key.as_deref()) {
                    c.load_optimized_fragment(key)?
                } else {
                    None
                };
            let (optimized_code, optimized_map, optimized_cache_hit) =
                if let Some((optimized_code, optimized_map)) = maybe_optimized_hit {
                    (Some(optimized_code), Some(optimized_map), true)
                } else if let (Some(c), Some(key)) = (cache, optimized_key.as_deref()) {
                    let (optimized_code, optimized_map) = optimize_emitted_fragment(
                        &code,
                        &map,
                        direct_builtin_call_map,
                        shared_pure_user_calls.as_ref(),
                        shared_fresh_user_calls.as_ref(),
                        output_opts,
                    );
                    c.store_optimized_fragment(key, &optimized_code, &optimized_map)?;
                    (Some(optimized_code), Some(optimized_map), false)
                } else {
                    (None, None, false)
                };
            Ok(EmittedFnFragment {
                code,
                map,
                cache_hit,
                optimized_code,
                optimized_map,
                optimized_cache_hit,
            })
        },
    )?;
    let fragment_build_elapsed_ns = fragment_build_started.elapsed().as_nanos();

    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let optimized_fragment_cache_hits = emitted_fragments
        .iter()
        .filter(|fragment| fragment.optimized_cache_hit)
        .count();
    let optimized_fragment_cache_misses = emitted_fragments
        .iter()
        .filter(|fragment| fragment.optimized_code.is_some() && !fragment.optimized_cache_hit)
        .count();
    for fragment in &emitted_fragments {
        if fragment.cache_hit {
            cache_hits += 1;
        } else {
            cache_misses += 1;
        }
    }
    let fragment_assembly_started = Instant::now();
    let (mut final_output, final_source_map) =
        assemble_emitted_fragments(&emitted_fragments, false);
    let fragment_assembly_elapsed_ns = fragment_assembly_started.elapsed().as_nanos();
    ui.step_line_ok(&format!(
        "Emitted {} functions ({} debug maps) in {}",
        emit_order.len(),
        final_source_map.len(),
        format_duration(step_emit.elapsed())
    ));

    let optimized_fragment_variants_ready = emitted_fragments.iter().all(|fragment| {
        fragment.optimized_code.is_some() && fragment.optimized_map.is_some()
    });
    let optimized_assembly_key = cache.map(|_| {
        crate::compiler::pipeline::optimized_assembly_cache_key(
            &final_output,
            direct_builtin_call_map,
            &pure_user_calls,
            &fresh_user_calls,
            output_opts.preserve_all_defs,
            output_opts.compile_mode,
        )
    });
    let cached_optimized_final_source_map =
        if let (Some(c), Some(key)) = (cache, optimized_assembly_key.as_deref()) {
            c.load_optimized_assembly_source_map(key)?
        } else {
            None
        };
    let cached_optimized_final_artifact =
        if let (Some(c), Some(key)) = (cache, optimized_assembly_key.as_deref()) {
            c.load_optimized_assembly_artifact(key)?
        } else {
            None
        };
    if optimized_fragment_variants_ready
        && let Some((cached_output, cached_source_map)) = cached_optimized_final_artifact
    {
        let raw_debug_elapsed_ns =
            maybe_emit_raw_debug_output(&final_output, &pure_user_calls, output_opts, cache)?;
        return Ok((
            cached_output,
            cached_source_map,
            cache_hits,
            cache_misses,
            EmitMetrics {
                elapsed_ns: step_emit.elapsed().as_nanos(),
                emitted_functions: emit_order.len(),
                cache_hits,
                cache_misses,
                breakdown: EmitBreakdownProfile {
                    fragment_build_elapsed_ns,
                    cache_load_elapsed_ns: cache_load_elapsed_ns.load(Ordering::Relaxed) as u128,
                    emitter_elapsed_ns: emitter_elapsed_ns.load(Ordering::Relaxed) as u128,
                    cache_store_elapsed_ns: cache_store_elapsed_ns.load(Ordering::Relaxed) as u128,
                    optimized_fragment_cache_hits,
                    optimized_fragment_cache_misses,
                    optimized_fragment_final_artifact_hits: 1,
                    optimized_fragment_fast_path_direct_hits: 0,
                    optimized_fragment_fast_path_raw_hits: 0,
                    optimized_fragment_fast_path_peephole_hits: 0,
                    quote_wrap_elapsed_ns: quote_wrap_elapsed_ns.load(Ordering::Relaxed) as u128,
                    fragment_assembly_elapsed_ns,
                    raw_rewrite_elapsed_ns: raw_debug_elapsed_ns,
                    peephole_elapsed_ns: 0,
                    source_map_remap_elapsed_ns: 0,
                    quoted_wrapped_functions: quoted_wrapped_functions.load(Ordering::Relaxed),
                    ..EmitBreakdownProfile::default()
                },
            },
        ));
    }
    let optimized_fragment_fast_path = cache.is_some()
        && optimized_fragment_variants_ready
        && cached_optimized_final_source_map.is_some()
        && cache.is_some_and(|c| {
            optimized_assembly_key.as_deref().is_some_and(|key| {
                c.has_optimized_assembly_safe(key).unwrap_or(false)
            })
        });
    let optimized_fragment_raw_fast_path = !optimized_fragment_fast_path
        && cache.is_some()
        && optimized_fragment_variants_ready
        && cached_optimized_final_source_map.is_some()
        && cache.is_some_and(|c| {
            optimized_assembly_key.as_deref().is_some_and(|key| {
                c.has_optimized_raw_assembly_safe(key).unwrap_or(false)
            })
        });
    let optimized_fragment_peephole_fast_path = !optimized_fragment_fast_path
        && !optimized_fragment_raw_fast_path
        && cache.is_some()
        && optimized_fragment_variants_ready
        && cached_optimized_final_source_map.is_some()
        && cache.is_some_and(|c| {
            optimized_assembly_key.as_deref().is_some_and(|key| {
                c.has_optimized_peephole_assembly_safe(key)
                    .unwrap_or(false)
            })
        });
    if optimized_fragment_fast_path {
        let fast_started = Instant::now();
        let raw_debug_elapsed_ns =
            maybe_emit_raw_debug_output(&final_output, &pure_user_calls, output_opts, cache)?;
        let (optimized_output, optimized_source_map) =
            assemble_emitted_fragments(&emitted_fragments, true);
        let final_source_map = cached_optimized_final_source_map
            .clone()
            .unwrap_or(optimized_source_map);
        let fragment_assembly_elapsed_ns =
            fragment_assembly_elapsed_ns + fast_started.elapsed().as_nanos();
        return Ok((
            optimized_output,
            final_source_map,
            cache_hits,
            cache_misses,
            EmitMetrics {
                elapsed_ns: step_emit.elapsed().as_nanos(),
                emitted_functions: emit_order.len(),
                cache_hits,
                cache_misses,
                breakdown: EmitBreakdownProfile {
                    fragment_build_elapsed_ns,
                    cache_load_elapsed_ns: cache_load_elapsed_ns.load(Ordering::Relaxed) as u128,
                    emitter_elapsed_ns: emitter_elapsed_ns.load(Ordering::Relaxed) as u128,
                    cache_store_elapsed_ns: cache_store_elapsed_ns.load(Ordering::Relaxed) as u128,
                    optimized_fragment_cache_hits,
                    optimized_fragment_cache_misses,
                    optimized_fragment_final_artifact_hits: 0,
                    optimized_fragment_fast_path_direct_hits: 1,
                    optimized_fragment_fast_path_raw_hits: 0,
                    optimized_fragment_fast_path_peephole_hits: 0,
                    quote_wrap_elapsed_ns: quote_wrap_elapsed_ns.load(Ordering::Relaxed) as u128,
                    fragment_assembly_elapsed_ns,
                    raw_rewrite_elapsed_ns: raw_debug_elapsed_ns,
                    peephole_elapsed_ns: 0,
                    source_map_remap_elapsed_ns: 0,
                    quoted_wrapped_functions: quoted_wrapped_functions.load(Ordering::Relaxed),
                    ..EmitBreakdownProfile::default()
                },
            },
        ));
    }
    if optimized_fragment_raw_fast_path {
        let fast_started = Instant::now();
        let raw_debug_elapsed_ns =
            maybe_emit_raw_debug_output(&final_output, &pure_user_calls, output_opts, cache)?;
        let (optimized_output, _) = assemble_emitted_fragments(&emitted_fragments, true);
        let final_source_map = cached_optimized_final_source_map.clone().unwrap();
        let optimized_output = apply_post_assembly_finalize_rewrites(apply_full_raw_rewrites(
            optimized_output,
            &pure_user_calls,
            output_opts,
        ));
        let fragment_assembly_elapsed_ns =
            fragment_assembly_elapsed_ns + fast_started.elapsed().as_nanos();
        return Ok((
            optimized_output,
            final_source_map,
            cache_hits,
            cache_misses,
            EmitMetrics {
                elapsed_ns: step_emit.elapsed().as_nanos(),
                emitted_functions: emit_order.len(),
                cache_hits,
                cache_misses,
                breakdown: EmitBreakdownProfile {
                    fragment_build_elapsed_ns,
                    cache_load_elapsed_ns: cache_load_elapsed_ns.load(Ordering::Relaxed) as u128,
                    emitter_elapsed_ns: emitter_elapsed_ns.load(Ordering::Relaxed) as u128,
                    cache_store_elapsed_ns: cache_store_elapsed_ns.load(Ordering::Relaxed) as u128,
                    optimized_fragment_cache_hits,
                    optimized_fragment_cache_misses,
                    optimized_fragment_final_artifact_hits: 0,
                    optimized_fragment_fast_path_direct_hits: 0,
                    optimized_fragment_fast_path_raw_hits: 1,
                    optimized_fragment_fast_path_peephole_hits: 0,
                    quote_wrap_elapsed_ns: quote_wrap_elapsed_ns.load(Ordering::Relaxed) as u128,
                    fragment_assembly_elapsed_ns,
                    raw_rewrite_elapsed_ns: fast_started.elapsed().as_nanos() + raw_debug_elapsed_ns,
                    peephole_elapsed_ns: 0,
                    source_map_remap_elapsed_ns: 0,
                    quoted_wrapped_functions: quoted_wrapped_functions.load(Ordering::Relaxed),
                    ..EmitBreakdownProfile::default()
                },
            },
        ));
    }
    if optimized_fragment_peephole_fast_path {
        let fast_started = Instant::now();
        let raw_debug_elapsed_ns =
            maybe_emit_raw_debug_output(&final_output, &pure_user_calls, output_opts, cache)?;
        let (optimized_output, optimized_source_map) =
            assemble_emitted_fragments(&emitted_fragments, true);
        let (optimized_output, optimized_source_map) = apply_full_peephole_to_output(
            &optimized_output,
            &optimized_source_map,
            direct_builtin_call_map,
            &pure_user_calls,
            &fresh_user_calls,
            output_opts,
        );
        let fragment_assembly_elapsed_ns =
            fragment_assembly_elapsed_ns + fast_started.elapsed().as_nanos();
        let final_source_map = cached_optimized_final_source_map
            .clone()
            .unwrap_or(optimized_source_map);
        return Ok((
            optimized_output,
            final_source_map,
            cache_hits,
            cache_misses,
            EmitMetrics {
                elapsed_ns: step_emit.elapsed().as_nanos(),
                emitted_functions: emit_order.len(),
                cache_hits,
                cache_misses,
                breakdown: EmitBreakdownProfile {
                    fragment_build_elapsed_ns,
                    cache_load_elapsed_ns: cache_load_elapsed_ns.load(Ordering::Relaxed) as u128,
                    emitter_elapsed_ns: emitter_elapsed_ns.load(Ordering::Relaxed) as u128,
                    cache_store_elapsed_ns: cache_store_elapsed_ns.load(Ordering::Relaxed) as u128,
                    optimized_fragment_cache_hits,
                    optimized_fragment_cache_misses,
                    optimized_fragment_final_artifact_hits: 0,
                    optimized_fragment_fast_path_direct_hits: 0,
                    optimized_fragment_fast_path_raw_hits: 0,
                    optimized_fragment_fast_path_peephole_hits: 1,
                    quote_wrap_elapsed_ns: quote_wrap_elapsed_ns.load(Ordering::Relaxed) as u128,
                    fragment_assembly_elapsed_ns,
                    raw_rewrite_elapsed_ns: raw_debug_elapsed_ns,
                    peephole_elapsed_ns: fast_started.elapsed().as_nanos(),
                    source_map_remap_elapsed_ns: 0,
                    quoted_wrapped_functions: quoted_wrapped_functions.load(Ordering::Relaxed),
                    ..EmitBreakdownProfile::default()
                },
            },
        ));
    }

    let skip_generated_poly_loop_rewrites = contains_generated_poly_loop_controls(&final_output);
    let raw_rewrite_cache_key = cache.map(|_| {
        crate::compiler::pipeline::raw_rewrite_output_cache_key(
            &final_output,
            &pure_user_calls,
            output_opts.preserve_all_defs,
            output_opts.compile_mode,
        )
    });
    let raw_rewrite_started = Instant::now();
    if !skip_generated_poly_loop_rewrites {
        if let (Some(cache), Some(cache_key)) = (cache, raw_rewrite_cache_key.as_deref()) {
            if let Some(cached_output) = cache.load_raw_rewrite(cache_key)? {
                final_output = cached_output;
            } else {
                final_output = apply_full_raw_rewrites(final_output, &pure_user_calls, output_opts);
                cache.store_raw_rewrite(cache_key, &final_output)?;
            }
        } else {
            final_output = apply_full_raw_rewrites(final_output, &pure_user_calls, output_opts);
        }
    }
    let raw_rewrite_elapsed_ns = raw_rewrite_started.elapsed().as_nanos();
    if let Some(path) = std::env::var_os("RR_DEBUG_RAW_R_PATH") {
        let _ = std::fs::write(path, &final_output);
    }
    let peephole_cache_key = cache.map(|_| {
        crate::compiler::pipeline::peephole_output_cache_key(
            &final_output,
            direct_builtin_call_map,
            &pure_user_calls,
            &fresh_user_calls,
            output_opts.preserve_all_defs,
            output_opts.compile_mode,
        )
    });
    let peephole_started = Instant::now();
    let ((final_output, line_map), peephole_profile) = if skip_generated_poly_loop_rewrites {
        let line_map = (1..=final_output.lines().count() as u32).collect::<Vec<_>>();
        (
            (final_output, line_map),
            crate::compiler::peephole::PeepholeProfile::default(),
        )
    } else if let (Some(cache), Some(cache_key)) = (cache, peephole_cache_key.as_deref()) {
        if let Some((cached_output, cached_line_map)) = cache.load_peephole(cache_key)? {
            (
                (cached_output, cached_line_map),
                crate::compiler::peephole::PeepholeProfile::default(),
            )
        } else {
            let ((optimized_output, line_map), profile) =
                crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_options_and_profile(
                    &final_output,
                    direct_builtin_call_map,
                    &pure_user_calls,
                    &fresh_user_calls,
                    output_opts.preserve_all_defs,
                    matches!(output_opts.compile_mode, CompileMode::FastDev),
                );
            cache.store_peephole(cache_key, &optimized_output, &line_map)?;
            ((optimized_output, line_map), profile)
        }
    } else {
        crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_options_and_profile(
            &final_output,
            direct_builtin_call_map,
            &pure_user_calls,
            &fresh_user_calls,
            output_opts.preserve_all_defs,
            matches!(output_opts.compile_mode, CompileMode::FastDev),
        )
    };
    let peephole_elapsed_ns = peephole_started.elapsed().as_nanos();
    let remap_started = Instant::now();
    let final_source_map = remap_source_map_lines(final_source_map, &line_map);
    let source_map_remap_elapsed_ns = remap_started.elapsed().as_nanos();

    if let Some(c) = cache {
        let (optimized_output, _optimized_source_map) =
            assemble_emitted_fragments(&emitted_fragments, true);
        let key = optimized_assembly_key.clone().unwrap_or_else(|| {
            crate::compiler::pipeline::optimized_assembly_cache_key(
                &assemble_emitted_fragments(&emitted_fragments, false).0,
                direct_builtin_call_map,
                &pure_user_calls,
                &fresh_user_calls,
                output_opts.preserve_all_defs,
                output_opts.compile_mode,
            )
        });
        c.store_optimized_assembly_artifact(&key, &final_output, &final_source_map)?;
        if optimized_output == final_output {
            c.store_optimized_assembly_source_map(&key, &final_source_map)?;
            c.store_optimized_assembly_safe(&key)?;
        } else {
            let optimized_raw_output = apply_post_assembly_finalize_rewrites(
                apply_full_raw_rewrites(optimized_output, &pure_user_calls, output_opts),
            );
            if optimized_raw_output == final_output {
                c.store_optimized_assembly_source_map(&key, &final_source_map)?;
                c.store_optimized_raw_assembly_safe(&key)?;
            } else {
                let (optimized_peephole_output, optimized_peephole_map) =
                    apply_full_peephole_to_output(
                        &assemble_emitted_fragments(&emitted_fragments, true).0,
                        &assemble_emitted_fragments(&emitted_fragments, true).1,
                        direct_builtin_call_map,
                        &pure_user_calls,
                        &fresh_user_calls,
                        output_opts,
                    );
                if optimized_peephole_output == final_output {
                    let _ = optimized_peephole_map;
                    c.store_optimized_assembly_source_map(&key, &final_source_map)?;
                    c.store_optimized_peephole_assembly_safe(&key)?;
                }
            }
        }
    }

    Ok((
        final_output,
        final_source_map,
        cache_hits,
        cache_misses,
        EmitMetrics {
            elapsed_ns: step_emit.elapsed().as_nanos(),
            emitted_functions: emit_order.len(),
            cache_hits,
            cache_misses,
            breakdown: EmitBreakdownProfile {
                fragment_build_elapsed_ns,
                cache_load_elapsed_ns: cache_load_elapsed_ns.load(Ordering::Relaxed) as u128,
                emitter_elapsed_ns: emitter_elapsed_ns.load(Ordering::Relaxed) as u128,
                cache_store_elapsed_ns: cache_store_elapsed_ns.load(Ordering::Relaxed) as u128,
                optimized_fragment_cache_hits,
                optimized_fragment_cache_misses,
                optimized_fragment_final_artifact_hits: 0,
                optimized_fragment_fast_path_direct_hits: 0,
                optimized_fragment_fast_path_raw_hits: 0,
                optimized_fragment_fast_path_peephole_hits: 0,
                quote_wrap_elapsed_ns: quote_wrap_elapsed_ns.load(Ordering::Relaxed) as u128,
                fragment_assembly_elapsed_ns,
                raw_rewrite_elapsed_ns,
                peephole_elapsed_ns,
                peephole_linear_scan_elapsed_ns: peephole_profile.linear_scan_elapsed_ns,
                peephole_primary_rewrite_elapsed_ns: peephole_profile.primary_rewrite_elapsed_ns,
                peephole_primary_flow_elapsed_ns: peephole_profile.primary_flow_elapsed_ns,
                peephole_primary_inline_elapsed_ns: peephole_profile.primary_inline_elapsed_ns,
                peephole_primary_reuse_elapsed_ns: peephole_profile.primary_reuse_elapsed_ns,
                peephole_primary_loop_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_cleanup_elapsed_ns,
                peephole_primary_loop_dead_zero_elapsed_ns: peephole_profile
                    .primary_loop_dead_zero_elapsed_ns,
                peephole_primary_loop_normalize_elapsed_ns: peephole_profile
                    .primary_loop_normalize_elapsed_ns,
                peephole_primary_loop_hoist_elapsed_ns: peephole_profile
                    .primary_loop_hoist_elapsed_ns,
                peephole_primary_loop_repeat_to_for_elapsed_ns: peephole_profile
                    .primary_loop_repeat_to_for_elapsed_ns,
                peephole_primary_loop_tail_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_tail_cleanup_elapsed_ns,
                peephole_primary_loop_guard_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_guard_cleanup_elapsed_ns,
                peephole_primary_loop_helper_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_helper_cleanup_elapsed_ns,
                peephole_primary_loop_exact_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_exact_cleanup_elapsed_ns,
                peephole_primary_loop_exact_pre_elapsed_ns: peephole_profile
                    .primary_loop_exact_pre_elapsed_ns,
                peephole_primary_loop_exact_reuse_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_elapsed_ns,
                peephole_primary_loop_exact_reuse_prepare_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_prepare_elapsed_ns,
                peephole_primary_loop_exact_reuse_forward_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_forward_elapsed_ns,
                peephole_primary_loop_exact_reuse_pure_call_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_pure_call_elapsed_ns,
                peephole_primary_loop_exact_reuse_expr_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_expr_elapsed_ns,
                peephole_primary_loop_exact_reuse_vector_alias_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_vector_alias_elapsed_ns,
                peephole_primary_loop_exact_reuse_rebind_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_rebind_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_prepare_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_prepare_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_forward_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_forward_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_pure_call_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_pure_call_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_expr_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_expr_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_rebind_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_rebind_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_rounds: peephole_profile
                    .primary_loop_exact_fixpoint_rounds,
                peephole_primary_loop_exact_finalize_elapsed_ns: peephole_profile
                    .primary_loop_exact_finalize_elapsed_ns,
                peephole_primary_loop_dead_temp_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_dead_temp_cleanup_elapsed_ns,
                peephole_secondary_rewrite_elapsed_ns: peephole_profile
                    .secondary_rewrite_elapsed_ns,
                peephole_secondary_inline_elapsed_ns: peephole_profile.secondary_inline_elapsed_ns,
                peephole_secondary_inline_branch_hoist_elapsed_ns: peephole_profile
                    .secondary_inline_branch_hoist_elapsed_ns,
                peephole_secondary_inline_immediate_scalar_elapsed_ns: peephole_profile
                    .secondary_inline_immediate_scalar_elapsed_ns,
                peephole_secondary_inline_named_index_elapsed_ns: peephole_profile
                    .secondary_inline_named_index_elapsed_ns,
                peephole_secondary_inline_named_expr_elapsed_ns: peephole_profile
                    .secondary_inline_named_expr_elapsed_ns,
                peephole_secondary_inline_scalar_region_elapsed_ns: peephole_profile
                    .secondary_inline_scalar_region_elapsed_ns,
                peephole_secondary_inline_immediate_index_elapsed_ns: peephole_profile
                    .secondary_inline_immediate_index_elapsed_ns,
                peephole_secondary_inline_adjacent_dedup_elapsed_ns: peephole_profile
                    .secondary_inline_adjacent_dedup_elapsed_ns,
                peephole_secondary_exact_elapsed_ns: peephole_profile.secondary_exact_elapsed_ns,
                peephole_secondary_helper_cleanup_elapsed_ns: peephole_profile
                    .secondary_helper_cleanup_elapsed_ns,
                peephole_secondary_helper_wrapper_elapsed_ns: peephole_profile
                    .secondary_helper_wrapper_elapsed_ns,
                peephole_secondary_helper_metric_elapsed_ns: peephole_profile
                    .secondary_helper_metric_elapsed_ns,
                peephole_secondary_helper_alias_elapsed_ns: peephole_profile
                    .secondary_helper_alias_elapsed_ns,
                peephole_secondary_helper_simple_expr_elapsed_ns: peephole_profile
                    .secondary_helper_simple_expr_elapsed_ns,
                peephole_secondary_helper_full_range_elapsed_ns: peephole_profile
                    .secondary_helper_full_range_elapsed_ns,
                peephole_secondary_helper_named_copy_elapsed_ns: peephole_profile
                    .secondary_helper_named_copy_elapsed_ns,
                peephole_secondary_finalize_cleanup_elapsed_ns: peephole_profile
                    .secondary_finalize_cleanup_elapsed_ns,
                peephole_secondary_finalize_bundle_elapsed_ns: peephole_profile
                    .secondary_finalize_bundle_elapsed_ns,
                peephole_secondary_finalize_dead_temp_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_elapsed_ns,
                peephole_secondary_finalize_dead_temp_facts_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_facts_elapsed_ns,
                peephole_secondary_finalize_dead_temp_mark_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_mark_elapsed_ns,
                peephole_secondary_finalize_dead_temp_reverse_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_reverse_elapsed_ns,
                peephole_secondary_finalize_dead_temp_compact_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_compact_elapsed_ns,
                peephole_finalize_elapsed_ns: peephole_profile.finalize_elapsed_ns,
                source_map_remap_elapsed_ns,
                quoted_wrapped_functions: quoted_wrapped_functions.load(Ordering::Relaxed),
            },
        },
    ))
}

pub(crate) fn run_mir_synthesis(
    ui: &CliLog,
    total_steps: usize,
    desugared_hir: crate::hir::def::HirProgram,
    global_symbols: &FxHashMap<crate::hir::def::SymbolId, String>,
    type_cfg: TypeConfig,
    scheduler: &CompilerScheduler,
) -> crate::error::RR<(ProgramIR, MirSynthesisMetrics)> {
    let step_ssa = ui.step_start(
        3,
        total_steps,
        "SSA Graph Synthesis",
        "build dominator tree & phi nodes",
    );
    let mut all_fns = FxHashMap::default();
    let mut emit_order: Vec<String> = Vec::new();
    let mut emit_roots: Vec<String> = Vec::new();
    let mut top_level_calls: Vec<String> = Vec::new();
    let mut known_fn_arities: FxHashMap<String, usize> = FxHashMap::default();
    let mut fn_jobs = Vec::new();
    let mut top_jobs = Vec::new();
    let mut meta_by_name: FxHashMap<String, (bool, bool)> = FxHashMap::default();

    for module in &desugared_hir.modules {
        for item in &module.items {
            if let crate::hir::def::HirItem::Fn(f) = item
                && let Some(name) = global_symbols.get(&f.name).cloned()
            {
                known_fn_arities.insert(name, f.params.len());
            }
        }
    }

    for module in desugared_hir.modules {
        let mut top_level_stmts: Vec<crate::hir::def::HirStmt> = Vec::new();

        for item in module.items {
            match item {
                crate::hir::def::HirItem::Fn(f) => {
                    let fn_name = format!("Sym_{}", f.name.0);
                    let is_public = f.public;
                    let mut params = Vec::with_capacity(f.params.len());
                    for p in &f.params {
                        let Some(name) = global_symbols.get(&p.name).cloned() else {
                            return Err(InternalCompilerError::new(
                                Stage::Mir,
                                format!(
                                    "missing parameter symbol during MIR lowering pipeline: {:?}",
                                    p.name
                                ),
                            )
                            .into_exception());
                        };
                        params.push(name);
                    }
                    let var_names = f.local_names.clone().into_iter().collect();

                    fn_jobs.push(MirLowerJob {
                        fn_name: fn_name.clone(),
                        is_public,
                        params,
                        var_names,
                        hir_fn: f,
                    });
                    meta_by_name.insert(fn_name.clone(), (is_public, false));
                    if is_public {
                        emit_roots.push(fn_name.clone());
                    }
                    emit_order.push(fn_name);
                }
                crate::hir::def::HirItem::Stmt(s) => top_level_stmts.push(s),
                _ => {}
            }
        }

        if !top_level_stmts.is_empty() {
            let top_fn_name = format!("Sym_top_{}", module.id.0);
            let top_fn = crate::hir::def::HirFn {
                id: crate::hir::def::FnId(1_000_000 + module.id.0),
                name: crate::hir::def::SymbolId(1_000_000 + module.id.0),
                params: Vec::new(),
                has_varargs: false,
                ret_ty: None,
                body: crate::hir::def::HirBlock {
                    stmts: top_level_stmts,
                    span: crate::utils::Span::default(),
                },
                attrs: crate::hir::def::HirFnAttrs {
                    inline_hint: crate::hir::def::InlineHint::Never,
                    tidy_safe: false,
                },
                span: crate::utils::Span::default(),
                local_names: FxHashMap::default(),
                public: false,
            };
            top_jobs.push(TopLevelMirLowerJob {
                fn_name: top_fn_name.clone(),
                hir_fn: top_fn,
            });
            meta_by_name.insert(top_fn_name.clone(), (false, true));
            emit_order.push(top_fn_name.clone());
            emit_roots.push(top_fn_name.clone());
            top_level_calls.push(top_fn_name);
        }
    }

    let total_hir_work = fn_jobs
        .iter()
        .map(|job| hir_fn_work_size(&job.hir_fn))
        .sum::<usize>()
        .saturating_add(
            top_jobs
                .iter()
                .map(|job| hir_fn_work_size(&job.hir_fn))
                .sum::<usize>(),
        );
    let lowered_fns = scheduler.map_try_stage(
        crate::compiler::scheduler::CompilerParallelStage::MirLowering,
        fn_jobs,
        total_hir_work,
        |job| {
            let lowerer = crate::mir::lower_hir::MirLowerer::new(
                job.fn_name.clone(),
                job.params,
                job.var_names,
                global_symbols,
                &known_fn_arities,
            );
            lowerer
                .lower_fn(job.hir_fn)
                .map(|fn_ir| (job.fn_name, job.is_public, fn_ir))
        },
    )?;
    for (fn_name, _is_public, fn_ir) in lowered_fns {
        all_fns.insert(fn_name, fn_ir);
    }
    let lowered_tops = scheduler.map_try_stage(
        crate::compiler::scheduler::CompilerParallelStage::MirLowering,
        top_jobs,
        total_hir_work,
        |job| {
            let lowerer = crate::mir::lower_hir::MirLowerer::new(
                job.fn_name.clone(),
                Vec::new(),
                FxHashMap::default(),
                global_symbols,
                &known_fn_arities,
            );
            lowerer
                .lower_fn(job.hir_fn)
                .map(|fn_ir| (job.fn_name, fn_ir))
        },
    )?;
    for (fn_name, fn_ir) in lowered_tops {
        all_fns.insert(fn_name, fn_ir);
    }
    ui.step_line_ok(&format!(
        "Synthesized {} MIR functions in {}",
        all_fns.len(),
        format_duration(step_ssa.elapsed())
    ));

    crate::typeck::solver::analyze_program_with_compiler_parallel(
        &mut all_fns,
        type_cfg,
        scheduler,
    )?;
    crate::mir::semantics::validate_program(&all_fns)?;
    crate::mir::semantics::validate_runtime_safety(&all_fns)?;

    let program = ProgramIR::from_parts(
        all_fns,
        emit_order,
        emit_roots,
        top_level_calls,
        meta_by_name,
    )?;
    let lowered_functions = program.fns.len();
    Ok((
        program,
        MirSynthesisMetrics {
            elapsed_ns: step_ssa.elapsed().as_nanos(),
            lowered_functions,
        },
    ))
}
