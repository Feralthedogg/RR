use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn compile_cache_salt_uses_build_hash_instead_of_manual_include_list() {
    let root = repo_root();
    let build_script = read(&root.join("build.rs"));
    let source_fingerprint = read(
        &root
            .join("src")
            .join("compiler")
            .join("pipeline")
            .join("source_fingerprint.rs"),
    );
    let cache_and_ir = read(
        &root
            .join("src")
            .join("compiler")
            .join("pipeline")
            .join("cache_and_ir.rs"),
    );

    assert!(
        build_script.contains("collect_rs_files(&manifest_dir.join(\"src\")"),
        "build.rs must keep recursively hashing src/**/*.rs for compiler cache invalidation"
    );
    assert!(
        build_script.contains("cargo:rustc-env=RR_COMPILER_BUILD_HASH="),
        "build.rs must export the compiler source fingerprint"
    );
    assert!(
        source_fingerprint.contains("RR_COMPILER_BUILD_HASH"),
        "function emit cache salt must use the build-level compiler source fingerprint"
    );
    assert!(
        cache_and_ir.contains("crate::runtime::R_RUNTIME"),
        "output cache salt must still include the generated R runtime contents"
    );
    assert!(
        !source_fingerprint.contains("include_str!(") && !cache_and_ir.contains("include_str!("),
        "cache salts must not manually enumerate compiler source files with include_str!"
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
