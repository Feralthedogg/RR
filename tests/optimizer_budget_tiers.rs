mod common;

use common::run_compile_case;

const TEST_BASE_PROG_LIMIT: &str = "300";
const TEST_BASE_FN_LIMIT: &str = "80";

fn build_budget_fixture() -> String {
    let mut src = String::new();
    src.push_str("fn big() {\n");
    for i in 0..420 {
        src.push_str(&format!("  let v{} = {}L\n", i, i));
    }
    src.push_str("  return v419\n");
    src.push_str("}\n\n");
    for fn_idx in 0..8 {
        src.push_str(&format!("fn s{}() {{\n", fn_idx));
        for i in 0..40 {
            src.push_str(&format!("  let x{} = {}L\n", i, i));
        }
        src.push_str("  return x39\n");
        src.push_str("}\n\n");
    }
    src.push_str("fn main() {\n");
    src.push_str("  print(s0())\n");
    src.push_str("  return 0L\n");
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
fn legacy_budget_env_knobs_do_not_disable_current_budget_policy() {
    let source = build_budget_fixture();
    let (ok, stdout, stderr) = run_compile_case(
        "optimizer_budget_tiers",
        &source,
        "large_budget_case.rr",
        "-O2",
        &[
            ("RR_SELECTIVE_OPT_BUDGET", "0"),
            ("RR_ADAPTIVE_IR_BUDGET", "0"),
            ("RR_MAX_FULL_OPT_IR", TEST_BASE_PROG_LIMIT),
            ("RR_MAX_FULL_OPT_FN_IR", TEST_BASE_FN_LIMIT),
            ("RR_VERBOSE_LOG", "1"),
        ],
    );
    assert!(
        ok,
        "compile failed\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );
    let log = format!("{}\n{}", stdout, stderr);
    assert!(log.contains("Budget: IR"), "missing budget line:\n{}", log);
    let ir_limit = extract_budget_limit(&log, "Budget: IR ").unwrap_or(0);
    let fn_limit = extract_budget_limit(&log, " | MaxFn ").unwrap_or(0);
    let optimized = extract_metric(&log, "OptimizedFns").unwrap_or(0);
    assert!(
        ir_limit > TEST_BASE_PROG_LIMIT.parse::<usize>().unwrap_or(0),
        "legacy env knobs should not clamp the current program IR cap:\n{}",
        log
    );
    assert!(
        fn_limit > TEST_BASE_FN_LIMIT.parse::<usize>().unwrap_or(0),
        "legacy env knobs should not clamp the current function IR cap:\n{}",
        log
    );
    assert!(
        optimized > 0,
        "legacy env knobs should not disable heavy-tier optimization under the current fixed policy:\n{}",
        log
    );
}

#[test]
fn legacy_selective_budget_env_knobs_do_not_force_selective_mode() {
    let source = build_budget_fixture();
    let (ok, stdout, stderr) = run_compile_case(
        "optimizer_budget_tiers",
        &source,
        "large_budget_case_selective.rr",
        "-O2",
        &[
            ("RR_SELECTIVE_OPT_BUDGET", "1"),
            ("RR_ADAPTIVE_IR_BUDGET", "0"),
            ("RR_MAX_FULL_OPT_IR", TEST_BASE_PROG_LIMIT),
            ("RR_MAX_FULL_OPT_FN_IR", TEST_BASE_FN_LIMIT),
            ("RR_VERBOSE_LOG", "1"),
        ],
    );
    assert!(
        ok,
        "compile failed\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );
    let log = format!("{}\n{}", stdout, stderr);
    assert!(
        !log.contains(" | selective"),
        "legacy selective env knobs should not force the current optimizer into selective-marker mode:\n{}",
        log
    );
    let optimized = extract_metric(&log, "OptimizedFns").unwrap_or(0);
    assert!(
        optimized > 0,
        "legacy selective env knobs should not prevent the current optimizer from optimizing functions:\n{}",
        log
    );
}

#[test]
fn default_budget_policy_uses_raised_caps_for_large_fixture() {
    let source = build_budget_fixture();
    let (ok, stdout, stderr) = run_compile_case(
        "optimizer_budget_tiers",
        &source,
        "large_budget_case_default_adaptive.rr",
        "-O2",
        &[
            ("RR_MAX_FULL_OPT_IR", TEST_BASE_PROG_LIMIT),
            ("RR_MAX_FULL_OPT_FN_IR", TEST_BASE_FN_LIMIT),
            ("RR_VERBOSE_LOG", "1"),
        ],
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
    assert!(
        ir_limit > TEST_BASE_PROG_LIMIT.parse::<usize>().unwrap_or(0),
        "adaptive budget should raise program IR cap above the fixed default:\n{}",
        log
    );
    assert!(
        fn_limit > TEST_BASE_FN_LIMIT.parse::<usize>().unwrap_or(0),
        "adaptive budget should raise function IR cap above the fixed default:\n{}",
        log
    );
    assert!(optimized > 0, "optimized functions missing:\n{}", log);
}
