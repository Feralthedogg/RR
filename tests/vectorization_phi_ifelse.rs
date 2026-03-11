use regex::Regex;
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
    let smagorinsky_vectorized = Regex::new(
        r"(?s)mix_sq <- \(\(Cs \* DX\) \* \(Cs \* DX\)\).*?(rr_gather|rr_index1_read_vec)\(\.arg_n_r, .*?(rr_gather|rr_index1_read_vec)\(\.arg_u, .*?(rr_gather|rr_index1_read_vec)\(\.arg_n_l, .*?(rr_gather|rr_index1_read_vec)\(\.arg_v, .*?visc <- rr_assign_slice\(visc, i, \.arg_size, \(mix_sq \* sqrt\(",
    )
    .expect("invalid Smagorinsky regex");
    assert!(
        smagorinsky_vectorized.is_match(&code),
        "expected Smagorinsky kernel to lower to vector gather/slice form"
    );
    let tendency_vectorized = Regex::new(
        r"(?s)(rr_gather|rr_index1_read_vec)\(\.arg_n_r, .*?(rr_gather|rr_index1_read_vec)\(\.arg_n_l, .*?du <- rr_assign_slice\(du, i, \.arg_size, .*?(rr_gather|rr_index1_read_vec)\(\.arg_h, .*?(rr_gather|rr_index1_read_vec)\(\.arg_u, ",
    )
    .expect("invalid tendency regex");
    assert!(
        tendency_vectorized.is_match(&code),
        "expected tendency kernel to lower to vector gather/slice form"
    );
    assert!(
        code.contains("rr_wrap_index_vec_i(")
            && code.contains("lap <- rr_assign_slice(lap, i, size, (")
            && !code.contains("(NULL -"),
        "expected laplacian kernel to lower to vector wrap-index gather/slice form without NULL arithmetic"
    );
    let weno_vectorized = Regex::new(
        r"(?s)rr_index1_read_vec\(\.arg_n_rr, .*?flux <- rr_assign_slice\(flux, i, \.arg_size, rr_ifelse_strict\(.*?Sym_",
    )
    .expect("invalid WENO regex");
    assert!(
        weno_vectorized.is_match(&code),
        "expected WENO advection kernel to lower to vector conditional form"
    );
    assert!(
        code.contains("return(rr_named_list(\"px\", .arg_px, \"py\", .arg_py, \"pf\", .arg_pf))")
            && Regex::new(r"particles <- Sym_\d+\(p_x, p_y, p_f, u, v, 0\.1, 40, TOTAL\)")
                .expect("invalid particle regex")
                .is_match(&code)
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
