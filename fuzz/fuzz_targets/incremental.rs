#![no_main]

mod common;
#[allow(dead_code)]
#[path = "../../tests/common/random_rr.rs"]
mod random_rr;

use RR::compiler::{
    CompileOutputOptions, IncrementalOptions, IncrementalSession, OptLevel, ParallelBackend,
    ParallelConfig, ParallelMode, compile_with_configs_incremental_with_output_options,
};
use RR::error::{RRCode, RRException};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use libfuzzer_sys::{Corpus, fuzz_target};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct CompileConfig {
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
}

#[derive(Clone, Copy)]
struct IncrementalScenario {
    name: &'static str,
    options: IncrementalOptions,
    output_opts: CompileOutputOptions,
}

fn case_root(data: &[u8]) -> PathBuf {
    common::temp_case_root("incremental", common::stable_hash(&data))
}

fn decode_seed(data: &[u8]) -> (u64, usize) {
    let mut seed_bytes = [0u8; 8];
    for (idx, byte) in data.iter().take(8).enumerate() {
        seed_bytes[idx] = *byte;
    }
    let seed = u64::from_le_bytes(seed_bytes);
    let count = if data.len() > 8 {
        1 + (data[8] as usize % 2)
    } else {
        1
    };
    (seed, count)
}

fn write_case(root: &Path, entry_src: &str, helper_src: Option<&str>) -> Option<PathBuf> {
    fs::create_dir_all(root).ok()?;
    let entry_path = root.join("entry.rr");
    fs::write(&entry_path, entry_src).ok()?;
    if let Some(helper) = helper_src {
        fs::write(root.join("helper.rr"), helper).ok()?;
    }
    Some(entry_path)
}

fn compile_configs() -> [CompileConfig; 3] {
    [
        CompileConfig {
            opt_level: OptLevel::O0,
            type_cfg: TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Off,
            },
            parallel_cfg: ParallelConfig::default(),
        },
        CompileConfig {
            opt_level: OptLevel::O1,
            type_cfg: TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Optional,
            },
            parallel_cfg: ParallelConfig {
                mode: ParallelMode::Optional,
                backend: ParallelBackend::Auto,
                threads: 0,
                min_trip: 16,
            },
        },
        CompileConfig {
            opt_level: OptLevel::O2,
            type_cfg: TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Required,
            },
            parallel_cfg: ParallelConfig {
                mode: ParallelMode::Required,
                backend: ParallelBackend::OpenMp,
                threads: 2,
                min_trip: 8,
            },
        },
    ]
}

