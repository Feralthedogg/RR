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

fn contains_all(code: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| code.contains(needle))
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
        contains_all(
            &code,
            &[
                "visc <-",
                "rr_gather(u, rr_index_vec_floor(adj_r))",
                "rr_gather(u, rr_index_vec_floor(adj_l))",
                "rr_gather(v, rr_index_vec_floor(adj_u))",
                "rr_gather(v, rr_index_vec_floor(adj_d))",
            ],
        ) || contains_all(
            &code,
            &[
                "visc <-",
                "n_d <- rr_index_vec_floor(n_d)",
                "n_l <- rr_index_vec_floor(n_l)",
                "n_r <- rr_index_vec_floor(n_r)",
                "n_u <- rr_index_vec_floor(n_u)",
                "rr_gather(u, n_r)",
                "rr_gather(u, n_l)",
                "rr_gather(v, n_u)",
                "rr_gather(v, n_d)",
            ],
        ),
        "expected Smagorinsky kernel to lower to vector gather form"
    );
    assert!(
        contains_all(
            &code,
            &[
                "rr_gather(h, rr_index_vec_floor(adj_r))",
                "rr_gather(h, rr_index_vec_floor(adj_l))",
                "rr_gather(u, rr_index_vec_floor(adj_l))",
                "rr_gather(u, rr_index_vec_floor(adj_r))",
            ],
        ) || contains_all(
            &code,
            &[
                "n_d <- rr_index_vec_floor(n_d)",
                "n_l <- rr_index_vec_floor(n_l)",
                "n_r <- rr_index_vec_floor(n_r)",
                "n_u <- rr_index_vec_floor(n_u)",
                "rr_gather(h, n_r)",
                "rr_gather(h, n_l)",
                "rr_gather(u, n_l)",
                "rr_gather(u, n_r)",
            ],
        ),
        "expected tendency kernel to lower to vector gather form"
    );
    assert!(
        code.contains("rr_wrap_index_vec_i(")
            && code.contains("lapA <-")
            && code.contains("lapB <-")
            && !code.contains("(NULL -"),
        "expected laplacian kernel to lower to vector wrap-index gather/slice form without NULL arithmetic"
    );
    assert!(
        (code.contains("adv_u <- (ifelse((u > 0),")
            || code.contains("adv_u <- (ifelse((u > 0.0),"))
            && code.contains("rr_gather(u, rr_index_vec_floor(adj_l))")
            && code.contains("rr_gather(u, rr_index_vec_floor(adj_r))"),
        "expected WENO advection kernel to lower to vector conditional form"
    );
    assert!(
        (code.contains("return(rr_named_list(\"px\", px, \"py\", py, \"pf\", pf))")
            || code.contains("return(list(px = px, py = py, pf = pf))"))
            && (Regex::new(r"particles <- Sym_\d+\(p_x, p_y, p_f, u, v, (?:dt|0\.1), (?:N|40)\)",)
                .expect("invalid particle regex")
                .is_match(&code)
                || code.contains("particles <- advect_particles(p_x, p_y, p_f, u, v, dt, N)")
                || code.contains("particles <- advect_particles(p_x, p_y, p_f, u, v, 0.1, 40)"))
            && ((code.contains("p_x <- rr_field_get(particles, \"px\")")
                || code.contains("p_x <- particles[[\"px\"]]"))
                && (code.contains("p_y <- rr_field_get(particles, \"py\")")
                    || code.contains("p_y <- particles[[\"py\"]]"))
                && (code.contains("p_f <- rr_field_get(particles, \"pf\")")
                    || code.contains("p_f <- particles[[\"pf\"]]"))
                || code.contains("particles[[\"px\"]]")
                || code.contains("rr_field_get(particles, \"px\")")),
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
        code.contains("rr_gather("),
        "expected optimized tesseract output to retain vector gather lowering"
    );
    assert!(
        code.contains("lapA <-") && code.contains("lapB <-"),
        "expected optimized tesseract output to retain direct vector laplacian lowering"
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
