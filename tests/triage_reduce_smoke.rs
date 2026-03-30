mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn pass_verify_reducer_preserves_compile_failure() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("triage_reduce_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create triage reduce root");
    let case_dir = unique_dir(&sandbox_root, "pass_verify_case");
    fs::create_dir_all(&case_dir).expect("failed to create pass verify case dir");

    let source = r#"
fn bad_case() -> int {
  let x = 1.0
  let y =
  return 1L
}

print(bad_case())
"#;
    let rr_path = case_dir.join("case.rr");
    fs::write(&rr_path, source).expect("failed to write pass verify case");

    let compile = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(case_dir.join("out.R"))
        .arg("-O2")
        .env("RR_VERIFY_EACH_PASS", "1")
        .output()
        .expect("failed to run RR compiler");
    assert!(
        !compile.status.success(),
        "expected invalid case to fail compilation"
    );

    fs::write(case_dir.join("compiler.stdout"), &compile.stdout).expect("failed to write stdout");
    fs::write(case_dir.join("compiler.stderr"), &compile.stderr).expect("failed to write stderr");
    fs::write(
        case_dir.join("bundle.manifest"),
        format!(
            "schema: rr-triage-bundle\nversion: 1\nkind: pass-verify\ncase: {}\nstatus: {}\n",
            rr_path.display(),
            compile.status.code().unwrap_or(-1)
        ),
    )
    .expect("failed to write manifest");

    let reducer = root.join("scripts").join("reduce_triage_case.sh");
    let reduced = case_dir.join("reduced.rr");
    let output = Command::new("bash")
        .arg(&reducer)
        .arg("pass-verify")
        .arg(&case_dir)
        .arg(&reduced)
        .env("RR_BIN", &rr_bin)
        .output()
        .expect("failed to run pass-verify reducer");
    assert!(
        output.status.success(),
        "reducer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(reduced.exists(), "reduced RR case was not written");

    let original_len = fs::read_to_string(&rr_path)
        .expect("failed to read original")
        .len();
    let reduced_text = fs::read_to_string(&reduced).expect("failed to read reduced");
    assert!(
        reduced_text.len() <= original_len,
        "reduced case should not be larger than original"
    );

    let reduced_compile = Command::new(&rr_bin)
        .arg(&reduced)
        .arg("-o")
        .arg(case_dir.join("reduced_out.R"))
        .arg("-O2")
        .env("RR_VERIFY_EACH_PASS", "1")
        .output()
        .expect("failed to compile reduced case");
    assert!(
        !reduced_compile.status.success(),
        "reduced case should preserve compile failure"
    );
}

#[test]
fn triage_scripts_emit_replay_reduce_and_meta_files() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rscript = match common::rscript_path() {
        Some(p) if common::rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping triage bundle smoke: Rscript unavailable.");
            return;
        }
    };

    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("triage_bundle_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create triage bundle root");
    let diff_root = unique_dir(&sandbox_root, "differential_failures");
    let pass_root = unique_dir(&sandbox_root, "pass_verify_failures");
    let diff_out = unique_dir(&sandbox_root, "differential_out");
    let pass_out = unique_dir(&sandbox_root, "pass_verify_out");
    fs::create_dir_all(&diff_root).expect("failed to create diff root");
    fs::create_dir_all(&pass_root).expect("failed to create pass root");
    fs::create_dir_all(&diff_out).expect("failed to create diff out");
    fs::create_dir_all(&pass_out).expect("failed to create pass out");

    let diff_case_dir = diff_root.join("diff_case_o0");
    fs::create_dir_all(&diff_case_dir).expect("failed to create diff case dir");
    let diff_rr = r#"
fn main() -> int {
  let a = 1L
  print(a)
  return a
}

