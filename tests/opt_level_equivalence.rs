mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn opt_levels_match_o0_semantics() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping opt-level equivalence tests: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("opt_levels");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/opt_levels");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let cases: [(&str, &str); 2] = [
        (
            "loop_inline_tco",
            r#"
fn inc(x) {
    return x + 1

}

fn test_sum(n) {
    let x = seq_along(n)

    let s = 0

    for (i in 1..length(x)) {
        s = s + inc(x[i])

    }
    return s

}

fn test_map(n) {
    let x = seq_along(n)

    let y = seq_along(n)

    for (i in 1..length(x)) {
        y[i] = x[i] * 2

    }
    return y

}

fn test_recursive_sum(n, acc) {
    if (n <= 0) {
        return acc

    } else {
        return test_recursive_sum(n - 1, acc + n)

    }
}

print(test_sum(10))

print(test_map(6))

print(test_recursive_sum(10, 0))

"#,
        ),
        (
            "branch_and_swap",
            r#"
fn branchy(n) {
    let a = 0

    let b = 1

    for (i in 1..n) {
        if (i <= 10) {
            a = a + i

        } else {
            b = b + i

        }
    }
    return a + b

}

fn swapn(n) {
    let a = 1

    let b = 2

    for (i in 1..n) {
        let t = a

        a = b

        b = t

    }
    print(a)

    print(b)

    return a + b

}

print(branchy(20))

print(swapn(7))

"#,
        ),
    ];

    for (name, src) in cases {
        let rr_path = out_dir.join(format!("{}.rr", name));
        fs::write(&rr_path, src).expect("failed to write case source");

        let o0 = out_dir.join(format!("{}_o0.R", name));
        let o1 = out_dir.join(format!("{}_o1.R", name));
        let o2 = out_dir.join(format!("{}_o2.R", name));

        compile_rr(&rr_bin, &rr_path, &o0, "-O0");
        compile_rr(&rr_bin, &rr_path, &o1, "-O1");
        compile_rr(&rr_bin, &rr_path, &o2, "-O2");

        let base = run_rscript(&rscript, &o0);
        let run_o1 = run_rscript(&rscript, &o1);
        let run_o2 = run_rscript(&rscript, &o2);

        assert_eq!(
            base.status, run_o1.status,
            "status mismatch in case {} between O0 and O1",
            name
        );
        assert_eq!(
            base.status, run_o2.status,
            "status mismatch in case {} between O0 and O2",
            name
        );
        assert_eq!(
            normalize(&base.stdout),
            normalize(&run_o1.stdout),
            "stdout mismatch in case {} between O0 and O1",
            name
        );
        assert_eq!(
            normalize(&base.stdout),
            normalize(&run_o2.stdout),
            "stdout mismatch in case {} between O0 and O2",
            name
        );
        assert_eq!(
            normalize(&base.stderr),
            normalize(&run_o1.stderr),
            "stderr mismatch in case {} between O0 and O1",
            name
        );
        assert_eq!(
            normalize(&base.stderr),
            normalize(&run_o2.stderr),
            "stderr mismatch in case {} between O0 and O2",
            name
        );

        // Semantic anchor for the known swap-sensitive case.
        if name == "branch_and_swap" {
            let expected = "[1] 211\n[1] 2\n[1] 1\n[1] 3\n";
            assert_eq!(
                normalize(&base.stdout),
                expected,
                "unexpected baseline semantics for case {}",
                name
            );
        }
    }
}
