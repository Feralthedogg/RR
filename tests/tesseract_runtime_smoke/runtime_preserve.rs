use super::common::{
    compile_rr, compile_rr_env, compile_rr_env_with_args, normalize, rscript_available,
    rscript_path, run_rscript,
};
use super::compile_helpers::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
pub(crate) fn tesseract_raw_o2_recovers_sym17_whole_range_replays_before_peephole() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = tesseract_test_dir("examples_tesseract_raw");

    let out = out_dir.join("tesseract_o2.R");
    let raw = out_dir.join("tesseract_o2_raw.R");
    let raw_env = raw
        .to_str()
        .expect("raw tesseract output path should be valid unicode")
        .to_string();
    compile_rr_env(
        &rr_bin,
        &rr_src,
        &out,
        "-O2",
        &[("RR_DEBUG_RAW_R_PATH", raw_env.as_str())],
    );
    if !raw.exists() {
        eprintln!(
            "Skipping raw tesseract audit: RR_DEBUG_RAW_R_PATH did not produce {}",
            raw.display()
        );
        return;
    }
    let raw_code = fs::read_to_string(&raw).expect("failed to read raw tesseract output");
    let raw_sym_186 = extract_r_function(&raw_code, "Sym_186").unwrap_or_default();

    assert!(
        raw_code.contains(
            "Ap <- ((4.0001 * p) - (((rr_gather(p, rr_index_vec_floor(n_l)) + rr_gather(p, rr_index_vec_floor(n_r))) + rr_gather(p, rr_index_vec_floor(n_d))) + rr_gather(p, rr_index_vec_floor(n_u))))"
        )
            && !raw_code.contains("Sym_119 <- function(x, n_l, n_r, n_d, n_u)"),
        "expected raw tesseract output to inline the Sym_119 helper all the way to a direct whole-vector gather expression"
    );
    assert!(
        !raw_code.contains("y <- rr_assign_slice(y, i, .arg_size,"),
        "raw tesseract output unexpectedly still contains the Sym_119 whole-range rr_assign_slice helper replay"
    );
    for needle in [
        "h <- rep.int(8000.0, TOTAL)",
        "temp <- rep.int(300.0, TOTAL)",
        "qv <- rep.int(0.015, TOTAL)",
        "qc <- rep.int(0.001, TOTAL)",
        "v <- rep.int(0.0, TOTAL)",
        "qr <- v",
        "qi <- v",
        "qs <- v",
        "qg <- v",
        "h_trn <- v",
        "coriolis <- v",
        "u_new <- v",
        "p_x <- Sym_183(1000.0)",
        "p_y <- p_x",
        "p_f <- rep.int(1.0, 1000.0)",
        "adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))",
        "adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))",
        "heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)",
    ] {
        assert!(
            raw_code.contains(needle),
            "expected raw tesseract output to contain `{needle}` after raw fill-helper inlining, zero-fill reuse, and WENO replay collapse"
        );
    }
    assert!(
        raw_code.contains("qr <- v")
            && raw_code.contains("qi <- v")
            && raw_code.contains("qs <- v")
            && raw_code.contains("qg <- v")
            && !raw_code.contains("qg <- qs"),
        "expected raw tesseract output to keep the zero-fill alias-reuse chain for qs/qg after raw helper cleanup"
    );
    assert!(
        raw_code.contains("u_new <- v")
            && raw_code.contains("u_stage <- (u + (dt * (du1 - adv_u)))"),
        "expected raw tesseract output to keep the zero-fill alias-reuse chain for u_stage/u_new after raw helper cleanup"
    );
    for needle in [
        "qi <- rep.int(0.0, TOTAL)",
        "Sym_17 <- function(n, val)",
        "h <- Sym_17(TOTAL, 8000)",
        "p_f <- Sym_17(1000, 1)",
        "p_y <- Sym_183(1000)",
        "heat2 <- Sym_287(temp, qv, qc, qs, qg, TOTAL)",
        "heat3 <- Sym_287(temp, qv, qc, qs, qg, TOTAL)",
    ] {
        assert!(
            !raw_code.contains(needle),
            "expected raw tesseract output to drop `{needle}` after raw fill-helper inlining, zero-fill reuse, and WENO replay collapse"
        );
    }
    assert!(
        !raw_code.contains("x <- rr_assign_slice(x, i, .arg_size,"),
        "raw tesseract output unexpectedly still contains the CG whole-range helper replay for x"
    );
    assert!(
        raw_code.contains("x <- (x + (alpha * p))")
            && !raw_code.contains("rr_assign_slice(r, i, .arg_size,")
            && raw_code.contains("r <- (r - (alpha * Ap))")
            && raw_code.contains("rs_new <- Sym_117(r, r, size)"),
        "expected raw tesseract output to lower the CG x/r stages to direct whole-range vector expressions while keeping the rs_new recurrence explicit"
    );
    assert!(
        raw_code.contains("Sym_117 <- function(a, b, n)")
            && raw_code.contains("rs_old <- Sym_117(r, r, size)")
            && raw_code.contains("p_Ap <- Sym_117(p, Ap, size)")
            && raw_code.contains("rs_new <- Sym_117(r, r, size)"),
        "expected raw tesseract output to keep the shared dot-product helper and explicit rs_old/p_Ap/rs_new recurrence in Sym_123"
    );
    let raw_sym_123 = extract_r_function(&raw_code, "Sym_123")
        .unwrap_or_else(|| panic!("raw tesseract output should contain Sym_123"));
    assert!(
        raw_sym_123.contains("alpha <- (rs_old / p_Ap)")
            && raw_sym_123.contains("beta <- (rs_new / rs_old)")
            && raw_sym_123.contains("repeat {")
            && raw_sym_123.contains("if (!(iter <= 20.0)) break")
            && raw_sym_123.contains("iter <- (iter + 1.0)")
            && raw_sym_123.contains("r <- (r - (alpha * Ap))")
            && raw_sym_123.contains("rs_new <- Sym_117(r, r, size)")
            && raw_sym_123.contains("p <- (r + (beta * p))")
            && raw_sym_123.contains("rs_old <- rs_new")
            && raw_sym_123.contains("if (!(is.finite(alpha))) {")
            && raw_sym_123.contains("if (!(is.finite(beta))) {"),
        "expected raw Sym_123 to keep the CG alpha/beta/r/p/rs_old recurrence with the current repeat-loop and finite-guard lowering"
    );
    assert!(
        !raw_code.contains("x <- (rep.int(0, size))")
            && !raw_code.contains("x <- Sym_17(.arg_size, 0)"),
        "expected raw tesseract output to avoid re-emitting the same zero-vector x init inside the invalid-rs_old guard"
    );
    assert!(
        !raw_code.contains("qc <- q_c[i]")
            && !raw_code.contains("qv <- q_v[i]")
            && !raw_code.contains("qs <- q_s[i]")
            && !raw_code.contains("qg <- q_g[i]")
            && !raw_code.contains("T_c <- T_c")
            && !raw_code.contains("qv <- .arg_q_v[ii]")
            && !raw_code.contains("qs <- .arg_q_s[ii]")
            && !raw_code.contains("qg <- .arg_q_g[ii]")
            && !raw_code.contains("L_v <- 2500000")
            && raw_code.contains("T_c <- (temp[i] - 273.15)")
            && raw_code.contains("melt_rate <- 0.0")
            && raw_code.contains("melt_rate <- (q_s[i] * 0.05)")
            && raw_code.contains("melt_rate <- (melt_rate + (q_g[i] * 0.02))")
            && raw_code.contains("tendency_T <- (tendency_T - (melt_rate * L_f))")
            && raw_code.contains("heat[rr_index1_write(i, \"index\")] <- (tendency_T / cp)"),
        "expected raw tesseract output to keep direct q_*[i] reads and the current T_c/melt_rate lowering inside Sym_287 without reintroducing arg-alias temps"
    );
    assert!(
        raw_code.contains("id <- rr_wrap_index_vec_i(x, y, W, H)")
            && raw_code.contains("B[id] <- 1.0")
            && raw_code.contains("center_idx <- rr_wrap_index_vec_i(32.0, 32.0, W, H)")
            && raw_code.contains("side_idx <- rr_wrap_index_vec_i(40.0, 32.0, W, H)")
            && raw_code.contains("print(B[center_idx])")
            && raw_code.contains("print(B[side_idx])"),
        "expected raw tesseract output to keep the current explicit wrap-index temporaries in the Morphogenesis update and probe paths"
    );
    let raw_trait_probe_arg_is_bound_pair = raw_code.contains(
        "probe_pair <- list(left = c(1.0, 2.0, 3.0, 4.0), right = c(4.0, 3.0, 2.0, 1.0))",
    ) && raw_code
        .contains("probe_energy <- Sym_313(probe_pair)");
    let raw_trait_probe_arg_is_direct_pair = raw_code.contains(
        "probe_energy <- Sym_313(list(left = c(1.0, 2.0, 3.0, 4.0), right = c(4.0, 3.0, 2.0, 1.0)))",
    );
    assert!(
        raw_code.contains("probe_vec <- Sym_49(c(1.0, 2.0, 3.0, 4.0), c(4.0, 3.0, 2.0, 1.0))")
            && (raw_trait_probe_arg_is_bound_pair || raw_trait_probe_arg_is_direct_pair)
            && raw_code.contains("Sym_316 <- function(self)")
            && raw_code.contains("return(Sym_51(self[[\"left\"]], self[[\"right\"]]))")
            && !raw_code.contains("FieldEnergy.energy")
            && !raw_code.contains("typed_trait_energy("),
        "expected raw tesseract output to route probe_energy through the static FieldEnergy trait dispatch path"
    );
    let raw_sym_183 = extract_r_function(&raw_code, "Sym_183")
        .unwrap_or_else(|| panic!("raw tesseract output should contain Sym_183"));
    assert!(
        raw_sym_183.contains("p <- seq_len(n)")
            && raw_sym_183.contains("seed <- 12345.0")
            && raw_sym_183.contains("repeat {")
            && (raw_sym_183.contains("if (!(i <= n)) break")
                || raw_sym_183.contains("if (!rr_truthy1((i <= n), \"condition\")) break"))
            && raw_sym_183.contains("p[i] <- (seed / 2147483648.0)")
            && raw_sym_183.contains("i <- (i + 1.0)"),
        "expected raw Sym_183 to keep the current seq_len seed and repeat-loop RNG lowering"
    );
    assert!(
        raw_code.contains("return(list(px = px, py = py, pf = pf))")
            && !raw_code.contains("return(rr_named_list(\"px\", px, \"py\", py, \"pf\", pf))"),
        "expected raw tesseract output to rewrite the particle-state rr_named_list return to a direct base list"
    );
    assert!(
        raw_code.contains("p_x <- particles[[\"px\"]]")
            && raw_code.contains("p_y <- particles[[\"py\"]]")
            && raw_code.contains("p_f <- particles[[\"pf\"]]")
            && !raw_code.contains("rr_field_get(particles, \"px\")")
            && !raw_code.contains("rr_field_get(particles, \"py\")")
            && !raw_code.contains("rr_field_get(particles, \"pf\")"),
        "expected raw tesseract output to rewrite literal particle rr_field_get calls to direct base indexing"
    );
    assert!(
        raw_code.contains("Sym_287 <- function(temp, q_v, q_c, q_s, q_g, size)")
            && !raw_code.contains("Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)")
            && raw_code.contains("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
            && !raw_code.contains("heat <- Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL)"),
        "expected raw tesseract output to drop unused middle helper params and callsite args in Sym_287"
    );
    let raw_sym_287 = extract_r_function(&raw_code, "Sym_287")
        .unwrap_or_else(|| panic!("raw tesseract output should contain Sym_287"));
    assert!(
        raw_sym_287.contains("repeat {")
            && (raw_sym_287.contains("if (!(i <= size)) break")
                || raw_sym_287.contains("if (!rr_truthy1((i <= size), \"condition\")) break"))
            && raw_sym_287.contains("i <- (i + 1.0)")
            && raw_sym_287.contains("T_c <- (temp[i] - 273.15)")
            && raw_sym_287.contains("heat[rr_index1_write(i, \"index\")] <- (tendency_T / cp)"),
        "expected raw Sym_287 to keep the current repeat-loop thermodynamics lowering with direct q_*[i] reads"
    );
    assert!(
        raw_code
            .matches("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
            .count()
            == 1
            && !raw_code.contains("heat2 <- heat")
            && !raw_code.contains("heat3 <- heat")
            && raw_code.contains("steps <- 0.0")
            && raw_code.contains("if (!(steps < (5.0))) break"),
        "expected raw tesseract RK loop to keep a single heat stage compute with the current repeat-loop lowering"
    );
    assert!(
        raw_code.matches("p_x <- particles[[\"px\"]]").count() == 1
            && raw_code.matches("p_y <- particles[[\"py\"]]").count() == 1
            && raw_code.matches("p_f <- particles[[\"pf\"]]").count() == 1,
        "expected raw tesseract RK loop to keep only one particle reload triplet from the fresh particles state"
    );
    assert!(
        !raw_code.contains(".arg_dir <- dir")
            && !raw_code.contains(".arg_size <- size")
            && !raw_code.contains(".arg_n <- n"),
        "expected raw tesseract helper bodies to rewrite readonly .arg_* aliases back to bare params where safe"
    );
    assert!(
        raw_sym_186.contains("repeat {")
            && raw_sym_186.contains("if (!(i <= (1000.0))) break")
            && raw_sym_186.contains("gx <- ((x * N) + 1.0)")
            && raw_sym_186.contains("if ((gx < 1.0)) {")
            && raw_sym_186.contains("if ((gy > N)) {")
            && raw_sym_186.contains("ix <- floor(gx)")
            && raw_sym_186.contains("iy <- floor(gy)")
            && raw_sym_186.contains("idx <- rr_idx_cube_vec_i(f, ix, iy, N)")
            && raw_sym_186.contains("dx <- ((u[idx] * dt) / 400000.0)")
            && raw_sym_186.contains("dy <- ((v[idx] * dt) / 400000.0)"),
        "expected raw Sym_186 to keep the current repeat-loop particle advection with explicit gx/gy clamp and ix/iy cube-index temporaries"
    );
    assert!(
        !raw_code.contains("rr_index1_read(.arg_u, idx, \"index\")"),
        "expected raw tesseract output to avoid rr_index1_read(.arg_u, idx, \"index\") in particle advection"
    );
    assert!(
        !raw_code.contains("rr_index1_read(.arg_v, idx, \"index\")"),
        "expected raw tesseract output to avoid rr_index1_read(.arg_v, idx, \"index\") in particle advection"
    );
    assert!(
        !raw_code.contains("rr_index1_read_vec(x, .__rr_cse_218)")
            && !raw_code.contains("rr_index1_read_vec(r, .__rr_cse_218)"),
        "raw tesseract output unexpectedly still contains stale full-range rr_index1_read_vec wrappers in CG x/r updates"
    );
    assert!(
        !raw_code.contains("y <- Sym_17(.arg_size, 0, 2)")
            && !raw_code.contains("r <- rep.int(0, .arg_size)")
            && !raw_code.contains("p <- rep.int(0, .arg_size)")
            && !raw_code.contains("visc <- rep.int(0, .arg_size)")
            && !raw_code.contains("du <- rep.int(0, .arg_size)")
            && !raw_code.contains("lap <- rep.int(0, size)")
            && !raw_code.contains("licm_71 <- (x + 1)")
            && !raw_code.contains("num_p <- 0")
            && !raw_code.contains("x_idx <- 0")
            && !raw_code.contains("y_idx <- 0")
            && !raw_code.contains("idx_bl <- 0"),
        "expected raw tesseract output to prune dead straight-line zero/fresh initializations that are immediately overwritten"
    );
    assert!(
        !raw_code.contains("# rr-cse-pruned\n  # rr-cse-pruned\n  # rr-cse-pruned"),
        "expected raw tesseract output to compact adjacent rr-cse-pruned markers after dead-init cleanup"
    );
    assert!(
        !raw_code.contains("# rr-cse-pruned"),
        "expected raw tesseract output to drop synthetic rr-cse-pruned markers after late raw cleanup"
    );
    assert!(
        !raw_code.contains("iter <- 1\n\n  repeat {")
            && !raw_code.contains("rs_old <- 0.0000001\n\n  }"),
        "expected raw tesseract output to strip single blank spacer lines around raw helper control flow"
    );
    assert!(
        !raw_code.contains("one_row <- rep.int(1, size)\n\n  if ((x > 1)) {"),
        "expected raw tesseract helper bodies to avoid blank spacer lines between cached row constants and the following branch"
    );
    assert!(
        !raw_code.contains("} else {\n      }") && !raw_code.contains("} else {\n    }"),
        "expected raw tesseract output to strip empty else blocks after raw helper cleanup"
    );
    assert!(
        raw_code.contains(
            "return(rr_idx_cube_vec_i(replace(nf, 1.0, 6.0), nx, replace(ny, 1.0, size), size))"
        ) && !raw_code
            .contains("return(rr_idx_cube_vec_i(rr_assign_slice(nf, 1, 1, rep.int(6, 1))"),
        "expected raw tesseract output to lower neighbor_row_down singleton edge edits to replace(...) expressions"
    );
    assert!(
        raw_code.contains(
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)"
        ) && raw_code.contains(
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)"
        ) && raw_code.contains(
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, ys, size)"
        ) && raw_code.contains(
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, ys, size)"
        ) && !raw_code.contains(
            "neighbors <- rr_assign_slice(neighbors, start, end, Sym_60(f, x, ys, size))"
        ),
        "expected raw tesseract output to keep direct base-R slice writes for the topology neighbor rows"
    );
    assert!(
        raw_code.contains("r <- b") && (raw_code.contains("p <- r") || raw_code.contains("p <- b")),
        "expected raw tesseract CG helper to keep the current b-seeded p initialization"
    );
    let raw_sym_83 = extract_r_function(&raw_code, "Sym_83")
        .unwrap_or_else(|| panic!("raw tesseract output should contain Sym_83"));
    assert!(
        raw_sym_83.contains("ys <- seq_len(size)")
            && raw_sym_83.contains("repeat {")
            && raw_sym_83.contains("if (!(f <= 6.0)) break")
            && raw_sym_83.contains("if (!(x <= size)) break")
            && raw_sym_83.contains("licm_28 <- (dir == 1.0)")
            && raw_sym_83.contains("licm_59 <- (dir == 4.0)")
            && (raw_sym_83.contains("if (licm_28) {")
                || raw_sym_83.contains("if (rr_truthy1(licm_28, \"condition\")) {"))
            && (raw_sym_83.contains("if (licm_59) {")
                || raw_sym_83.contains("if (rr_truthy1(licm_59, \"condition\")) {")),
        "expected raw Sym_83 topology setup to keep the current repeat-loop dir-dispatch lowering"
    );
    assert!(
        !raw_code.contains("neighbors <- rr_assign_slice(neighbors, rr_idx_cube_vec_i(f, x, 1, .arg_size), rr_idx_cube_vec_i(f, x, .arg_size, .arg_size), Sym_64(f, x, ys, .arg_size))")
            && !raw_code.contains("neighbors <- rr_assign_slice(neighbors, rr_idx_cube_vec_i(f, x, 1, size), rr_idx_cube_vec_i(f, x, size, size), Sym_64(f, x, ys, size))")
            && !raw_code.contains("neighbors <- rr_assign_slice(neighbors, rr_idx_cube_vec_i(f, x, 1, .arg_size), rr_idx_cube_vec_i(f, x, .arg_size, .arg_size), Sym_66(f, x, ys, .arg_size))")
            && !raw_code.contains("neighbors <- rr_assign_slice(neighbors, rr_idx_cube_vec_i(f, x, 1, size), rr_idx_cube_vec_i(f, x, size, size), Sym_66(f, x, ys, size))")
            && !raw_code.contains("neighbors <- rr_assign_slice(neighbors, rr_idx_cube_vec_i(f, x, 1, .arg_size), rr_idx_cube_vec_i(f, x, .arg_size, .arg_size), Sym_72(f, x, ys, .arg_size))")
            && !raw_code.contains("neighbors <- rr_assign_slice(neighbors, rr_idx_cube_vec_i(f, x, 1, size), rr_idx_cube_vec_i(f, x, size, size), Sym_72(f, x, ys, size))")
            && !raw_code.contains("neighbors[start:end] <- Sym_64(f, x, ys, size)")
            && !raw_code.contains("neighbors[start:end] <- Sym_66(f, x, ys, size)")
            && !raw_code.contains("neighbors[start:end] <- Sym_72(f, x, ys, size)")
            && !raw_code.contains("start <- rr_idx_cube_vec_i(f, x, 1, size)")
            && !raw_code.contains("end <- rr_idx_cube_vec_i(f, x, size, size)"),
        "raw tesseract output unexpectedly still keeps the slice-bound aliases or helper-heavy row writes"
    );
    assert!(
        raw_code.contains(
            "return(rr_idx_cube_vec_i(replace(nf, size, 5.0), nx, rr_assign_slice(ny, size, size, 1.0), size))"
        ) && raw_code.contains(
            "return(rr_idx_cube_vec_i(replace(nf, size, 3.0), replace(nx, size, (size - (x - 1.0))), rr_assign_slice(ny, size, size, size), size))"
        ),
        "expected raw tesseract output to keep the current neighbor_row_up edge remap with replace(nf, ...) and singleton ny rr_assign_slice edits"
    );
    assert!(
        !raw_code.contains("rr_assign_slice(nf, .arg_size, .arg_size, rep.int(5, 1))")
            && !raw_code.contains("rr_assign_slice(nf, size, size, rep.int(5, 1))")
            && !raw_code.contains("rr_assign_slice(nf, .arg_size, .arg_size, 5)")
            && !raw_code.contains("rr_assign_slice(nf, size, size, 5)")
            && !raw_code.contains("rr_assign_slice(ny, .arg_size, .arg_size, rep.int(1, 1))")
            && !raw_code.contains("rr_assign_slice(ny, size, size, rep.int(1, 1))")
            && !raw_code.contains("rr_assign_slice(ny, .arg_size, .arg_size, 1)")
            && !raw_code.contains("rr_assign_slice(ny, size, size, 1)"),
        "raw tesseract output unexpectedly still contains singleton rr_assign_slice helper edits in neighbor_row_up"
    );
    assert!(
        raw_code.contains(
            "return(rr_idx_cube_vec_i(replace(nf, size, 1.0), nx, rr_assign_slice(ny, size, size, 1.0), size))"
        ) && raw_code.contains(
            "return(rr_idx_cube_vec_i(replace(nf, size, 5.0), replace(nx, size, 1.0), rr_assign_slice(ny, size, size, x), size))"
        ),
        "expected raw tesseract output to keep the neighbor_row_up edge remap in the current mixed replace/rr_assign_slice form"
    );
}

