mod common;

use RR::compiler::{
    CompileMode, CompileOutputOptions, CompileProfile, CompilerParallelConfig,
    CompilerParallelMode, OptLevel, compile_with_configs_with_options_and_compiler_parallel_and_profile,
    default_parallel_config, default_type_config,
};
use common::unique_dir;
use std::path::PathBuf;
use std::sync::MutexGuard;

fn compile_with_profile(
    env_guard: &MutexGuard<'static, ()>,
    entry_path: &str,
    source: &str,
    compiler_parallel_cfg: CompilerParallelConfig,
    cache_dir: &std::path::Path,
    output_opts: CompileOutputOptions,
) -> CompileProfile {
    common::set_env_var_for_test(env_guard, "RR_INCREMENTAL_CACHE_DIR", cache_dir);
    let mut profile = CompileProfile::default();
    compile_with_configs_with_options_and_compiler_parallel_and_profile(
        entry_path,
        source,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        compiler_parallel_cfg,
        output_opts,
        Some(&mut profile),
    )
    .expect("compile failed");
    common::remove_env_var_for_test(env_guard, "RR_INCREMENTAL_CACHE_DIR");
    profile
}

fn output_opts(compile_mode: CompileMode) -> CompileOutputOptions {
    CompileOutputOptions {
        compile_mode,
        ..CompileOutputOptions::default()
    }
}

fn stage<'a>(
    profile: &'a CompileProfile,
    label: &str,
) -> &'a RR::compiler::CompilerParallelStageProfile {
    profile
        .compiler_parallel
        .stage(label)
        .unwrap_or_else(|| panic!("missing compiler parallel stage {label}"))
}

fn threshold_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::Auto,
        threads: 4,
        min_functions: 6,
        min_fn_ir: 1,
        max_jobs: 4,
    }
}

fn small_fixture() -> &'static str {
    r#"
fn sq(x) {
  return x * x
}

fn main() {
  print(sq(4.0))
}
main()
"#
}

fn large_fixture() -> String {
    let mut source = String::new();
    for idx in 0..8 {
        source.push_str(&format!(
            "fn helper_{idx}(x) {{\n  let a = x + {idx}.0\n  let b = a * (x + 1.0)\n  return a + b\n}}\n\n"
        ));
    }
    source.push_str(
        "fn main() {\n  let out = numeric(8L)\n  let i = 1L\n  while (i <= 8L) {\n    out[i] = helper_0(i) + helper_1(i) + helper_2(i) + helper_3(i) + helper_4(i) + helper_5(i) + helper_6(i) + helper_7(i)\n    i = i + 1L\n  }\n  print(out)\n}\nmain()\n",
    );
    source
}

fn small_inline_fixture() -> &'static str {
    r#"
fn add1(x) {
  let y = x + 1L
  if x > 0L {
    return y
  } else {
    return y + 1L
  }
}

fn add2(x) {
  let y = x + 2L
  if x > 0L {
    return y
  } else {
    return y + 1L
  }
}

fn pair_sum(a, b) {
  let lhs = add1(a)
  let rhs = add2(b)
  return lhs + rhs
}

fn main() {
  print(pair_sum(4L, 6L))
}
main()
"#
}

fn large_inline_fixture() -> String {
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
fn compiler_parallel_thresholds_only_parallelize_large_lowering_and_emit_jobs() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compiler_parallel_thresholds");
    std::fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "profile_cache");
    std::fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let small_profile = compile_with_profile(
        &env_guard,
        "compiler_parallel_threshold_small.rr",
        small_fixture(),
        threshold_cfg(),
        &proj_dir.join("small-cache"),
        output_opts(CompileMode::Standard),
    );
    let large_source = large_fixture();
    let large_profile = compile_with_profile(
        &env_guard,
        "compiler_parallel_threshold_large.rr",
        &large_source,
        threshold_cfg(),
        &proj_dir.join("large-cache"),
        output_opts(CompileMode::Standard),
    );

    let small_lower = stage(&small_profile, "mir_lowering");
    let small_type_analysis = stage(&small_profile, "type_analysis");
    let small_always = stage(&small_profile, "tachyon_always");
    let small_heavy = stage(&small_profile, "tachyon_heavy");
    let small_fresh_alias = stage(&small_profile, "tachyon_fresh_alias");
    let small_de_ssa = stage(&small_profile, "tachyon_de_ssa");
    let small_emit = stage(&small_profile, "emit");
    assert_eq!(small_lower.parallel_invocations, 0);
    assert_eq!(small_type_analysis.parallel_invocations, 0);
    assert_eq!(small_always.parallel_invocations, 0);
    assert_eq!(small_heavy.parallel_invocations, 0);
    assert_eq!(small_fresh_alias.parallel_invocations, 0);
    assert_eq!(small_de_ssa.parallel_invocations, 0);
    assert_eq!(small_emit.parallel_invocations, 0);
    assert!(
        small_lower
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        small_type_analysis
            .reason_counts
            .get("single_job")
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        small_always
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        small_heavy
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        small_fresh_alias
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        small_de_ssa
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        small_emit
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );

    let large_lower = stage(&large_profile, "mir_lowering");
    let large_type_analysis = stage(&large_profile, "type_analysis");
    let large_always = stage(&large_profile, "tachyon_always");
    let large_heavy = stage(&large_profile, "tachyon_heavy");
    let large_fresh_alias = stage(&large_profile, "tachyon_fresh_alias");
    let large_de_ssa = stage(&large_profile, "tachyon_de_ssa");
    let large_emit = stage(&large_profile, "emit");
    assert!(large_lower.parallel_invocations > 0);
    assert!(large_type_analysis.parallel_invocations > 0);
    assert!(large_always.parallel_invocations > 0);
    assert!(large_heavy.parallel_invocations > 0);
    assert!(large_fresh_alias.parallel_invocations > 0);
    assert!(large_de_ssa.parallel_invocations > 0);
    assert!(large_emit.parallel_invocations > 0);
}

#[test]
fn compiler_parallel_thresholds_parallelize_inline_cleanup_only_for_large_inline_jobs() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compiler_parallel_thresholds");
    std::fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "inline_cleanup_profile_cache");
    std::fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let small_profile = compile_with_profile(
        &env_guard,
        "compiler_parallel_inline_cleanup_small.rr",
        small_inline_fixture(),
        threshold_cfg(),
        &proj_dir.join("small-inline-cache"),
        output_opts(CompileMode::FastDev),
    );
    let large_source = large_inline_fixture();
    let large_profile = compile_with_profile(
        &env_guard,
        "compiler_parallel_inline_cleanup_large.rr",
        &large_source,
        threshold_cfg(),
        &proj_dir.join("large-inline-cache"),
        output_opts(CompileMode::FastDev),
    );

    let small_inline_cleanup = stage(&small_profile, "tachyon_inline_cleanup");
    assert_eq!(small_inline_cleanup.parallel_invocations, 0);
    assert!(
        small_inline_cleanup
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );

    let large_inline_cleanup = stage(&large_profile, "tachyon_inline_cleanup");
    assert!(large_inline_cleanup.invocations > 0);
    assert!(large_inline_cleanup.parallel_invocations > 0);
}
