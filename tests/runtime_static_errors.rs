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
  if (NA) { return 1L } else { return 0L }
}
main()
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
    assert!(
        stdout.contains("fix: guard NA before branching, for example with is.na(...) checks"),
        "missing NA condition fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_divide_by_zero_must_fail() {
    let src = r#"
fn main() {
  return 1L / 0L
}
main()
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
    assert!(
        stdout.contains("fix: guard the divisor or clamp it away from zero before division"),
        "missing divide-by-zero fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_invalid_write_index_must_fail() {
    let src = r#"
fn main() {
  let x = c(1L, 2L, 3L)
  x[0L] <- 10L
  return x
}
main()
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
    assert!(
        stdout.contains("fix: shift the index into the 1-based domain before indexing"),
        "missing write-index fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_invalid_read_index_above_length_must_fail() {
    let src = r#"
fn main() {
  let x = c(1L, 2L, 3L)
  let i = length(x) + 1L
  return x[i]
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "bad_read_upper_index.rr");
    assert!(
        !ok,
        "compile must fail for guaranteed upper out-of-bounds read"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("> length(base)"),
        "missing upper-bound detail:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fix: clamp or guard the index against length(base) before reading"),
        "missing upper-bound fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_seq_len_negative_via_dataflow_must_fail() {
    let src = r#"
fn main() {
  let x = c(1L, 2L, 3L)
  let n = -(length(x) + 1L)
  return seq_len(n)
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "seq_len_negative_dataflow.rr");
    assert!(
        !ok,
        "compile must fail for dataflow-proven negative seq_len"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("seq_len() with negative length is guaranteed to fail"),
        "missing seq_len negative detail:\n{}",
        stdout
    );
    assert!(
        stdout.contains(
            "fix: clamp the length to 0 or prove it non-negative before calling seq_len()"
        ),
        "missing seq_len fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_negative_index_via_dataflow_must_fail() {
    let src = r#"
fn main() {
  let x = c(1L, 2L, 3L)
  let i = -(length(x))
  return x[i]
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "negative_index_dataflow.rr");
    assert!(!ok, "compile must fail for dataflow-proven negative index");
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("index is guaranteed out of bounds (must be >= 1)"),
        "missing negative index detail:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fix: shift the index into the 1-based domain before reading"),
        "missing negative index fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_negative_index_via_record_field_must_fail() {
    let src = r#"
fn main() {
  let x = c(1L, 2L, 3L)
  let rec = { i: -(length(x)) }
  return x[rec.i]
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "negative_index_record_field.rr");
    assert!(!ok, "compile must fail for record-field negative index");
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("out of bounds (must be >= 1)"),
        "missing negative index detail:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fix: shift the index into the 1-based domain before"),
        "missing negative index fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_negative_index_via_nested_record_field_must_fail() {
    let src = r#"
fn main() {
  let x = c(1L, 2L, 3L)
  let rec = { inner: { i: -(length(x)) } }
  return x[rec.inner.i]
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "negative_index_nested_record_field.rr");
    assert!(
        !ok,
        "compile must fail for nested record-field negative index"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("out of bounds (must be >= 1)"),
        "missing negative index detail:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fix: shift the index into the 1-based domain before"),
        "missing negative index fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_invalid_matrix_read_via_nested_record_field_must_fail() {
    let src = r#"
fn main() {
  let rec = { inner: { m: matrix(seq_len(6L), 2L, 3L) } }
  return rec.inner.m[3L, 1L]
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "matrix_nested_record_field_oob.rr");
    assert!(
        !ok,
        "compile must fail for nested record-field matrix oob read"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("matrix row index is guaranteed out of bounds"),
        "missing matrix row oob detail:\n{}",
        stdout
    );
    assert!(
        stdout
            .contains("fix: clamp or guard the row index against the matrix extent before reading"),
        "missing matrix row oob fix hint:\n{}",
        stdout
    );
}

#[test]
#[ignore = "branch-assigned record field detection requires Phi-merged const_eval, not yet implemented"]
fn static_negative_index_via_branch_assigned_record_field_must_fail() {
    let src = r#"
fn main(flag: bool) {
  let x = c(1L, 2L, 3L)
  let rec = { i: 1L }
  if (flag) {
    rec = { i: -1L }
  } else {
    rec = { i: -1L }
  }
  return x[rec.i]
}
main(TRUE)
"#;
    let (ok, stdout, _stderr) = run_compile(src, "negative_index_branch_record_field.rr");
    assert!(
        !ok,
        "compile must fail for branch-assigned record-field negative index"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("out of bounds (must be >= 1)"),
        "missing negative index detail:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fix: shift the index into the 1-based domain before"),
        "missing negative index fix hint:\n{}",
        stdout
    );
}

