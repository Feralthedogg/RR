use super::*;

pub(crate) fn cmd_mod(args: &[String]) -> i32 {
    let ui = CliLog::new();
    match args {
        [subcommand] if subcommand == "graph" => {
            let cwd = match env::current_dir() {
                Ok(path) => path,
                Err(e) => {
                    ui.error(&format!("Failed to determine current directory: {}", e));
                    return 1;
                }
            };
            let Some(project_root) = RR::pkg::find_manifest_root(&cwd) else {
                ui.error("RR mod graph requires an rr.mod manifest in the current directory or a parent directory");
                ui.warn("run RR init first, then retry RR mod graph from inside that project");
                return 1;
            };
            match RR::pkg::graph_project_dependencies(&project_root) {
                Ok(edges) => {
                    for (from, to) in edges {
                        println!("{from} {to}");
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, target] if subcommand == "why" => {
            let cwd = match env::current_dir() {
                Ok(path) => path,
                Err(e) => {
                    ui.error(&format!("Failed to determine current directory: {}", e));
                    return 1;
                }
            };
            let Some(project_root) = RR::pkg::find_manifest_root(&cwd) else {
                ui.error("RR mod why requires an rr.mod manifest in the current directory or a parent directory");
                ui.warn("run RR init first, then retry RR mod why from inside that project");
                return 1;
            };
            match RR::pkg::why_project_dependency(&project_root, target) {
                Ok(path) => {
                    for (idx, node) in path.iter().enumerate() {
                        if idx == 0 {
                            println!("{node}");
                        } else {
                            println!("-> {node}");
                        }
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "verify" => {
            let cwd = match env::current_dir() {
                Ok(path) => path,
                Err(e) => {
                    ui.error(&format!("Failed to determine current directory: {}", e));
                    return 1;
                }
            };
            let Some(project_root) = RR::pkg::find_manifest_root(&cwd) else {
                ui.error("RR mod verify requires an rr.mod manifest in the current directory or a parent directory");
                ui.warn("run RR init first, then retry RR mod verify from inside that project");
                return 1;
            };
            match RR::pkg::verify_project_dependencies(&project_root) {
                Ok(report) => {
                    if report.mismatches.is_empty() {
                        ui.success(&format!("Verified {} module(s)", report.checked));
                        return 0;
                    }
                    for mismatch in report.mismatches {
                        ui.error(&format!(
                            "{} expected={} actual={} root={}",
                            mismatch.path,
                            mismatch.expected_sum,
                            mismatch.actual_sum,
                            mismatch.source_root.display()
                        ));
                    }
                    1
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "tidy" => {
            let cwd = match env::current_dir() {
                Ok(path) => path,
                Err(e) => {
                    ui.error(&format!("Failed to determine current directory: {}", e));
                    return 1;
                }
            };
            let Some(project_root) = RR::pkg::find_manifest_root(&cwd) else {
                ui.error("RR mod tidy requires an rr.mod manifest in the current directory or a parent directory");
                ui.warn("run RR init first, then retry RR mod tidy from inside that project");
                return 1;
            };
            match RR::pkg::tidy_project(&project_root) {
                Ok((added, removed, total)) => {
                    ui.success(&format!(
                        "Tidied manifest: added {}, removed {}",
                        added, removed
                    ));
                    ui.success(&format!("Lock entries: {}", total));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "vendor" => {
            let cwd = match env::current_dir() {
                Ok(path) => path,
                Err(e) => {
                    ui.error(&format!("Failed to determine current directory: {}", e));
                    return 1;
                }
            };
            let Some(project_root) = RR::pkg::find_manifest_root(&cwd) else {
                ui.error("RR mod vendor requires an rr.mod manifest in the current directory or a parent directory");
                ui.warn("run RR init first, then retry RR mod vendor from inside that project");
                return 1;
            };
            match RR::pkg::vendor_project_dependencies(&project_root) {
                Ok(count) => {
                    ui.success(&format!("Vendored {} module(s)", count));
                    ui.success(&format!(
                        "Vendor dir: {}",
                        project_root.join("vendor").display()
                    ));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        _ => {
            ui.error("RR mod expects a supported subcommand");
            ui.warn("use RR mod graph, RR mod why, RR mod verify, RR mod tidy, or RR mod vendor");
            1
        }
    }
}

fn managed_entry_path(dir: &Path) -> PathBuf {
    dir.join("src").join("main.rr")
}

fn legacy_entry_path(dir: &Path) -> PathBuf {
    dir.join("main.rr")
}

fn resolve_project_entry_in_dir(dir: &Path) -> Option<PathBuf> {
    let managed_entry = managed_entry_path(dir);
    if managed_entry.is_file() {
        return Some(fs::canonicalize(&managed_entry).unwrap_or(managed_entry));
    }
    let legacy_entry = legacy_entry_path(dir);
    if legacy_entry.is_file() {
        return Some(fs::canonicalize(&legacy_entry).unwrap_or(legacy_entry));
    }
    None
}

fn file_name_is_main_rr(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("main.rr")
}

fn expr_contains_plain_main_call(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            matches!(&callee.kind, ExprKind::Name(name) if name == "main")
                || expr_contains_plain_main_call(callee)
                || args.iter().any(expr_contains_plain_main_call)
        }
        ExprKind::Unary { rhs, .. } => expr_contains_plain_main_call(rhs),
        ExprKind::Formula { lhs, rhs } => {
            lhs.as_deref().is_some_and(expr_contains_plain_main_call)
                || expr_contains_plain_main_call(rhs)
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            expr_contains_plain_main_call(lhs) || expr_contains_plain_main_call(rhs)
        }
        ExprKind::Range { a, b } => {
            expr_contains_plain_main_call(a) || expr_contains_plain_main_call(b)
        }
        ExprKind::Lambda { body, .. } => body.stmts.iter().any(stmt_contains_plain_main_call),
        ExprKind::NamedArg { value, .. } => expr_contains_plain_main_call(value),
        ExprKind::Index { base, idx } => {
            expr_contains_plain_main_call(base) || idx.iter().any(expr_contains_plain_main_call)
        }
        ExprKind::Field { base, .. } => expr_contains_plain_main_call(base),
        ExprKind::VectorLit(items) => items.iter().any(expr_contains_plain_main_call),
        ExprKind::RecordLit(items) => items
            .iter()
            .any(|(_, expr)| expr_contains_plain_main_call(expr)),
        ExprKind::Pipe { lhs, rhs_call } => {
            expr_contains_plain_main_call(lhs) || expr_contains_plain_main_call(rhs_call)
        }
        ExprKind::Try { expr } => expr_contains_plain_main_call(expr),
        ExprKind::Match { scrutinee, arms } => {
            expr_contains_plain_main_call(scrutinee)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_deref()
                        .is_some_and(expr_contains_plain_main_call)
                        || expr_contains_plain_main_call(&arm.body)
                })
        }
        ExprKind::Unquote(expr) => expr_contains_plain_main_call(expr),
        ExprKind::Lit(_) | ExprKind::Name(_) | ExprKind::ColRef(_) | ExprKind::Column(_) => false,
    }
}

fn stmt_contains_plain_main_call(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Let { init, .. } => init.as_ref().is_some_and(expr_contains_plain_main_call),
        StmtKind::Assign { value, .. } => expr_contains_plain_main_call(value),
        StmtKind::FnDecl { .. } | StmtKind::Export(_) | StmtKind::Import { .. } => false,
        StmtKind::If {
            cond,
            then_blk,
            else_blk,
        } => {
            expr_contains_plain_main_call(cond)
                || then_blk.stmts.iter().any(stmt_contains_plain_main_call)
                || else_blk
                    .as_ref()
                    .is_some_and(|blk| blk.stmts.iter().any(stmt_contains_plain_main_call))
        }
        StmtKind::While { cond, body } => {
            expr_contains_plain_main_call(cond)
                || body.stmts.iter().any(stmt_contains_plain_main_call)
        }
        StmtKind::For { iter, body, .. } => {
            expr_contains_plain_main_call(iter)
                || body.stmts.iter().any(stmt_contains_plain_main_call)
        }
        StmtKind::Return { value } => value.as_ref().is_some_and(expr_contains_plain_main_call),
        StmtKind::ExprStmt { expr } | StmtKind::Expr(expr) => expr_contains_plain_main_call(expr),
        StmtKind::Break | StmtKind::Next => false,
    }
}

fn source_defines_main_function(source: &str) -> Result<(bool, bool), RRException> {
    let mut parser = Parser::new(source);
    let program = parser.parse_program()?;
    let has_main_fn = program.stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::FnDecl { name, .. } => name == "main",
        StmtKind::Export(fndecl) => fndecl.name == "main",
        _ => false,
    });
    let has_top_level_main_call = program.stmts.iter().any(stmt_contains_plain_main_call);
    Ok((has_main_fn, has_top_level_main_call))
}

fn prepare_project_entry_source(
    input_path: &Path,
    source: &str,
    command: &str,
) -> Result<String, RRException> {
    let (has_main_fn, has_top_level_main_call) = source_defines_main_function(source)?;
    if !has_main_fn {
        return Err(RRException::new(
            "RR.SemanticError",
            RRCode::E1001,
            Stage::Parse,
            format!(
                "project entry '{}' must define fn main()",
                input_path.display()
            ),
        )
        .help(format!(
            "add `fn main() {{ ... }}` to the entry file before running `RR {}`",
            command
        )));
    }
    if has_top_level_main_call {
        return Ok(source.to_string());
    }

    let mut patched = source.to_string();
    if !patched.ends_with('\n') {
        patched.push('\n');
    }
    patched.push_str("\nmain()\n");
    Ok(patched)
}

fn resolve_command_input(raw: &str, command: &str) -> Result<PathBuf, TargetResolutionError> {
    let path = PathBuf::from(raw);
    if path.is_dir() || raw == "." {
        if let Some(entry) = resolve_project_entry_in_dir(&path) {
            Ok(entry)
        } else {
            Err(TargetResolutionError {
                message: format!(
                    "src/main.rr or main.rr not found in '{}'",
                    path.to_string_lossy()
                ),
                help: Some(format!(
                    "add src/main.rr for a managed project, keep a legacy main.rr, or run RR {command} with an explicit .rr file path"
                )),
            })
        }
    } else if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            Ok(fs::canonicalize(&path).unwrap_or(path))
        } else {
            Err(TargetResolutionError {
                message: format!("{command} target must be a .rr file or directory"),
                help: Some(format!(
                    "pass a .rr file directly, or point RR {command} at a directory containing src/main.rr or main.rr"
                )),
            })
        }
    } else {
        Err(TargetResolutionError {
            message: format!("{command} target not found: '{}'", raw),
            help: Some(format!(
                "use RR {command} . inside a project directory, or pass an existing .rr file path"
            )),
        })
    }
}

pub(crate) fn cmd_run(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Run, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let input_path = match resolve_command_input(&opts.target, "run") {
        Ok(p) => p,
        Err(err) => {
            ui.error(&err.message);
            if let Some(help) = err.help {
                ui.warn(&help);
            }
            return 1;
        }
    };
    let input_path_str = input_path.to_string_lossy().to_string();
    let raw_input = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            report_path_read_failure(&ui, &input_path, &e, "run input");
            return 1;
        }
    };
    let input = if file_name_is_main_rr(&input_path) {
        match prepare_project_entry_source(&input_path, &raw_input, "run") {
            Ok(source) => source,
            Err(err) => {
                err.display(Some(&raw_input), Some(&input_path_str));
                return 1;
            }
        }
    } else {
        raw_input
    };

    let output_opts = CompileOutputOptions {
        inject_runtime: true,
        preserve_all_defs: opts.preserve_all_defs,
        strict_let: opts.strict_let,
        warn_implicit_decl: opts.warn_implicit_decl,
        compile_mode: opts.compile_mode,
    };
    let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
    let mut session = IncrementalSession::default();
    let result = super::with_compile_cache_override(opts.cold_compile, || {
        if opts.incremental.enabled {
            compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile(
                &input_path_str,
                &input,
                opts.opt_level,
                opts.type_cfg,
                opts.parallel_cfg,
                opts.compiler_parallel_cfg,
                opts.incremental,
                output_opts,
                Some(&mut session),
                compile_profile.as_mut(),
            )
            .map(|v| (v.r_code, v.source_map))
        } else {
            compile_with_configs_with_options_and_compiler_parallel_and_profile(
                &input_path_str,
                &input,
                opts.opt_level,
                opts.type_cfg,
                opts.parallel_cfg,
                opts.compiler_parallel_cfg,
                output_opts,
                compile_profile.as_mut(),
            )
        }
    });

    match result {
        Ok((r_code, source_map)) => {
            if let Some(profile) = compile_profile.as_ref()
                && let Err(code) = write_compile_profile_artifact(
                    &ui,
                    profile,
                    opts.profile_compile_out.as_deref(),
                )
            {
                return code;
            }
            Runner::run(
                &input_path_str,
                &input,
                &r_code,
                &source_map,
                None,
                opts.keep_r,
            )
        }
        Err(e) => {
            e.display(Some(&input), Some(&input_path_str));
            1
        }
    }
}

fn collect_rr_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "Build"
                || name == "build"
                || name == "target"
                || name == ".git"
                || name == "vendor"
            {
                continue;
            }
            collect_rr_files(&path, files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            files.push(path);
        }
    }
    Ok(())
}

pub(crate) fn cmd_build(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Build, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let target = opts.target;
    let target_path = PathBuf::from(&target);
    if !target_path.exists() {
        ui.error(&format!("build target not found: '{}'", target));
        ui.warn("pass an existing directory or .rr file; use --out-dir to choose where emitted R files go");
        return 1;
    }

    let out_dir = opts.output_path.unwrap_or_else(|| {
        default_build_output_dir(&target_path)
            .to_string_lossy()
            .to_string()
    });

    let out_root = PathBuf::from(&out_dir);
    if let Err(e) = fs::create_dir_all(&out_root) {
        report_dir_create_failure(&ui, &out_root, &e, "build --out-dir destination");
        return 1;
    }
    println!("{} {}", ui.yellow_bold("[+]"), ui.red_bold("RR Build"));
    println!(
        " {} {}",
        ui.dim("|-"),
        ui.white_bold(&format!(
            "Target: {} | Out: {} ({})",
            target,
            out_dir,
            opts.opt_level.label()
        ))
    );

    let mut rr_files = Vec::new();
    let dir_mode = target_path.is_dir();
    let project_entry = if dir_mode {
        resolve_project_entry_in_dir(&target_path)
    } else {
        None
    };
    if let Some(entry) = &project_entry {
        rr_files.push(entry.clone());
    } else if dir_mode {
        if let Err(e) = collect_rr_files(&target_path, &mut rr_files) {
            ui.error(&format!("Failed while scanning '{}': {}", target, e));
            ui.warn(
                "make sure the build target directory is readable, and that RR can descend into its source tree",
            );
            return 1;
        }
    } else if target_path.extension().and_then(|s| s.to_str()) == Some("rr") {
        rr_files.push(target_path.clone());
    } else {
        ui.error("build target must be a directory or .rr file");
        ui.warn("use RR build <dir> to compile a project tree, or RR build path/to/file.rr for a single file");
        return 1;
    }

    rr_files.sort();
    if rr_files.is_empty() {
        ui.error(&format!("no .rr files found under '{}'", target));
        ui.warn(
            "add at least one .rr source file under that directory, or point RR build at a specific .rr file instead",
        );
        return 1;
    }

    let root_abs = if dir_mode {
        fs::canonicalize(&target_path).ok()
    } else {
        None
    };

    let mut built = 0usize;
    let mut compile_profiles: Vec<(String, CompileProfile)> = Vec::new();
    for rr in rr_files {
        let rr_abs = fs::canonicalize(&rr).unwrap_or(rr.clone());
        let rr_path_str = rr_abs.to_string_lossy().to_string();
        let raw_input = match fs::read_to_string(&rr_abs) {
            Ok(s) => s,
            Err(e) => {
                report_path_read_failure(&ui, &rr_abs, &e, "build input");
                return 1;
            }
        };
        let input = if file_name_is_main_rr(&rr_abs) && project_entry.as_ref() == Some(&rr_abs) {
            match prepare_project_entry_source(&rr_abs, &raw_input, "build") {
                Ok(source) => source,
                Err(err) => {
                    err.display(Some(&raw_input), Some(&rr_path_str));
                    return 1;
                }
            }
        } else {
            raw_input
        };

        let output_opts = CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: opts.preserve_all_defs,
            strict_let: opts.strict_let,
            warn_implicit_decl: opts.warn_implicit_decl,
            compile_mode: opts.compile_mode,
        };
        let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
        let build_out = super::with_compile_cache_override(opts.cold_compile, || {
            if opts.incremental.enabled {
                compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile(
                    &rr_path_str,
                    &input,
                    opts.opt_level,
                    opts.type_cfg,
                    opts.parallel_cfg,
                    opts.compiler_parallel_cfg,
                    opts.incremental,
                    output_opts,
                    None,
                    compile_profile.as_mut(),
                )
                .map(|v| (v.r_code, v.source_map))
            } else {
                compile_with_configs_with_options_and_compiler_parallel_and_profile(
                    &rr_path_str,
                    &input,
                    opts.opt_level,
                    opts.type_cfg,
                    opts.parallel_cfg,
                    opts.compiler_parallel_cfg,
                    output_opts,
                    compile_profile.as_mut(),
                )
            }
        });

        let (r_code, _source_map) = match build_out {
            Ok(v) => v,
            Err(e) => {
                e.display(Some(&input), Some(&rr_path_str));
                return 1;
            }
        };

        let out_file = if dir_mode {
            let rel = rr
                .strip_prefix(&target_path)
                .ok()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .or_else(|| {
                    root_abs.as_ref().and_then(|root| {
                        rr_abs
                            .strip_prefix(root)
                            .ok()
                            .filter(|p| !p.as_os_str().is_empty())
                            .map(Path::to_path_buf)
                    })
                })
                .or_else(|| rr.file_name().map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("out.rr"));
            out_root.join(rel).with_extension("R")
        } else {
            let stem = rr.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
            out_root.join(format!("{}.R", stem))
        };

        if let Some(parent) = out_file.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            report_dir_create_failure(&ui, parent, &e, "build output directory");
            return 1;
        }
        if let Err(e) = fs::write(&out_file, r_code) {
            report_file_write_failure(&ui, &out_file, &e, "build output path");
            return 1;
        }

        ui.success(&format!("Built {} -> {}", rr.display(), out_file.display()));
        built += 1;
        if let Some(profile) = compile_profile.take() {
            compile_profiles.push((rr_path_str, profile));
        }
    }

    ui.success(&format!(
        "Build complete: {} file(s) -> {}",
        built,
        out_root.display()
    ));
    if !compile_profiles.is_empty()
        && let Err(code) = if compile_profiles.len() == 1 {
            write_compile_profile_artifact(
                &ui,
                &compile_profiles[0].1,
                opts.profile_compile_out.as_deref(),
            )
        } else {
            write_compile_profile_collection(
                &ui,
                &compile_profiles,
                opts.profile_compile_out.as_deref(),
            )
        }
    {
        return code;
    }
    0
}

pub(crate) fn cmd_watch(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut opts = match parse_command_opts(args, CommandMode::Watch, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };

    if opts.incremental.enabled && !opts.incremental.auto {
        opts.incremental.phase3 = true;
    }

    let input_path = match resolve_command_input(&opts.target, "watch") {
        Ok(p) => p,
        Err(err) => {
            ui.error(&err.message);
            if let Some(help) = err.help {
                ui.warn(&help);
            }
            return 1;
        }
    };
    let input_path_str = input_path.to_string_lossy().to_string();
    let out_file = if let Some(out) = opts.output_path.clone() {
        PathBuf::from(out)
    } else {
        default_watch_output_file(&input_path)
    };
    if let Some(parent) = out_file.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        report_dir_create_failure(&ui, parent, &e, "watch output directory");
        return 1;
    }

    ui.success(&format!(
        "Watching {} (poll={}ms)",
        input_path.display(),
        opts.watch_poll_ms
    ));

    let mut session = IncrementalSession::default();
    let mut last_successful_snapshot: Option<Vec<(PathBuf, u64)>> = None;
    let mut last_successful_fingerprint: Option<u64> = None;
    let mut last_successful_output_hash: Option<u64> = None;
    let mut last_announced_fingerprint: Option<u64> = None;
    let mut reported_idle_wait = false;
    loop {
        let raw_input = match fs::read_to_string(&input_path) {
            Ok(s) => s,
            Err(e) => {
                report_path_read_failure(&ui, &input_path, &e, "watch input");
                return 1;
            }
        };
        let input = if file_name_is_main_rr(&input_path) {
            match prepare_project_entry_source(&input_path, &raw_input, "watch") {
                Ok(source) => source,
                Err(err) => {
                    err.display(Some(&raw_input), Some(&input_path_str));
                    if opts.watch_once {
                        return 1;
                    }
                    thread::sleep(Duration::from_millis(opts.watch_poll_ms));
                    continue;
                }
            }
        } else {
            raw_input
        };
        let snapshot = match module_tree_snapshot(&input_path_str, &input) {
            Ok(snapshot) => snapshot,
            Err(e) => {
                e.display(Some(&input), Some(&input_path_str));
                if opts.watch_once {
                    return 1;
                }
                thread::sleep(Duration::from_millis(opts.watch_poll_ms));
                continue;
            }
        };
        let fingerprint = match module_tree_fingerprint(&input_path_str, &input) {
            Ok(fp) => fp,
            Err(e) => {
                e.display(Some(&input), Some(&input_path_str));
                if opts.watch_once {
                    return 1;
                }
                thread::sleep(Duration::from_millis(opts.watch_poll_ms));
                continue;
            }
        };
        let output_current = watch_output_matches_hash(&out_file, last_successful_output_hash);
        if last_successful_fingerprint == Some(fingerprint) && output_current {
            if opts.watch_once {
                return 0;
            }
            if !reported_idle_wait {
                ui.success("unchanged module tree; waiting for changes");
                reported_idle_wait = true;
            }
            thread::sleep(Duration::from_millis(opts.watch_poll_ms));
            continue;
        }
        reported_idle_wait = false;
        if last_announced_fingerprint != Some(fingerprint) {
            if let Some(prev) = &last_successful_snapshot
                && let Some(summary) = summarize_watch_changes(prev, &snapshot)
            {
                ui.success(&format!("change detected in {summary}"));
            }
            last_announced_fingerprint = Some(fingerprint);
        }
        if last_successful_fingerprint == Some(fingerprint)
            && last_successful_output_hash.is_some()
            && !output_current
        {
            ui.success("watch output missing or changed; restoring");
        }

        let output_opts = CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: opts.preserve_all_defs,
            strict_let: opts.strict_let,
            warn_implicit_decl: opts.warn_implicit_decl,
            compile_mode: opts.compile_mode,
        };
        let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
        let watch_result = super::with_compile_cache_override(opts.cold_compile, || {
            if opts.incremental.enabled {
                compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile(
                    &input_path_str,
                    &input,
                    opts.opt_level,
                    opts.type_cfg,
                    opts.parallel_cfg,
                    opts.compiler_parallel_cfg,
                    opts.incremental,
                    output_opts,
                    Some(&mut session),
                    compile_profile.as_mut(),
                )
            } else {
                compile_with_configs_with_options_and_compiler_parallel_and_profile(
                    &input_path_str,
                    &input,
                    opts.opt_level,
                    opts.type_cfg,
                    opts.parallel_cfg,
                    opts.compiler_parallel_cfg,
                    output_opts,
                    compile_profile.as_mut(),
                )
                .map(|(r_code, source_map)| IncrementalCompileOutput {
                    r_code,
                    source_map,
                    stats: IncrementalStats::default(),
                })
            }
        });

        match watch_result {
            Ok(out) => {
                let output_hash = watch_output_hash(&out.r_code);
                if let Err(e) = fs::write(&out_file, out.r_code.as_bytes()) {
                    report_file_write_failure(&ui, &out_file, &e, "watch output path");
                    return 1;
                }
                last_successful_snapshot = Some(snapshot);
                last_successful_fingerprint = Some(fingerprint);
                last_successful_output_hash = Some(output_hash);
                if out.stats.phase3_memory_hit || out.stats.phase1_artifact_hit {
                    ui.success(&format!(
                        "cache hit (phase1_hit={}, phase3_hit={}) -> {}",
                        out.stats.phase1_artifact_hit,
                        out.stats.phase3_memory_hit,
                        out_file.display()
                    ));
                } else {
                    ui.success(&format!(
                        "rebuilt (phase2 hits={}, misses={}{}) -> {}",
                        out.stats.phase2_emit_hits,
                        out.stats.phase2_emit_misses,
                        if out.stats.miss_reasons.is_empty() {
                            String::new()
                        } else {
                            format!(", reasons={}", out.stats.miss_reasons.join(","))
                        },
                        out_file.display()
                    ));
                }
                if let Some(profile) = compile_profile.as_ref()
                    && let Err(code) = write_compile_profile_artifact(
                        &ui,
                        profile,
                        opts.profile_compile_out.as_deref(),
                    )
                {
                    return code;
                }
            }
            Err(e) => {
                e.display(Some(&input), Some(&input_path_str));
                if opts.watch_once {
                    return 1;
                }
            }
        }

        if opts.watch_once {
            return 0;
        }
        thread::sleep(Duration::from_millis(opts.watch_poll_ms));
    }
}

