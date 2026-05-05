mod common;

use common::unique_dir;
use rr::compiler::internal::codegen::mir_emit::MapEntry;
use rr::compiler::{
    CompileMode, CompileOutputOptions, CompileProfile, CompileWithProfileRequest,
    CompilerParallelConfig, CompilerParallelMode, OptLevel, compile_with_profile_request,
    default_parallel_config, default_type_config,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::MutexGuard;

fn compile_with_profile(
    env_guard: &MutexGuard<'static, ()>,
    entry_path: &str,
    source: &str,
    compiler_parallel_cfg: CompilerParallelConfig,
    cache_dir: &std::path::Path,
    output_opts: CompileOutputOptions,
) -> (String, Vec<MapEntry>, CompileProfile) {
    common::set_env_var_for_test(env_guard, "RR_INCREMENTAL_CACHE_DIR", cache_dir);
    let mut profile = CompileProfile::default();
    let (code, map) = compile_with_profile_request(CompileWithProfileRequest {
        entry_path,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg,
        output_opts,
        profile: Some(&mut profile),
    })
    .expect("compile failed");
    common::remove_env_var_for_test(env_guard, "RR_INCREMENTAL_CACHE_DIR");
    (code, map, profile)
}

fn output_opts(compile_mode: CompileMode) -> CompileOutputOptions {
    CompileOutputOptions {
        compile_mode,
        ..CompileOutputOptions::default()
    }
}

fn profile_stage<'a>(
    profile: &'a CompileProfile,
    stage: &str,
) -> &'a rr::compiler::CompilerParallelStageProfile {
    profile
        .compiler_parallel
        .stage(stage)
        .unwrap_or_else(|| panic!("missing compiler parallel stage profile for {stage}"))
}

fn artifact_hash(code: &str, map: &[MapEntry]) -> u64 {
    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    format!("{map:?}").hash(&mut hasher);
    hasher.finish()
}

fn auto_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::Auto,
        threads: 4,
        min_functions: 2,
        min_fn_ir: 1,
        max_jobs: 4,
    }
}

fn on_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 4,
        min_functions: usize::MAX,
        min_fn_ir: usize::MAX,
        max_jobs: 4,
    }
}

fn off_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::Off,
        threads: 4,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 4,
    }
}

fn large_parallel_fixture() -> String {
    let mut source = String::new();
    for idx in 0..8 {
        source.push_str(&format!(
            "fn helper_{idx}(x) {{\n  let a = x + {idx}.0\n  let b = a * (x + 1.0)\n  let c = b - {idx}.0\n  let d = c / (x + 2.0)\n  return d + a + b\n}}\n\n"
        ));
    }
    source.push_str(
        "fn main() {\n  let out = numeric(8L)\n  let i = 1L\n  while (i <= 8L) {\n    out[i] = helper_0(i) + helper_1(i) + helper_2(i) + helper_3(i) + helper_4(i) + helper_5(i) + helper_6(i) + helper_7(i)\n    i = i + 1L\n  }\n  print(out)\n}\nmain()\n",
    );
    source
}

fn large_inline_parallel_fixture() -> String {
    let mut source = String::new();
    for idx in 0..8 {
        source.push_str(&format!(
            "fn helper{idx}(x) {{\n  let y = x + {}L\n  if x > 0L {{\n    return y\n  }} else {{\n    return y + 1L\n  }}\n}}\n",
            idx + 1
        ));
    }
    source.push_str(
        r#"
fn fused(x) {
  let a0 = helper0(x)
  let a1 = helper1(x)
  let a2 = helper2(x)
  let a3 = helper3(x)
  let a4 = helper4(x)
  let a5 = helper5(x)
  let a6 = helper6(x)
  let a7 = helper7(x)
  return a0 + a1 + a2 + a3 + a4 + a5 + a6 + a7
}

fn main() {
  let out = numeric(8L)
  let i = 1L
  while (i <= 8L) {
    out[i] = fused(i)
    i = i + 1L
  }
  print(out)
}
main()
"#,
    );
    source
}