#[test]
pub(crate) fn tesseract_runs_at_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping tesseract runtime smoke: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = tesseract_test_dir("examples_tesseract_runtime");

    let out = out_dir.join("tesseract_o2.R");
    compile_rr(&rr_bin, &rr_src, &out, "-O2");
    let run = run_rscript(&rscript, &out);
    let stdout = normalize(&run.stdout);
    let stderr = normalize(&run.stderr);

    assert!(
        run.status == 0,
        "tesseract O2 runtime failed:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("Morphogenesis Complete. The pattern has formed.")
            && stdout.contains("TESSERACT: UNIFIED Model Complete."),
        "tesseract O2 runtime output was missing expected milestones:\nstdout={stdout}"
    );
    let particle_x = extract_numeric_series(&stdout, "Particle 1 Position (X):");
    assert_eq!(
        particle_x.len(),
        5,
        "expected five particle position samples in tesseract output:\nstdout={stdout}"
    );
    assert!(
        particle_x.iter().all(|value| value.is_finite()) && particle_x.first() != particle_x.last(),
        "expected particle x position to evolve across steps: {particle_x:?}\nstdout={stdout}"
    );
    assert!(
        !stdout.trim().is_empty(),
        "tesseract O2 runtime produced empty stdout"
    );
}

#[test]
pub(crate) fn tesseract_runtime_markers_match_between_o1_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping tesseract parity test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = tesseract_test_dir("examples_tesseract_parity");

    let o1_path = out_dir.join("tesseract_o1.R");
    let o2_path = out_dir.join("tesseract_o2.R");
    compile_rr(&rr_bin, &rr_src, &o1_path, "-O1");
    compile_rr(&rr_bin, &rr_src, &o2_path, "-O2");

    let o1 = run_rscript(&rscript, &o1_path);
    let o2 = run_rscript(&rscript, &o2_path);
    let stdout_o1 = normalize(&o1.stdout);
    let stdout_o2 = normalize(&o2.stdout);
    let stderr_o1 = normalize(&o1.stderr);
    let stderr_o2 = normalize(&o2.stderr);

    assert_eq!(
        o1.status, 0,
        "tesseract O1 runtime failed:\nstdout={stdout_o1}\nstderr={stderr_o1}"
    );
    assert_eq!(
        o2.status, 0,
        "tesseract O2 runtime failed:\nstdout={stdout_o2}\nstderr={stderr_o2}"
    );

    let center_b_o1 = extract_numeric_series(&stdout_o1, "Center B:");
    let center_b_o2 = extract_numeric_series(&stdout_o2, "Center B:");
    let wave_b_o1 = extract_numeric_series(&stdout_o1, "Wave B:");
    let wave_b_o2 = extract_numeric_series(&stdout_o2, "Wave B:");
    let particle_x_o1 = extract_numeric_series(&stdout_o1, "Particle 1 Position (X):");
    let particle_x_o2 = extract_numeric_series(&stdout_o2, "Particle 1 Position (X):");
    let max_u_o1 = extract_numeric_series(&stdout_o1, "Step Complete. Max U:");
    let max_u_o2 = extract_numeric_series(&stdout_o2, "Step Complete. Max U:");

    assert!(
        !center_b_o1.is_empty(),
        "missing Center B series in O1 stdout"
    );
    assert!(!wave_b_o1.is_empty(), "missing Wave B series in O1 stdout");
    assert!(
        !particle_x_o1.is_empty(),
        "missing particle x series in O1 stdout"
    );
    assert!(!max_u_o1.is_empty(), "missing max_u series in O1 stdout");

    assert_series_close("Center B", &center_b_o1, &center_b_o2);
    assert_series_close("Wave B", &wave_b_o1, &wave_b_o2);
    assert_series_close("Particle 1 Position (X)", &particle_x_o1, &particle_x_o2);
    assert_series_close("Max U", &max_u_o1, &max_u_o2);
}

