mod common;

#[test]
fn quiet_log_suppresses_pipeline_progress_output() {
    let source = r#"
fn main() {
  let x = 1
  print(x)
}
main()
"#;
    let (ok, stdout, stderr) = common::run_compile_case(
        "cli_quiet_log",
        source,
        "case.rr",
        "-O1",
        &[("RR_QUIET_LOG", "1")],
    );

    assert!(ok, "compile failed\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stdout.contains("RR Tachyon v"),
        "quiet mode should suppress banner\nstdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Source Analysis"),
        "quiet mode should suppress step logs\nstdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Tachyon Pulse Successful"),
        "quiet mode should suppress success summary\nstdout:\n{stdout}"
    );
}
