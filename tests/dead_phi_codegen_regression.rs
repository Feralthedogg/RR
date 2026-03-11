mod common;

#[test]
fn top_level_compound_assign_with_dead_phi_compiles() {
    let source = "ff0000000000000009999-=0005536697820790000000000997818195\n";
    let (ok_o1, stdout_o1, stderr_o1) = common::run_compile_case(
        "dead_phi_codegen_regression",
        source,
        "case.rr",
        "-O1",
        &[("RR_STRICT_LET", "0")],
    );
    assert!(
        ok_o1,
        "O1 compile failed\nstdout:\n{stdout_o1}\nstderr:\n{stderr_o1}"
    );
    assert!(
        !stderr_o1.contains("Phi should be eliminated before codegen"),
        "unexpected codegen phi failure at O1\nstdout:\n{stdout_o1}\nstderr:\n{stderr_o1}"
    );

    let (ok_o2, stdout_o2, stderr_o2) = common::run_compile_case(
        "dead_phi_codegen_regression",
        source,
        "case.rr",
        "-O2",
        &[("RR_STRICT_LET", "0")],
    );
    assert!(
        ok_o2,
        "O2 compile failed\nstdout:\n{stdout_o2}\nstderr:\n{stderr_o2}"
    );
    assert!(
        !stderr_o2.contains("Phi should be eliminated before codegen"),
        "unexpected codegen phi failure at O2\nstdout:\n{stdout_o2}\nstderr:\n{stderr_o2}"
    );
}
