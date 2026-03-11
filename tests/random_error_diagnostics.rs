mod common;

use common::random_error_cases::{generate_cases, suite_summary};
use common::{normalize, run_compile_case};
use std::env;

fn env_case_count() -> usize {
    env::var("RR_RANDOM_ERROR_CASE_COUNT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v >= 5)
        .unwrap_or(20)
}

fn env_seed() -> u64 {
    env::var("RR_RANDOM_ERROR_SEED")
        .ok()
        .and_then(|v| {
            u64::from_str_radix(v.trim_start_matches("0x"), 16)
                .ok()
                .or_else(|| v.parse::<u64>().ok())
        })
        .unwrap_or(0xE220_6D1A_600D_F00D)
}

#[test]
fn generated_invalid_programs_emit_expected_diagnostics_across_opt_levels() {
    let case_count = env_case_count();
    let cases = generate_cases(env_seed(), case_count);
    assert_eq!(
        cases.len(),
        case_count,
        "generator returned unexpected case count"
    );

    for case in &cases {
        let file_name = format!("{}.rr", case.name);
        let env_kv = if case.strict_let {
            vec![("RR_STRICT_LET", "1")]
        } else {
            Vec::new()
        };

        for opt_flag in ["-O0", "-O1", "-O2"] {
            let (ok, stdout, stderr) = run_compile_case(
                "random_error_diagnostics",
                &case.rr_src,
                &file_name,
                opt_flag,
                &env_kv,
            );
            let stdout = normalize(&stdout);
            let stderr = normalize(&stderr);

            assert!(
                !ok,
                "invalid case unexpectedly compiled: {} ({})\nsource:\n{}\nstdout:\n{}\nstderr:\n{}",
                case.name, opt_flag, case.rr_src, stdout, stderr
            );
            assert!(
                stdout.contains(&format!("** ({})", case.expected_module)),
                "wrong diagnostic module for {} ({})\nexpected: {}\nstdout:\n{}",
                case.name,
                opt_flag,
                case.expected_module,
                stdout
            );
            assert!(
                stdout.contains(&case.expected_message_fragment),
                "missing diagnostic fragment for {} ({})\nexpected: {}\nstdout:\n{}",
                case.name,
                opt_flag,
                case.expected_message_fragment,
                stdout
            );
            if let Some(help_fragment) = &case.expected_help_fragment {
                assert!(
                    stdout.contains("help:") && stdout.contains(help_fragment),
                    "missing suggestion for {} ({})\nexpected help fragment: {}\nstdout:\n{}",
                    case.name,
                    opt_flag,
                    help_fragment,
                    stdout
                );
            }
            assert!(
                !stdout.contains("RR.InternalError"),
                "case produced internal compiler error: {} ({})\nstdout:\n{}",
                case.name,
                opt_flag,
                stdout
            );
            assert!(
                !stderr.to_ascii_lowercase().contains("panicked"),
                "case panicked: {} ({})\nstderr:\n{}",
                case.name,
                opt_flag,
                stderr
            );
        }
    }
}

#[test]
fn generated_invalid_suite_covers_multiple_error_families() {
    let cases = generate_cases(0xBAD5_EED5_1234_5678, 15);
    let summary = suite_summary(&cases);
    assert!(summary.contains("undefined_variable"));
    assert!(summary.contains("undefined_function"));
    assert!(summary.contains("arity_mismatch"));
    assert!(summary.contains("strict_let"));
    assert!(summary.contains("parse_missing_brace"));
}
