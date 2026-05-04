use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg(level)
        .arg("--no-incremental")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_src.display(),
        level
    );
}

fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    Some("Rscript".to_string())
}

fn rscript_available(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_rscript(path: &str, script: &Path) -> (i32, String, String) {
    let output = Command::new(path)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn lt_bound_map_vectorizes_as_slice_range() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_lt_bound");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = r#"
fn lt_bound_map(n) {
  let x = seq_len(n)
  let y = seq_len(n)
  let i = 1L

  while (i < length(x)) {
    y[i] = x[i] + 10L
    i = i + 1L

  }
  return y

}

print(lt_bound_map(6))
"#;

    let rr_path = out_dir.join("lt_bound_map.rr");
    let o0 = out_dir.join("lt_bound_map_o0.R");
    let o1 = out_dir.join("lt_bound_map_o1.R");
    let o2 = out_dir.join("lt_bound_map_o2.R");
    fs::write(&rr_path, rr_src).expect("failed to write source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o1, "-O1");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let o1_code = fs::read_to_string(&o1).expect("failed to read O1 output");
    assert!(
        (o1_code.contains("rr_assign_slice(")
            && o1_code.contains("y <- rr_assign_slice(y, i, (length(x) + -1L),"))
            || o1_code.contains("y <- rr_assign_slice(y, i, (n + -1L),")
            || (o1_code.contains("rr_assign_slice(seq_len(6L), 1L, (6L + -1L),")
                && o1_code.contains(
                    "rr_index1_read_vec(seq_len(6L), rr_index_vec_floor(1L:(6L + -1L)))"
                ))
            || o1_code.contains("print(((seq_len(6L) + 10L))))")
            || o1_code.contains("return(print(((seq_len(6L) + 10L))))"),
        "expected < bound loop to lower through a partial slice assignment"
    );
    assert!(
        !o1_code.contains("repeat {"),
        "expected < bound loop to be vectorized"
    );

    if let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) {
        let reference = run_rscript(&rscript, &o0);
        assert_eq!(reference.0, 0, "O0 execution failed:\n{}", reference.2);

        for (label, out) in [("-O1", &o1), ("-O2", &o2)] {
            let optimized = run_rscript(&rscript, out);
            assert_eq!(
                optimized.0, 0,
                "{} execution failed:\n{}",
                label, optimized.2
            );
            assert_eq!(
                normalize(&reference.1),
                normalize(&optimized.1),
                "{} stdout mismatch\nref:\n{}\nrr:\n{}",
                label,
                reference.1,
                optimized.1
            );
            assert_eq!(
                normalize(&reference.2),
                normalize(&optimized.2),
                "{} stderr mismatch\nref:\n{}\nrr:\n{}",
                label,
                reference.2,
                optimized.2
            );
        }
    }
}
