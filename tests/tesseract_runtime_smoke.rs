mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

fn extract_numeric_series(stdout: &str, marker: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut lines = stdout.lines();
    while let Some(line) = lines.next() {
        if line.contains(marker)
            && let Some(next) = lines.next()
        {
            let value_text = next.trim().trim_start_matches("[1]").trim();
            if let Ok(value) = value_text.parse::<f64>() {
                out.push(value);
            }
        }
    }
    out
}

fn assert_series_close(label: &str, a: &[f64], b: &[f64]) {
    assert_eq!(
        a.len(),
        b.len(),
        "{label} series length mismatch:\nleft={a:?}\nright={b:?}"
    );
    for (idx, (lhs, rhs)) in a.iter().zip(b.iter()).enumerate() {
        let diff = (lhs - rhs).abs();
        assert!(
            diff <= 1e-12,
            "{label} mismatch at index {idx}: left={lhs}, right={rhs}, diff={diff}"
        );
    }
}

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
            code.contains("Initializing Project MORPHOGENESIS: The Beauty of Chaos...")
                && code.contains("TESSERACT: UNIFIED Model Complete.")
                && code.contains("Sym_top_0 <- function()"),
            "expected compiled tesseract output to contain both top-level workflows for {}",
            flag
        );
        assert!(
            code.contains("particles <- Sym_")
                && code.contains("p_x <- rr_field_get(particles, \"px\")")
                && code.contains("p_y <- rr_field_get(particles, \"py\")")
                && code.contains("p_f <- rr_field_get(particles, \"pf\")"),
            "expected compiled tesseract output to thread particle state back for {}",
            flag
        );
        assert!(
            !code.contains("p_check <- Sym_89("),
            "stale particle state placeholder should not remain in compiled output for {}",
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
    let particle_x = extract_numeric_series(&stdout, "Particle 1 Position (X):");
    assert_eq!(
        particle_x.len(),
        5,
        "expected five particle position samples in tesseract output:\nstdout={stdout}"
    );
    assert!(
        particle_x.windows(2).all(|w| w[1] > w[0]),
        "expected particle x position to increase across steps: {particle_x:?}\nstdout={stdout}"
    );
    assert!(
        !stdout.trim().is_empty(),
        "tesseract O2 runtime produced empty stdout"
    );
}

#[test]
fn tesseract_runtime_markers_match_between_o1_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping tesseract parity test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = root.join("target").join("examples_tesseract_parity");
    fs::create_dir_all(&out_dir).expect("failed to create tesseract parity dir");

    let o1_path = out_dir.join("tesseract_o1.R");
    let o2_path = out_dir.join("tesseract_o2.R");
    compile_rr(&rr_bin, &rr_src, &o1_path, "-O1");
    compile_rr(&rr_bin, &rr_src, &o2_path, "-O2");

    let o1 = run_rscript(&rscript, &o1_path);
    let o2 = run_rscript(&rscript, &o2_path);
    let stdout_o1 = normalize(&o1.stdout);
    let stdout_o2 = normalize(&o2.stdout);
    let stderr_o1 = normalize(&o1.stderr);
    let stderr_o2 = normalize(&o2.stderr);

    assert_eq!(
        o1.status, 0,
        "tesseract O1 runtime failed:\nstdout={stdout_o1}\nstderr={stderr_o1}"
    );
    assert_eq!(
        o2.status, 0,
        "tesseract O2 runtime failed:\nstdout={stdout_o2}\nstderr={stderr_o2}"
    );

    let center_b_o1 = extract_numeric_series(&stdout_o1, "Center B:");
    let center_b_o2 = extract_numeric_series(&stdout_o2, "Center B:");
    let wave_b_o1 = extract_numeric_series(&stdout_o1, "Wave B:");
    let wave_b_o2 = extract_numeric_series(&stdout_o2, "Wave B:");
    let particle_x_o1 = extract_numeric_series(&stdout_o1, "Particle 1 Position (X):");
    let particle_x_o2 = extract_numeric_series(&stdout_o2, "Particle 1 Position (X):");
    let max_u_o1 = extract_numeric_series(&stdout_o1, "Step Complete. Max U:");
    let max_u_o2 = extract_numeric_series(&stdout_o2, "Step Complete. Max U:");

    assert!(
        !center_b_o1.is_empty(),
        "missing Center B series in O1 stdout"
    );
    assert!(!wave_b_o1.is_empty(), "missing Wave B series in O1 stdout");
    assert!(
        !particle_x_o1.is_empty(),
        "missing particle x series in O1 stdout"
    );
    assert!(!max_u_o1.is_empty(), "missing max_u series in O1 stdout");

    assert_series_close("Center B", &center_b_o1, &center_b_o2);
    assert_series_close("Wave B", &wave_b_o1, &wave_b_o2);
    assert_series_close("Particle 1 Position (X)", &particle_x_o1, &particle_x_o2);
    assert_series_close("Max U", &max_u_o1, &max_u_o2);
}
