mod common;

use common::run_compile_case;

fn build_large_ir_program() -> String {
    let mut src = String::new();
    src.push_str("fn main() {\n");
    src.push_str("  let v0 = 1L\n");
    for i in 1..3200 {
        src.push_str(&format!("  let v{} = v{} + 1L\n", i, i - 1));
    }
    src.push_str("  print(v3199)\n");
    src.push_str("  return v3199\n");
    src.push_str("}\n\n");
    src.push_str("main()\n");
    src
}

fn extract_metric(log: &str, key: &str) -> Option<usize> {
    let marker = format!("{} ", key);
    let start = log.find(&marker)?;
    let tail = &log[start + marker.len()..];
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<usize>().ok()
    }
}

fn extract_budget_limit(log: &str, marker: &str) -> Option<usize> {
    let start = log.find(marker)?;
    let tail = &log[start + marker.len()..];
    let slash = tail.find('/')?;
    let after = &tail[slash + 1..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<usize>().ok()
    }
}

#[test]
fn over_budget_skips_heavy_tier_when_selective_is_explicitly_disabled() {
    let source = build_large_ir_program();
    let (ok, stdout, stderr) = run_compile_case(
        "optimizer_budget_tiers",
        &source,
        "large_budget_case.rr",
        "-O2",
        &[
            ("RR_SELECTIVE_OPT_BUDGET", "0"),
            ("RR_ADAPTIVE_IR_BUDGET", "0"),
        ],
    );
    assert!(
        ok,
        "compile failed\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );
    let log = format!("{}\n{}", stdout, stderr);
    assert!(log.contains("Budget: IR"), "missing budget line:\n{}", log);
    let always = extract_metric(&log, "AlwaysFns").unwrap_or(0);
    let optimized = extract_metric(&log, "OptimizedFns").unwrap_or(usize::MAX);
    assert!(always > 0, "AlwaysFns must be > 0:\n{}", log);
    assert_eq!(
        optimized, 0,
        "heavy tier should be skipped by default over-budget path:\n{}",
        log
    );
}

#[test]
fn selective_budget_enables_heavy_tier_for_subset() {
    let source = build_large_ir_program();
    let (ok, stdout, stderr) = run_compile_case(
        "optimizer_budget_tiers",
        &source,
        "large_budget_case_selective.rr",
        "-O2",
        &[
            ("RR_SELECTIVE_OPT_BUDGET", "1"),
            ("RR_ADAPTIVE_IR_BUDGET", "0"),
        ],
    );
    assert!(
        ok,
        "compile failed\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );
    let log = format!("{}\n{}", stdout, stderr);
    assert!(
        log.contains(" | selective"),
        "selective marker missing:\n{}",
        log
    );
    let optimized = extract_metric(&log, "OptimizedFns").unwrap_or(0);
    assert!(
        optimized > 0,
        "selective heavy tier should optimize at least one function:\n{}",
        log
    );
}

#[test]
fn adaptive_budget_expands_limits_by_default() {
    let source = build_large_ir_program();
    let (ok, stdout, stderr) = run_compile_case(
        "optimizer_budget_tiers",
        &source,
        "large_budget_case_default_adaptive.rr",
        "-O2",
        &[],
    );
    assert!(
        ok,
        "compile failed\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );
    let log = format!("{}\n{}", stdout, stderr);
    let ir_limit = extract_budget_limit(&log, "Budget: IR ").unwrap_or(0);
    let fn_limit = extract_budget_limit(&log, " | MaxFn ").unwrap_or(0);
    let optimized = extract_metric(&log, "OptimizedFns").unwrap_or(0);
    let skipped = extract_metric(&log, "SkippedFns").unwrap_or(0);
    assert!(
        ir_limit > 2500,
        "adaptive budget should raise program IR cap above the fixed default:\n{}",
        log
    );
    assert!(
        fn_limit > 900,
        "adaptive budget should raise function IR cap above the fixed default:\n{}",
        log
    );
    assert!(optimized > 0, "optimized functions missing:\n{}", log);
    assert!(
        skipped > 0,
        "large single-function workloads should still be able to stay selective under the raised cap:\n{}",
        log
    );
}