print(main())
"#;
    let diff_ref = r#"print(2L)"#;
    fs::write(diff_case_dir.join("case.rr"), diff_rr).expect("failed to write diff rr");
    fs::write(diff_case_dir.join("reference.R"), diff_ref).expect("failed to write diff ref");
    let rr_path = diff_case_dir.join("case.rr");
    let compiled_r = diff_case_dir.join("compiled.R");
    let compile = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&compiled_r)
        .arg("-O0")
        .output()
        .expect("failed to compile diff case");
    assert!(compile.status.success(), "diff RR compile failed");
    let reference = Command::new(&rscript)
        .arg("--vanilla")
        .arg(diff_case_dir.join("reference.R"))
        .output()
        .expect("failed to run reference");
    let compiled = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&compiled_r)
        .output()
        .expect("failed to run compiled");
    fs::write(diff_case_dir.join("reference.stdout"), &reference.stdout)
        .expect("failed to write diff ref stdout");
    fs::write(diff_case_dir.join("reference.stderr"), &reference.stderr)
        .expect("failed to write diff ref stderr");
    fs::write(diff_case_dir.join("compiled.stdout"), &compiled.stdout)
        .expect("failed to write diff compiled stdout");
    fs::write(diff_case_dir.join("compiled.stderr"), &compiled.stderr)
        .expect("failed to write diff compiled stderr");
    fs::write(
        diff_case_dir.join("bundle.manifest"),
        format!(
            "schema: rr-triage-bundle\nversion: 1\nkind: differential\ncase: diff_case\nopt: O0\nreference_status: {}\ncompiled_status: {}\n",
            reference.status.code().unwrap_or(-1),
            compiled.status.code().unwrap_or(-1),
        ),
    )
    .expect("failed to write diff manifest");

    let pass_case_dir = pass_root.join("verify_case");
    fs::create_dir_all(&pass_case_dir).expect("failed to create pass case dir");
    let pass_rr = "fn broken() -> int {\n  let x =\n}\n";
    let pass_rr_path = pass_case_dir.join("case.rr");
    fs::write(&pass_rr_path, pass_rr).expect("failed to write pass rr");
    let pass_compile = Command::new(&rr_bin)
        .arg(&pass_rr_path)
        .arg("-o")
        .arg(pass_case_dir.join("out.R"))
        .arg("-O2")
        .env("RR_VERIFY_EACH_PASS", "1")
        .output()
        .expect("failed to compile pass case");
    assert!(!pass_compile.status.success(), "expected pass case to fail");
    fs::write(pass_case_dir.join("compiler.stdout"), &pass_compile.stdout)
        .expect("failed to write pass stdout");
    fs::write(pass_case_dir.join("compiler.stderr"), &pass_compile.stderr)
        .expect("failed to write pass stderr");
    fs::write(
        pass_case_dir.join("bundle.manifest"),
        format!(
            "schema: rr-triage-bundle\nversion: 1\nkind: pass-verify\ncase: {}\nstatus: {}\n",
            pass_rr_path.display(),
            pass_compile.status.code().unwrap_or(-1),
        ),
    )
    .expect("failed to write pass manifest");

    let diff_script = root.join("scripts").join("differential_triage.sh");
    let diff_outcome = Command::new("bash")
        .arg(&diff_script)
        .env("RR_DIFFERENTIAL_FAILURE_ROOT", &diff_root)
        .env("RR_DIFFERENTIAL_TRIAGE_OUT_DIR", &diff_out)
        .env("RR_BIN", &rr_bin)
        .env("RSCRIPT_BIN", &rscript)
        .output()
        .expect("failed to run differential triage");
    assert!(
        diff_outcome.status.success(),
        "differential triage failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&diff_outcome.stdout),
        String::from_utf8_lossy(&diff_outcome.stderr)
    );

    let pass_script = root.join("scripts").join("pass_verify_triage.sh");
    let pass_outcome = Command::new("bash")
        .arg(&pass_script)
        .env("RR_PASS_VERIFY_FAILURE_ROOT", &pass_root)
        .env("RR_PASS_VERIFY_TRIAGE_OUT_DIR", &pass_out)
        .env("RR_BIN", &rr_bin)
        .output()
        .expect("failed to run pass-verify triage");
    assert!(
        pass_outcome.status.success(),
        "pass-verify triage failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&pass_outcome.stdout),
        String::from_utf8_lossy(&pass_outcome.stderr)
    );

    let generated_diff_case = diff_out.join("diff_case_o0");
    let generated_pass_case = pass_out.join("verify_case");
    for path in [
        generated_diff_case.join("replay.sh"),
        generated_diff_case.join("reduce.sh"),
        generated_diff_case.join("meta.json"),
        generated_diff_case.join("regression.rs"),
        generated_pass_case.join("replay.sh"),
        generated_pass_case.join("reduce.sh"),
        generated_pass_case.join("meta.json"),
        generated_pass_case.join("regression.rs"),
    ] {
        assert!(
            path.exists(),
            "expected generated triage file: {}",
            path.display()
        );
    }

    let diff_meta = fs::read_to_string(generated_diff_case.join("meta.json"))
        .expect("failed to read diff meta");
    let pass_meta = fs::read_to_string(generated_pass_case.join("meta.json"))
        .expect("failed to read pass meta");
    assert!(diff_meta.contains(r#""kind": "differential""#));
    assert!(
        diff_meta.contains(r#""reduce_script":"#) || diff_meta.contains(r#""reduce_script": "#)
    );
    assert!(pass_meta.contains(r#""kind": "pass-verify""#));
    assert!(
        pass_meta.contains(r#""replay_script":"#) || pass_meta.contains(r#""replay_script": "#)
    );
}
