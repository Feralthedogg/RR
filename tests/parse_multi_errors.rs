mod common;

use common::run_compile_case;

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case("parse_multi_errors", source, file_name, "-O1", &[])
}

#[test]
fn parse_errors_are_reported_together() {
    let src = r#"
fn main() {
  let x = 1$
  let y =
  return x +
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "parse_multi.rr");
    assert!(!ok, "compile must fail");
    assert!(
        stdout.contains("parse failed"),
        "missing aggregate parse header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("found "),
        "missing aggregate count:\n{}",
        stdout
    );
}
