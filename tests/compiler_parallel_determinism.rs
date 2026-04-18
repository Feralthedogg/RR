use RR::codegen::mir_emit::MapEntry;
use RR::compiler::{
    CompileOutputOptions, CompileProfile, CompilerParallelConfig, CompilerParallelMode, OptLevel,
    compile_with_configs_with_options_and_compiler_parallel_and_profile, default_parallel_config,
    default_type_config,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn compile_with_profile(
    entry_path: &str,
    source: &str,
    compiler_parallel_cfg: CompilerParallelConfig,
) -> (String, Vec<MapEntry>, CompileProfile) {
    let mut profile = CompileProfile::default();
    let (code, map) = compile_with_configs_with_options_and_compiler_parallel_and_profile(
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
    (code, map, profile)
}

fn profile_stage<'a>(
    profile: &'a CompileProfile,
    stage: &str,
) -> &'a RR::compiler::CompilerParallelStageProfile {
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

#[test]
fn compiler_parallel_off_auto_on_emit_identical_artifacts_and_hashes() {
    let source = large_parallel_fixture();
    let entry_path = "compiler_parallel_determinism.rr";

    let (off_code, off_map, off_profile) =
        compile_with_profile(entry_path, &source, off_parallel_cfg());
    let (auto_code, auto_map, auto_profile) =
        compile_with_profile(entry_path, &source, auto_parallel_cfg());
    let (on_code, on_map, on_profile) =
        compile_with_profile(entry_path, &source, on_parallel_cfg());

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
    assert_eq!(off_emit.parallel_invocations, 0);
    assert!(off_emit.reason_counts.get("mode_off").copied().unwrap_or(0) > 0);

    let auto_lower = profile_stage(&auto_profile, "mir_lowering");
    let auto_emit = profile_stage(&auto_profile, "emit");
    assert!(auto_lower.parallel_invocations > 0);
    assert!(auto_emit.parallel_invocations > 0);

    let on_lower = profile_stage(&on_profile, "mir_lowering");
    let on_emit = profile_stage(&on_profile, "emit");
    assert!(on_lower.parallel_invocations > 0);
    assert!(on_emit.parallel_invocations > 0);
}
