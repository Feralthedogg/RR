mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn configured_rscript() -> Option<PathBuf> {
    match rscript_path() {
        Some(p) if rscript_available(&p) => Some(PathBuf::from(p)),
        _ => None,
    }
}

fn compile_visualization_example(rr_src_name: &str, compiled_name: &str) -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("visualization").join(rr_src_name);
    let out_dir = root.join("target").join("examples_visualization");
    fs::create_dir_all(&out_dir).expect("failed to create target/examples_visualization");

    let compiled = out_dir.join(compiled_name);
    compile_rr(&rr_bin, &rr_src, &compiled, "-O2");
    (compiled, out_dir)
}

fn run_rscript_and_assert_png(
    rscript: &Path,
    compiled: &Path,
    run_dir: &Path,
    png_name: &str,
) -> String {
    fs::create_dir_all(run_dir).expect("failed to create visualization runtime dir");
    let output = Command::new(rscript)
        .current_dir(run_dir)
        .arg("--vanilla")
        .arg(compiled)
        .output()
        .expect("failed to execute Rscript");

    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    let stderr = normalize(&String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "visualization example runtime failed:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("saved_plot") && stdout.contains(png_name),
        "visualization example stdout missing save markers:\nstdout={stdout}"
    );

    let png_path = run_dir.join(png_name);
    let meta = fs::metadata(&png_path).expect("expected visualization PNG output");
    assert!(meta.len() > 0, "expected non-empty PNG output");
    stdout
}

fn assert_graphics_code(code: &str) {
    assert!(
        code.contains("grDevices::png(filename = outfile, width = 960L, height = 540L)")
            && code.contains("graphics::plot(x, signal")
            && code.contains("graphics::lines(x, trend")
            && code.contains("graphics::legend(\"topright\"")
            && code.contains("grDevices::dev.off()")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected compiled RR visualization example to preserve namespaced R plotting calls without hybrid fallback"
    );
}

fn ggplot2_available(rscript: &Path) -> bool {
    packages_available(rscript, &["ggplot2"])
}

