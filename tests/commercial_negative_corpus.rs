mod common;

use common::run_compile_case;

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case("commercial_negative_corpus", source, file_name, "-O2", &[])
}

#[test]
fn malformed_and_invalid_programs_fail_without_ice() {
    let corpus: [(&str, &str); 14] = [
        (
            "lex_bad_char",
            r#"
fn main() {
  let x = 1$
  return x
}
main()
"#,
        ),
        (
            "lex_unterminated_string",
            r#"
fn main() {
  let s = "abc
  return s
}
main()
"#,
        ),
        (
            "parse_missing_expr",
            r#"
fn main() {
  let x =
  return x
}
main()
"#,
        ),
        (
            "parse_same_line_statement_boundary",
            r#"
fn main() {
  let x = 1L return x
}
main()
"#,
        ),
        (
            "parse_unbalanced",
            r#"
fn main() {
  if (1L < 2L) {
    return 1L
  }
main()
"#,
        ),
        (
            "semantic_undef_var",
            r#"
fn main() {
  return nope + 1L
}
main()
"#,
        ),
        (
            "semantic_undef_fn",
            r#"
fn main() {
  return missing_fn(1L)
}
main()
"#,
        ),
        (
            "semantic_arity",
            r#"
fn f(a, b) {
  return a + b
}
fn main() {
  return f(1L)
}
main()
"#,
        ),
        (
            "runtime_static_na_cond",
            r#"
fn main() {
  if (NA) {
    return 1L
  }
  return 0L
}
main()
"#,
        ),
        (
            "runtime_static_div0",
            r#"
fn main() {
  return 1L / 0L
}
main()
"#,
        ),
        (
            "runtime_static_oob_write",
            r#"
fn main() {
  let x = c(1L, 2L)
  x[0L] = 3L
  return x
}
main()
"#,
        ),
        (
            "multi_semantic",
            r#"
fn main() {
  return a + b + c
}
main()
"#,
        ),
        (
            "multi_parse",
            r#"
fn main() {
  let x =
  let y =
  return
}
main()
"#,
        ),
        (
            "multi_runtime",
            r#"
fn main() {
  let x = c(1L, 2L)
  y <- x[0L]
  z <- 1L / 0L
  if (NA) {
    return 1L
  }
  return y + z
}
main()
"#,
        ),
    ];

    for (name, src) in corpus {
        let (ok, stdout, _stderr) = run_compile(src, &format!("{name}.rr"));
        assert!(!ok, "case '{name}' must fail");
        assert!(
            stdout.contains("** (RR."),
            "case '{name}' must print formatted RR error:\n{}",
            stdout
        );
        assert!(
            stdout.contains("error["),
            "case '{name}' must include machine-readable error code:\n{}",
            stdout
        );
        assert!(
            !stdout.contains("ICE9001") && !stdout.contains("RR.InternalError"),
            "case '{name}' hit internal compiler error unexpectedly:\n{}",
            stdout
        );
    }
}

#[test]
fn aggregate_diagnostics_report_all_relevant_findings() {
    let src = r#"
fn main() {
  x <- c(1L, 2L)
  y <- x[0L]
  z <- 1L / 0L
  if (NA) { return 1L }
  return y + z
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "aggregate_all.rr");
    assert!(!ok, "compile must fail");
    assert!(
        stdout.contains("found "),
        "aggregate diagnostics must include count:\n{}",
        stdout
    );
    assert!(
        stdout.contains("condition is statically NA"),
        "missing NA static runtime diagnostic:\n{}",
        stdout
    );
    assert!(
        stdout.contains("division by zero is guaranteed at compile-time"),
        "missing divide-by-zero diagnostic:\n{}",
        stdout
    );
    assert!(
        stdout.contains("out of bounds"),
        "missing index bounds diagnostic:\n{}",
        stdout
    );
}
