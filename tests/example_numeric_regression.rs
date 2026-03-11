mod common;

use common::{compile_rr, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

const NUMERIC_CASES: &[&str] = &[
    "physics/heat_diffusion_1d",
    "physics/reaction_diffusion_1d",
    "benchmarks/heat_diffusion_bench",
];

#[test]
fn diffusion_examples_do_not_emit_non_finite_metrics_at_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping numeric regression test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root.join("target").join("example_numeric_regression");
    fs::create_dir_all(&out_dir).expect("failed to create target/example_numeric_regression");

    for case in NUMERIC_CASES {
        let example = root.join("example").join(format!("{case}.rr"));
        let stem = example
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("case");
        let out = out_dir.join(format!("{stem}_o2.R"));
        compile_rr(&rr_bin, &example, &out, "-O2");
        let run = run_rscript(&rscript, &out);
        assert_eq!(
            run.status,
            0,
            "runtime failed for {}:\nstdout={}\nstderr={}",
            example.display(),
            run.stdout,
            run.stderr
        );
        for needle in ["NA", "NaN", "Inf"] {
            assert!(
                !run.stdout.contains(needle),
                "non-finite metric detected for {}: {}\nstdout={}",
                example.display(),
                needle,
                run.stdout
            );
        }
    }
}

#[test]
fn kalman_filter_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping kalman regression test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let example = root
        .join("example")
        .join("data_science")
        .join("kalman_filter_1d.rr");
    let out_dir = root.join("target").join("example_numeric_regression");
    fs::create_dir_all(&out_dir).expect("failed to create target/example_numeric_regression");

    let o0 = out_dir.join("kalman_filter_1d_o0.R");
    let o2 = out_dir.join("kalman_filter_1d_o2.R");
    compile_rr(&rr_bin, &example, &o0, "-O0");
    compile_rr(&rr_bin, &example, &o2, "-O2");

    let run_o0 = run_rscript(&rscript, &o0);
    let run_o2 = run_rscript(&rscript, &o2);
    assert_eq!(
        run_o0.status,
        0,
        "O0 runtime failed for {}:\nstdout={}\nstderr={}",
        example.display(),
        run_o0.stdout,
        run_o0.stderr
    );
    assert_eq!(
        run_o2.status,
        0,
        "O2 runtime failed for {}:\nstdout={}\nstderr={}",
        example.display(),
        run_o2.stdout,
        run_o2.stderr
    );
    assert_eq!(
        run_o0.stdout, run_o2.stdout,
        "kalman_filter_1d stdout mismatch between O0 and O2"
    );
    assert_eq!(
        run_o0.stderr, run_o2.stderr,
        "kalman_filter_1d stderr mismatch between O0 and O2"
    );
}
