mod common;

use common::{normalize, rscript_available, rscript_path};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

struct PerfCase {
    label: &'static str,
    path: &'static str,
    require_exact_output_parity: bool,
}

const PERF_CASES: &[PerfCase] = &[
    PerfCase {
        label: "bootstrap_resample_bench",
        path: "example/benchmarks/bootstrap_resample_bench.rr",
        require_exact_output_parity: true,
    },
    PerfCase {
        label: "heat_diffusion_bench",
        path: "example/benchmarks/heat_diffusion_bench.rr",
        require_exact_output_parity: true,
    },
    PerfCase {
        label: "orbital_sweep_bench",
        path: "example/benchmarks/orbital_sweep_bench.rr",
        require_exact_output_parity: true,
    },
    PerfCase {
        label: "reaction_diffusion_bench",
        path: "example/benchmarks/reaction_diffusion_bench.rr",
        require_exact_output_parity: true,
    },
    PerfCase {
        label: "vector_fusion_bench",
        path: "example/benchmarks/vector_fusion_bench.rr",
        require_exact_output_parity: true,
    },
    PerfCase {
        label: "tesseract",
        path: "example/tesseract.rr",
        require_exact_output_parity: false,
    },
];

#[derive(Clone, Copy, Default)]
struct SampleStats {
    median_ms: u128,
    iqr_ms: u128,
}

#[derive(Default)]
struct MetricSamples {
    compile_o1_ms: Vec<u128>,
    compile_o2_ms: Vec<u128>,
    runtime_o0_ms: Vec<u128>,
    runtime_o1_ms: Vec<u128>,
    runtime_o2_ms: Vec<u128>,
}

struct PerfSummary {
    compile_o1: SampleStats,
    compile_o2: SampleStats,
    runtime_o0: SampleStats,
    runtime_o1: SampleStats,
    runtime_o2: SampleStats,
}

