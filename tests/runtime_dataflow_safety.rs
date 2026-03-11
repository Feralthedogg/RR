mod common;

use common::run_compile_case;

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case("runtime_dataflow_safety", source, file_name, "-O1", &[])
}

#[test]
fn range_and_dataflow_runtime_hazards_are_detected() {
    let src = r#"
fn bad(x) {
  let y = x[length(x) - length(x)]

  let z = 10L / (length(x) - length(x))

  let w = seq_len((length(x) - length(x)) - 1L)

  return z + length(w) + y

}

bad(c(1L, 2L, 3L))

"#;

    let (ok, stdout, _stderr) = run_compile(src, "runtime_dataflow.rr");
    assert!(!ok, "compile must fail");
    assert!(
        stdout.contains("division by zero is guaranteed by range/dataflow analysis"),
        "missing range/dataflow division diagnostic:\n{stdout}"
    );
    assert!(
        stdout.contains("origin:") && stdout.contains("constraint:") && stdout.contains("use:"),
        "expected labeled runtime diagnostics:\n{stdout}"
    );
    assert!(
        stdout.contains("fix:"),
        "expected runtime fix-it guidance:\n{stdout}"
    );
}
