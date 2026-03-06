use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

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
fn tesseract_emits_phi_ifelse_vectorized_kernels() {
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
        code.contains("Sym_70 <- function(temp, q_v, q_c, q_r, size)")
            && code.contains("heating <- rr_assign_slice(heating, i, .arg_size, ((2500000 / 1004)")
            && code.contains("rr_ifelse_strict((.arg_q_r > 0), ((0.0001 * (.__rr_cse_126 - .arg_q_v)) * .arg_q_r), 0)"),
        "expected Sym_70 to lower phi-conditional scalar state into vectorized rr_ifelse_strict/rr_assign_slice form"
    );
    assert!(
        code.contains("Sym_118 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)")
            && Regex::new(
                r#"Sym_118 <- function\(temp, q_v, q_c, q_r, q_i, q_s, q_g, size\)[\s\S]*?heat <- rr_assign_slice\(heat, i, \.arg_size, \(rr_ifelse_strict\([\s\S]*?rr_ifelse_strict\(\(\.arg_q_g > 0\),"#
            )
            .expect("valid Sym_118 vectorization regex")
            .is_match(&code),
        "expected Sym_118 to lower nested phi-conditional state into vectorized rr_ifelse_strict/rr_assign_slice form"
    );
    assert!(
        code.contains("Sym_66 <- function(b, n_l, n_r, n_d, n_u, size)")
            && code.contains("r <- rr_assign_slice(r, k, .arg_size, .arg_b)")
            && code.contains("p <- rr_assign_slice(p, k, .arg_size, .arg_b)")
            && code.contains(".tachyon_exprmap0_0 <- (x + (alpha * p))")
            && code.contains(".tachyon_exprmap1_0 <- (r - (alpha * Ap))")
            && code.contains("x <- rr_assign_slice(x, i, .arg_size, .tachyon_exprmap0_0)")
            && code.contains("r <- rr_assign_slice(r, i, .arg_size, .tachyon_exprmap1_0)")
            && code.contains("p <- rr_assign_slice(p, i, .arg_size, (r + (beta * p)))"),
        "expected Sym_66 loops to lower independent slice stores into vectorized rr_assign_slice updates"
    );
    assert!(
        code.contains("Sym_103 <- function()")
            && code.contains(".tachyon_exprmap0_0 <- rr_ifelse_strict(")
            && code.contains(".tachyon_exprmap1_0 <- rr_ifelse_strict(")
            && code.contains("A <- rr_assign_slice(A, i, SIZE, .tachyon_exprmap0_0)")
            && code.contains("B <- rr_assign_slice(B, i, SIZE, .tachyon_exprmap1_0)"),
        "expected Sym_103 morphogenesis update loop to stage vector RHS values before rr_assign_slice updates"
    );
    let sym89 = Regex::new(
        r#"Sym_89 <- function\(px, py, pf, u, v, dt, N, total_grid\)[\s\S]*?return\(\.arg_px\)\n}"#,
    )
    .expect("valid Sym_89 extraction regex")
    .find(&code)
    .expect("expected Sym_89 in emitted code")
    .as_str();
    assert!(
        sym89.contains(".arg_px <- rr_assign_slice(.arg_px, i, 1000,")
            && sym89.contains(".arg_py <- rr_assign_slice(.arg_py, i, 1000,")
            && sym89.contains(".arg_pf <- rr_assign_slice(.arg_pf, i, 1000,")
            && sym89.contains("rr_idx_cube_vec_i("),
        "expected Sym_89 particle update loop to lower into staged slice assignments with vector index helper"
    );
    assert!(
        Regex::new(r#"rr_ifelse_strict\([^,\n]+, \.arg_N, [^\n]+\)"#)
            .expect("valid Sym_89 upper clamp regex")
            .find_iter(sym89)
            .count()
            >= 2,
        "expected Sym_89 to preserve both gx/gy upper clamps when vectorized"
    );
    let temp_re = Regex::new(r#"\.tachyon_exprcse\d+_\d+"#).expect("valid temp regex");
    let mut temp_names = BTreeSet::new();
    for m in temp_re.find_iter(sym89) {
        temp_names.insert(m.as_str().to_string());
    }
    for temp in temp_names {
        let def_pat = format!(r#"{}\s*<-"#, regex::escape(&temp));
        assert!(
            Regex::new(&def_pat)
                .expect("valid temp definition regex")
                .is_match(sym89),
            "expected Sym_89 temp {temp} to be defined within the same emitted function"
        );
    }
    assert!(
        !code.contains("# rr-hybrid-fallback:"),
        "expected optimized tesseract output to avoid hybrid fallback comments after MIR validation"
    );
}
