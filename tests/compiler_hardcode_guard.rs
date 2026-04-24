use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn rust_files_under(path: &Path) -> Vec<PathBuf> {
    let mut pending = vec![path.to_path_buf()];
    let mut files = Vec::new();

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("failed to read directory {}: {e}", dir.display()))
        {
            let entry = entry.unwrap_or_else(|e| panic!("failed to read directory entry: {e}"));
            let path = entry.path();
            let file_type = entry
                .file_type()
                .unwrap_or_else(|e| panic!("failed to read file type for {}: {e}", path.display()));
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                && path.file_name().and_then(|name| name.to_str()) != Some("tests.rs")
            {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

fn slash_relative(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or_else(|e| {
            panic!(
                "failed to strip {} from {}: {e}",
                base.display(),
                path.display()
            )
        })
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[test]
fn split_raw_rewrite_modules_are_part_of_compile_cache_salt() {
    let root = repo_root();
    let compiler_root = root.join("src").join("compiler");
    let codegen_emit_root = root.join("src").join("codegen").join("emit");
    let pipeline = read(&compiler_root.join("pipeline.rs"));

    let mut missing = Vec::new();

    let mut codegen_paths = vec![codegen_emit_root.join("rewrite.rs")];
    codegen_paths.extend(rust_files_under(&codegen_emit_root.join("rewrite")));
    for path in codegen_paths {
        let rel = slash_relative(&path, &codegen_emit_root);
        let needle = format!("../codegen/emit/{rel}");
        if !pipeline.contains(&needle) {
            missing.push(needle);
        }
    }

    let mut emitted_ir_paths = vec![compiler_root.join("peephole").join("emitted_ir.rs")];
    emitted_ir_paths.extend(rust_files_under(
        &compiler_root.join("peephole").join("emitted_ir"),
    ));
    for path in emitted_ir_paths {
        let needle = slash_relative(&path, &compiler_root);
        if !pipeline.contains(&needle) {
            missing.push(needle);
        }
    }

    assert!(
        missing.is_empty(),
        "split raw rewrite modules are missing from compiler cache salt: {:?}",
        missing
    );
}

#[test]
fn compiler_core_does_not_reintroduce_benchmark_named_rewrites() {
    let root = repo_root();
    let files = [
        root.join("src").join("compiler").join("pipeline.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("raw_rewrites.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("helper_raw_rewrites.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("scalar_raw_rewrites.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("cleanup_raw_rewrites.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("structural_raw_rewrites.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("raw_utils.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("function_props.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("compile_api.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("phases.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("phases")
            .join("source_emit.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("phases")
            .join("tachyon_runtime.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("loop_repairs.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("late_raw_rewrites.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("late_raw_rewrites")
            .join("buffer_swap.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("late_raw_rewrites")
            .join("cg.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("late_raw_rewrites")
            .join("clamp.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("late_raw_rewrites")
            .join("melt_rate.rs"),
        root.join("src")
            .join("compiler")
            .join("pipeline")
            .join("late_raw_rewrites")
            .join("prune.rs"),
        root.join("src").join("compiler").join("r_peephole.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("mod.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("patterns.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("alias.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("helpers.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("helpers")
            .join("cleanup.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("helpers")
            .join("helper_calls.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("helpers")
            .join("metric.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("core_utils.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("dead_code.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("expr_reuse.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("expr_reuse")
            .join("temp_tail.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("expr_reuse")
            .join("forward.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("facts.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("full_range.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("guard_simplify.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("inline_scalar.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("index_reads.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("late_pass.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("pipeline_impl.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("pipeline_stage.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("scalar_reuse.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("loop_restore.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("shadow_alias.rs"),
        root.join("src")
            .join("compiler")
            .join("peephole")
            .join("vector.rs"),
        root.join("src").join("mir").join("opt.rs"),
        root.join("src").join("mir").join("opt").join("helpers.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("analysis.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("api.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("planning.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("reconstruct.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("transform.rs"),
    ];
    let forbidden = [
        "RR_ENABLE_BENCH_REWRITES",
        "signal_pipeline",
        "heat_diffusion",
        "reaction_diffusion",
        "vector_fusion",
        "orbital_sweep",
        "bootstrap_resample",
        "morphogenesis",
        "tesseract",
    ];

    let mut hits = Vec::new();
    for path in files {
        let src = read(&path);
        for needle in forbidden {
            if src.contains(needle) {
                hits.push(format!("{} => {}", path.display(), needle));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "compiler/optimizer core unexpectedly contains benchmark-named logic again: {:?}",
        hits
    );
}
