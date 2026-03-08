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
        code.contains("mix_sq <- ((Cs * DX) * (Cs * DX))")
            && code.contains("visc <- rr_assign_slice(visc, i, .arg_size, (mix_sq * sqrt(")
            && code.contains("rr_index1_read_vec(.arg_u, .arg_n_r)")
            && code.contains("rr_index1_read_vec(.arg_v, .arg_n_l)"),
        "expected Smagorinsky kernel to lower to vector gather/slice form"
    );
    assert!(
        code.contains("du <- rr_assign_slice(du, i, .arg_size, (((0 - ((9.81 * (")
            && code.contains("rr_index1_read_vec(.arg_h, .arg_n_r)")
            && code.contains("rr_index1_read_vec(.arg_u, .arg_n_l)"),
        "expected tendency kernel to lower to vector gather/slice form"
    );
    assert!(
        code.contains("rr_wrap_index_vec_i(")
            && code.contains("lap <- rr_assign_slice(lap, i, size, (")
            && !code.contains("(NULL -"),
        "expected laplacian kernel to lower to vector wrap-index gather/slice form without NULL arithmetic"
    );
    let has_weno_flux_guard = code.contains("flux <- rr_ifelse_strict((.arg_u_vel > 0), (")
        || code.contains("flux <- rr_ifelse_strict((.arg_u_vel > idx_rrr), (");
    assert!(
        has_weno_flux_guard
            && code.contains("Sym_109(.__rr_cse_102, .arg_field, .__rr_cse_104, .__rr_cse_106")
            && code.contains("rr_index_vec_floor(.arg_n_rr)"),
        "expected WENO advection kernel to lower to vector conditional form"
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
        code.contains("rr_index1_read_vec("),
        "expected optimized tesseract output to retain vector gather lowering"
    );
    assert!(
        code.contains("rr_assign_slice("),
        "expected optimized tesseract output to retain vector slice lowering"
    );
    assert!(
        !code.contains("Sym_1("),
        "expected floor helper calls to rewrite to builtin floor paths in optimized output"
    );
    assert!(
        code.contains("Particle 1 Position (X):"),
        "expected tesseract runtime markers to remain in emitted output"
    );
}
