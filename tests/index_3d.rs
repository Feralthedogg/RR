mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn three_dimensional_indexing_matches_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D indexing test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("index_3d");
    fs::create_dir_all(&out_dir).expect("failed to create 3D test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn main() {
  let a = base.array(seq_len(8), base.c(2, 2, 2))
  print(a[1, 1, 1])
  print(a[2, 1, 2])

  a[2, 2, 2] = 99
  a[1, 2, 1] = a[2, 1, 2] + 10

  print(a[1, 2, 1])
  print(a[2, 2, 2])
  return a[1, 2, 1] + a[2, 2, 2]
}

print(main())
"#;

    let rr_path = out_dir.join("index_3d.rr");
    fs::write(&rr_path, rr_src).expect("failed to write 3D source");

    let o0 = out_dir.join("index_3d_o0.R");
    let o1 = out_dir.join("index_3d_o1.R");
    let o2 = out_dir.join("index_3d_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o1, "-O1");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let o2_code = fs::read_to_string(&o2).expect("failed to read emitted O2 code");
    assert!(
        (o2_code.contains("rr_mark(9, 3);") || o2_code.contains("rr_mark(9L, 3L);"))
            && o2_code.contains("a[2L, 2L, 2L] <- 99L")
            && o2_code.contains("a[1L, 2L, 1L] <- (a[2L, 1L, 2L] + 10L)"),
        "expected emitted R to contain 3D read/write lowering:\n{}",
        o2_code
    );

    let base = run_rscript(&rscript, &o0);
    let run_o1 = run_rscript(&rscript, &o1);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(base.status, 0, "O0 runtime failed:\n{}", base.stderr);
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o1.stdout),
        "stdout mismatch between O0 and O1"
    );
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch between O0 and O2"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o1.stderr),
        "stderr mismatch between O0 and O1"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch between O0 and O2"
    );
    assert_eq!(
        normalize(&base.stdout),
        "[1] 1\n[1] 6\n[1] 16\n[1] 99\n[1] 115\n",
        "unexpected 3D baseline output"
    );
}

#[test]
fn statically_invalid_three_dimensional_index_must_fail() {
    let src = r#"
import r * as base from "base"

fn main() {
  let a = base.array(seq_len(8), base.c(2, 2, 2))
  return a[0, 1, 1]
}

main()
"#;

    let (ok, stdout, _stderr) = run_compile_case(
        "index_3d",
        src,
        "bad_index_3d.rr",
        "-O1",
        &[("RR_TYPE_MODE", "strict")],
    );
    assert!(!ok, "compile must fail for statically invalid 3D index");
    assert!(
        stdout.contains("** (RR.RuntimeError)"),
        "missing runtime error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("out of bounds"),
        "missing 3D index-out-of-bounds detail:\n{}",
        stdout
    );
}