#[test]
pub(crate) fn tesseract_preserve_all_defs_keeps_helper_definitions_sound() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = tesseract_test_dir("examples_tesseract_preserve_defs");

    let out = out_dir.join("tesseract_o2_preserve_defs.R");
    let status = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&out)
        .arg("-O2")
        .arg("--preserve-all-defs")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "tesseract preserve-all-defs compile failed"
    );

    let code = fs::read_to_string(&out).expect("failed to read compiled tesseract output");
    assert!(
        code.contains("Sym_60 <- function(f, x, size)")
            || code.contains("Sym_60 <- function(f, x, ys, size)")
            || code.contains("neighbor_row_left <- function(f, x, ys, size)")
    );
    assert!(
        code.contains("Sym_64 <- function(f, x, size)")
            || code.contains("Sym_64 <- function(f, x, ys, size)")
            || code.contains("neighbor_row_right <- function(f, x, ys, size)")
    );
    assert!(
        code.contains("Sym_66 <- function(f, x, size)")
            || code.contains("Sym_66 <- function(f, x, ys, size)")
            || code.contains("neighbor_row_down <- function(f, x, ys, size)")
    );
    assert!(
        code.contains("Sym_72 <- function(f, x, size)")
            || code.contains("Sym_72 <- function(f, x, ys, size)")
            || code.contains("neighbor_row_up <- function(f, x, ys, size)")
    );
    assert!(
        code.contains("return(rr_idx_cube_vec_i(face4_row, rev(ys), one_row, size))")
            || code.contains(
                "return(rr_idx_cube_vec_i(rep.int(4, width), (size - (ys - 1)), rep.int(1, width), size))"
            )
            || code.contains(
                "return(rr_idx_cube_vec_i(rep.int(4.0, width), (size - (ys - 1.0)), rep.int(1.0, width), size))"
            )
    );
    assert!(
        code.contains("return(rr_idx_cube_vec_i(replace(nf, 1, 1), nx, ny1_size, size))")
            || code.contains(
                "return(rr_idx_cube_vec_i(replace(nf, 1, 1), nx, replace(ny, 1, size), size))"
            )
            || code.contains(
                "return(rr_idx_cube_vec_i(replace(nf, 1.0, 1.0), nx, replace(ny, 1.0, size), size))"
            )
    );
    assert!(
        code.contains("return(rr_idx_cube_vec_i(replace(nf, 1, 3), nx1_rev, ny1_one, size))")
            || code.contains(
                "return(rr_idx_cube_vec_i(replace(nf, 1, 3), replace(nx, 1, (size - (x - 1))), replace(ny, 1, 1), size))"
            )
            || code.contains(
                "return(rr_idx_cube_vec_i(replace(nf, 1.0, 3.0), replace(nx, 1.0, (size - (x - 1.0))), replace(ny, 1.0, 1.0), size))"
            )
    );
    assert!(!code.contains("rr_assign_slice(nf, 1, 1, rep.int(1, 1))"));
    assert!(!code.contains("rr_assign_slice(nf, 1, 1, rep.int(3, 1))"));
    assert!(!code.contains("if ((f < f)) {"));
    assert!(!code.contains("if ((f > f)) {"));

    assert!(code.contains("xx <- ifelse(x < 1, w, x)"));
    assert!(code.contains("xx <- ifelse(xx > w, 1, xx)"));
    assert!(code.contains("yy <- ifelse(y < 1, h, y)"));
    assert!(code.contains("yy <- ifelse(yy > h, 1, yy)"));
    assert!(!code.contains("if ((w > w)) {"));
    assert!(!code.contains("if ((h > h)) {"));

    let sym_186 = extract_r_function(&code, "Sym_186")
        .or_else(|| extract_r_function(&code, "advect_particles"))
        .expect("expected preserve-all-defs output to keep particle advection helper");
    assert!(
        sym_186.starts_with("Sym_186 <- function(px, py, pf, u, v, dt, N")
            || sym_186.starts_with("advect_particles <- function(px, py, pf, u, v, dt, N")
    );
    assert!(sym_186.contains("px[i] <- x"));
    assert!(sym_186.contains("py[i] <- y"));
    assert!(sym_186.contains("pf[i] <- f"));
    assert!(!sym_186.contains("rr_index1_write(i, \"index\")"));

    let sym_287 = extract_r_function(&code, "Sym_287")
        .or_else(|| extract_r_function(&code, "ice_physics"))
        .expect("expected preserve-all-defs output to keep microphysics helper");
    assert!(
        sym_287.starts_with("Sym_287 <- function(temp, q_v, q_c, q_s, q_g, size)")
            || sym_287.starts_with("Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)")
            || sym_287.starts_with("ice_physics <- function(temp, q_v, q_c, q_s, q_g, size)")
    );
    assert!(sym_287.contains("<- (tendency_T / 1004)") || sym_287.contains("<- (tendency_T / cp)"));
    assert!(sym_287.contains("(0.005 * 2800000)") || sym_287.contains("L_s <- 2800000"));
    assert!(!sym_287.contains("cp <- 1004") || sym_287.contains("<- (tendency_T / cp)"));

    assert!(code.contains("Sym_49__typed_impl <- function(a, b)"));
    assert!(
        code.contains(
            "return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))"
        ) || code.contains(
            "return(rr_parallel_typed_vec_call(\"typed.vec.fused\", Sym_49__typed_impl, c(1L, 2L), a, b))"
        )
    );
}

