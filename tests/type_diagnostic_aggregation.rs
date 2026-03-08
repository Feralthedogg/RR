mod common;

use common::run_compile_case;

fn run_compile_strict(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case(
        "type_diagnostic_aggregation",
        source,
        file_name,
        "-O1",
        &[("RR_TYPE_MODE", "strict")],
    )
}

#[test]
fn strict_type_errors_aggregate_with_labeled_spans_and_fixits() {
    let src = r#"
fn bad_ret() -> float {
  return "oops";
}

fn expects_int(x: int) -> int {
  return x;
}

fn main() -> int {
  y <- expects_int("bad");
  return bad_ret() + y;
}

main();
"#;

    let (ok, stdout, _stderr) = run_compile_strict(src, "type_multi.rr");
    assert!(!ok, "strict compile must fail");
    assert!(
        stdout.contains("type checking failed"),
        "missing aggregate type header:\n{stdout}"
    );
    assert!(
        stdout.contains("found 3 error(s)"),
        "missing aggregate type count:\n{stdout}"
    );
    assert!(
        stdout.contains("E1010") && stdout.contains("E1011"),
        "expected both return-hint and call-signature diagnostics:\n{stdout}"
    );
    assert!(
        stdout.contains("origin:") && stdout.contains("constraint:") && stdout.contains("use:"),
        "expected origin/constraint/use labels in diagnostic output:\n{stdout}"
    );
    assert!(
        stdout.contains("fix:"),
        "expected fix-it guidance in diagnostic output:\n{stdout}"
    );
}
