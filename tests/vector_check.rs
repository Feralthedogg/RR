use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_src.display(),
        level
    );
}

fn compile_rr_with_modes(
    rr_bin: &Path,
    rr_src: &Path,
    out: &Path,
    level: &str,
    type_mode: &str,
    native_backend: &str,
) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg(level)
        .arg("--type-mode")
        .arg(type_mode)
        .arg("--native-backend")
        .arg(native_backend)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({}, {}, {})",
        rr_src.display(),
        level,
        type_mode,
        native_backend
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

fn norm(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn vector_check_compiles_and_preserves_semantics() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_src = root.join("tests").join("golden").join("vector_math.rr");
    assert!(rr_src.exists(), "missing {}", rr_src.display());

    let out_dir = root.join("target").join("tests").join("vector_check");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let o0 = out_dir.join("vector_check_o0.R");
    let o1 = out_dir.join("vector_check_o1.R");
    let o2 = out_dir.join("vector_check_o2.R");
    let o2_typed_optional = out_dir.join("vector_check_o2_typed_optional.R");
    compile_rr(&rr_bin, &rr_src, &o0, "-O0");
    compile_rr(&rr_bin, &rr_src, &o1, "-O1");
    compile_rr(&rr_bin, &rr_src, &o2, "-O2");
    compile_rr_with_modes(
        &rr_bin,
        &rr_src,
        &o2_typed_optional,
        "-O2",
        "strict",
        "optional",
    );

    let o1_code = fs::read_to_string(&o1).expect("failed to read O1 output");
    assert!(o1_code.contains("sum("), "expected numeric reduction path");
    assert!(
        o1_code.contains("x + x")
            || o1_code.contains("(x + x)")
            || o1_code.contains("seq_len(20L) + seq_len(20L)")
            || o1_code.contains("rr_intrinsic_vec_add_f64("),
        "expected optimized arithmetic expression in O1 output (direct add or intrinsic add)"
    );
    if let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) {
        let (s0, out0, err0) = run_rscript(&rscript, &o0);
        let (s1, out1, err1) = run_rscript(&rscript, &o1);
        let (s2, out2, err2) = run_rscript(&rscript, &o2);
        let (s3, out3, err3) = run_rscript(&rscript, &o2_typed_optional);
        assert_eq!(s0, 0, "O0 failed: {}", err0);
        assert_eq!(s1, 0, "O1 failed: {}", err1);
        assert_eq!(s2, 0, "O2 failed: {}", err2);
        assert_eq!(s3, 0, "O2 strict/optional failed: {}", err3);
        assert_eq!(norm(&out0), norm(&out1), "stdout mismatch O0 vs O1");
        assert_eq!(norm(&out0), norm(&out2), "stdout mismatch O0 vs O2");
        assert_eq!(
            norm(&out0),
            norm(&out3),
            "stdout mismatch O0 vs O2(strict+optional)"
        );
        assert_eq!(norm(&err0), norm(&err1), "stderr mismatch O0 vs O1");
        assert_eq!(norm(&err0), norm(&err2), "stderr mismatch O0 vs O2");
        assert_eq!(
            norm(&err0),
            norm(&err3),
            "stderr mismatch O0 vs O2(strict+optional)"
        );
    }
}
