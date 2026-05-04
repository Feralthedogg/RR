mod common;

use common::unique_dir;
use rr::compiler::{
    CompileMode, CompileOutputOptions, CompileProfile, CompilerParallelConfig,
    CompilerParallelMode, IncrementalCompileOutput, IncrementalCompileRequest, IncrementalOptions,
    IncrementalSession, OptLevel, compile_incremental_request, default_compiler_parallel_config,
    default_parallel_config, default_type_config,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Clone, Copy)]
struct PerfCase {
    label: &'static str,
    path: &'static str,
}

#[derive(Clone, Copy)]
struct PerfConfig {
    label: &'static str,
    compile_mode: &'static str,
    compiler_parallel_mode: &'static str,
}

#[derive(Clone, Copy, Default)]
struct ProfileSample {
    wall_ms: u128,
    tachyon_ms: u128,
    emit_ms: u128,
    phase1_artifact_hit: bool,
    phase2_emit_hits: usize,
    phase2_emit_misses: usize,
    phase3_memory_hit: bool,
    parallel_stage_invocations: usize,
}

#[derive(Default)]
struct SampleBucket {
    cold_total_ms: Vec<u128>,
    warm_total_ms: Vec<u128>,
    cold_tachyon_ms: Vec<u128>,
    warm_tachyon_ms: Vec<u128>,
    cold_emit_ms: Vec<u128>,
    warm_emit_ms: Vec<u128>,
    warm_phase2_emit_hits: Vec<u128>,
    warm_parallel_stage_invocations: Vec<u128>,
    warm_phase1_artifact_hits: usize,
    warm_phase3_memory_hits: usize,
}

#[derive(Default)]
struct IncrementalReuseBucket {
    phase2_seed_ms: Vec<u128>,
    phase2_hit_ms: Vec<u128>,
    phase2_hit_emit_hits: Vec<u128>,
    phase2_hit_parallel_stage_invocations: Vec<u128>,
    phase3_seed_ms: Vec<u128>,
    phase3_hit_ms: Vec<u128>,
    phase3_hit_parallel_stage_invocations: Vec<u128>,
    phase3_memory_hits: usize,
}

#[derive(Clone, Copy, Default)]
struct SummaryStats {
    median: u128,
    min: u128,
    max: u128,
}

const DEFAULT_REPEATS: usize = 2;
const PERF_CASES: &[PerfCase] = &[
    PerfCase {
        label: "signal_pipeline",
        path: "example/benchmarks/signal_pipeline_bench.rr",
    },
    PerfCase {
        label: "heat_diffusion",
        path: "example/benchmarks/heat_diffusion_bench.rr",
    },
    PerfCase {
        label: "reaction_diffusion",
        path: "example/benchmarks/reaction_diffusion_bench.rr",
    },
];
const PERF_CONFIGS: &[PerfConfig] = &[
    PerfConfig {
        label: "standard/serial",
        compile_mode: "standard",
        compiler_parallel_mode: "off",
    },
    PerfConfig {
        label: "standard/auto",
        compile_mode: "standard",
        compiler_parallel_mode: "auto",
    },
    PerfConfig {
        label: "standard/parallel",
        compile_mode: "standard",
        compiler_parallel_mode: "on",
    },
    PerfConfig {
        label: "fast-dev/serial",
        compile_mode: "fast-dev",
        compiler_parallel_mode: "off",
    },
    PerfConfig {
        label: "fast-dev/auto",
        compile_mode: "fast-dev",
        compiler_parallel_mode: "auto",
    },
    PerfConfig {
        label: "fast-dev/parallel",
        compile_mode: "fast-dev",
        compiler_parallel_mode: "on",
    },
];

fn parse_repeats() -> usize {
    env::var("RR_COMPILE_MODE_PERF_REPEATS")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|repeats| *repeats > 0)
        .unwrap_or(DEFAULT_REPEATS)
}

