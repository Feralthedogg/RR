mod common;

use common::{RunResult, compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn golden_r_semantics() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping golden tests: Rscript not available. Set RRSCRIPT to override.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let golden_dir = root.join("tests").join("golden");
    let out_dir = root.join("target").join("golden");
    fs::create_dir_all(&out_dir).expect("failed to create target/golden");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let mut rr_files: Vec<PathBuf> = fs::read_dir(&golden_dir)
        .expect("missing tests/golden directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("rr"))
        .collect();
    rr_files.sort();
    assert!(
        rr_files.len() >= 8,
        "golden set is too small (found {} cases)",
        rr_files.len()
    );

    let levels = [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")];

    for rr_path in rr_files {
        let stem = rr_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("case");

        let ref_r = golden_dir.join(format!("{}.R", stem));
        let ref_path = if ref_r.exists() {
            ref_r
        } else {
            rr_path.clone()
        };

        let ref_run = run_rscript(&rscript, &ref_path);

        let mut opt_runs: Vec<(String, RunResult)> = Vec::new();
        for (flag, tag) in levels {
            let compiled_path = out_dir.join(format!("{}_compiled_{}.R", stem, tag));
            compile_rr(&rr_bin, &rr_path, &compiled_path, flag);
            let compiled_run = run_rscript(&rscript, &compiled_path);

            assert_eq!(
                ref_run.status,
                compiled_run.status,
                "exit status mismatch for {} ({})",
                rr_path.display(),
                flag
            );
            assert_eq!(
                normalize(&ref_run.stdout),
                normalize(&compiled_run.stdout),
                "stdout mismatch for {} ({})",
                rr_path.display(),
                flag
            );
            assert_eq!(
                normalize(&ref_run.stderr),
                normalize(&compiled_run.stderr),
                "stderr mismatch for {} ({})",
                rr_path.display(),
                flag
            );
            opt_runs.push((flag.to_string(), compiled_run));
        }

        if let Some((base_flag, base_run)) = opt_runs.first() {
            for (flag, run) in opt_runs.iter().skip(1) {
                assert_eq!(
                    base_run.status,
                    run.status,
                    "cross-opt status mismatch for {} ({} vs {})",
                    rr_path.display(),
                    base_flag,
                    flag
                );
                assert_eq!(
                    normalize(&base_run.stdout),
                    normalize(&run.stdout),
                    "cross-opt stdout mismatch for {} ({} vs {})",
                    rr_path.display(),
                    base_flag,
                    flag
                );
                assert_eq!(
                    normalize(&base_run.stderr),
                    normalize(&run.stderr),
                    "cross-opt stderr mismatch for {} ({} vs {})",
                    rr_path.display(),
                    base_flag,
                    flag
                );
            }
        }
    }
}