fn incremental_scenarios() -> [IncrementalScenario; 6] {
    [
        IncrementalScenario {
            name: "phase1_runtime",
            options: IncrementalOptions {
                enabled: true,
                auto: false,
                phase1: true,
                phase2: false,
                phase3: false,
                strict_verify: false,
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        IncrementalScenario {
            name: "phase2_runtime",
            options: IncrementalOptions {
                enabled: true,
                auto: false,
                phase1: false,
                phase2: true,
                phase3: false,
                strict_verify: false,
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        IncrementalScenario {
            name: "phase3_runtime",
            options: IncrementalOptions {
                enabled: true,
                auto: false,
                phase1: false,
                phase2: false,
                phase3: true,
                strict_verify: false,
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        IncrementalScenario {
            name: "all_runtime",
            options: IncrementalOptions::all_phases(),
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
        IncrementalScenario {
            name: "all_helper_only",
            options: IncrementalOptions::all_phases(),
            output_opts: CompileOutputOptions {
                inject_runtime: false,
            },
        },
        IncrementalScenario {
            name: "all_strict_verify",
            options: IncrementalOptions {
                strict_verify: true,
                ..IncrementalOptions::all_phases()
            },
            output_opts: CompileOutputOptions {
                inject_runtime: true,
            },
        },
    ]
}

fn compile_once(
    entry_path: &Path,
    entry_src: &str,
    cfg: CompileConfig,
    scenario: IncrementalScenario,
    session: Option<&mut IncrementalSession>,
) -> RR::error::RR<RR::compiler::IncrementalCompileOutput> {
    compile_with_configs_incremental_with_output_options(
        &entry_path.to_string_lossy(),
        entry_src,
        cfg.opt_level,
        cfg.type_cfg,
        cfg.parallel_cfg,
        scenario.options,
        scenario.output_opts,
        session,
    )
}

fn assert_helper_mode(output: &str, inject_runtime: bool) {
    assert!(
        output.contains("# --- RR runtime (auto-generated) ---"),
        "incremental output must include RR runtime helpers"
    );
    if inject_runtime {
        assert!(
            output.contains(".rr_env$file <- "),
            "runtime-injected incremental output must include source bootstrap"
        );
    } else {
        assert!(
            !output.contains(".rr_env$file <- \"entry.rr\";"),
            "helper-only incremental output must omit source bootstrap"
        );
        assert!(
            !output.contains(".rr_env$native_anchor_roots <- unique(vapply(c("),
            "helper-only incremental output must omit native root bootstrap"
        );
    }
}

fn is_internal_incremental_error(err: &RRException) -> bool {
    matches!(err.code, RRCode::ICE9001 | RRCode::E9999) || err.module == "RR.InternalError"
}

fn initial_incremental_compile(
    entry_path: &Path,
    entry_src: &str,
    cfg: CompileConfig,
    scenario: IncrementalScenario,
    session: Option<&mut IncrementalSession>,
) -> Option<RR::compiler::IncrementalCompileOutput> {
    match compile_once(entry_path, entry_src, cfg, scenario, session) {
        Ok(output) => Some(output),
        Err(err) => {
            if is_internal_incremental_error(&err) {
                panic!(
                    "incremental compile ICE for scenario {} at {}: {:?}\nsource:\n{}",
                    scenario.name,
                    entry_path.display(),
                    err,
                    entry_src
                );
            }
            None
        }
    }
}

fn replay_incremental_compile(
    entry_path: &Path,
    entry_src: &str,
    cfg: CompileConfig,
    scenario: IncrementalScenario,
    session: Option<&mut IncrementalSession>,
) -> RR::compiler::IncrementalCompileOutput {
    compile_once(entry_path, entry_src, cfg, scenario, session).unwrap_or_else(|err| {
        panic!(
            "incremental replay failed for accepted scenario {} at {}: {:?}\nsource:\n{}",
            scenario.name,
            entry_path.display(),
            err,
            entry_src
        )
    })
}

fn exercise_incremental(entry_path: &Path, entry_src: &str, seed_tag: u64) -> bool {
    let mut kept = false;
    for cfg in compile_configs() {
        for scenario in incremental_scenarios() {
            let cache_dir = common::temp_case_root(
                "incremental-cache",
                seed_tag
                    ^ (cfg.opt_level.label().as_bytes()[1] as u64)
                    ^ common::stable_hash(&scenario.name),
            );
            let _ = fs::create_dir_all(&cache_dir);
            let _quiet = common::ScopedEnvVar::set("RR_QUIET_LOG", Some("1"));
            let cache_dir_str = cache_dir.to_string_lossy().to_string();
            let _cache =
                common::ScopedEnvVar::set("RR_INCREMENTAL_CACHE_DIR", Some(cache_dir_str.as_str()));

            match scenario.name {
                "phase1_runtime" => {
                    let Some(first) =
                        initial_incremental_compile(entry_path, entry_src, cfg, scenario, None)
                    else {
                        continue;
                    };
                    let second =
                        replay_incremental_compile(entry_path, entry_src, cfg, scenario, None);
                    assert_eq!(first.r_code, second.r_code);
                    assert!(second.stats.phase1_artifact_hit);
                    assert_helper_mode(&second.r_code, true);
                    kept = true;
                }
                "phase2_runtime" => {
                    let Some(first) =
                        initial_incremental_compile(entry_path, entry_src, cfg, scenario, None)
                    else {
                        continue;
                    };
                    let second =
                        replay_incremental_compile(entry_path, entry_src, cfg, scenario, None);
                    assert_eq!(first.r_code, second.r_code);
                    assert!(
                        second.stats.phase2_emit_hits > 0 || second.stats.phase2_emit_misses == 0,
                        "phase2 replay should observe cached function emits"
                    );
                    assert_helper_mode(&second.r_code, true);
                    kept = true;
                }
                "phase3_runtime" => {
                    let mut session = IncrementalSession::default();
                    let Some(first) = initial_incremental_compile(
                        entry_path,
                        entry_src,
                        cfg,
                        scenario,
                        Some(&mut session),
                    ) else {
                        continue;
                    };
                    let second = replay_incremental_compile(
                        entry_path,
                        entry_src,
                        cfg,
                        scenario,
                        Some(&mut session),
                    );
                    assert_eq!(first.r_code, second.r_code);
                    assert!(second.stats.phase3_memory_hit);
                    assert_helper_mode(&second.r_code, true);
                    kept = true;
                }
                "all_runtime" | "all_helper_only" => {
                    let inject_runtime = scenario.output_opts.inject_runtime;
                    let mut session = IncrementalSession::default();
                    let Some(first) = initial_incremental_compile(
                        entry_path,
                        entry_src,
                        cfg,
                        scenario,
                        Some(&mut session),
                    ) else {
                        continue;
                    };
                    let second = replay_incremental_compile(
                        entry_path,
                        entry_src,
                        cfg,
                        scenario,
                        Some(&mut session),
                    );
                    assert_eq!(first.r_code, second.r_code);
                    assert!(second.stats.phase3_memory_hit);
                    assert_helper_mode(&second.r_code, inject_runtime);
                    kept = true;
                }
                "all_strict_verify" => {
                    let mut session = IncrementalSession::default();
                    let Some(first) = initial_incremental_compile(
                        entry_path,
                        entry_src,
                        cfg,
                        scenario,
                        Some(&mut session),
                    ) else {
                        continue;
                    };
                    let second = replay_incremental_compile(
                        entry_path,
                        entry_src,
                        cfg,
                        scenario,
                        Some(&mut session),
                    );
                    let mut fresh_session = IncrementalSession::default();
                    let third = replay_incremental_compile(
                        entry_path,
                        entry_src,
                        cfg,
                        scenario,
                        Some(&mut fresh_session),
                    );
                    assert_eq!(first.r_code, second.r_code);
                    assert_eq!(second.r_code, third.r_code);
                    assert!(second.stats.phase3_memory_hit);
                    assert!(second.stats.strict_verification_checked);
                    assert!(second.stats.strict_verification_passed);
                    assert!(third.stats.phase1_artifact_hit);
                    assert!(third.stats.strict_verification_checked);
                    assert!(third.stats.strict_verification_passed);
                    assert_helper_mode(&third.r_code, true);
                    kept = true;
                }
                _ => unreachable!("unknown incremental scenario"),
            }
        }
    }
    kept
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.is_empty() {
        return Corpus::Reject;
    }

    let (seed, count) = decode_seed(data);
    let cases = random_rr::generate_cases(seed, count);

    let helper_src = r#"
fn helper_bias(x) {
  return x + 1
}

fn helper_pick(flag) {
  if (flag) {
    return 2
  }
  return 1
}
"#;

    let root = case_root(data);
    let mut kept = false;

    for (case_idx, case) in cases.into_iter().enumerate() {
        for (variant_idx, variant) in common::source_variants(&case.rr_src)
            .into_iter()
            .filter(|variant| !variant.starts_with("fn __fuzz_entry() {\n"))
            .take(2)
            .enumerate()
        {
            let base_root = root.join(format!("case_{case_idx}_base_{variant_idx}"));
            if let Some(entry_path) = write_case(&base_root, &variant, None) {
                kept |= exercise_incremental(&entry_path, &variant, common::stable_hash(&variant));
            }

            let imported = format!(
                "import \"helper.rr\"\n{variant}\nprint(helper_bias(1L))\nprint(helper_pick(TRUE))\n"
            );
            let import_root = root.join(format!("case_{case_idx}_import_{variant_idx}"));
            if let Some(entry_path) = write_case(&import_root, &imported, Some(helper_src)) {
                kept |=
                    exercise_incremental(&entry_path, &imported, common::stable_hash(&imported));
            }
        }
    }

    if kept { Corpus::Keep } else { Corpus::Reject }
});
