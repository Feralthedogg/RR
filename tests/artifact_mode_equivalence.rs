mod common;

use common::{env_lock, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DATA_SCIENCE_ARTIFACT_EQUIV_CASES: &[&str] = &[
    "bootstrap_mean",
    "lm_predict_quantile_band",
    "markov_weather_chain",
    "monte_carlo_pi",
];

const PHYSICS_ARTIFACT_EQUIV_CASES: &[&str] = &[
    "advection_1d",
    "heat_diffusion_1d",
    "orbital_two_body",
    "reaction_diffusion_1d",
];

const HELPER_PRESERVE_EQUIV_FIXTURE: &str = r#"
fn unused_preserved(xs) {
    let acc = 0.0
    let i = 1.0
    while i <= length(xs) {
        acc += xs[i]
        i += 1.0
    }
    acc
}

fn live_summary() {
    let xs = c(3.0, 5.0, 7.0)
    let ys = (xs + 1.0) * 2.0
    let meta = {second: ys[2.0], total: sum(ys), avg: mean(ys)}
    print("artifact fixture")
    print(meta.second)
    print(meta.total)
    print(meta.avg)
    return 0.0
}

print(live_summary())
"#;

fn packages_available(rscript: &str, packages: &[&str]) -> bool {
    let package_checks = packages
        .iter()
        .map(|pkg| format!("requireNamespace('{pkg}', quietly = TRUE)"))
        .collect::<Vec<_>>()
        .join(" && ");
    Command::new(rscript)
        .arg("--vanilla")
        .arg("-e")
        .arg(format!("quit(status = if ({package_checks}) 0 else 1)"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn compile_with_flags(rr_bin: &Path, rr_src: &Path, out_path: &Path, flags: &[&str]) {
    let mut cmd = Command::new(rr_bin);
    cmd.arg(rr_src).arg("-o").arg(out_path);
    for flag in flags {
        cmd.arg(flag);
    }
    let output = cmd.output().expect("failed to run RR compiler");
    assert!(
        output.status.success(),
        "RR compile failed for {} with flags {:?}:\nstdout={}\nstderr={}",
        rr_src.display(),
        flags,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn assert_runs_match(
    label: &str,
    baseline_name: &str,
    baseline: &common::RunResult,
    other_name: &str,
    other: &common::RunResult,
) {
    assert_eq!(
        baseline.status, other.status,
        "{label}: status mismatch between {baseline_name} and {other_name}"
    );
    assert_eq!(
        normalize(&baseline.stdout),
        normalize(&other.stdout),
        "{label}: stdout mismatch between {baseline_name} and {other_name}"
    );
    assert_eq!(
        normalize(&baseline.stderr),
        normalize(&other.stderr),
        "{label}: stderr mismatch between {baseline_name} and {other_name}"
    );
}

fn run_rscript_in_dir(path: &str, script: &Path, run_dir: &Path) -> common::RunResult {
    fs::create_dir_all(run_dir).expect("failed to create artifact-mode run dir");
    let output = Command::new(path)
        .current_dir(run_dir)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    common::RunResult {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn assert_file_bytes_match(
    label: &str,
    baseline_name: &str,
    baseline: &Path,
    other_name: &str,
    other: &Path,
) {
    let baseline_bytes = fs::read(baseline).unwrap_or_else(|err| {
        panic!(
            "{label}: failed to read {baseline_name} artifact {}: {err}",
            baseline.display()
        )
    });
    let other_bytes = fs::read(other).unwrap_or_else(|err| {
        panic!(
            "{label}: failed to read {other_name} artifact {}: {err}",
            other.display()
        )
    });
    assert_eq!(
        baseline_bytes, other_bytes,
        "{label}: file bytes mismatch between {baseline_name} and {other_name}"
    );
}

#[test]
fn signal_pipeline_helper_only_matches_runtime_injected_stdout() {
    let _guard = env_lock().lock().expect("env lock poisoned");
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping artifact-mode equivalence: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root
        .join("example")
        .join("benchmarks")
        .join("signal_pipeline_bench.rr");
    let out_dir = root
        .join("target")
        .join("tests")
        .join("artifact_mode_equivalence_signal");
    fs::create_dir_all(&out_dir).expect("failed to create signal equivalence dir");

    let runtime_out = out_dir.join("signal_pipeline_runtime.R");
    let helper_out = out_dir.join("signal_pipeline_helper.R");

    compile_with_flags(&rr_bin, &rr_src, &runtime_out, &["-O2"]);
    compile_with_flags(&rr_bin, &rr_src, &helper_out, &["-O2", "--no-runtime"]);

    let runtime_run = run_rscript(&rscript, &runtime_out);
    let helper_run = run_rscript(&rscript, &helper_out);

    assert_eq!(
        runtime_run.status, 0,
        "runtime-injected signal pipeline failed"
    );
    assert_eq!(helper_run.status, 0, "helper-only signal pipeline failed");
    assert_eq!(
        normalize(&runtime_run.stdout),
        normalize(&helper_run.stdout),
        "signal pipeline stdout mismatch between runtime-injected and helper-only artifacts"
    );
    assert_eq!(
        normalize(&runtime_run.stderr),
        normalize(&helper_run.stderr),
        "signal pipeline stderr mismatch between runtime-injected and helper-only artifacts"
    );
}

#[test]
fn fixture_default_helper_and_preserve_modes_match_stdout() {
    let _guard = env_lock().lock().expect("env lock poisoned");
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping artifact-mode equivalence: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("artifact_mode_equivalence_fixture");
    fs::create_dir_all(&out_dir).expect("failed to create fixture equivalence dir");

    // Keep this runtime-equivalence check lightweight. The full tesseract example
    // already has dedicated compile/runtime coverage elsewhere.
    let rr_src = out_dir.join("fixture.rr");
    fs::write(&rr_src, HELPER_PRESERVE_EQUIV_FIXTURE)
        .expect("failed to write artifact-mode equivalence fixture");

    let runtime_out = out_dir.join("fixture_runtime.R");
    let helper_out = out_dir.join("fixture_helper.R");
    let preserve_out = out_dir.join("fixture_preserve.R");

    compile_with_flags(&rr_bin, &rr_src, &runtime_out, &["-O2"]);
    compile_with_flags(&rr_bin, &rr_src, &helper_out, &["-O2", "--no-runtime"]);
    compile_with_flags(
        &rr_bin,
        &rr_src,
        &preserve_out,
        &["-O2", "--preserve-all-defs"],
    );

    let runtime_run = run_rscript(&rscript, &runtime_out);
    let helper_run = run_rscript(&rscript, &helper_out);
    let preserve_run = run_rscript(&rscript, &preserve_out);

    assert_eq!(runtime_run.status, 0, "default fixture artifact failed");
    assert_eq!(helper_run.status, 0, "helper-only fixture artifact failed");
    assert_eq!(
        preserve_run.status, 0,
        "preserve-all-defs fixture artifact failed"
    );

    let runtime_stdout = normalize(&runtime_run.stdout);
    assert_eq!(
        runtime_stdout,
        normalize(&helper_run.stdout),
        "fixture stdout mismatch between default and helper-only artifacts"
    );
    assert_eq!(
        runtime_stdout,
        normalize(&preserve_run.stdout),
        "fixture stdout mismatch between default and preserve-all-defs artifacts"
    );
}

#[test]
fn data_science_examples_match_across_artifact_modes() {
    let _guard = env_lock().lock().expect("env lock poisoned");
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping artifact-mode equivalence: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let src_dir = root.join("example").join("data_science");
    let out_dir = root
        .join("target")
        .join("tests")
        .join("artifact_mode_equivalence_data_science");
    fs::create_dir_all(&out_dir).expect("failed to create data science equivalence dir");

    for stem in DATA_SCIENCE_ARTIFACT_EQUIV_CASES {
        let rr_src = src_dir.join(format!("{stem}.rr"));
        assert!(
            rr_src.exists(),
            "missing data science example {}",
            rr_src.display()
        );

        let runtime_out = out_dir.join(format!("{stem}_runtime.R"));
        let helper_out = out_dir.join(format!("{stem}_helper.R"));
        let preserve_out = out_dir.join(format!("{stem}_preserve.R"));

        compile_with_flags(&rr_bin, &rr_src, &runtime_out, &["-O2"]);
        compile_with_flags(&rr_bin, &rr_src, &helper_out, &["-O2", "--no-runtime"]);
        compile_with_flags(
            &rr_bin,
            &rr_src,
            &preserve_out,
            &["-O2", "--preserve-all-defs"],
        );

        let runtime_run = run_rscript(&rscript, &runtime_out);
        let helper_run = run_rscript(&rscript, &helper_out);
        let preserve_run = run_rscript(&rscript, &preserve_out);

        assert_eq!(
            runtime_run.status, 0,
            "{stem}: default runtime-injected artifact failed"
        );
        assert_eq!(helper_run.status, 0, "{stem}: helper-only artifact failed");
        assert_eq!(
            preserve_run.status, 0,
            "{stem}: preserve-all-defs artifact failed"
        );

        assert_runs_match(stem, "runtime", &runtime_run, "helper", &helper_run);
        assert_runs_match(stem, "runtime", &runtime_run, "preserve", &preserve_run);
    }
}

#[test]
fn physics_examples_match_across_artifact_modes() {
    let _guard = env_lock().lock().expect("env lock poisoned");
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping artifact-mode equivalence: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let src_dir = root.join("example").join("physics");
    let out_dir = root
        .join("target")
        .join("tests")
        .join("artifact_mode_equivalence_physics");
    fs::create_dir_all(&out_dir).expect("failed to create physics equivalence dir");

    for stem in PHYSICS_ARTIFACT_EQUIV_CASES {
        let rr_src = src_dir.join(format!("{stem}.rr"));
        assert!(
            rr_src.exists(),
            "missing physics example {}",
            rr_src.display()
        );

        let runtime_out = out_dir.join(format!("{stem}_runtime.R"));
        let helper_out = out_dir.join(format!("{stem}_helper.R"));
        let preserve_out = out_dir.join(format!("{stem}_preserve.R"));

        compile_with_flags(&rr_bin, &rr_src, &runtime_out, &["-O2"]);
        compile_with_flags(&rr_bin, &rr_src, &helper_out, &["-O2", "--no-runtime"]);
        compile_with_flags(
            &rr_bin,
            &rr_src,
            &preserve_out,
            &["-O2", "--preserve-all-defs"],
        );

        let runtime_run = run_rscript(&rscript, &runtime_out);
        let helper_run = run_rscript(&rscript, &helper_out);
        let preserve_run = run_rscript(&rscript, &preserve_out);

        assert_eq!(
            runtime_run.status, 0,
            "{stem}: default runtime-injected artifact failed"
        );
        assert_eq!(helper_run.status, 0, "{stem}: helper-only artifact failed");
        assert_eq!(
            preserve_run.status, 0,
            "{stem}: preserve-all-defs artifact failed"
        );

        assert_runs_match(stem, "runtime", &runtime_run, "helper", &helper_run);
        assert_runs_match(stem, "runtime", &runtime_run, "preserve", &preserve_run);
    }
}

#[test]
fn graphics_example_matches_across_artifact_modes() {
    let _guard = env_lock().lock().expect("env lock poisoned");
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping artifact-mode equivalence: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root
        .join("example")
        .join("visualization")
        .join("graphics_sine_plot.rr");
    let out_dir = root
        .join("target")
        .join("tests")
        .join("artifact_mode_equivalence_graphics");
    fs::create_dir_all(&out_dir).expect("failed to create graphics equivalence dir");

    let runtime_out = out_dir.join("graphics_runtime.R");
    let helper_out = out_dir.join("graphics_helper.R");
    let preserve_out = out_dir.join("graphics_preserve.R");

    compile_with_flags(&rr_bin, &rr_src, &runtime_out, &["-O2"]);
    compile_with_flags(&rr_bin, &rr_src, &helper_out, &["-O2", "--no-runtime"]);
    compile_with_flags(
        &rr_bin,
        &rr_src,
        &preserve_out,
        &["-O2", "--preserve-all-defs"],
    );

    let runtime_run_dir = out_dir.join("runtime_run");
    let helper_run_dir = out_dir.join("helper_run");
    let preserve_run_dir = out_dir.join("preserve_run");

    let runtime_run = run_rscript_in_dir(&rscript, &runtime_out, &runtime_run_dir);
    let helper_run = run_rscript_in_dir(&rscript, &helper_out, &helper_run_dir);
    let preserve_run = run_rscript_in_dir(&rscript, &preserve_out, &preserve_run_dir);

    assert_eq!(
        runtime_run.status, 0,
        "graphics runtime-injected artifact failed"
    );
    assert_eq!(helper_run.status, 0, "graphics helper-only artifact failed");
    assert_eq!(
        preserve_run.status, 0,
        "graphics preserve-all-defs artifact failed"
    );

    for run_dir in [&runtime_run_dir, &helper_run_dir, &preserve_run_dir] {
        let png_path = run_dir.join("rr_graphics_sine_plot.png");
        let meta = fs::metadata(&png_path).expect("expected graphics PNG output");
        assert!(meta.len() > 0, "expected non-empty graphics PNG output");
    }

    assert_runs_match(
        "graphics_sine_plot",
        "runtime",
        &runtime_run,
        "helper",
        &helper_run,
    );
    assert_runs_match(
        "graphics_sine_plot",
        "runtime",
        &runtime_run,
        "preserve",
        &preserve_run,
    );

    assert_file_bytes_match(
        "graphics_sine_plot",
        "runtime",
        &runtime_run_dir.join("rr_graphics_sine_plot.png"),
        "helper",
        &helper_run_dir.join("rr_graphics_sine_plot.png"),
    );
    assert_file_bytes_match(
        "graphics_sine_plot",
        "runtime",
        &runtime_run_dir.join("rr_graphics_sine_plot.png"),
        "preserve",
        &preserve_run_dir.join("rr_graphics_sine_plot.png"),
    );
}

#[test]
fn interop_visualization_examples_match_across_artifact_modes() {
    let _guard = env_lock().lock().expect("env lock poisoned");
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping artifact-mode equivalence: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let src_dir = root.join("example").join("visualization");
    let out_dir = root
        .join("target")
        .join("tests")
        .join("artifact_mode_equivalence_visualization");
    fs::create_dir_all(&out_dir).expect("failed to create interop visualization equivalence dir");

    let cases = [
        (
            "dplyr_join_facet_grid_plot_modern",
            vec!["ggplot2", "dplyr"],
            vec!["rr_dplyr_join_facet_grid_plot_modern.png"],
        ),
        (
            "readr_tidyr_facet_grid_workflow_modern",
            vec!["ggplot2", "readr", "tidyr"],
            vec![
                "rr_readr_tidyr_facet_grid_workflow_modern.png",
                "rr_readr_tidyr_facet_grid_workflow_modern.tsv",
            ],
        ),
    ];

    for (stem, packages, outputs) in cases {
        if !packages_available(&rscript, &packages) {
            eprintln!(
                "Skipping {stem} artifact-mode equivalence: required packages not available."
            );
            continue;
        }

        let rr_src = src_dir.join(format!("{stem}.rr"));
        assert!(
            rr_src.exists(),
            "missing visualization example {}",
            rr_src.display()
        );

        let runtime_out = out_dir.join(format!("{stem}_runtime.R"));
        let helper_out = out_dir.join(format!("{stem}_helper.R"));
        let preserve_out = out_dir.join(format!("{stem}_preserve.R"));

        compile_with_flags(&rr_bin, &rr_src, &runtime_out, &["-O2"]);
        compile_with_flags(&rr_bin, &rr_src, &helper_out, &["-O2", "--no-runtime"]);
        compile_with_flags(
            &rr_bin,
            &rr_src,
            &preserve_out,
            &["-O2", "--preserve-all-defs"],
        );

        let runtime_run_dir = out_dir.join(format!("{stem}_runtime_run"));
        let helper_run_dir = out_dir.join(format!("{stem}_helper_run"));
        let preserve_run_dir = out_dir.join(format!("{stem}_preserve_run"));

        let runtime_run = run_rscript_in_dir(&rscript, &runtime_out, &runtime_run_dir);
        let helper_run = run_rscript_in_dir(&rscript, &helper_out, &helper_run_dir);
        let preserve_run = run_rscript_in_dir(&rscript, &preserve_out, &preserve_run_dir);

        assert_eq!(
            runtime_run.status, 0,
            "{stem}: runtime-injected artifact failed"
        );
        assert_eq!(helper_run.status, 0, "{stem}: helper-only artifact failed");
        assert_eq!(
            preserve_run.status, 0,
            "{stem}: preserve-all-defs artifact failed"
        );

        assert_runs_match(stem, "runtime", &runtime_run, "helper", &helper_run);
        assert_runs_match(stem, "runtime", &runtime_run, "preserve", &preserve_run);

        for rel in outputs {
            let runtime_path = runtime_run_dir.join(rel);
            let helper_path = helper_run_dir.join(rel);
            let preserve_path = preserve_run_dir.join(rel);
            let runtime_meta = fs::metadata(&runtime_path)
                .unwrap_or_else(|err| panic!("{stem}: expected runtime output {rel}: {err}"));
            assert!(
                runtime_meta.len() > 0,
                "{stem}: expected non-empty runtime output {rel}"
            );
            assert_file_bytes_match(stem, "runtime", &runtime_path, "helper", &helper_path);
            assert_file_bytes_match(stem, "runtime", &runtime_path, "preserve", &preserve_path);
        }
    }
}
