use RR::compiler::{
    CompileOutputOptions, CompileProfile, CompilerParallelConfig, CompilerParallelMode, OptLevel,
    compile_with_configs_with_options_and_compiler_parallel_and_profile, default_parallel_config,
    default_type_config,
};

fn compile_with_profile(
    entry_path: &str,
    source: &str,
    compiler_parallel_cfg: CompilerParallelConfig,
) -> CompileProfile {
    let mut profile = CompileProfile::default();
    compile_with_configs_with_options_and_compiler_parallel_and_profile(
        entry_path,
        source,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        compiler_parallel_cfg,
        CompileOutputOptions::default(),
        Some(&mut profile),
    )
    .expect("compile failed");
    profile
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

#[test]
fn compiler_parallel_thresholds_only_parallelize_large_lowering_and_emit_jobs() {
    let small_profile = compile_with_profile(
        "compiler_parallel_threshold_small.rr",
        small_fixture(),
        threshold_cfg(),
    );
    let large_source = large_fixture();
    let large_profile = compile_with_profile(
        "compiler_parallel_threshold_large.rr",
        &large_source,
        threshold_cfg(),
    );

    let small_lower = stage(&small_profile, "mir_lowering");
    let small_emit = stage(&small_profile, "emit");
    assert_eq!(small_lower.parallel_invocations, 0);
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
        small_emit
            .reason_counts
            .get("below_min_functions")
            .copied()
            .unwrap_or(0)
            > 0
    );

    let large_lower = stage(&large_profile, "mir_lowering");
    let large_emit = stage(&large_profile, "emit");
    assert!(large_lower.parallel_invocations > 0);
    assert!(large_emit.parallel_invocations > 0);

    let type_analysis = stage(&large_profile, "type_analysis");
    let tachyon_heavy = stage(&large_profile, "tachyon_heavy");
    assert_eq!(type_analysis.parallel_invocations, 0);
    assert_eq!(tachyon_heavy.parallel_invocations, 0);
    assert!(
        type_analysis
            .reason_counts
            .get("stage_disabled")
            .copied()
            .unwrap_or(0)
            > 0
    );
    assert!(
        tachyon_heavy
            .reason_counts
            .get("stage_disabled")
            .copied()
            .unwrap_or(0)
            > 0
            || tachyon_heavy.invocations == 0
    );
}
