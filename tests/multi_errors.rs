mod common;

use common::run_compile_case;

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case("multi_errors", source, file_name, "-O1", &[])
}

#[test]
fn semantic_errors_are_reported_together() {
    let src = r#"
fn main() {
  return a + b
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "semantic_multi.rr");
    assert!(!ok, "compile must fail");
    assert!(
        stdout.contains("semantic validation failed"),
        "missing aggregate header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("found "),
        "missing aggregate count:\n{}",
        stdout
    );
    assert!(
        stdout.contains("undefined variable 'a'"),
        "missing undefined variable a:\n{}",
        stdout
    );
    assert!(
        stdout.contains("undefined variable 'b'"),
        "missing undefined variable b:\n{}",
        stdout
    );
}