#[test]
fn compiler_parallel_off_auto_on_emit_identical_artifacts_and_hashes() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compiler_parallel_determinism");
    std::fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "profile_cache");
    std::fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let source = large_parallel_fixture();
    let entry_path = "compiler_parallel_determinism.rr";

    let (off_code, off_map, off_profile) = compile_with_profile(
        &env_guard,
        entry_path,
        &source,
        off_parallel_cfg(),
        &proj_dir.join("off-cache"),
        output_opts(CompileMode::Standard),
    );
    let (auto_code, auto_map, auto_profile) = compile_with_profile(
        &env_guard,
        entry_path,
        &source,
        auto_parallel_cfg(),
        &proj_dir.join("auto-cache"),
        output_opts(CompileMode::Standard),
    );
    let (on_code, on_map, on_profile) = compile_with_profile(
        &env_guard,
        entry_path,
        &source,
        on_parallel_cfg(),
        &proj_dir.join("on-cache"),
        output_opts(CompileMode::Standard),
    );

    assert_eq!(off_code, auto_code);
    assert_eq!(off_code, on_code);
    assert_eq!(off_map, auto_map);
    assert_eq!(off_map, on_map);
    assert_eq!(
        artifact_hash(&off_code, &off_map),
        artifact_hash(&auto_code, &auto_map)
    );
    assert_eq!(
        artifact_hash(&off_code, &off_map),
        artifact_hash(&on_code, &on_map)
    );

    let off_emit = profile_stage(&off_profile, "emit");
    let off_type_analysis = profile_stage(&off_profile, "type_analysis");
    let off_always = profile_stage(&off_profile, "tachyon_always");
    let off_heavy = profile_stage(&off_profile, "tachyon_heavy");
    let off_fresh_alias = profile_stage(&off_profile, "tachyon_fresh_alias");
    let off_de_ssa = profile_stage(&off_profile, "tachyon_de_ssa");
    assert_eq!(off_emit.parallel_invocations, 0);
    assert_eq!(off_type_analysis.parallel_invocations, 0);
    assert_eq!(off_always.parallel_invocations, 0);
    assert_eq!(off_heavy.parallel_invocations, 0);
    assert_eq!(off_fresh_alias.parallel_invocations, 0);
    assert_eq!(off_de_ssa.parallel_invocations, 0);
    assert!(off_emit.reason_counts.get("mode_off").copied().unwrap_or(0) > 0);

    let auto_lower = profile_stage(&auto_profile, "mir_lowering");
    let auto_type_analysis = profile_stage(&auto_profile, "type_analysis");
    let auto_always = profile_stage(&auto_profile, "tachyon_always");
    let auto_heavy = profile_stage(&auto_profile, "tachyon_heavy");
    let auto_fresh_alias = profile_stage(&auto_profile, "tachyon_fresh_alias");
    let auto_de_ssa = profile_stage(&auto_profile, "tachyon_de_ssa");
    let auto_emit = profile_stage(&auto_profile, "emit");
    assert!(auto_lower.parallel_invocations > 0);
    assert!(auto_type_analysis.parallel_invocations > 0);
    assert!(auto_always.parallel_invocations > 0);
    assert!(auto_heavy.parallel_invocations > 0);
    assert!(auto_fresh_alias.parallel_invocations > 0);
    assert!(auto_de_ssa.parallel_invocations > 0);
    assert!(auto_emit.parallel_invocations > 0);

    let on_lower = profile_stage(&on_profile, "mir_lowering");
    let on_type_analysis = profile_stage(&on_profile, "type_analysis");
    let on_always = profile_stage(&on_profile, "tachyon_always");
    let on_heavy = profile_stage(&on_profile, "tachyon_heavy");
    let on_fresh_alias = profile_stage(&on_profile, "tachyon_fresh_alias");
    let on_de_ssa = profile_stage(&on_profile, "tachyon_de_ssa");
    let on_emit = profile_stage(&on_profile, "emit");
    assert!(on_lower.parallel_invocations > 0);
    assert!(on_type_analysis.parallel_invocations > 0);
    assert!(on_always.parallel_invocations > 0);
    assert!(on_heavy.parallel_invocations > 0);
    assert!(on_fresh_alias.parallel_invocations > 0);
    assert!(on_de_ssa.parallel_invocations > 0);
    assert!(on_emit.parallel_invocations > 0);
}

#[test]
fn compiler_parallel_off_auto_on_inline_cleanup_identical_artifacts_and_hashes() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compiler_parallel_determinism");
    std::fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "inline_cleanup_profile_cache");
    std::fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let source = large_inline_parallel_fixture();
    let entry_path = "compiler_parallel_inline_cleanup_determinism.rr";

    let (off_code, off_map, off_profile) = compile_with_profile(
        &env_guard,
        entry_path,
        &source,
        off_parallel_cfg(),
        &proj_dir.join("off-inline-cache"),
        output_opts(CompileMode::FastDev),
    );
    let (auto_code, auto_map, auto_profile) = compile_with_profile(
        &env_guard,
        entry_path,
        &source,
        auto_parallel_cfg(),
        &proj_dir.join("auto-inline-cache"),
        output_opts(CompileMode::FastDev),
    );
    let (on_code, on_map, on_profile) = compile_with_profile(
        &env_guard,
        entry_path,
        &source,
        on_parallel_cfg(),
        &proj_dir.join("on-inline-cache"),
        output_opts(CompileMode::FastDev),
    );

    assert_eq!(off_code, auto_code);
    assert_eq!(off_code, on_code);
    assert_eq!(off_map, auto_map);
    assert_eq!(off_map, on_map);
    assert_eq!(
        artifact_hash(&off_code, &off_map),
        artifact_hash(&auto_code, &auto_map)
    );
    assert_eq!(
        artifact_hash(&off_code, &off_map),
        artifact_hash(&on_code, &on_map)
    );

    let off_inline_cleanup = profile_stage(&off_profile, "tachyon_inline_cleanup");
    assert!(off_inline_cleanup.invocations > 0);
    assert_eq!(off_inline_cleanup.parallel_invocations, 0);

    let auto_inline_cleanup = profile_stage(&auto_profile, "tachyon_inline_cleanup");
    assert!(auto_inline_cleanup.parallel_invocations > 0);

    let on_inline_cleanup = profile_stage(&on_profile, "tachyon_inline_cleanup");
    assert!(on_inline_cleanup.parallel_invocations > 0);
}