fn compile_rr_timed(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) -> u128 {
    let started = Instant::now();
    let output = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg(level)
        .output()
        .expect("failed to run RR compiler");
    let elapsed = started.elapsed().as_millis();
    assert!(
        output.status.success(),
        "RR compile failed for {} ({level})\nstdout={}\nstderr={}",
        rr_src.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    elapsed
}

fn runtime_timed(rscript: &str, script: &Path) -> (u128, String, String, i32) {
    let started = Instant::now();
    let output = Command::new(rscript)
        .arg("--vanilla")
        .arg(script)
        .env("RR_RUNTIME_MODE", "release")
        .env("RR_ENABLE_MARKS", "0")
        .output()
        .expect("failed to execute Rscript");
    (
        started.elapsed().as_millis(),
        normalize(&String::from_utf8_lossy(&output.stdout)),
        normalize(&String::from_utf8_lossy(&output.stderr)),
        output.status.code().unwrap_or(-1),
    )
}

fn parse_env_limit_ms(key: &str) -> Option<u128> {
    env::var(key).ok()?.trim().parse::<u128>().ok()
}

fn parse_env_repeats(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn summarize(samples: &[u128]) -> SampleStats {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let len = sorted.len();
    let median_ms = if len % 2 == 1 {
        sorted[len / 2]
    } else {
        (sorted[(len / 2) - 1] + sorted[len / 2]) / 2
    };
    let q1 = sorted[(len - 1) / 4];
    let q3 = sorted[((len - 1) * 3) / 4];
    SampleStats {
        median_ms,
        iqr_ms: q3.saturating_sub(q1),
    }
}

impl MetricSamples {
    fn summarize(&self) -> PerfSummary {
        PerfSummary {
            compile_o1: summarize(&self.compile_o1_ms),
            compile_o2: summarize(&self.compile_o2_ms),
            runtime_o0: summarize(&self.runtime_o0_ms),
            runtime_o1: summarize(&self.runtime_o1_ms),
            runtime_o2: summarize(&self.runtime_o2_ms),
        }
    }
}

#[test]
#[ignore = "perf smoke is intended for explicit local/CI runs"]
fn example_perf_smoke_reports_compile_and_runtime() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping example perf smoke: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root.join("target").join("example_perf_smoke");
    fs::create_dir_all(&out_dir).expect("failed to create perf smoke output dir");
    let repeats = parse_env_repeats("RR_EXAMPLE_PERF_REPEATS", 3);

    let mut case_samples: Vec<MetricSamples> = PERF_CASES
        .iter()
        .map(|_| MetricSamples::default())
        .collect();
    let mut total_samples = MetricSamples::default();
    let mut max_runtime_o2_ms = 0u128;
    let mut max_runtime_o2_label = "";

    println!("example perf smoke ({repeats} repeats, median/iqr in ms):");
    for round in 0..repeats {
        let mut round_total_compile_o1_ms = 0u128;
        let mut round_total_compile_o2_ms = 0u128;
        let mut round_total_runtime_o0_ms = 0u128;
        let mut round_total_runtime_o1_ms = 0u128;
        let mut round_total_runtime_o2_ms = 0u128;

        for (idx, case) in PERF_CASES.iter().enumerate() {
            let rr_src = root.join(case.path);
            assert!(
                rr_src.exists(),
                "missing perf smoke case {}",
                rr_src.display()
            );

            let out_o0 = out_dir.join(format!("{}_round{}_o0.R", case.label, round));
            let out_o1 = out_dir.join(format!("{}_round{}_o1.R", case.label, round));
            let out_o2 = out_dir.join(format!("{}_round{}_o2.R", case.label, round));

            let _compile_o0_ms = compile_rr_timed(&rr_bin, &rr_src, &out_o0, "-O0");
            let compile_o1_ms = compile_rr_timed(&rr_bin, &rr_src, &out_o1, "-O1");
            let compile_o2_ms = compile_rr_timed(&rr_bin, &rr_src, &out_o2, "-O2");

            let (runtime_o0_ms, stdout_o0, stderr_o0, status_o0) = runtime_timed(&rscript, &out_o0);
            let (runtime_o1_ms, stdout_o1, stderr_o1, status_o1) = runtime_timed(&rscript, &out_o1);
            let (runtime_o2_ms, stdout_o2, stderr_o2, status_o2) = runtime_timed(&rscript, &out_o2);

            assert!(
                status_o0 == 0,
                "example perf smoke runtime failed for {} at -O0:\nstdout={stdout_o0}\nstderr={stderr_o0}",
                rr_src.display()
            );
            assert!(
                status_o1 == 0,
                "example perf smoke runtime failed for {} at -O1:\nstdout={stdout_o1}\nstderr={stderr_o1}",
                rr_src.display()
            );
            assert!(
                status_o2 == 0,
                "example perf smoke runtime failed for {} at -O2:\nstdout={stdout_o2}\nstderr={stderr_o2}",
                rr_src.display()
            );
            assert!(
                !stdout_o0.trim().is_empty(),
                "example perf smoke runtime produced empty stdout for {}",
                rr_src.display()
            );
            if case.require_exact_output_parity {
                assert_eq!(
                    stdout_o1,
                    stdout_o0,
                    "example perf smoke output mismatch for {} between -O0 and -O1",
                    rr_src.display()
                );
                assert_eq!(
                    stdout_o2,
                    stdout_o0,
                    "example perf smoke output mismatch for {} between -O0 and -O2",
                    rr_src.display()
                );
                assert_eq!(
                    stderr_o1,
                    stderr_o0,
                    "example perf smoke stderr mismatch for {} between -O0 and -O1",
                    rr_src.display()
                );
                assert_eq!(
                    stderr_o2,
                    stderr_o0,
                    "example perf smoke stderr mismatch for {} between -O0 and -O2",
                    rr_src.display()
                );
            } else {
                assert!(
                    !stdout_o1.trim().is_empty() && !stdout_o2.trim().is_empty(),
                    "example perf smoke runtime produced empty optimized stdout for {}",
                    rr_src.display()
                );
            }

            case_samples[idx].compile_o1_ms.push(compile_o1_ms);
            case_samples[idx].compile_o2_ms.push(compile_o2_ms);
            case_samples[idx].runtime_o0_ms.push(runtime_o0_ms);
            case_samples[idx].runtime_o1_ms.push(runtime_o1_ms);
            case_samples[idx].runtime_o2_ms.push(runtime_o2_ms);

            round_total_compile_o1_ms += compile_o1_ms;
            round_total_compile_o2_ms += compile_o2_ms;
            round_total_runtime_o0_ms += runtime_o0_ms;
            round_total_runtime_o1_ms += runtime_o1_ms;
            round_total_runtime_o2_ms += runtime_o2_ms;
        }

        total_samples.compile_o1_ms.push(round_total_compile_o1_ms);
        total_samples.compile_o2_ms.push(round_total_compile_o2_ms);
        total_samples.runtime_o0_ms.push(round_total_runtime_o0_ms);
        total_samples.runtime_o1_ms.push(round_total_runtime_o1_ms);
        total_samples.runtime_o2_ms.push(round_total_runtime_o2_ms);
    }

    for (case, samples) in PERF_CASES.iter().zip(case_samples.iter()) {
        let summary = samples.summarize();
        if summary.runtime_o2.median_ms > max_runtime_o2_ms {
            max_runtime_o2_ms = summary.runtime_o2.median_ms;
            max_runtime_o2_label = case.label;
        }

        println!(
            "  {:>28} | cO1 {:>5}/{:<4} ms | cO2 {:>5}/{:<4} ms | rO0 {:>5}/{:<4} ms | rO1 {:>5}/{:<4} ms | rO2 {:>5}/{:<4} ms",
            case.label,
            summary.compile_o1.median_ms,
            summary.compile_o1.iqr_ms,
            summary.compile_o2.median_ms,
            summary.compile_o2.iqr_ms,
            summary.runtime_o0.median_ms,
            summary.runtime_o0.iqr_ms,
            summary.runtime_o1.median_ms,
            summary.runtime_o1.iqr_ms,
            summary.runtime_o2.median_ms,
            summary.runtime_o2.iqr_ms
        );
    }

    let total_summary = total_samples.summarize();
    println!(
        "  {:>28} | cO1 {:>5}/{:<4} ms | cO2 {:>5}/{:<4} ms | rO0 {:>5}/{:<4} ms | rO1 {:>5}/{:<4} ms | rO2 {:>5}/{:<4} ms",
        "TOTAL",
        total_summary.compile_o1.median_ms,
        total_summary.compile_o1.iqr_ms,
        total_summary.compile_o2.median_ms,
        total_summary.compile_o2.iqr_ms,
        total_summary.runtime_o0.median_ms,
        total_summary.runtime_o0.iqr_ms,
        total_summary.runtime_o1.median_ms,
        total_summary.runtime_o1.iqr_ms,
        total_summary.runtime_o2.median_ms,
        total_summary.runtime_o2.iqr_ms
    );

    if let Some(limit) = parse_env_limit_ms("RR_EXAMPLE_PERF_TOTAL_COMPILE_O2_MS") {
        assert!(
            total_summary.compile_o2.median_ms <= limit,
            "example perf compile budget exceeded: O2 total median={}ms > limit={}ms",
            total_summary.compile_o2.median_ms,
            limit
        );
    }
    if let Some(limit) = parse_env_limit_ms("RR_EXAMPLE_PERF_TOTAL_RUNTIME_O2_MS") {
        assert!(
            total_summary.runtime_o2.median_ms <= limit,
            "example perf runtime budget exceeded: O2 total median={}ms > limit={}ms",
            total_summary.runtime_o2.median_ms,
            limit
        );
    }
    if let Some(limit) = parse_env_limit_ms("RR_EXAMPLE_PERF_MAX_CASE_RUNTIME_O2_MS") {
        assert!(
            max_runtime_o2_ms <= limit,
            "example perf per-case runtime budget exceeded: {} O2 runtime={}ms > limit={}ms",
            max_runtime_o2_label,
            max_runtime_o2_ms,
            limit
        );
    }
}
