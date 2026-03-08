mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn import_r_package_lowers_to_package_name_namespace_alias() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping r_package_import test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("r_package_import");
    fs::create_dir_all(&out_dir).expect("failed to create r_package_import dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r "graphics";

main <- function() {
  graphics.plot(c(1, 2, 3), c(1, 4, 9), type = "l")
  print("graphics_loaded")
  return 0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import.rr");
    fs::write(&rr_path, rr_src).expect("failed to write r_package_import source");
    let out = out_dir.join("r_package_import_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("graphics::plot(")
            && !code.contains("library(\"graphics\")")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected `import r \"graphics\"` to lower to package-name namespace interop"
    );

    let run = run_rscript(&rscript, &out);
    assert_eq!(run.status, 0, "runtime failed: {}", run.stderr);
    assert!(
        normalize(&run.stdout).contains("graphics_loaded"),
        "unexpected stdout: {}",
        run.stdout
    );
}

#[test]
fn named_alias_and_default_r_imports_lower_to_namespaced_calls() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping named alias/default R import test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_named");
    fs::create_dir_all(&out_dir).expect("failed to create named import dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r { plot as draw_plot } from "graphics";
import r default from "grDevices";

main <- function() {
  outfile <- "named_import_plot.png"
  grDevices.png(filename = outfile, width = 320, height = 240)
  draw_plot(c(1, 2, 3), c(1, 4, 9), type = "l")
  grDevices.dev.off()
  print("named_import_loaded")
  print(outfile)
  return 0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_named.rr");
    fs::write(&rr_path, rr_src).expect("failed to write named import source");
    let out = out_dir.join("r_package_import_named_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("graphics::plot(")
            && code.contains("grDevices::png(")
            && code.contains("grDevices::dev.off()")
            && !code.contains("library(\"graphics\")")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected named and default R imports to lower to directly understood namespaced calls"
    );

    let output = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&out)
        .output()
        .expect("failed to execute Rscript");
    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "runtime failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("named_import_loaded"),
        "unexpected stdout: {}",
        stdout
    );
    let png_path = out_dir.join("named_import_plot.png");
    let meta = fs::metadata(&png_path).expect("expected named import PNG output");
    assert!(meta.len() > 0, "expected non-empty named import PNG output");
}

#[test]
fn namespace_r_import_lowers_to_namespaced_calls() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping namespace R import test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_namespace");
    fs::create_dir_all(&out_dir).expect("failed to create namespace import dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as graphics from "graphics";
import r * as grDevices from "grDevices";

main <- function() {
  outfile <- "namespace_import_plot.png"
  grDevices.png(filename = outfile, width = 320, height = 240)
  graphics.plot(c(1, 2, 3), c(1, 4, 9), type = "l")
  grDevices.dev.off()
  print("namespace_import_loaded")
  print(outfile)
  return 0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_namespace.rr");
    fs::write(&rr_path, rr_src).expect("failed to write namespace import source");
    let out = out_dir.join("r_package_import_namespace_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("graphics::plot(")
            && code.contains("grDevices::png(")
            && code.contains("grDevices::dev.off()")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected namespace R imports to lower to directly understood namespaced calls"
    );

    let output = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&out)
        .output()
        .expect("failed to execute Rscript");
    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "runtime failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("namespace_import_loaded"),
        "unexpected stdout: {}",
        stdout
    );
    let png_path = out_dir.join("namespace_import_plot.png");
    let meta = fs::metadata(&png_path).expect("expected namespace import PNG output");
    assert!(
        meta.len() > 0,
        "expected non-empty namespace import PNG output"
    );
}

#[test]
fn direct_stats_readr_tidyr_interop_avoids_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_direct_surface");
    fs::create_dir_all(&out_dir).expect("failed to create direct surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base";
import r default from "stats";
import r default from "readr";
import r default from "tidyr";

fn main() {
    raw = base.data.frame(name = c("a", "b"), value_a = c(1.0, 3.0), value_b = c(2.0, 4.0))
    long = tidyr.pivot_longer(raw, cols = value_a)
    m = stats.median(c(1.0, 2.0, 3.0))
    s = stats.sd(c(1.0, 2.0, 3.0))
    readr.write_csv(long, "direct_surface.csv")
    print(m)
    print(s)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_direct_surface.rr");
    fs::write(&rr_path, rr_src).expect("failed to write direct surface import source");
    let out = out_dir.join("r_package_import_direct_surface_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("tidyr::pivot_longer(")
            && code.contains("stats::median(")
            && code.contains("stats::sd(")
            && code.contains("readr::write_csv(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected supported stats/readr/tidyr package calls to lower as direct interop:\n{code}"
    );
}

#[test]
fn unsupported_namespaced_r_call_uses_opaque_interop_not_hybrid_fallback() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping opaque interop R import test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_opaque");
    fs::create_dir_all(&out_dir).expect("failed to create opaque import dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
	import r * as utils from "utils";

	main <- function() {
	  print(utils.head(c(1, 2, 3), 2))
	  return 0L
	}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_opaque.rr");
    fs::write(&rr_path, rr_src).expect("failed to write opaque import source");
    let out = out_dir.join("r_package_import_opaque_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("utils::head(")
            && code.contains("# rr-opaque-interop:")
            && code.contains("tier=opaque")
            && code.contains("kind=package-call")
            && code.contains("package=utils")
            && code.contains("symbol=head")
            && code.contains("why=")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected unsupported namespaced package calls to lower as opaque interop"
    );

    let output = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&out)
        .output()
        .expect("failed to execute Rscript");
    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "runtime failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    assert!(stdout.contains("[1] 1 2"), "unexpected stdout: {}", stdout);
}