#[test]
fn static_invalid_matrix_read_via_branch_assigned_record_field_must_fail() {
    let src = r#"
fn main(flag: bool) {
  let rec = { m: matrix(seq_len(6L), 2L, 3L) }
  if (flag) {
    rec = { m: matrix(seq_len(6L), 2L, 3L) }
  } else {
    rec = { m: matrix(seq_len(6L), 2L, 3L) }
  }
  return rec.m[3L, 1L]
}
main(TRUE)
"#;
    let (ok, stdout, _stderr) = run_compile(src, "matrix_branch_record_field_oob.rr");
    assert!(
        !ok,
        "compile must fail for branch-assigned record-field matrix oob read"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("matrix row index is guaranteed out of bounds"),
        "missing matrix row oob detail:\n{}",
        stdout
    );
    assert!(
        stdout
            .contains("fix: clamp or guard the row index against the matrix extent before reading"),
        "missing matrix row oob fix hint:\n{}",
        stdout
    );
}

#[test]
fn multiple_static_runtime_errors_are_reported_together() {
    let src = r#"
fn main() {
  let x = c(1L, 2L)
  let y = x[0L]
  let z = 1L / 0L
  if (NA) { return 1L }
  return z + y
}
main()
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
  return "oops"
}
bad(1.0)
"#;
    let (ok, stdout, _stderr) = run_compile_strict(src, "strict_type_conflict.rr");
    assert!(!ok, "strict compile must fail for hint conflict");
    assert!(
        stdout.contains("E1010"),
        "missing strict type conflict error code:\n{}",
        stdout
    );
}

#[test]
fn strict_mode_rejects_two_dimensional_index_on_vector_hint() {
    let src = r#"
fn bad(a: vector<int>) -> int {
  return a[1, 1]
}
bad(c(1L, 2L, 3L))
"#;
    let (ok, stdout, _stderr) = run_compile_strict(src, "strict_matrix_base_conflict.rr");
    assert!(
        !ok,
        "strict compile must fail for 2D indexing on vector-typed base"
    );
    assert!(
        stdout.contains("E1002"),
        "missing strict matrix-base conflict error code:\n{}",
        stdout
    );
    assert!(
        stdout.contains("2D indexing requires matrix-typed base"),
        "missing strict matrix-base conflict detail:\n{}",
        stdout
    );
}

#[test]
fn static_invalid_matrix_read_index_above_dimension_must_fail() {
    let src = r#"
fn main() {
  let m = matrix(seq_len(4L), 2L, 2L)
  let i = nrow(m) + 1L
  return m[i, 1L]
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "bad_matrix_read_upper_index.rr");
    assert!(
        !ok,
        "compile must fail for guaranteed upper out-of-bounds matrix read"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("matrix row index is guaranteed out of bounds"),
        "missing matrix upper-bound detail:\n{}",
        stdout
    );
}

#[test]
fn static_invalid_matrix_write_index_above_dimension_must_fail() {
    let src = r#"
fn main() {
  let m = matrix(seq_len(4L), 2L, 2L)
  let j = ncol(m) + 1L
  m[1L, j] <- 99L
  return m
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "bad_matrix_write_upper_index.rr");
    assert!(
        !ok,
        "compile must fail for guaranteed upper out-of-bounds matrix write"
    );
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("matrix column assignment index is guaranteed out of bounds"),
        "missing matrix write upper-bound detail:\n{}",
        stdout
    );
}
