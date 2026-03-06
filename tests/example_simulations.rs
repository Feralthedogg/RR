mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::{Path, PathBuf};

const REFERENCE_EQUIV_CASES: &[&str] = &[
    "bootstrap_mean",
    "markov_weather_chain",
    "monte_carlo_pi",
    "projectile_drag",
    "sir_epidemic",
];

fn collect_rr_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .expect("missing example directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rr_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            out.push(path);
        }
    }
}

fn collect_example_entries(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rr_files(&root.join("example").join("data_science"), &mut files);
    collect_rr_files(&root.join("example").join("physics"), &mut files);
    files.sort();
    files
}

#[test]
fn simulation_examples_compile_across_opt_levels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root.join("target").join("examples");
    fs::create_dir_all(&out_dir).expect("failed to create target/examples");

    let examples = collect_example_entries(&root);
    assert!(
        examples.len() >= 16,
        "expected at least 16 simulation examples, found {}",
        examples.len()
    );

    for example in examples {
        let stem = example
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("case");
        for (flag, tag) in [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")] {
            let out_path = out_dir.join(format!("{}_{}.R", stem, tag));
            compile_rr(&rr_bin, &example, &out_path, flag);
            let code = fs::read_to_string(&out_path).expect("failed to read compiled example");
            assert!(
                code.contains("<- function(") || code.contains("print("),
                "compiled output for {} looked empty",
                example.display()
            );
        }
    }
}

#[test]
fn simulation_examples_run_at_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping simulation example runtime test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root.join("target").join("examples_runtime");
    fs::create_dir_all(&out_dir).expect("failed to create target/examples_runtime");

    let examples = collect_example_entries(&root);
    assert!(
        examples.len() >= 16,
        "expected at least 16 simulation examples"
    );

    for example in examples {
        let stem = example
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("case");
        let o2 = out_dir.join(format!("{}_o2.R", stem));
        compile_rr(&rr_bin, &example, &o2, "-O2");
        let run_o2 = run_rscript(&rscript, &o2);

        assert!(
            run_o2.status == 0,
            "O2 runtime failed for {}:\nstdout={}\nstderr={}",
            example.display(),
            run_o2.stdout,
            run_o2.stderr
        );
        assert!(
            !normalize(&run_o2.stdout).is_empty(),
            "O2 runtime produced empty stdout for {}",
            example.display()
        );
    }
}

#[test]
fn simulation_reference_examples_match_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping simulation equivalence test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root.join("target").join("examples_runtime_eq");
    fs::create_dir_all(&out_dir).expect("failed to create target/examples_runtime_eq");

    for stem in REFERENCE_EQUIV_CASES {
        let mut example = root
            .join("example")
            .join("data_science")
            .join(format!("{stem}.rr"));
        if !example.exists() {
            example = root
                .join("example")
                .join("physics")
                .join(format!("{stem}.rr"));
        }
        assert!(
            example.exists(),
            "missing reference example {}",
            example.display()
        );

        let o0 = out_dir.join(format!("{}_o0.R", stem));
        let o2 = out_dir.join(format!("{}_o2.R", stem));
        compile_rr(&rr_bin, &example, &o0, "-O0");
        compile_rr(&rr_bin, &example, &o2, "-O2");

        let run_o0 = run_rscript(&rscript, &o0);
        let run_o2 = run_rscript(&rscript, &o2);

        assert_eq!(
            run_o0.status,
            run_o2.status,
            "status mismatch for {}",
            example.display()
        );
        assert_eq!(
            normalize(&run_o0.stdout),
            normalize(&run_o2.stdout),
            "stdout mismatch for {}",
            example.display()
        );
        assert_eq!(
            normalize(&run_o0.stderr),
            normalize(&run_o2.stderr),
            "stderr mismatch for {}",
            example.display()
        );
    }
}
