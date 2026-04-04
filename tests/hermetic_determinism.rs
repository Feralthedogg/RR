mod common;

use common::unique_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_bytes(
    rr_bin: &Path,
    rr_path: &Path,
    out_path: &Path,
    extra_args: &[&str],
    env_kv: &[(String, String)],
    cwd: &Path,
) -> Vec<u8> {
    let mut cmd = Command::new(rr_bin);
    cmd.current_dir(cwd)
        .arg(rr_path)
        .arg("-o")
        .arg(out_path)
        .env("RR_QUIET_LOG", "1");
    for (key, value) in env_kv {
        cmd.env(key, value);
    }
    for arg in extra_args {
        cmd.arg(arg);
    }
    let output = cmd.output().expect("failed to run RR compiler");
    assert!(
        output.status.success(),
        "compile failed for {} with args {:?}\nstdout:\n{}\nstderr:\n{}",
        rr_path.display(),
        extra_args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read(out_path).expect("failed to read emitted artifact")
}

#[test]
fn emitted_artifact_is_hermetic_across_environment_matrix() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("hermetic_determinism");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
fn kernel(n) {
  let x = seq_len(n)
  let y = x
  for (i in 1..length(y)) {
    y[i] = (x[i] * 2L) + 1L
  }
  return y
}

print(sum(kernel(12L)))
"#;

    let flag_sets: [&[&str]; 4] = [
        &["-O0"],
        &["-O2"],
        &["-O2", "--no-runtime"],
        &["-O2", "--preserve-all-defs"],
    ];
    let env_sets: [Vec<(String, String)>; 4] = [
        vec![],
        vec![
            ("NO_COLOR".to_string(), "1".to_string()),
            ("LC_ALL".to_string(), "C".to_string()),
            ("LANG".to_string(), "C".to_string()),
        ],
        vec![
            ("TZ".to_string(), "UTC".to_string()),
            ("TMPDIR".to_string(), "/tmp".to_string()),
        ],
        vec![
            ("NO_COLOR".to_string(), "1".to_string()),
            ("TZ".to_string(), "Asia/Seoul".to_string()),
            ("LC_ALL".to_string(), "C.UTF-8".to_string()),
        ],
    ];

    for flag_set in flag_sets {
        let mut baseline: Option<Vec<u8>> = None;
        for (idx, env_set) in env_sets.iter().enumerate() {
            let proj = unique_dir(&sandbox_root, &format!("case_{}", idx));
            let nested_cwd = proj.join("cwd");
            let tmpdir = proj.join("tmpdir");
            fs::create_dir_all(&nested_cwd).expect("failed to create cwd dir");
            fs::create_dir_all(&tmpdir).expect("failed to create tmp dir");
            let rr_path = proj.join("main.rr");
            let out_path = proj.join("out.R");
            fs::write(&rr_path, src).expect("failed to write RR source");

            let mut child_envs = env_set.clone();
            if !child_envs.iter().any(|(key, _)| key == "TMPDIR") {
                child_envs.push(("TMPDIR".to_string(), tmpdir.to_string_lossy().to_string()));
            }

            let emitted = compile_bytes(
                &rr_bin,
                &rr_path,
                &out_path,
                flag_set,
                &child_envs,
                &nested_cwd,
            );
            if let Some(expected) = baseline.as_ref() {
                assert_eq!(
                    expected, &emitted,
                    "artifact drifted across environment matrix for flags {:?}",
                    flag_set
                );
            } else {
                baseline = Some(emitted);
            }
        }
    }
}
