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
import r "graphics"


let main <- function() {
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
import r { plot as draw_plot } from "graphics"

import r default from "grDevices"


let main <- function() {
  let outfile <- "named_import_plot.png"
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
import r * as graphics from "graphics"

import r * as grDevices from "grDevices"


let main <- function() {
  let outfile <- "namespace_import_plot.png"
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
import r default from "base"

import r default from "stats"

import r default from "readr"

import r default from "tidyr"


fn main() {
    let raw = base.data.frame(name = c("a", "b"), value_a = c(1.0, 3.0), value_b = c(2.0, 4.0))
    let long = tidyr.pivot_longer(raw, cols = value_a)
    let m = stats.median(c(1.0, 2.0, 3.0))
    let s = stats.sd(c(1.0, 2.0, 3.0))
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
fn direct_ggplot2_surface_accepts_geom_col_labs_and_theme_bw() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_ggplot2_surface");
    fs::create_dir_all(&out_dir).expect("failed to create ggplot2 direct surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "ggplot2"
import r default from "base"

fn main() {
    let df = base.data.frame(name = c("a", "b"), value = c(1.0, 2.0))
    let p = ggplot2.ggplot(df, ggplot2.aes(x = name, y = value)) +
        ggplot2.geom_col(fill = "steelblue") +
        ggplot2.labs(title = "direct surface") +
        ggplot2.theme_bw()
    print("ok")
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_ggplot2_surface.rr");
    fs::write(&rr_path, rr_src).expect("failed to write ggplot2 direct surface source");
    let out = out_dir.join("r_package_import_ggplot2_surface_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::aes(")
            && code.contains("ggplot2::geom_col(")
            && code.contains("ggplot2::labs(")
            && code.contains("ggplot2::theme_bw(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected expanded ggplot2 direct interop surface to lower without opaque or hybrid fallback:\n{code}"
    );
}

#[test]
fn direct_readr_tsv_and_dplyr_left_join_avoid_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_join_surface");
    fs::create_dir_all(&out_dir).expect("failed to create join surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "readr"
import r default from "dplyr"

fn main() {
    let left = base.data.frame(id = c(1.0, 2.0), x = c(10.0, 20.0))
    let right = base.data.frame(id = c(1.0, 2.0), y = c(30.0, 40.0))
    readr.write_tsv(left, "join_surface.tsv")
    let loaded = readr.read_tsv("join_surface.tsv", show_col_types = FALSE)
    let joined = dplyr.left_join(loaded, right, by = "id")
    print(joined)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_join_surface.rr");
    fs::write(&rr_path, rr_src).expect("failed to write join surface import source");
    let out = out_dir.join("r_package_import_join_surface_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("readr::write_tsv(")
            && code.contains("readr::read_tsv(")
            && code.contains("dplyr::left_join(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected readr TSV and dplyr left_join calls to lower as direct interop:\n{code}"
    );
}

#[test]
fn direct_ggplot2_bar_with_inner_join_avoid_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_bar_surface");
    fs::create_dir_all(&out_dir).expect("failed to create bar surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "dplyr"
import r default from "ggplot2"

fn main() {
    let left = base.data.frame(id = c(1.0, 2.0), grp = c("a", "b"), value = c(3.0, 4.0))
    let right = base.data.frame(id = c(1.0, 2.0), label = c("L1", "L2"))
    let joined = dplyr.inner_join(left, right, by = "id")
    let p = ggplot2.ggplot(joined, ggplot2.aes(x = grp, y = value)) +
        ggplot2.geom_bar(stat = "identity", fill = "steelblue") +
        ggplot2.ggtitle("bar surface")
    print("ok")
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_bar_surface.rr");
    fs::write(&rr_path, rr_src).expect("failed to write bar surface import source");
    let out = out_dir.join("r_package_import_bar_surface_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("dplyr::inner_join(")
            && code.contains("ggplot2::ggplot(")
            && code.contains("ggplot2::geom_bar(")
            && code.contains("ggplot2::ggtitle(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected expanded join/bar direct interop surface to lower without opaque or hybrid fallback:\n{code}"
    );
}

#[test]
fn direct_ggplot2_facet_wrap_formula_shorthand_lowers_without_fallback() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_facet_wrap");
    fs::create_dir_all(&out_dir).expect("failed to create facet_wrap surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "ggplot2"

fn main() {
    let df = base.data.frame(label = c("a", "b"), grp = c("x", "x"), value = c(3.0, 4.0))
    let p = ggplot2.ggplot(df, ggplot2.aes(x = grp, y = value)) +
        ggplot2.geom_col(fill = "steelblue") +
        ggplot2.facet_wrap(~label)
    print("ok")
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_facet_wrap.rr");
    fs::write(&rr_path, rr_src).expect("failed to write facet_wrap source");
    let out = out_dir.join("r_package_import_facet_wrap_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("ggplot2::facet_wrap(")
            && code.contains("stats::as.formula(\"~label\")")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected facet_wrap formula shorthand to lower through direct interop without fallback:\n{code}"
    );
}

#[test]
fn direct_ggplot2_facet_wrap_multi_term_formula_lowers_without_fallback() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_facet_wrap_multi");
    fs::create_dir_all(&out_dir).expect("failed to create facet_wrap multi surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "ggplot2"

fn main() {
    let df = base.data.frame(label = c("a", "b"), grp = c("x", "y"), value = c(3.0, 4.0))
    let p = ggplot2.ggplot(df, ggplot2.aes(x = grp, y = value)) +
        ggplot2.geom_col(fill = "steelblue") +
        ggplot2.facet_wrap(~label + grp)
    print("ok")
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_facet_wrap_multi.rr");
    fs::write(&rr_path, rr_src).expect("failed to write facet_wrap multi source");
    let out = out_dir.join("r_package_import_facet_wrap_multi_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("ggplot2::facet_wrap(")
            && code.contains("stats::as.formula(\"~label + grp\")")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected multi-term facet_wrap formula shorthand to lower through direct interop without fallback:\n{code}"
    );
}

#[test]
fn direct_ggplot2_facet_wrap_division_formula_lowers_without_fallback() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_facet_wrap_division");
    fs::create_dir_all(&out_dir).expect("failed to create facet_wrap division surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "ggplot2"

fn main() {
    let df = base.data.frame(label = c("a", "b"), grp = c("x", "y"), value = c(3.0, 4.0))
    let p = ggplot2.ggplot(df, ggplot2.aes(x = grp, y = value)) +
        ggplot2.geom_col(fill = "steelblue") +
        ggplot2.facet_wrap(~label / grp)
    print("ok")
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_facet_wrap_division.rr");
    fs::write(&rr_path, rr_src).expect("failed to write facet_wrap division source");
    let out = out_dir.join("r_package_import_facet_wrap_division_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("ggplot2::facet_wrap(")
            && code.contains("stats::as.formula(\"~label / grp\")")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected division-formula facet_wrap shorthand to lower through direct interop without fallback:\n{code}"
    );
}

#[test]
fn direct_ggplot2_facet_grid_formula_shorthand_lowers_without_fallback() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_facet_grid");
    fs::create_dir_all(&out_dir).expect("failed to create facet_grid surface dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "ggplot2"

fn main() {
    let df = base.data.frame(label = c("a", "b"), grp = c("x", "y"), value = c(3.0, 4.0))
    let p = ggplot2.ggplot(df, ggplot2.aes(x = grp, y = value)) +
        ggplot2.geom_col(fill = "steelblue") +
        ggplot2.facet_grid(rows = ~label, cols = ~grp)
    print("ok")
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_facet_grid.rr");
    fs::write(&rr_path, rr_src).expect("failed to write facet_grid source");
    let out = out_dir.join("r_package_import_facet_grid_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("ggplot2::facet_grid(")
            && code.contains("rows = stats::as.formula(\"~label\")")
            && code.contains("cols = stats::as.formula(\"~grp\")")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected facet_grid formula shorthand to lower through direct interop without fallback:\n{code}"
    );
}

#[test]
fn direct_stats_glm_model_formula_shorthand_lowers_without_fallback() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_glm_formula");
    fs::create_dir_all(&out_dir).expect("failed to create glm formula dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "stats"

fn main() {
    let train = base.data.frame(x = c(0.0, 1.0, 2.0), y = c(1.0, 2.0, 4.0))
    let model = stats.glm(y ~ x, data = train)
    print(model)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_glm_formula.rr");
    fs::write(&rr_path, rr_src).expect("failed to write glm formula source");
    let out = out_dir.join("r_package_import_glm_formula_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("stats::glm(")
            && code.contains("stats::as.formula(\"y ~ x\")")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected model-formula shorthand to lower through direct interop without fallback:\n{code}"
    );
}

#[test]
fn direct_stats_lm_model_formula_with_additive_rhs_lowers_without_fallback() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_lm_formula_add");
    fs::create_dir_all(&out_dir).expect("failed to create lm additive formula dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "stats"

fn main() {
    let train = base.data.frame(
        x = c(0.0, 1.0, 2.0),
        z = c(1.0, 0.0, 1.0),
        y = c(1.0, 2.0, 4.0)
    )
    let model = stats.lm(y ~ x + z, data = train)
    print(model)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_lm_formula_add.rr");
    fs::write(&rr_path, rr_src).expect("failed to write lm additive formula source");
    let out = out_dir.join("r_package_import_lm_formula_add_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("stats::lm(")
            && code.contains("stats::as.formula(\"y ~ x + z\")")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected additive model-formula shorthand to lower through direct interop without fallback:\n{code}"
    );
}

#[test]
fn direct_stats_lm_model_formula_with_interaction_rhs_lowers_without_fallback() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_lm_formula_interaction");
    fs::create_dir_all(&out_dir).expect("failed to create lm interaction formula dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "stats"

fn main() {
    let train = base.data.frame(
        x = c(0.0, 1.0, 2.0),
        z = c(1.0, 0.0, 1.0),
        y = c(1.0, 2.0, 4.0)
    )
    let model = stats.lm(y ~ x * z, data = train)
    print(model)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_lm_formula_interaction.rr");
    fs::write(&rr_path, rr_src).expect("failed to write lm interaction formula source");
    let out = out_dir.join("r_package_import_lm_formula_interaction_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("stats::lm(")
            && code.contains("stats::as.formula(\"y ~ x * z\")")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected interaction model-formula shorthand to lower through direct interop without fallback:\n{code}"
    );
}

#[test]
fn direct_dplyr_full_and_right_join_avoid_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_join_variants");
    fs::create_dir_all(&out_dir).expect("failed to create join variants dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "dplyr"

fn main() {
    let left = base.data.frame(id = c(1.0, 2.0), x = c(10.0, 20.0))
    let right = base.data.frame(id = c(2.0, 3.0), y = c(40.0, 50.0))
    let rj = dplyr.right_join(left, right, by = "id")
    let fj = dplyr.full_join(left, right, by = "id")
    print(rj)
    print(fj)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_join_variants.rr");
    fs::write(&rr_path, rr_src).expect("failed to write join variants source");
    let out = out_dir.join("r_package_import_join_variants_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("dplyr::right_join(")
            && code.contains("dplyr::full_join(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected right_join/full_join calls to lower as direct interop:\n{code}"
    );
}

#[test]
fn direct_readr_delim_surface_avoids_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_readr_delim");
    fs::create_dir_all(&out_dir).expect("failed to create readr delim dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "readr"

fn main() {
    let df = base.data.frame(id = c(1.0, 2.0), value = c(10.0, 20.0))
    readr.write_delim(df, "delim_surface.tsv", delim = "\t")
    let loaded = readr.read_delim("delim_surface.tsv", delim = "\t", show_col_types = FALSE)
    print(loaded)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_readr_delim.rr");
    fs::write(&rr_path, rr_src).expect("failed to write readr delim source");
    let out = out_dir.join("r_package_import_readr_delim_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("readr::write_delim(")
            && code.contains("readr::read_delim(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected readr read_delim/write_delim calls to lower as direct interop:\n{code}"
    );
}

#[test]
fn direct_readr_rds_surface_avoids_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_readr_rds");
    fs::create_dir_all(&out_dir).expect("failed to create readr rds dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "readr"

fn main() {
    let df = base.data.frame(id = c(1.0, 2.0), value = c(10.0, 20.0))
    readr.write_rds(df, "surface.rds")
    let loaded = readr.read_rds("surface.rds")
    print(loaded)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_readr_rds.rr");
    fs::write(&rr_path, rr_src).expect("failed to write readr rds source");
    let out = out_dir.join("r_package_import_readr_rds_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("readr::write_rds(")
            && code.contains("readr::read_rds(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected readr read_rds/write_rds calls to lower as direct interop:\n{code}"
    );
}

#[test]
fn direct_tidyr_separate_unite_and_dplyr_bind_rows_avoid_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_tidyr_bind_rows");
    fs::create_dir_all(&out_dir).expect("failed to create tidyr/bind_rows dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "tidyr"
import r default from "dplyr"

fn main() {
    let raw = base.data.frame(name = c("alpha-beta", "gamma-delta"), value = c(1.0, 2.0))
    let split = tidyr.separate(raw, col = "name", into = c("left", "right"), sep = "-")
    let joined = tidyr.unite(split, "combined", left, right, sep = ":")
    let stacked = dplyr.bind_rows(joined, joined)
    print(stacked)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_tidyr_bind_rows.rr");
    fs::write(&rr_path, rr_src).expect("failed to write tidyr/bind_rows source");
    let out = out_dir.join("r_package_import_tidyr_bind_rows_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("tidyr::separate(")
            && code.contains("tidyr::unite(")
            && code.contains("dplyr::bind_rows(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected tidyr separate/unite and dplyr bind_rows calls to lower as direct interop:\n{code}"
    );
}

#[test]
fn direct_dplyr_semi_and_anti_join_avoid_opaque_and_hybrid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_join_filters");
    fs::create_dir_all(&out_dir).expect("failed to create join filter dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "base"
import r default from "dplyr"

fn main() {
    let left = base.data.frame(id = c(1.0, 2.0, 3.0), x = c(10.0, 20.0, 30.0))
    let right = base.data.frame(id = c(2.0, 4.0), y = c(40.0, 50.0))
    let sj = dplyr.semi_join(left, right, by = "id")
    let aj = dplyr.anti_join(left, right, by = "id")
    print(sj)
    print(aj)
    0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_join_filters.rr");
    fs::write(&rr_path, rr_src).expect("failed to write join filter source");
    let out = out_dir.join("r_package_import_join_filters_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("base::data.frame(")
            && code.contains("dplyr::semi_join(")
            && code.contains("dplyr::anti_join(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected semi_join/anti_join calls to lower as direct interop:\n{code}"
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
	import r * as utils from "utils"


	let main <- function() {
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