fn packages_available(rscript: &Path, packages: &[&str]) -> bool {
    let package_checks = packages
        .iter()
        .map(|pkg| format!("requireNamespace('{}', quietly = TRUE)", pkg))
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

fn assert_ggplot2_code(code: &str) {
    assert!(
        code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::aes(")
            && code.contains("ggplot2::geom_line(")
            && code.contains("ggplot2::geom_point(")
            && code.contains("ggplot2::ggsave(")
            && code.contains("base::data.frame(")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected compiled RR ggplot2 example to preserve namespaced package calls without hybrid fallback"
    );
}

fn assert_dplyr_ggplot2_pipeline_code(code: &str) {
    assert!(
        code.contains("dplyr::mutate(")
            && code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::aes(")
            && code.contains("ggplot2::geom_line(")
            && code.contains("ggplot2::geom_point(")
            && code.contains("ggplot2::ggsave(")
            && code.contains("base::data.frame(")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected compiled RR dplyr + ggplot2 example to preserve namespaced package calls without hybrid fallback"
    );
}

fn assert_readr_tidyr_ggplot2_pipeline_code(code: &str) {
    assert!(
        code.contains("readr::write_csv(")
            && code.contains("readr::read_csv(")
            && code.contains("tidyr::pivot_longer(")
            && code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::aes(")
            && code.contains("ggplot2::geom_line(")
            && code.contains("ggplot2::geom_point(")
            && code.contains("ggplot2::ggsave(")
            && code.contains("base::data.frame(")
            && !code.contains("# rr-hybrid-fallback:")
            && !code.contains("# rr-opaque-interop:"),
        "expected compiled RR readr + tidyr + ggplot2 example to preserve namespaced package calls as direct interop"
    );
}

fn assert_stats_quantile_band_code(code: &str) {
    assert!(
        code.contains("stats::quantile(")
            && code.contains("stats::sd(")
            && code.contains("base::data.frame(")
            && code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::geom_line(")
            && code.contains("ggplot2::geom_point(")
            && code.contains("ggplot2::ggsave(")
            && !code.contains("# rr-hybrid-fallback:")
            && !code.contains("# rr-opaque-interop:"),
        "expected compiled RR stats + ggplot2 example to preserve direct interop package calls"
    );
}

fn assert_tidyr_summary_pivot_wider_code(code: &str) {
    assert!(
        code.contains("dplyr::group_by(")
            && code.contains("dplyr::summarise(")
            && code.contains("stats::median(")
            && code.contains("stats::sd(")
            && code.contains("tidyr::pivot_wider(")
            && code.contains("tidyr::pivot_longer(")
            && code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::geom_point(")
            && code.contains("ggplot2::geom_line(")
            && !code.contains("# rr-hybrid-fallback:")
            && !code.contains("# rr-opaque-interop:"),
        "expected compiled RR tidyr summary/pivot_wider example to preserve direct interop package calls"
    );
}

fn assert_stats_glm_predict_code(code: &str) {
    assert!(
        code.contains("stats::glm(")
            && code.contains("stats::as.formula(\"y ~ x\")")
            && code.contains("stats::predict(")
            && code.contains("base::data.frame(")
            && code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::geom_point(")
            && code.contains("ggplot2::geom_line(")
            && !code.contains("# rr-hybrid-fallback:")
            && !code.contains("# rr-opaque-interop:"),
        "expected compiled RR stats::glm/predict example to preserve direct interop package calls"
    );
}

fn assert_readr_dplyr_tidyr_workflow_code(code: &str) {
    assert!(
        code.contains("readr::write_csv(")
            && code.contains("readr::read_csv(")
            && code.contains("tidyr::pivot_longer(")
            && code.contains("dplyr::mutate(")
            && code.contains("stats::median(")
            && code.contains("stats::sd(")
            && code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::geom_line(")
            && code.contains("ggplot2::geom_point(")
            && !code.contains("# rr-hybrid-fallback:")
            && !code.contains("# rr-opaque-interop:"),
        "expected compiled RR readr + dplyr + tidyr + ggplot2 workflow example to preserve direct interop package calls"
    );
}

#[test]
fn graphics_library_example_compiles_and_generates_png() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping visualization example test: Rscript not available.");
            return;
        }
    };

    let (compiled, out_dir) =
        compile_visualization_example("graphics_sine_plot.rr", "graphics_sine_plot_o2.R");

    let code = fs::read_to_string(&compiled).expect("failed to read compiled visualization output");
    assert_graphics_code(&code);

    let run_dir = out_dir.join("graphics_runtime");
    run_rscript_and_assert_png(&rscript, &compiled, &run_dir, "rr_graphics_sine_plot.png");
}

#[test]
fn graphics_library_modern_example_compiles_and_generates_png() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping visualization example test: Rscript not available.");
            return;
        }
    };

    let (compiled, out_dir) = compile_visualization_example(
        "graphics_sine_plot_modern.rr",
        "graphics_sine_plot_modern_o2.R",
    );

    let code =
        fs::read_to_string(&compiled).expect("failed to read compiled modern visualization output");
    assert_graphics_code(&code);

    let run_dir = out_dir.join("graphics_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_graphics_sine_plot_modern.png",
    );
}

#[test]
fn ggplot2_visualization_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping ggplot2 visualization test: Rscript not available.");
            return;
        }
    };

    if !ggplot2_available(&rscript) {
        eprintln!("Skipping ggplot2 visualization test: ggplot2 not available.");
        return;
    }

    let (compiled, out_dir) =
        compile_visualization_example("ggplot2_line_plot.rr", "ggplot2_line_plot_o2.R");

    let code = fs::read_to_string(&compiled).expect("failed to read compiled ggplot2 output");
    assert_ggplot2_code(&code);

    let run_dir = out_dir.join("ggplot2_runtime");
    run_rscript_and_assert_png(&rscript, &compiled, &run_dir, "rr_ggplot2_line_plot.png");
}

#[test]
fn ggplot2_visualization_modern_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping ggplot2 visualization test: Rscript not available.");
            return;
        }
    };

    if !ggplot2_available(&rscript) {
        eprintln!("Skipping ggplot2 visualization test: ggplot2 not available.");
        return;
    }

    let (compiled, out_dir) = compile_visualization_example(
        "ggplot2_line_plot_modern.rr",
        "ggplot2_line_plot_modern_o2.R",
    );

    let code =
        fs::read_to_string(&compiled).expect("failed to read compiled modern ggplot2 output");
    assert_ggplot2_code(&code);

    let run_dir = out_dir.join("ggplot2_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_ggplot2_line_plot_modern.png",
    );
}

#[test]
fn dplyr_ggplot2_pipeline_modern_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping dplyr + ggplot2 visualization test: Rscript not available.");
            return;
        }
    };

    if !packages_available(&rscript, &["ggplot2", "dplyr"]) {
        eprintln!("Skipping dplyr + ggplot2 visualization test: required packages not available.");
        return;
    }

    let (compiled, out_dir) = compile_visualization_example(
        "dplyr_ggplot2_pipeline_modern.rr",
        "dplyr_ggplot2_pipeline_modern_o2.R",
    );

    let code = fs::read_to_string(&compiled)
        .expect("failed to read compiled dplyr + ggplot2 modern output");
    assert_dplyr_ggplot2_pipeline_code(&code);

    let run_dir = out_dir.join("dplyr_ggplot2_pipeline_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_dplyr_ggplot2_pipeline_modern.png",
    );
}