fn summarize_watch_changes(prev: &[(PathBuf, u64)], next: &[(PathBuf, u64)]) -> Option<String> {
    let prev_map: FxHashMap<&PathBuf, u64> =
        prev.iter().map(|(path, hash)| (path, *hash)).collect();
    let next_map: FxHashMap<&PathBuf, u64> =
        next.iter().map(|(path, hash)| (path, *hash)).collect();

    let mut changed = Vec::new();
    for (path, hash) in &next_map {
        match prev_map.get(path) {
            Some(prev_hash) if prev_hash == hash => {}
            Some(_) | None => changed.push(display_watch_path(path)),
        }
    }
    for path in prev_map.keys() {
        if !next_map.contains_key(path) {
            changed.push(display_watch_path(path));
        }
    }

    changed.sort();
    changed.dedup();
    if changed.is_empty() {
        return None;
    }
    let first = changed[0].clone();
    if changed.len() == 1 {
        Some(first)
    } else {
        Some(format!("{first} (+{} more)", changed.len() - 1))
    }
}

fn display_watch_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{display_watch_path, summarize_watch_changes};
    use std::path::PathBuf;

    #[test]
    fn summarize_watch_changes_reports_single_changed_module() {
        let prev = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/module.rr"), 2_u64),
        ];
        let next = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/module.rr"), 3_u64),
        ];
        assert_eq!(
            summarize_watch_changes(&prev, &next),
            Some("module.rr".to_string())
        );
    }

    #[test]
    fn summarize_watch_changes_reports_added_and_removed_modules_compactly() {
        let prev = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/old.rr"), 2_u64),
        ];
        let next = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/new.rr"), 9_u64),
        ];
        assert_eq!(
            summarize_watch_changes(&prev, &next),
            Some("new.rr (+1 more)".to_string())
        );
    }

    #[test]
    fn display_watch_path_prefers_file_name() {
        assert_eq!(
            display_watch_path(&PathBuf::from("/tmp/sub/module.rr")),
            "module.rr".to_string()
        );
    }
}
