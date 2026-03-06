mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn tesseract_compiles_across_opt_levels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = root.join("target").join("examples_tesseract");
    fs::create_dir_all(&out_dir).expect("failed to create tesseract output dir");

    for (flag, tag) in [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")] {
        let out = out_dir.join(format!("tesseract_{tag}.R"));
        compile_rr(&rr_bin, &rr_src, &out, flag);
        let code = fs::read_to_string(&out).expect("failed to read compiled tesseract output");
        assert!(
            code.contains("Sym_103 <- function()") && code.contains("Sym_123 <- function()"),
            "expected compiled tesseract output to contain both top-level kernels for {}",
            flag
        );
    }
}

#[test]
fn tesseract_runs_at_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping tesseract runtime smoke: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = root.join("target").join("examples_tesseract_runtime");
    fs::create_dir_all(&out_dir).expect("failed to create tesseract runtime dir");

    let out = out_dir.join("tesseract_o2.R");
    compile_rr(&rr_bin, &rr_src, &out, "-O2");
    let run = run_rscript(&rscript, &out);
    let stdout = normalize(&run.stdout);
    let stderr = normalize(&run.stderr);

    assert!(
        run.status == 0,
        "tesseract O2 runtime failed:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("Morphogenesis Complete. The pattern has formed.")
            && stdout.contains("TESSERACT: UNIFIED Model Complete."),
        "tesseract O2 runtime output was missing expected milestones:\nstdout={stdout}"
    );
    assert!(
        !stdout.trim().is_empty(),
        "tesseract O2 runtime produced empty stdout"
    );
}
