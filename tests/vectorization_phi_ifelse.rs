use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg("--no-runtime")
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_src.display(),
        level
    );
}

#[test]
fn tesseract_emits_expected_vectorized_kernels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_src = root.join("example").join("tesseract.rr");
    assert!(rr_src.exists(), "missing {}", rr_src.display());

    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_phi_ifelse");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out = out_dir.join("tesseract_phi_ifelse_o2.R");
    compile_rr(&rr_bin, &rr_src, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read O2 output");
    assert!(
        code.contains("Sym_78 <- function(u, v, n_l, n_r, n_d, n_u, size)")
            && code
                .contains(".__rr_cse_105 <- rr_index1_read_vec(.arg_n_r, .tachyon_scatter_idx_0)")
            && code.contains(
                "visc <- rr_assign_index_vec(visc, .tachyon_scatter_idx_0, .tachyon_scatter_val_0)"
            ),
        "expected Smagorinsky kernel to lower to vector gather/scatter form"
    );
    assert!(
        code.contains(
            "Sym_83 <- function(u, v, h, h_trn, coriolis, visc, n_l, n_r, n_d, n_u, size)"
        ) && code.contains(
            "du <- rr_assign_index_vec(du, .tachyon_scatter_idx_0, .tachyon_scatter_val_0)"
        ),
        "expected tendency kernel to lower to vector gather/scatter form"
    );
    assert!(
        code.contains("Sym_103 <- function(field, w, h)")
            && code.contains("rr_wrap_index_vec_i(")
            && code.contains(
                "lap <- rr_assign_index_vec(lap, .tachyon_scatter_idx_0, .tachyon_scatter_val_0)"
            )
            && !code.contains("(NULL -"),
        "expected laplacian kernel to lower to vector wrap-index gather/scatter form without NULL arithmetic"
    );
    assert!(
        code.contains("return(rr_named_list(\"px\", .arg_px, \"py\", .arg_py, \"pf\", .arg_pf))")
            && code.contains("particles <- Sym_92(p_x, p_y, p_f, u, v, 0.1, 40, TOTAL)")
            && code.contains("p_x <- rr_field_get(particles, \"px\")")
            && code.contains("p_y <- rr_field_get(particles, \"py\")")
            && code.contains("p_f <- rr_field_get(particles, \"pf\")"),
        "expected particle state to be threaded back through a record return and field rebinding"
    );
    assert!(
        !code.contains("p_check <- Sym_"),
        "stale particle state placeholder should not remain in optimized tesseract output"
    );
    assert!(
        !code.contains("# rr-hybrid-fallback:"),
        "expected optimized tesseract output to avoid hybrid fallback comments after MIR validation"
    );
    assert!(
        code.contains("rr_assign_index_vec("),
        "expected optimized tesseract output to retain vector scatter lowering"
    );
    assert!(
        code.contains("rr_index1_read_vec("),
        "expected optimized tesseract output to retain vector gather lowering"
    );
    assert!(
        code.contains("Particle 1 Position (X):"),
        "expected tesseract runtime markers to remain in emitted output"
    );
}