fn selected_cases() -> Vec<&'static PerfCase> {
    let Some(raw) = env::var("RR_COMPILE_MODE_PERF_CASES").ok() else {
        return PERF_CASES.iter().collect();
    };
    let wanted: Vec<String> = raw
        .split(',')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .collect();
    if wanted.is_empty() {
        return PERF_CASES.iter().collect();
    }
    PERF_CASES
        .iter()
        .filter(|case| wanted.iter().any(|want| want == case.label))
        .collect()
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .unwrap_or_else(|| panic!("missing key {} in compile profile", path.join(".")));
    }
    current
}

fn json_u128(value: &Value, path: &[&str]) -> u128 {
    json_at(value, path)
        .as_u64()
        .unwrap_or_else(|| panic!("expected u64 at {}", path.join("."))) as u128
}

fn json_usize(value: &Value, path: &[&str]) -> usize {
    json_u128(value, path) as usize
}

fn json_bool(value: &Value, path: &[&str]) -> bool {
    json_at(value, path)
        .as_bool()
        .unwrap_or_else(|| panic!("expected bool at {}", path.join(".")))
}

fn parse_profile(profile_path: &Path, wall_ms: u128) -> ProfileSample {
    let profile: Value = serde_json::from_str(
        &fs::read_to_string(profile_path).expect("failed to read compile profile json"),
    )
    .expect("compile profile should be valid json");

    let parallel_stage_invocations = json_at(&profile, &["compiler_parallel", "stages"])
        .as_array()
        .expect("compiler_parallel.stages should be an array")
        .iter()
        .map(|stage| {
            stage
                .get("parallel_invocations")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize
        })
        .sum();

    ProfileSample {
        wall_ms,
        tachyon_ms: json_u128(&profile, &["tachyon", "elapsed_ns"]) / 1_000_000,
        emit_ms: json_u128(&profile, &["emit", "elapsed_ns"]) / 1_000_000,
        phase1_artifact_hit: json_bool(&profile, &["incremental", "phase1_artifact_hit"]),
        phase2_emit_hits: json_usize(&profile, &["incremental", "phase2_emit_hits"]),
        phase2_emit_misses: json_usize(&profile, &["incremental", "phase2_emit_misses"]),
        phase3_memory_hit: json_bool(&profile, &["incremental", "phase3_memory_hit"]),
        parallel_stage_invocations,
    }
}

