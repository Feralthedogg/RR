mod common;

use common::unique_dir;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn build_perf_program() -> String {
    let mut src = String::new();

    for f in 0..40 {
        src.push_str(&format!("fn h{}(x) {{\n", f));
        src.push_str("  let t = x\n");
        for i in 0..10 {
            src.push_str(&format!("  let v{} = (t + {}L) * 2L\n", i, (i + f) % 9));
        }
        src.push_str("  return t + 1L\n");
        src.push_str("}\n\n");
    }

    src.push_str("fn giant(n) {\n");
    src.push_str("  let acc = 0L\n");
    src.push_str("  let i = 1L\n");
    src.push_str("  while (i <= n) {\n");
    src.push_str("    let t0 = i\n");
    for f in 0..40 {
        src.push_str(&format!("    let t{} = h{}(t{})\n", f + 1, f, f));
    }
    src.push_str("    acc = acc + t40\n");
    src.push_str("    i = i + 1L\n");
    src.push_str("  }\n");
    src.push_str("  return acc\n");
    src.push_str("}\n\n");

    src.push_str("fn main() {\n");
    src.push_str("  print(giant(64L))\n");
    src.push_str("}\n\n");
    src.push_str("main()\n");

    src
}

fn compile_elapsed_ms(rr_bin: &PathBuf, input: &PathBuf, output: &PathBuf, level: &str) -> u128 {
    let start = Instant::now();
    let status = Command::new(rr_bin)
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("--no-runtime")
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(status.success(), "compile failed for {}", level);
    start.elapsed().as_millis()
}

#[test]
fn compile_time_regression_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("perf_regression_gate");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join("perf_case.rr");
    let out_o1 = proj_dir.join("perf_o1.R");
    let out_o2 = proj_dir.join("perf_o2.R");
    fs::write(&rr_path, build_perf_program()).expect("failed to write perf case");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o1_ms = compile_elapsed_ms(&rr_bin, &rr_path, &out_o1, "-O1");
    let o2_ms = compile_elapsed_ms(&rr_bin, &rr_path, &out_o2, "-O2");

    let budget_ms: u128 = env::var("RR_PERF_GATE_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(45_000);
    let ratio_limit: f64 = env::var("RR_PERF_O2_O1_RATIO")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(12.0);
    let o2_vs_o1_limit = (o1_ms as f64 * ratio_limit + 1_500.0) as u128;

    assert!(
        o2_ms <= budget_ms,
        "compile-time budget exceeded: O2={}ms > budget={}ms (set RR_PERF_GATE_MS to tune)",
        o2_ms,
        budget_ms
    );
    assert!(
        o2_ms <= o2_vs_o1_limit,
        "O2 slowdown regression: O1={}ms, O2={}ms, limit={}ms (ratio={}, slack=1500ms)",
        o1_ms,
        o2_ms,
        o2_vs_o1_limit,
        ratio_limit
    );
}
