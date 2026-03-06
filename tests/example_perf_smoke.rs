mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

struct PerfCase {
    label: &'static str,
    path: &'static str,
}

const PERF_CASES: &[PerfCase] = &[
    PerfCase {
        label: "bootstrap_resample_bench",
        path: "example/benchmarks/bootstrap_resample_bench.rr",
    },
    PerfCase {
        label: "heat_diffusion_bench",
        path: "example/benchmarks/heat_diffusion_bench.rr",
    },
    PerfCase {
        label: "orbital_sweep_bench",
        path: "example/benchmarks/orbital_sweep_bench.rr",
    },
    PerfCase {
        label: "reaction_diffusion_bench",
        path: "example/benchmarks/reaction_diffusion_bench.rr",
    },
    PerfCase {
        label: "vector_fusion_bench",
        path: "example/benchmarks/vector_fusion_bench.rr",
    },
    PerfCase {
        label: "tesseract",
        path: "example/tesseract.rr",
    },
];

fn compile_rr_timed(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) -> u128 {
    let started = Instant::now();
    let output = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg("--no-runtime")
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

fn runtime_o2_timed(rscript: &str, script: &Path) -> (u128, String, String, i32) {
    let started = Instant::now();
    let run = run_rscript(rscript, script);
    (
        started.elapsed().as_millis(),
        normalize(&run.stdout),
        normalize(&run.stderr),
        run.status,
    )
}

fn parse_env_limit_ms(key: &str) -> Option<u128> {
    env::var(key).ok()?.trim().parse::<u128>().ok()
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

    let mut total_compile_o1_ms = 0u128;
    let mut total_compile_o2_ms = 0u128;
    let mut total_runtime_o2_ms = 0u128;
    let mut max_runtime_o2_ms = 0u128;
    let mut max_runtime_o2_label = "";

    println!("example perf smoke:");
    for case in PERF_CASES {
        let rr_src = root.join(case.path);
        assert!(rr_src.exists(), "missing perf smoke case {}", rr_src.display());

        let out_o1 = out_dir.join(format!("{}_o1.R", case.label));
        let out_o2 = out_dir.join(format!("{}_o2.R", case.label));

        let compile_o1_ms = compile_rr_timed(&rr_bin, &rr_src, &out_o1, "-O1");
        let compile_o2_ms = compile_rr_timed(&rr_bin, &rr_src, &out_o2, "-O2");
        let (runtime_o2_ms, stdout, stderr, status) = runtime_o2_timed(&rscript, &out_o2);

        assert!(
            status == 0,
            "example perf smoke runtime failed for {}:\nstdout={stdout}\nstderr={stderr}",
            rr_src.display()
        );
        assert!(
            !stdout.trim().is_empty(),
            "example perf smoke runtime produced empty stdout for {}",
            rr_src.display()
        );

        total_compile_o1_ms += compile_o1_ms;
        total_compile_o2_ms += compile_o2_ms;
        total_runtime_o2_ms += runtime_o2_ms;
        if runtime_o2_ms > max_runtime_o2_ms {
            max_runtime_o2_ms = runtime_o2_ms;
            max_runtime_o2_label = case.label;
        }

        println!(
            "  {:>28} | compile O1 {:>5} ms | compile O2 {:>5} ms | runtime O2 {:>5} ms",
            case.label, compile_o1_ms, compile_o2_ms, runtime_o2_ms
        );
    }

    println!(
        "  {:>28} | compile O1 {:>5} ms | compile O2 {:>5} ms | runtime O2 {:>5} ms",
        "TOTAL", total_compile_o1_ms, total_compile_o2_ms, total_runtime_o2_ms
    );

    if let Some(limit) = parse_env_limit_ms("RR_EXAMPLE_PERF_TOTAL_COMPILE_O2_MS") {
        assert!(
            total_compile_o2_ms <= limit,
            "example perf compile budget exceeded: O2 total={}ms > limit={}ms",
            total_compile_o2_ms,
            limit
        );
    }
    if let Some(limit) = parse_env_limit_ms("RR_EXAMPLE_PERF_TOTAL_RUNTIME_O2_MS") {
        assert!(
            total_runtime_o2_ms <= limit,
            "example perf runtime budget exceeded: O2 total={}ms > limit={}ms",
            total_runtime_o2_ms,
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