fn invoke_rr_build(
    rr_bin: &Path,
    rr_src: &Path,
    config: PerfConfig,
    out_dir: &Path,
    cache_dir: &Path,
    profile_path: &Path,
) -> ProfileSample {
    let mut cmd = Command::new(rr_bin);
    cmd.arg("build")
        .arg(rr_src)
        .arg("--out-dir")
        .arg(out_dir)
        .arg("-O1")
        .arg("--incremental-phases")
        .arg("all")
        .arg("--profile-compile-out")
        .arg(profile_path)
        .arg("--compile-mode")
        .arg(config.compile_mode)
        .arg("--compiler-parallel-mode")
        .arg(config.compiler_parallel_mode)
        .env("RR_INCREMENTAL_CACHE_DIR", cache_dir);

    if config.compiler_parallel_mode == "on" {
        cmd.arg("--compiler-parallel-threads")
            .arg("4")
            .arg("--compiler-parallel-min-functions")
            .arg("1")
            .arg("--compiler-parallel-min-fn-ir")
            .arg("1")
            .arg("--compiler-parallel-max-jobs")
            .arg("4");
    }

    let started = Instant::now();
    let output = cmd.output().expect("failed to execute RR build");
    let wall_ms = started.elapsed().as_millis();
    assert!(
        output.status.success(),
        "RR build failed for {} [{}]\nstdout={}\nstderr={}",
        rr_src.display(),
        config.label,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    parse_profile(profile_path, wall_ms)
}

fn config_compile_mode(config: PerfConfig) -> CompileMode {
    match config.compile_mode {
        "standard" => CompileMode::Standard,
        "fast-dev" => CompileMode::FastDev,
        other => panic!("unsupported compile mode {other}"),
    }
}

fn config_compiler_parallel(config: PerfConfig) -> CompilerParallelConfig {
    match config.compiler_parallel_mode {
        "off" => CompilerParallelConfig {
            mode: CompilerParallelMode::Off,
            threads: 4,
            min_functions: 1,
            min_fn_ir: 1,
            max_jobs: 4,
        },
        "auto" => default_compiler_parallel_config(),
        "on" => CompilerParallelConfig {
            mode: CompilerParallelMode::On,
            threads: 4,
            min_functions: 1,
            min_fn_ir: 1,
            max_jobs: 4,
        },
        other => panic!("unsupported compiler parallel mode {other}"),
    }
}

fn output_opts_for_mode(compile_mode: CompileMode) -> CompileOutputOptions {
    CompileOutputOptions {
        compile_mode,
        ..CompileOutputOptions::default()
    }
}

fn profile_sample_from_profile(profile: &CompileProfile, wall_ms: u128) -> ProfileSample {
    let parallel_stage_invocations = profile
        .compiler_parallel
        .stages
        .iter()
        .map(|stage| stage.parallel_invocations)
        .sum();
    ProfileSample {
        wall_ms,
        tachyon_ms: profile.tachyon.elapsed_ns / 1_000_000,
        emit_ms: profile.emit.elapsed_ns / 1_000_000,
        phase1_artifact_hit: profile.incremental.phase1_artifact_hit,
        phase2_emit_hits: profile.incremental.phase2_emit_hits,
        phase2_emit_misses: profile.incremental.phase2_emit_misses,
        phase3_memory_hit: profile.incremental.phase3_memory_hit,
        parallel_stage_invocations,
    }
}

fn invoke_incremental_compile(
    entry_path: &str,
    entry_input: &str,
    config: PerfConfig,
    options: IncrementalOptions,
    session: Option<&mut IncrementalSession>,
) -> (IncrementalCompileOutput, ProfileSample) {
    let mut profile = CompileProfile::default();
    let started = Instant::now();
    let output = compile_incremental_request(IncrementalCompileRequest {
        entry_path,
        entry_input,
        opt_level: OptLevel::O1,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: config_compiler_parallel(config),
        options,
        output_options: output_opts_for_mode(config_compile_mode(config)),
        session,
        profile: Some(&mut profile),
    })
    .unwrap_or_else(|err| {
        panic!(
            "incremental compile failed for {} [{}]: {err:?}",
            entry_path, config.label
        )
    });
    let wall_ms = started.elapsed().as_millis();
    (output, profile_sample_from_profile(&profile, wall_ms))
}

fn summarize(samples: &[u128]) -> SummaryStats {
    assert!(!samples.is_empty(), "expected at least one sample");
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    SummaryStats {
        median: sorted[sorted.len() / 2],
        min: *sorted.first().unwrap_or(&0),
        max: *sorted.last().unwrap_or(&0),
    }
}

fn print_case_line(case_label: &str, config_label: &str, samples: &SampleBucket) {
    let cold_total = summarize(&samples.cold_total_ms);
    let warm_total = summarize(&samples.warm_total_ms);
    let cold_tachyon = summarize(&samples.cold_tachyon_ms);
    let warm_tachyon = summarize(&samples.warm_tachyon_ms);
    let cold_emit = summarize(&samples.cold_emit_ms);
    let warm_emit = summarize(&samples.warm_emit_ms);
    let warm_phase2_hits = summarize(&samples.warm_phase2_emit_hits);
    let warm_parallel_invocations = summarize(&samples.warm_parallel_stage_invocations);
    let warm_speedup = if warm_total.median == 0 {
        0.0
    } else {
        cold_total.median as f64 / warm_total.median as f64
    };
    println!(
        "{:<18} {:<18} cold {:>4}ms warm {:>4}ms x{:.2} | tachyon {:>4}->{:>4} | emit {:>4}->{:>4} | warm phase1 hits {}/{} | warm phase2 hits {:>2} | warm parallel invocations {:>2} | phase3 warm hits {}/{}",
        case_label,
        config_label,
        cold_total.median,
        warm_total.median,
        warm_speedup,
        cold_tachyon.median,
        warm_tachyon.median,
        cold_emit.median,
        warm_emit.median,
        samples.warm_phase1_artifact_hits,
        samples.warm_total_ms.len(),
        warm_phase2_hits.median,
        warm_parallel_invocations.median,
        samples.warm_phase3_memory_hits,
        samples.warm_total_ms.len(),
    );
}

fn print_incremental_case_line(
    case_label: &str,
    config_label: &str,
    bucket: &IncrementalReuseBucket,
) {
    let phase2_seed = summarize(&bucket.phase2_seed_ms);
    let phase2_hit = summarize(&bucket.phase2_hit_ms);
    let phase2_emit_hits = summarize(&bucket.phase2_hit_emit_hits);
    let phase2_parallel = summarize(&bucket.phase2_hit_parallel_stage_invocations);
    let phase3_seed = summarize(&bucket.phase3_seed_ms);
    let phase3_hit = summarize(&bucket.phase3_hit_ms);
    let phase3_parallel = summarize(&bucket.phase3_hit_parallel_stage_invocations);
    let phase2_speedup = if phase2_hit.median == 0 {
        0.0
    } else {
        phase2_seed.median as f64 / phase2_hit.median as f64
    };
    let phase3_speedup = if phase3_hit.median == 0 {
        0.0
    } else {
        phase3_seed.median as f64 / phase3_hit.median as f64
    };
    println!(
        "{:<18} {:<18} phase2 {:>4}->{:>4}ms x{:.2} | emit hits {:>2} | parallel invocations {:>2} || phase3 {:>4}->{:>4}ms x{:.2} | phase3 hits {}/{} | parallel invocations {:>2}",
        case_label,
        config_label,
        phase2_seed.median,
        phase2_hit.median,
        phase2_speedup,
        phase2_emit_hits.median,
        phase2_parallel.median,
        phase3_seed.median,
        phase3_hit.median,
        phase3_speedup,
        bucket.phase3_memory_hits,
        bucket.phase3_hit_ms.len(),
        phase3_parallel.median,
    );
}

#[test]
#[ignore = "perf matrix is intended for explicit local/CI runs"]
fn compile_mode_perf_matrix_reports_cold_and_warm_compile_time_axes() {
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_mode_perf_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create perf sandbox root");

    let repeats = parse_repeats();
    let cases = selected_cases();
    assert!(
        !cases.is_empty(),
        "no perf cases selected; check RR_COMPILE_MODE_PERF_CASES"
    );

    println!(
        "compile-mode perf matrix (opt=-O1, repeats={}, cases={}):",
        repeats,
        cases
            .iter()
            .map(|case| case.label)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut aggregate: BTreeMap<&'static str, SampleBucket> = BTreeMap::new();

    for case in cases {
        let rr_src = root.join(case.path);
        assert!(rr_src.exists(), "missing perf case {}", rr_src.display());
        for config in PERF_CONFIGS {
            let proj_dir = unique_dir(&sandbox_root, case.label);
            fs::create_dir_all(&proj_dir).expect("failed to create perf project dir");
            let mut samples = SampleBucket::default();

            for repeat in 0..repeats {
                let cache_dir = proj_dir.join(format!(
                    "cache_{}_{}",
                    config.label.replace('/', "_"),
                    repeat
                ));
                let out_dir =
                    proj_dir.join(format!("out_{}_{}", config.label.replace('/', "_"), repeat));
                fs::create_dir_all(&out_dir).expect("failed to create perf output dir");
                let cold_profile_path = proj_dir.join(format!(
                    "{}_{}_cold_{}.json",
                    case.label,
                    config.label.replace('/', "_"),
                    repeat
                ));
                let warm_profile_path = proj_dir.join(format!(
                    "{}_{}_warm_{}.json",
                    case.label,
                    config.label.replace('/', "_"),
                    repeat
                ));

                let cold = invoke_rr_build(
                    &rr_bin,
                    &rr_src,
                    *config,
                    &out_dir,
                    &cache_dir,
                    &cold_profile_path,
                );
                let warm = invoke_rr_build(
                    &rr_bin,
                    &rr_src,
                    *config,
                    &out_dir,
                    &cache_dir,
                    &warm_profile_path,
                );

                samples.cold_total_ms.push(cold.wall_ms);
                samples.warm_total_ms.push(warm.wall_ms);
                samples.cold_tachyon_ms.push(cold.tachyon_ms);
                samples.warm_tachyon_ms.push(warm.tachyon_ms);
                samples.cold_emit_ms.push(cold.emit_ms);
                samples.warm_emit_ms.push(warm.emit_ms);
                samples
                    .warm_phase2_emit_hits
                    .push(warm.phase2_emit_hits as u128);
                samples
                    .warm_parallel_stage_invocations
                    .push(warm.parallel_stage_invocations as u128);
                samples.warm_phase1_artifact_hits += usize::from(warm.phase1_artifact_hit);
                samples.warm_phase3_memory_hits += usize::from(warm.phase3_memory_hit);

                assert!(
                    !cold.phase1_artifact_hit,
                    "{} {} cold compile unexpectedly hit phase1 artifact cache",
                    case.label, config.label
                );
                assert!(
                    cold.phase2_emit_misses > 0 || cold.phase2_emit_hits > 0,
                    "{} {} cold compile should populate incremental accounting",
                    case.label,
                    config.label
                );
            }

            print_case_line(case.label, config.label, &samples);
            let aggregate_bucket = aggregate.entry(config.label).or_default();
            aggregate_bucket
                .cold_total_ms
                .extend(samples.cold_total_ms.iter().copied());
            aggregate_bucket
                .warm_total_ms
                .extend(samples.warm_total_ms.iter().copied());
            aggregate_bucket
                .cold_tachyon_ms
                .extend(samples.cold_tachyon_ms.iter().copied());
            aggregate_bucket
                .warm_tachyon_ms
                .extend(samples.warm_tachyon_ms.iter().copied());
            aggregate_bucket
                .cold_emit_ms
                .extend(samples.cold_emit_ms.iter().copied());
            aggregate_bucket
                .warm_emit_ms
                .extend(samples.warm_emit_ms.iter().copied());
            aggregate_bucket
                .warm_phase2_emit_hits
                .extend(samples.warm_phase2_emit_hits.iter().copied());
            aggregate_bucket
                .warm_parallel_stage_invocations
                .extend(samples.warm_parallel_stage_invocations.iter().copied());
            aggregate_bucket.warm_phase1_artifact_hits += samples.warm_phase1_artifact_hits;
            aggregate_bucket.warm_phase3_memory_hits += samples.warm_phase3_memory_hits;
        }
    }

    println!("\naggregate medians by config:");
    for config in PERF_CONFIGS {
        let samples = aggregate
            .get(config.label)
            .unwrap_or_else(|| panic!("missing aggregate samples for {}", config.label));
        let cold_total = summarize(&samples.cold_total_ms);
        let warm_total = summarize(&samples.warm_total_ms);
        let cold_tachyon = summarize(&samples.cold_tachyon_ms);
        let warm_tachyon = summarize(&samples.warm_tachyon_ms);
        let warm_phase2_hits = summarize(&samples.warm_phase2_emit_hits);
        let warm_parallel_invocations = summarize(&samples.warm_parallel_stage_invocations);
        let warm_speedup = if warm_total.median == 0 {
            0.0
        } else {
            cold_total.median as f64 / warm_total.median as f64
        };
        println!(
            "{:<18} cold {:>4}ms [{:>4}-{:>4}] warm {:>4}ms [{:>4}-{:>4}] x{:.2} | tachyon {:>4}->{:>4} | warm phase1 hits {}/{} | warm phase2 hits {:>2} | warm parallel invocations {:>2}",
            config.label,
            cold_total.median,
            cold_total.min,
            cold_total.max,
            warm_total.median,
            warm_total.min,
            warm_total.max,
            warm_speedup,
            cold_tachyon.median,
            warm_tachyon.median,
            samples.warm_phase1_artifact_hits,
            samples.warm_total_ms.len(),
            warm_phase2_hits.median,
            warm_parallel_invocations.median,
        );
    }
}

#[test]
#[ignore = "incremental reuse perf is intended for explicit local/CI runs"]
fn compile_mode_perf_matrix_reports_phase2_and_phase3_reuse_axes() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_mode_perf_matrix_reuse");
    fs::create_dir_all(&sandbox_root).expect("failed to create reuse perf sandbox root");

    let repeats = parse_repeats();
    let cases = selected_cases();
    assert!(
        !cases.is_empty(),
        "no perf cases selected; check RR_COMPILE_MODE_PERF_CASES"
    );

    let phase2_opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };
    let phase3_opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: false,
        phase3: true,
        strict_verify: false,
    };

    println!(
        "compile-mode incremental reuse matrix (opt=-O1, repeats={}, cases={}):",
        repeats,
        cases
            .iter()
            .map(|case| case.label)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut aggregate: BTreeMap<&'static str, IncrementalReuseBucket> = BTreeMap::new();

    for case in cases {
        let rr_src = root.join(case.path);
        let entry_input = fs::read_to_string(&rr_src)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", rr_src.display()));
        let entry_path = rr_src
            .to_str()
            .unwrap_or_else(|| panic!("non-unicode benchmark path {}", rr_src.display()))
            .to_string();

        for config in PERF_CONFIGS {
            let proj_dir = unique_dir(&sandbox_root, case.label);
            fs::create_dir_all(&proj_dir).expect("failed to create reuse perf project dir");
            let mut bucket = IncrementalReuseBucket::default();

            for repeat in 0..repeats {
                let phase2_cache_dir = proj_dir.join(format!(
                    "phase2_cache_{}_{}",
                    config.label.replace('/', "_"),
                    repeat
                ));
                common::set_env_var_for_test(
                    &env_guard,
                    "RR_INCREMENTAL_CACHE_DIR",
                    &phase2_cache_dir,
                );
                let (phase2_seed_out, phase2_seed) = invoke_incremental_compile(
                    &entry_path,
                    &entry_input,
                    *config,
                    phase2_opts,
                    None,
                );
                let (phase2_hit_out, phase2_hit) = invoke_incremental_compile(
                    &entry_path,
                    &entry_input,
                    *config,
                    phase2_opts,
                    None,
                );
                assert!(
                    phase2_seed_out.stats.phase2_emit_misses > 0,
                    "{} {} phase2 seed should populate emit cache",
                    case.label,
                    config.label
                );
                assert!(
                    phase2_hit_out.stats.phase2_emit_hits > 0,
                    "{} {} phase2 warm should hit emit cache",
                    case.label,
                    config.label
                );
                bucket.phase2_seed_ms.push(phase2_seed.wall_ms);
                bucket.phase2_hit_ms.push(phase2_hit.wall_ms);
                bucket
                    .phase2_hit_emit_hits
                    .push(phase2_hit_out.stats.phase2_emit_hits as u128);
                bucket
                    .phase2_hit_parallel_stage_invocations
                    .push(phase2_hit.parallel_stage_invocations as u128);

                let phase3_cache_dir = proj_dir.join(format!(
                    "phase3_cache_{}_{}",
                    config.label.replace('/', "_"),
                    repeat
                ));
                common::set_env_var_for_test(
                    &env_guard,
                    "RR_INCREMENTAL_CACHE_DIR",
                    &phase3_cache_dir,
                );
                let mut session = IncrementalSession::default();
                let (phase3_seed_out, phase3_seed) = invoke_incremental_compile(
                    &entry_path,
                    &entry_input,
                    *config,
                    phase3_opts,
                    Some(&mut session),
                );
                let (phase3_hit_out, phase3_hit) = invoke_incremental_compile(
                    &entry_path,
                    &entry_input,
                    *config,
                    phase3_opts,
                    Some(&mut session),
                );
                assert!(
                    !phase3_seed_out.stats.phase3_memory_hit,
                    "{} {} phase3 seed should miss memory cache",
                    case.label, config.label
                );
                assert!(
                    phase3_hit_out.stats.phase3_memory_hit,
                    "{} {} phase3 warm should hit in-memory artifact cache",
                    case.label, config.label
                );
                assert_eq!(
                    phase3_seed_out.r_code, phase3_hit_out.r_code,
                    "{} {} phase3 warm changed generated R",
                    case.label, config.label
                );
                bucket.phase3_seed_ms.push(phase3_seed.wall_ms);
                bucket.phase3_hit_ms.push(phase3_hit.wall_ms);
                bucket
                    .phase3_hit_parallel_stage_invocations
                    .push(phase3_hit.parallel_stage_invocations as u128);
                bucket.phase3_memory_hits += usize::from(phase3_hit_out.stats.phase3_memory_hit);
            }

            print_incremental_case_line(case.label, config.label, &bucket);
            let aggregate_bucket = aggregate.entry(config.label).or_default();
            aggregate_bucket
                .phase2_seed_ms
                .extend(bucket.phase2_seed_ms.iter().copied());
            aggregate_bucket
                .phase2_hit_ms
                .extend(bucket.phase2_hit_ms.iter().copied());
            aggregate_bucket
                .phase2_hit_emit_hits
                .extend(bucket.phase2_hit_emit_hits.iter().copied());
            aggregate_bucket
                .phase2_hit_parallel_stage_invocations
                .extend(bucket.phase2_hit_parallel_stage_invocations.iter().copied());
            aggregate_bucket
                .phase3_seed_ms
                .extend(bucket.phase3_seed_ms.iter().copied());
            aggregate_bucket
                .phase3_hit_ms
                .extend(bucket.phase3_hit_ms.iter().copied());
            aggregate_bucket
                .phase3_hit_parallel_stage_invocations
                .extend(bucket.phase3_hit_parallel_stage_invocations.iter().copied());
            aggregate_bucket.phase3_memory_hits += bucket.phase3_memory_hits;
        }
    }

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");

    println!("\naggregate reuse medians by config:");
    for config in PERF_CONFIGS {
        let bucket = aggregate
            .get(config.label)
            .unwrap_or_else(|| panic!("missing aggregate reuse samples for {}", config.label));
        let phase2_seed = summarize(&bucket.phase2_seed_ms);
        let phase2_hit = summarize(&bucket.phase2_hit_ms);
        let phase2_emit_hits = summarize(&bucket.phase2_hit_emit_hits);
        let phase3_seed = summarize(&bucket.phase3_seed_ms);
        let phase3_hit = summarize(&bucket.phase3_hit_ms);
        let phase2_speedup = if phase2_hit.median == 0 {
            0.0
        } else {
            phase2_seed.median as f64 / phase2_hit.median as f64
        };
        let phase3_speedup = if phase3_hit.median == 0 {
            0.0
        } else {
            phase3_seed.median as f64 / phase3_hit.median as f64
        };
        println!(
            "{:<18} phase2 {:>4}->{:>4}ms x{:.2} | emit hits {:>2} || phase3 {:>4}->{:>4}ms x{:.2} | phase3 hits {}/{}",
            config.label,
            phase2_seed.median,
            phase2_hit.median,
            phase2_speedup,
            phase2_emit_hits.median,
            phase3_seed.median,
            phase3_hit.median,
            phase3_speedup,
            bucket.phase3_memory_hits,
            bucket.phase3_hit_ms.len(),
        );
    }
}