#[test]
fn readr_tidyr_ggplot2_pipeline_modern_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!(
                "Skipping readr + tidyr + ggplot2 visualization test: Rscript not available."
            );
            return;
        }
    };

    if !packages_available(&rscript, &["ggplot2", "readr", "tidyr"]) {
        eprintln!(
            "Skipping readr + tidyr + ggplot2 visualization test: required packages not available."
        );
        return;
    }

    let (compiled, out_dir) = compile_visualization_example(
        "readr_tidyr_ggplot2_pipeline_modern.rr",
        "readr_tidyr_ggplot2_pipeline_modern_o2.R",
    );

    let code = fs::read_to_string(&compiled)
        .expect("failed to read compiled readr + tidyr + ggplot2 output");
    assert_readr_tidyr_ggplot2_pipeline_code(&code);

    let run_dir = out_dir.join("readr_tidyr_ggplot2_pipeline_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_readr_tidyr_ggplot2_pipeline_modern.png",
    );
}

#[test]
fn stats_quantile_band_plot_modern_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping stats + ggplot2 visualization test: Rscript not available.");
            return;
        }
    };

    if !packages_available(&rscript, &["ggplot2"]) {
        eprintln!("Skipping stats + ggplot2 visualization test: ggplot2 not available.");
        return;
    }

    let (compiled, out_dir) = compile_visualization_example(
        "stats_quantile_band_plot_modern.rr",
        "stats_quantile_band_plot_modern_o2.R",
    );

    let code =
        fs::read_to_string(&compiled).expect("failed to read compiled stats + ggplot2 output");
    assert_stats_quantile_band_code(&code);

    let run_dir = out_dir.join("stats_quantile_band_plot_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_stats_quantile_band_plot_modern.png",
    );
}

#[test]
fn tidyr_summary_pivot_wider_modern_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping tidyr summary visualization test: Rscript not available.");
            return;
        }
    };

    if !packages_available(&rscript, &["ggplot2", "dplyr", "tidyr"]) {
        eprintln!("Skipping tidyr summary visualization test: required packages not available.");
        return;
    }

    let (compiled, out_dir) = compile_visualization_example(
        "tidyr_summary_pivot_wider_modern.rr",
        "tidyr_summary_pivot_wider_modern_o2.R",
    );

    let code = fs::read_to_string(&compiled)
        .expect("failed to read compiled tidyr summary/pivot_wider output");
    assert_tidyr_summary_pivot_wider_code(&code);

    let run_dir = out_dir.join("tidyr_summary_pivot_wider_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_tidyr_summary_pivot_wider_modern.png",
    );
}

#[test]
fn stats_glm_predict_plot_modern_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping stats::glm visualization test: Rscript not available.");
            return;
        }
    };

    if !packages_available(&rscript, &["ggplot2"]) {
        eprintln!("Skipping stats::glm visualization test: ggplot2 not available.");
        return;
    }

    let (compiled, out_dir) = compile_visualization_example(
        "stats_glm_predict_plot_modern.rr",
        "stats_glm_predict_plot_modern_o2.R",
    );

    let code = fs::read_to_string(&compiled).expect("failed to read compiled stats::glm output");
    assert_stats_glm_predict_code(&code);

    let run_dir = out_dir.join("stats_glm_predict_plot_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_stats_glm_predict_plot_modern.png",
    );
}

#[test]
fn readr_dplyr_tidyr_ggplot2_workflow_modern_example_compiles_and_generates_png_when_available() {
    let rscript = match configured_rscript() {
        Some(p) => p,
        _ => {
            eprintln!("Skipping readr+dplyr+tidyr visualization test: Rscript not available.");
            return;
        }
    };

    if !packages_available(&rscript, &["ggplot2", "readr", "dplyr", "tidyr"]) {
        eprintln!(
            "Skipping readr+dplyr+tidyr visualization test: required packages not available."
        );
        return;
    }

    let (compiled, out_dir) = compile_visualization_example(
        "readr_dplyr_tidyr_ggplot2_workflow_modern.rr",
        "readr_dplyr_tidyr_ggplot2_workflow_modern_o2.R",
    );

    let code = fs::read_to_string(&compiled)
        .expect("failed to read compiled readr+dplyr+tidyr workflow output");
    assert_readr_dplyr_tidyr_workflow_code(&code);

    let run_dir = out_dir.join("readr_dplyr_tidyr_ggplot2_workflow_modern_runtime");
    run_rscript_and_assert_png(
        &rscript,
        &compiled,
        &run_dir,
        "rr_readr_dplyr_tidyr_ggplot2_workflow_modern.png",
    );
}
