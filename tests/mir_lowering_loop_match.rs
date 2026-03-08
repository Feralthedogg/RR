mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn break_next_and_list_pattern_match_lower_to_mir() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping MIR lowering loop/match test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("mir_lowering_loop_match");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/mir_lowering_loop_match");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
fn loop_ops(n) {
    let s = 0
    for (i in 1..n) {
        if (i == 3) {
            next
        }
        if (i == 6) {
            break
        }
        s = s + i
    }
    return s
}

fn match_list(v) {
    return match(v) {
        [a, b, ..rest] => a + b + length(rest),
        [x] => x,
        _ => 0
    }
}

fn match_record(v) {
    return match(v) {
        {a: x, b: y} => x + y,
        {a: x} => x,
        _ => 0
    }
}

print(loop_ops(10))
print(match_list([10, 20, 30, 40]))
print(match_list([7]))
print(match_list(NULL))
print(match_record({a: 3, b: 4}))
print(match_record({a: 9}))
print(match_record({c: 1}))
"#;

    let rr_path = out_dir.join("mir_lowering_loop_match.rr");
    fs::write(&rr_path, rr_src).expect("failed to write source");

    let o0 = out_dir.join("mir_lowering_loop_match_o0.R");
    let o1 = out_dir.join("mir_lowering_loop_match_o1.R");
    let o2 = out_dir.join("mir_lowering_loop_match_o2.R");

    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o1, "-O1");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let base = run_rscript(&rscript, &o0);
    let run_o1 = run_rscript(&rscript, &o1);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(base.status, run_o1.status, "status mismatch O0 vs O1");
    assert_eq!(base.status, run_o2.status, "status mismatch O0 vs O2");
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o1.stdout),
        "stdout mismatch O0 vs O1"
    );
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o1.stderr),
        "stderr mismatch O0 vs O1"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch O0 vs O2"
    );

    let expected = "[1] 12\n[1] 32\n[1] 7\n[1] 0\n[1] 7\n[1] 9\n[1] 0\n";
    assert_eq!(
        normalize(&base.stdout),
        expected,
        "unexpected baseline semantics"
    );
}