#[test]
pub(crate) fn tesseract_o2_preserves_cg_recurrence_and_rk_buffer_swap() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = tesseract_test_dir("examples_tesseract_semantics");

    let out = out_dir.join("tesseract_o2_no_runtime.R");
    let status = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&out)
        .arg("-O2")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .status()
        .expect("failed to run RR compiler");
    assert!(status.success(), "tesseract O2 no-runtime compile failed");

    let code = fs::read_to_string(&out).expect("failed to read compiled tesseract output");

    let cg_fn = code
        .find("Sym_123 <- function")
        .or_else(|| code.find("solve_cg <- function"))
        .unwrap_or_else(|| panic!("expected CG solver function in tesseract output"));
    let cg_loop = code[cg_fn..]
        .find("for (iter in seq_len(20)) {")
        .or_else(|| code[cg_fn..].find("repeat {"))
        .map(|idx| cg_fn + idx)
        .unwrap_or_else(|| panic!("expected CG loop in solver function"));
    let cg_ap = code[cg_loop..]
        .find("Ap <- ")
        .map(|idx| cg_loop + idx)
        .unwrap_or_else(|| panic!("expected Ap recompute inside CG loop"));
    assert!(cg_ap > cg_loop, "expected Ap to stay inside the CG loop");
    assert!(
        code[cg_loop..].contains("rs_new <- rs_old"),
        "expected rs_new fallback to restore rs_old inside CG loop"
    );
    assert!(
        code[cg_loop..].contains("p <- (r + (beta * p))"),
        "expected CG search direction update to remain in solver function"
    );
    assert!(
        code[cg_loop..].contains("rs_old <- rs_new"),
        "expected CG residual carry update to remain in solver function"
    );

    let rk_fn = code
        .find("Sym_303 <- function")
        .or_else(|| code.find("tesseract_main <- function"))
        .unwrap_or_else(|| panic!("expected RK driver function in tesseract output"));
    assert!(
        code[rk_fn..].contains("tmp_u <- u")
            && code[rk_fn..].contains("u <- u_new")
            && code[rk_fn..].contains("u_new <- tmp_u"),
        "expected RK loop to preserve the u/u_new buffer swap"
    );
}

