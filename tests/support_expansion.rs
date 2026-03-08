mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn unary_field_and_seq_len_for_are_supported() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping support expansion test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("support_expansion");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/support_expansion");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
fn t_unary_field() {
    let a = -5
    let b = !FALSE
    let rec = {x: 1, y: 2}
    rec.x = 10
    print(a)
    print(b)
    print(rec.x)
    print(rec.y)
    return rec.x + rec.y
}

fn t_seq_len_sum(n) {
    let s = 0
    for (i in seq_len(n)) {
        s = s + i
    }
    return s
}

print(t_unary_field())
print(t_seq_len_sum(5))
"#;

    let rr_path = out_dir.join("support_expansion.rr");
    fs::write(&rr_path, rr_src).expect("failed to write source");

    let o0 = out_dir.join("support_expansion_o0.R");
    let o1 = out_dir.join("support_expansion_o1.R");
    let o2 = out_dir.join("support_expansion_o2.R");

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

    let expected = "[1] -5\n[1] TRUE\n[1] 10\n[1] 2\n[1] 12\n[1] 15\n";
    assert_eq!(
        normalize(&base.stdout),
        expected,
        "unexpected baseline semantics"
    );
}
