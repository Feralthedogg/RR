mod common;

use common::run_compile_case;

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case("runtime_static_errors", source, file_name, "-O1", &[])
}

fn run_compile_strict(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case(
        "runtime_static_errors",
        source,
        file_name,
        "-O1",
        &[("RR_TYPE_MODE", "strict")],
    )
}

#[test]
fn static_if_na_condition_must_fail() {
    let src = r#"
fn main() {
  if (NA) { return 1L; } else { return 0L; }
}
main();
"#;
    let (ok, stdout, _stderr) = run_compile(src, "if_na.rr");
    assert!(!ok, "compile must fail for statically NA condition");
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("condition is statically NA"),
        "missing NA condition detail:\n{}",
        stdout
    );
}

#[test]
fn static_divide_by_zero_must_fail() {
    let src = r#"
fn main() {
  return 1L / 0L;
}
main();
"#;
    let (ok, stdout, _stderr) = run_compile(src, "div_zero.rr");
    assert!(!ok, "compile must fail for guaranteed divide by zero");
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("division by zero is guaranteed at compile-time"),
        "missing divide-by-zero detail:\n{}",
        stdout
    );
}

#[test]
fn static_invalid_write_index_must_fail() {
    let src = r#"
fn main() {
  x <- c(1L, 2L, 3L);
  x[0L] <- 10L;
  return x;
}
main();
"#;
    let (ok, stdout, _stderr) = run_compile(src, "bad_write_index.rr");
    assert!(!ok, "compile must fail for statically invalid write index");
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("out of bounds"),
        "missing index-out-of-bounds detail:\n{}",
        stdout
    );
}

#[test]
fn multiple_static_runtime_errors_are_reported_together() {
    let src = r#"
fn main() {
  x <- c(1L, 2L);
  y <- x[0L];
  z <- 1L / 0L;
  if (NA) { return 1L; }
  return z + y;
}
main();
"#;
    let (ok, stdout, _stderr) = run_compile(src, "runtime_multi.rr");
    assert!(!ok, "compile must fail");
    assert!(
        stdout.contains("runtime safety validation failed"),
        "missing aggregate runtime header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("found "),
        "missing aggregate count:\n{}",
        stdout
    );
    assert!(
        stdout.contains("condition is statically NA"),
        "missing NA condition error:\n{}",
        stdout
    );
    assert!(
        stdout.contains("division by zero is guaranteed at compile-time"),
        "missing division-by-zero error:\n{}",
        stdout
    );
    assert!(
        stdout.contains("out of bounds"),
        "missing index error:\n{}",
        stdout
    );
}

#[test]
fn strict_mode_reports_type_hint_conflict() {
    let src = r#"
fn bad(a: float) -> float {
  return "oops";
}
bad(1.0);
"#;
    let (ok, stdout, _stderr) = run_compile_strict(src, "strict_type_conflict.rr");
    assert!(!ok, "strict compile must fail for hint conflict");
    assert!(
        stdout.contains("E1010"),
        "missing strict type conflict error code:\n{}",
        stdout
    );
}