#[test]
pub(crate) fn tesseract_o1_proof_does_not_rebroadcast_vector_values() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = tesseract_test_dir("examples_tesseract_proof_o1");

    let out = out_dir.join("tesseract_o1_proof_no_runtime.R");
    compile_rr_env_with_args(
        &rr_bin,
        &rr_src,
        &out,
        "-O1",
        &["--no-runtime", "--no-incremental"],
        &[("RR_VOPT_PROOF", "1")],
    );

    let code = fs::read_to_string(&out).expect("failed to read compiled tesseract proof output");
    let cg = extract_r_function(&code, "Sym_123")
        .or_else(|| extract_r_function(&code, "solve_cg"))
        .unwrap_or_else(|| panic!("expected CG solver in proof-enabled tesseract output"));
    assert!(
        !cg.contains("r <- rep.int(rr_index1_read_vec("),
        "proof-enabled CG solver must not rebroadcast an already-vector residual seed:\n{cg}"
    );
    assert!(
        !cg.contains("Ap <- (rep.int("),
        "proof-enabled CG solver must not rebroadcast an already-vector Ap expression:\n{cg}"
    );
    assert!(
        !cg.contains("x <- rep.int(.tachyon_exprmap"),
        "proof-enabled CG solver must not rebroadcast vector x updates:\n{cg}"
    );
    assert!(
        !cg.contains("r <- rep.int(.tachyon_exprmap"),
        "proof-enabled CG solver must not rebroadcast vector r updates:\n{cg}"
    );
    assert!(
        cg.contains("Ap <- ((4.0001 * p) -")
            && cg.contains("x <- (x + (alpha * p))")
            && cg.contains("r <- (r - (alpha * Ap))")
            && cg.contains("p <- (r + (beta * p))"),
        "proof-enabled CG solver must keep the CG vector recurrence intact:\n{cg}"
    );

    let sym_222 = extract_r_function(&code, "Sym_222")
        .or_else(|| extract_r_function(&code, "morphogenesis"))
        .unwrap_or_else(|| {
            panic!("expected morphogenesis function in proof-enabled tesseract output")
        });
    assert!(
        !sym_222.contains("lapA <- (rep.int(") && !sym_222.contains("lapB <- (rep.int("),
        "proof-enabled morphogenesis must not rebroadcast whole-grid laplacians:\n{sym_222}"
    );

    let rk = extract_r_function(&code, "Sym_303")
        .or_else(|| extract_r_function(&code, "tesseract_main"))
        .unwrap_or_else(|| panic!("expected RK driver in proof-enabled tesseract output"));
    assert!(
        !rk.contains("adj_ll <- rep.int(rr_gather(")
            && !rk.contains("adj_rr <- rep.int(rr_gather("),
        "proof-enabled RK driver must not rebroadcast topology gather vectors:\n{rk}"
    );
    assert!(
        !rk.contains("visc <- (rep.int(") && !rk.contains("adv_u <- (ifelse(rep.int("),
        "proof-enabled RK driver must not rebroadcast full-grid stencil vectors:\n{rk}"
    );
}
