use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(root: &Path, name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.join(format!("{}_{}_{}", name, std::process::id(), ts))
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
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

fn compile_rr(rr_bin: &Path, rr_path: &Path, out_path: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_path)
        .arg("-o")
        .arg(out_path)
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );
}

#[test]
fn conditional_map_with_scalar_accumulator_preserves_semantics() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping vectorization loop-state regression test: Rscript not available.");
            return;
        }
    };

    let rr_source = r#"
fn kernel(n, k, a, b, c) {
  let x = seq_len(n);
  let y = seq_len(n);
  let s = 0L;
  for (i in 1L..length(x)) {
    if ((((x[i] * a) + b) - c) > k) {
      y[i] = (((x[i] * a) + b) - c) - k;
    } else {
      y[i] = (((x[i] * a) + b) - c) + k;
    }
    s = s + y[i];
  }
  return s;
}

fn mix(n, k, a, b, c) {
  let i = 1L;
  let acc = 0L;
  while (i <= n) {
    acc = acc + (((i * a) + b) - c);
    i = i + 1L;
  }
  let ys = kernel(n, k, a, b, c);
  return acc + ys;
}

print(mix(12L, 10L, 1L, 2L, 0L));
"#;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("vectorization_loop_state_regression");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join("case.rr");
    let o0_path = proj_dir.join("case_o0.R");
    let o1_path = proj_dir.join("case_o1.R");
    let o2_path = proj_dir.join("case_o2.R");
    fs::write(&rr_path, rr_source).expect("failed to write RR case");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    for (level, out_path) in [("-O0", &o0_path), ("-O1", &o1_path), ("-O2", &o2_path)] {
        compile_rr(&rr_bin, &rr_path, out_path, level);
    }

    let reference = run_rscript(&rscript, &o0_path);
    assert_eq!(reference.0, 0, "O0 execution failed:\n{}", reference.2);

    for (label, out_path) in [("-O1", &o1_path), ("-O2", &o2_path)] {
        let compiled = run_rscript(&rscript, out_path);
        assert_eq!(compiled.0, 0, "{} execution failed:\n{}", label, compiled.2);
        assert_eq!(
            normalize(&reference.1),
            normalize(&compiled.1),
            "{} stdout mismatch\nref:\n{}\nrr:\n{}",
            label,
            reference.1,
            compiled.1
        );
        assert_eq!(
            normalize(&reference.2),
            normalize(&compiled.2),
            "{} stderr mismatch\nref:\n{}\nrr:\n{}",
            label,
            reference.2,
            compiled.2
        );
    }
}
