#![no_main]

mod common;
#[allow(dead_code)]
#[path = "../../tests/common/random_rr.rs"]
mod random_rr;

use RR::compiler::{
    CompileOutputOptions, OptLevel, ParallelBackend, ParallelConfig, ParallelMode,
    compile_with_configs_with_options,
};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use libfuzzer_sys::{Corpus, fuzz_target};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
enum ScenarioKind {
    BaselineO0,
    HelperOnlyO1,
    GradualOptionalO2,
    RequiredBackendsO2,
    VerifyEachPassO2,
    TunedOptimizerFlagsO2,
}

#[derive(Clone, Copy)]
struct CompileScenario {
    kind: ScenarioKind,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    output_opts: CompileOutputOptions,
}

fn decode_seed(data: &[u8]) -> (u64, usize) {
    let mut seed_bytes = [0u8; 8];
    for (idx, byte) in data.iter().take(8).enumerate() {
        seed_bytes[idx] = *byte;
    }
    let seed = u64::from_le_bytes(seed_bytes);
    let count = if data.len() > 8 {
        1 + (data[8] as usize % 3)
    } else {
        2
    };
    (seed, count)
}

fn compile_scenarios() -> [CompileScenario; 6] {
    [
        CompileScenario {
            kind: ScenarioKind::BaselineO0,
            opt_level: OptLevel::O0,
            type_cfg: TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Off,
            },
            parallel_cfg: ParallelConfig::default(),
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        CompileScenario {
            kind: ScenarioKind::HelperOnlyO1,
            opt_level: OptLevel::O1,
            type_cfg: TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Off,
            },
            parallel_cfg: ParallelConfig::default(),
            output_opts: CompileOutputOptions {
                inject_runtime: false,
            },
        },
        CompileScenario {
            kind: ScenarioKind::GradualOptionalO2,
            opt_level: OptLevel::O2,
            type_cfg: TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Optional,
            },
            parallel_cfg: ParallelConfig {
                mode: ParallelMode::Optional,
                backend: ParallelBackend::Auto,
                threads: 0,
                min_trip: 32,
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        CompileScenario {
            kind: ScenarioKind::RequiredBackendsO2,
            opt_level: OptLevel::O2,
            type_cfg: TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Required,
            },
            parallel_cfg: ParallelConfig {
                mode: ParallelMode::Required,
                backend: ParallelBackend::OpenMp,
                threads: 2,
                min_trip: 16,
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        CompileScenario {
            kind: ScenarioKind::VerifyEachPassO2,
            opt_level: OptLevel::O2,
            type_cfg: TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Off,
            },
            parallel_cfg: ParallelConfig {
                mode: ParallelMode::Optional,
                backend: ParallelBackend::R,
                threads: 0,
                min_trip: 8,
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        CompileScenario {
            kind: ScenarioKind::TunedOptimizerFlagsO2,
            opt_level: OptLevel::O2,
            type_cfg: TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Optional,
            },
            parallel_cfg: ParallelConfig {
                mode: ParallelMode::Optional,
                backend: ParallelBackend::Auto,
                threads: 1,
                min_trip: 4,
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
    ]
}

fn apply_env_profile(root: &Path, kind: ScenarioKind) -> Vec<common::ScopedEnvVar> {
    let mut guards = vec![common::ScopedEnvVar::set("RR_QUIET_LOG", Some("1"))];

    match kind {
        ScenarioKind::BaselineO0
        | ScenarioKind::HelperOnlyO1
        | ScenarioKind::GradualOptionalO2
        | ScenarioKind::RequiredBackendsO2 => {}
        ScenarioKind::VerifyEachPassO2 => {
            let verify_dir = root.join("verify-dumps");
            let _ = fs::create_dir_all(&verify_dir);
            let verify_dir_str = verify_dir.to_string_lossy().to_string();
            guards.push(common::ScopedEnvVar::set("RR_VERIFY_EACH_PASS", Some("1")));
            guards.push(common::ScopedEnvVar::set(
                "RR_VERIFY_DUMP_DIR",
                Some(verify_dir_str.as_str()),
            ));
        }
        ScenarioKind::TunedOptimizerFlagsO2 => {
            let profile_path = root.join("hot.profile");
            let _ = fs::write(&profile_path, "Sym_1=7\nSym_top_0=3\n");
            let profile_path_str = profile_path.to_string_lossy().to_string();
            for (key, value) in [
                ("RR_ENABLE_LICM", "0"),
                ("RR_ENABLE_GVN", "0"),
                ("RR_DISABLE_INLINE", "1"),
                ("RR_INLINE_ALLOW_LOOPS", "1"),
                ("RR_SELECTIVE_OPT_BUDGET", "0"),
                ("RR_ADAPTIVE_IR_BUDGET", "0"),
                ("RR_OPT_MAX_ITERS", "2"),
                ("RR_INLINE_MAX_ROUNDS", "1"),
                ("RR_INLINE_LOCAL_ROUNDS", "1"),
                ("RR_MAX_FULL_OPT_IR", "160"),
                ("RR_MAX_FULL_OPT_FN_IR", "96"),
                ("RR_HEAVY_PASS_FN_IR", "64"),
                ("RR_ALWAYS_BCE_FN_IR", "64"),
                ("RR_MAX_FN_OPT_MS", "10"),
                ("RR_ALWAYS_TIER_ITERS", "1"),
                ("RR_BCE_VISIT_LIMIT", "10000"),
                ("RR_INLINE_MAX_BLOCKS", "12"),
                ("RR_INLINE_MAX_INSTRS", "64"),
                ("RR_INLINE_MAX_COST", "96"),
                ("RR_INLINE_MAX_CALLSITE_COST", "96"),
                ("RR_INLINE_MAX_CALLER_INSTRS", "192"),
                ("RR_INLINE_MAX_TOTAL_INSTRS", "224"),
                ("RR_INLINE_MAX_UNIT_GROWTH_PCT", "10"),
                ("RR_INLINE_MAX_FN_GROWTH_PCT", "10"),
                ("RR_VECTORIZE_TRACE", "1"),
                ("RR_WRAP_TRACE", "1"),
            ] {
                guards.push(common::ScopedEnvVar::set(key, Some(value)));
            }
            guards.push(common::ScopedEnvVar::set(
                "RR_PROFILE_USE",
                Some(profile_path_str.as_str()),
            ));
        }
    }

    guards
}

fn assert_compile_matrix(case_name: &str, src: &str, seed: u64) {
    let root = common::temp_case_root(
        "generated-pipeline-matrix",
        seed ^ common::stable_hash(&src),
    );
    let _ = fs::create_dir_all(&root);
    let entry_path = root.join(format!("{case_name}.rr"));
    let entry_path_str = entry_path.to_string_lossy().to_string();

    for scenario in compile_scenarios() {
        let _env = apply_env_profile(&root, scenario.kind);
        let (code, map) = compile_with_configs_with_options(
            &entry_path_str,
            src,
            scenario.opt_level,
            scenario.type_cfg,
            scenario.parallel_cfg,
            scenario.output_opts,
        )
        .unwrap_or_else(|err| {
            panic!(
                "compile matrix failed for case {} scenario {:?}: {:?}\nsource:\n{}",
                case_name, scenario.kind, err, src
            )
        });

        assert!(!code.is_empty(), "compile matrix emitted empty R code");
        assert!(!map.is_empty(), "compile matrix emitted empty source map");
        assert!(
            code.contains("# --- RR runtime (auto-generated) ---"),
            "compiled output must include RR runtime helpers"
        );
        if scenario.output_opts.inject_runtime {
            assert!(
                code.contains(".rr_env$file <- "),
                "runtime-injected compile must include source bootstrap"
            );
        } else {
            assert!(
                !code.contains(".rr_env$file <- "),
                "helper-only compile must omit source bootstrap"
            );
            assert!(
                !code.contains(".rr_env$native_anchor_roots <- unique(vapply(c("),
                "helper-only compile must omit native root bootstrap"
            );
        }
    }
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.is_empty() {
        return Corpus::Reject;
    }

    let (seed, count) = decode_seed(data);
    let cases = random_rr::generate_cases(seed, count);
    let mut kept_any = false;

    for case in cases {
        let Some(all_fns) = common::build_mir(&case.rr_src) else {
            continue;
        };
        if all_fns.is_empty() {
            continue;
        }
        kept_any = true;

        for cfg in [
            TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Off,
            },
            TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Off,
            },
            TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Optional,
            },
        ] {
            common::run_full_pipeline(&all_fns, cfg);
        }

        assert_compile_matrix(&case.name, &case.rr_src, seed);
    }

    if kept_any {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});
