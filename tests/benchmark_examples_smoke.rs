mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::{Path, PathBuf};

fn collect_rr_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .expect("missing benchmark example directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            out.push(path);
        }
    }
}

#[test]
fn benchmark_examples_compile_at_o2() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let src_dir = root.join("example").join("benchmarks");
    let out_dir = root.join("target").join("benchmark_examples");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples");

    let mut files = Vec::new();
    collect_rr_files(&src_dir, &mut files);
    assert!(files.len() >= 5, "expected at least 5 benchmark workloads");

    for rr_path in files {
        let stem = rr_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("bench");
        let out_path = out_dir.join(format!("{}_o2.R", stem));
        compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
        let code = fs::read_to_string(&out_path).expect("failed to read compiled benchmark R");
        assert!(
            code.contains("function(") || code.contains("print("),
            "compiled benchmark output for {} looked empty",
            rr_path.display()
        );
    }
}

#[test]
fn benchmark_examples_run_at_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping benchmark runtime smoke: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let src_dir = root.join("example").join("benchmarks");
    let out_dir = root.join("target").join("benchmark_examples_runtime");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples_runtime");

    let mut files = Vec::new();
    collect_rr_files(&src_dir, &mut files);
    assert!(files.len() >= 5, "expected at least 5 benchmark workloads");

    for rr_path in files {
        let stem = rr_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("bench");
        let out_path = out_dir.join(format!("{}_o2.R", stem));
        compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
        let run = run_rscript(&rscript, &out_path);
        assert!(
            run.status == 0,
            "benchmark runtime failed for {}:\nstdout={}\nstderr={}",
            rr_path.display(),
            run.stdout,
            run.stderr
        );
        assert!(
            !normalize(&run.stdout).trim().is_empty(),
            "benchmark runtime produced empty stdout for {}",
            rr_path.display()
        );
    }
}
