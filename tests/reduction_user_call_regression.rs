mod common;

use common::{compile_rr_env, normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::PathBuf;

#[test]
fn scalar_user_call_in_reduction_does_not_vectorize_unsafely() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping reduction user-call regression: Rscript not available.");
            return;
        }
    };

    let src = r#"
fn mix(v, tweak) {
  if ((v % 2L) == 0L) {
    return v + tweak;
  } else {
    return v - tweak;
  }
}

fn main() {
  let rows = 2L;
  let cols = 5L;
  let vals = seq_len(rows * cols);
  let m = matrix(vals, rows, cols);
  let rs = rowSums(m);
  let total = 0L;
  let i = 1L;
  while (i <= length(rs)) {
    total = total + mix(rs[i], 2L);
    i = i + 1L;
  }
  print(total);
  return total;
}

print(main());
"#;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("reduction_user_call_regression");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let rr_path = proj_dir.join("case.rr");
    fs::write(&rr_path, src).expect("failed to write rr source");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let mut outputs = Vec::new();
    for (flag, tag) in [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")] {
        let out_path = proj_dir.join(format!("case_{tag}.R"));
        compile_rr_env(
            &rr_bin,
            &rr_path,
            &out_path,
            flag,
            &[("RR_VERIFY_EACH_PASS", "1")],
        );
        let compiled_code = fs::read_to_string(&out_path).expect("failed to read emitted R");
        if tag != "o0" {
            assert!(
                !compiled_code.contains("sum(Sym_1(rs, 2L))"),
                "unsafe reduction vectorization reappeared at {flag}\n{}",
                compiled_code
            );
        }
        let result = run_rscript(&rscript, &out_path);
        assert_eq!(
            result.status, 0,
            "compiled R failed at {flag}\nstdout:\n{}\nstderr:\n{}",
            result.stdout, result.stderr
        );
        outputs.push((tag, result));
    }

    let base_stdout = normalize(&outputs[0].1.stdout);
    let base_stderr = normalize(&outputs[0].1.stderr);
    for (tag, result) in outputs.iter().skip(1) {
        assert_eq!(
            base_stdout,
            normalize(&result.stdout),
            "stdout mismatch at {tag}"
        );
        assert_eq!(
            base_stderr,
            normalize(&result.stderr),
            "stderr mismatch at {tag}"
        );
    }
}
