mod common;

use std::fs;
use std::path::PathBuf;

fn parse_pass_count(stderr: &str, pass: &str) -> i32 {
    let needle = format!("{pass} ");
    stderr
        .split('|')
        .find_map(|part| {
            let part = part.trim();
            let idx = part.find(&needle)?;
            let rest = &part[idx + needle.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<i32>().ok()
        })
        .unwrap_or(0)
}

#[test]
fn gvn_eliminates_duplicate_pure_builtin_calls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("gvn_pure_calls");
    fs::create_dir_all(&out_dir).expect("failed to create gvn test output dir");

    let source = r#"
fn kernel(x) {
  let a = floor(x + 1.25)

  let b = floor(x + 1.25)

  let c = sqrt(a)

  let d = sqrt(a)

  return b + c + d

}

print(kernel(4.5))

"#;

    let rr_src = out_dir.join("case.rr");
    let out_o0 = out_dir.join("case_o0.R");
    let out_o2 = out_dir.join("case_o2.R");
    fs::write(&rr_src, source).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    common::compile_rr(&rr_bin, &rr_src, &out_o0, "-O0");

    let (ok_o2, stdout_o2, stderr_o2) =
        common::run_compile_case("gvn_pure_calls", source, "case.rr", "-O2", &[]);
    assert!(
        ok_o2,
        "O2 compile failed\nstdout:\n{stdout_o2}\nstderr:\n{stderr_o2}"
    );
    let compile_log = format!("{stdout_o2}\n{stderr_o2}");
    assert!(
        parse_pass_count(&compile_log, "GVN") > 0,
        "expected GVN to eliminate duplicate pure calls\ncompile log:\n{compile_log}"
    );

    common::compile_rr(&rr_bin, &rr_src, &out_o2, "-O2");

    let rscript = common::rscript_path().expect("Rscript path should be configured");
    if !common::rscript_available(&rscript) {
        return;
    }

    let run_o0 = common::run_rscript(&rscript, &out_o0);
    let run_o2 = common::run_rscript(&rscript, &out_o2);
    assert_eq!(run_o0.status, 0, "O0 runtime failed: {}", run_o0.stderr);
    assert_eq!(run_o2.status, 0, "O2 runtime failed: {}", run_o2.stderr);
    assert_eq!(
        common::normalize(&run_o0.stdout),
        common::normalize(&run_o2.stdout),
        "O0/O2 output mismatch\nO0:\n{}\nO2:\n{}",
        run_o0.stdout,
        run_o2.stdout
    );
}

#[test]
fn gvn_eliminates_duplicate_vector_helper_calls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("gvn_vector_helper_calls");
    fs::create_dir_all(&out_dir).expect("failed to create gvn vector helper output dir");

    let source = r#"
fn kernel(field, n_l, n_r, size) {
    let out = rep.int(0.0, size)
    let i = 1.0
    let ii = 0.0
  while (i <= size) {
    ii = floor(i)
    out[ii] = (field[n_l[ii]] + field[n_r[ii]]) - field[n_l[ii]]
    i += 1.0
  }
  return out
}

let field = c(1.0, 2.0, 3.0, 4.0)
let n_l = c(1.0, 2.0, 3.0, 4.0)
let n_r = c(4.0, 3.0, 2.0, 1.0)
print(kernel(field, n_l, n_r, 4.0))
"#;

    let rr_src = out_dir.join("case.rr");
    let out_o0 = out_dir.join("case_o0.R");
    let out_o2 = out_dir.join("case_o2.R");
    fs::write(&rr_src, source).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    common::compile_rr(&rr_bin, &rr_src, &out_o0, "-O0");

    let (ok_o2, stdout_o2, stderr_o2) =
        common::run_compile_case("gvn_vector_helper_calls", source, "case.rr", "-O2", &[]);
    assert!(
        ok_o2,
        "O2 compile failed\nstdout:\n{stdout_o2}\nstderr:\n{stderr_o2}"
    );
    let compile_log = format!("{stdout_o2}\n{stderr_o2}");
    assert!(
        parse_pass_count(&compile_log, "GVN") > 0,
        "expected GVN to eliminate duplicate vector helper calls\ncompile log:\n{compile_log}"
    );

    common::compile_rr(&rr_bin, &rr_src, &out_o2, "-O2");

    let rscript = common::rscript_path().expect("Rscript path should be configured");
    if !common::rscript_available(&rscript) {
        return;
    }

    let run_o0 = common::run_rscript(&rscript, &out_o0);
    let run_o2 = common::run_rscript(&rscript, &out_o2);
    assert_eq!(run_o0.status, 0, "O0 runtime failed: {}", run_o0.stderr);
    assert_eq!(run_o2.status, 0, "O2 runtime failed: {}", run_o2.stderr);
    assert_eq!(
        common::normalize(&run_o0.stdout),
        common::normalize(&run_o2.stdout),
        "O0/O2 output mismatch\nO0:\n{}\nO2:\n{}",
        run_o0.stdout,
        run_o2.stdout
    );
}
