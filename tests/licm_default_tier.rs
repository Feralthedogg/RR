mod common;

use common::run_compile_case;

#[test]
fn licm_runs_by_default_on_compact_safe_loop() {
    let source = r#"
fn f(n) {
  let i = 0L

  let sum = 0L

  while (i < 8L) {
    let seed = n + 1L

    sum = sum + seed

    i = i + 1L

  }
  return sum

}
print(f(4L))

"#;

    let (ok, stdout, stderr) = run_compile_case(
        "licm_default_tier",
        source,
        "licm_default_tier.rr",
        "-O2",
        &[("RR_VERBOSE_LOG", "1")],
    );
    assert!(
        ok,
        "compile failed\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );
    let log = format!("{}\n{}", stdout, stderr);
    assert!(
        log.contains("LICM 1") || log.contains("LICM 2") || log.contains("LICM 3"),
        "expected LICM hit in default optimized pipeline:\n{}",
        log
    );
}
