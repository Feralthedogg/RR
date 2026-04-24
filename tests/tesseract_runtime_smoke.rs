mod common;

use common::{
    compile_rr, compile_rr_env, compile_rr_env_with_args, normalize, rscript_available,
    rscript_path, run_rscript, unique_dir,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn extract_numeric_series(stdout: &str, marker: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut lines = stdout.lines();
    while let Some(line) = lines.next() {
        if line.contains(marker)
            && let Some(next) = lines.next()
        {
            let value_text = next.trim().trim_start_matches("[1]").trim();
            if let Ok(value) = value_text.parse::<f64>() {
                out.push(value);
            }
        }
    }
    out
}

fn assert_series_close(label: &str, a: &[f64], b: &[f64]) {
    assert_eq!(
        a.len(),
        b.len(),
        "{label} series length mismatch:\nleft={a:?}\nright={b:?}"
    );
    for (idx, (lhs, rhs)) in a.iter().zip(b.iter()).enumerate() {
        let diff = (lhs - rhs).abs();
        assert!(
            diff <= 1e-12,
            "{label} mismatch at index {idx}: left={lhs}, right={rhs}, diff={diff}"
        );
    }
}

fn contains_line(haystack: &str, needle: &str) -> bool {
    haystack.lines().any(|line| line.trim() == needle)
}

fn tesseract_test_dir(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("tesseract_runtime_smoke");
    fs::create_dir_all(&sandbox_root)
        .expect("failed to create tesseract runtime smoke sandbox root");
    let dir = unique_dir(&sandbox_root, name);
    fs::create_dir_all(&dir).expect("failed to create tesseract runtime smoke sandbox dir");
    dir
}

fn extract_r_function(code: &str, name: &str) -> Option<String> {
    let header = format!("{name} <- function(");
    let mut started = false;
    let mut brace_depth = 0usize;
    let mut saw_open = false;
    let mut out = Vec::new();

    for line in code.lines() {
        if !started {
            if !line.contains(&header) {
                continue;
            }
            started = true;
        }

        out.push(line);
        for ch in line.chars() {
            match ch {
                '{' => {
                    brace_depth += 1;
                    saw_open = true;
                }
                '}' if brace_depth > 0 => brace_depth -= 1,
                _ => {}
            }
        }

        if saw_open && brace_depth == 0 {
            return Some(out.join("\n"));
        }
    }

    None
}

#[test]
fn tesseract_compiles_across_opt_levels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_src = root.join("example").join("tesseract.rr");
    let out_dir = tesseract_test_dir("examples_tesseract");

    for (flag, tag) in [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")] {
        let out = out_dir.join(format!("tesseract_{tag}.R"));
        compile_rr(&rr_bin, &rr_src, &out, flag);
        let code = fs::read_to_string(&out).expect("failed to read compiled tesseract output");
        assert!(
            code.contains("Initializing Project MORPHOGENESIS: The Beauty of Chaos...")
                && code.contains("TESSERACT: UNIFIED Model Complete.")
                && code.contains("Sym_top_0 <- function()"),
            "expected compiled tesseract output to contain both top-level workflows for {}",
            flag
        );
        assert!(
            code.contains("particles <- Sym_")
                && (code.contains("return(rr_named_list(\"px\", px, \"py\", py, \"pf\", pf))")
                    || code.contains("return(list(px = px, py = py, pf = pf))"))
                && (((code.contains("p_x <- rr_field_get(particles, \"px\")")
                    || code.contains("p_x <- particles[[\"px\"]]"))
                    && (code.contains("p_y <- rr_field_get(particles, \"py\")")
                        || code.contains("p_y <- particles[[\"py\"]]"))
                    && (code.contains("p_f <- rr_field_get(particles, \"pf\")")
                        || code.contains("p_f <- particles[[\"pf\"]]")))
                    || code.contains("particles[[\"px\"]]")
                    || code.contains("rr_field_get(particles, \"px\")")),
            "expected compiled tesseract output to thread particle state back for {}",
            flag
        );
        let enable_experimental_shape_checks =
            std::env::var_os("RR_ENABLE_EXPERIMENTAL_ASSERTS").is_some();
        if enable_experimental_shape_checks && flag != "-O0" {
            if code.contains("Sym_17 <- function(n, val, depth)") {
                assert!(
                    code.contains("return(rep.int(val, n))") && !code.contains(".arg_n <- n"),
                    "expected trivial alloc wrapper to avoid dead .arg aliases for {}",
                    flag
                );
            } else {
                assert!(
                    code.contains("h <- rep.int(8000, TOTAL)")
                        || code.contains("h <- (rep.int(8000, TOTAL))")
                        || code.contains("temp <- rep.int(300, TOTAL)")
                        || code.contains("temp <- (rep.int(300, TOTAL))")
                        || code.contains("coriolis <- rep.int(0, TOTAL)")
                        || code.contains("coriolis <- (rep.int(0, TOTAL))"),
                    "expected trivial alloc wrapper to either stay simplified or inline away for {}",
                    flag
                );
            }
            let specialized_neighbor_helpers =
                code.contains("Sym_60 <- function(f, x, size)")
                    && code.contains("Sym_64 <- function(f, x, size)")
                    && code.contains("Sym_66 <- function(f, x, size)")
                    && code.contains("Sym_72 <- function(f, x, size)")
                    && !code.contains("width <- size")
                    && !code.contains("Sym_60 <- function(f, x, ys, size)")
                    && !code.contains("width <- length(ys)")
                    && code.contains("ys <- seq_len(size)")
                    && code.contains("Sym_60(f, x, size)")
                    && code.contains("Sym_64(f, x, size)")
                    && code.contains("rev(ys)")
                    && code.contains(
                        "return(rr_idx_cube_vec_i(rep.int((((f + 2) %% 4) + 1), size), size_row, ys, size))"
                    )
                    && code.contains("return(rr_idx_cube_vec_i(face4_row, rev(ys), one_row, size))")
                    && code.contains(
                        "return(rr_idx_cube_vec_i(rep.int(((f %% 4) + 1), size), one_row, ys, size))"
                    )
                    && !code.contains("(size - (ys - 1))")
                    && code.contains("nf6 <- replace(nf, 1, 6)")
                    && code.contains("nf5 <- replace(nf, size, 5)")
                    && code.contains("ny1_size <- replace(ny, 1, size)")
                    && code.contains("ny1_one <- replace(ny, 1, 1)")
                    && code.contains("nx1_rev <- replace(nx, 1, rev_x)")
                    && code.contains("nysize_one <- replace(ny, size, 1)")
                    && code.contains("nysize_size <- replace(ny, size, size)")
                    && code.contains("nxsize_rev <- replace(nx, size, rev_x)");
            let generic_neighbor_helpers = code.contains("Sym_60 <- function(f, x, ys, size)")
                && code.contains("Sym_64 <- function(f, x, ys, size)")
                && code.contains("Sym_66 <- function(f, x, ys, size)")
                && code.contains("Sym_72 <- function(f, x, ys, size)")
                && code.contains("width <- length(ys)")
                && code.contains("Sym_60(f, x, ys, size)")
                && code.contains("Sym_64(f, x, ys, size)")
                && code.contains("Sym_66(f, x, ys, size)")
                && code.contains("Sym_72(f, x, ys, size)")
                && !code.contains(".arg_f <- f");
            let helper_branch_shape = code.contains("} else if ((f <= 4)) {")
                || (code.contains("} else if ((f == 1)) {")
                    && code.contains("} else if ((f == 2)) {"));
            assert!(
                (specialized_neighbor_helpers || generic_neighbor_helpers) && helper_branch_shape,
                "expected tesseract neighbor-row helpers to specialize away the fixed ys arg and readonly helper aliases for {}",
                flag
            );
            if code.contains("Sym_37 <- function(f, x, y, size)") {
                assert!(
                    code.contains(
                        "Sym_37 <- function(f, x, y, size) \n{\n  u <- (((x / size) + (x / size)) - 1)\n  v <- (((y / size) + (y / size)) - 1)"
                    )
                        && !code.contains("Sym_37 <- function(f, x, y, size) \n{\n  .arg_x <- x")
                        && !code.contains("Sym_37 <- function(f, x, y, size) \n{\n  .arg_y <- y")
                        && !code.contains("Sym_37 <- function(f, x, y, size) \n{\n  .arg_size <- size"),
                    "expected get_lat helper to avoid dead .arg aliases for {}",
                    flag
                );
            }
            if code.contains("rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl") {
                assert!(
                    code.contains("Sym_49__typed_impl <- function(a, b)"),
                    "expected typed parallel impl helper to remain reachable for {}",
                    flag
                );
                assert!(
                    !code.contains("Sym_49 <- function(a, b)"),
                    "expected unreachable typed parallel wrapper Sym_49 to be pruned once only the string callee name remains for {}",
                    flag
                );
                assert!(
                    (code.contains("probe_energy <- mean(abs(probe_vec))")
                        || code.contains("probe_energy <- (mean(abs(probe_vec)))"))
                        && !code.contains(
                            "probe_energy <- mean(abs(rr_parallel_typed_vec_call(\"Sym_49\""
                        )
                        && !code.contains(
                            "probe_energy <- (mean(abs((rr_parallel_typed_vec_call(\"Sym_49\""
                        ),
                    "expected probe_energy to reuse the earlier probe_vec typed call for {}",
                    flag
                );
            }
            let has_particle_loads = (code.contains("x <- .arg_px[i]")
                || code.contains("x <- px[i]"))
                && (code.contains("y <- .arg_py[i]") || code.contains("y <- py[i]"))
                && (code.contains("f <- .arg_pf[i]") || code.contains("f <- pf[i]"));
            let has_u_advection = code.contains("u_val <- rr_index1_read(.arg_u, idx, \"index\")")
                || code.contains("u_val <- rr_index1_read(u, idx, \"index\")")
                || code.contains("dx <- ((rr_index1_read(u, idx, \"index\") * dt) / 400000)")
                || code.contains(
                    "dx <- ((rr_index1_read(.arg_u, idx, \"index\") * .arg_dt) / 400000)",
                )
                || code.contains("dx <- ((u[rr_idx_cube_vec_i(f, ix, iy, N)] * dt) / 400000)")
                || code.contains(
                    "x <- (x + ((u[rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N)] * dt) / 400000))",
                )
                || code.contains("x <- (x + ((rr_index1_read(u, idx, \"index\") * dt) / 400000))")
                || code.contains(
                    "x <- (x + ((rr_index1_read(.arg_u, idx, \"index\") * .arg_dt) / 400000))",
                )
                || code.contains("x <- (x + dx)");
            let has_v_advection = code.contains("v_val <- rr_index1_read(.arg_v, idx, \"index\")")
                || code.contains("v_val <- rr_index1_read(v, idx, \"index\")")
                || code.contains("dy <- ((rr_index1_read(v, idx, \"index\") * dt) / 400000)")
                || code.contains(
                    "dy <- ((rr_index1_read(.arg_v, idx, \"index\") * .arg_dt) / 400000)",
                )
                || code.contains("dy <- ((v[rr_idx_cube_vec_i(f, ix, iy, N)] * dt) / 400000)")
                || code.contains(
                    "y <- (y + ((v[rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N)] * dt) / 400000))",
                )
                || code.contains("y <- (y + ((rr_index1_read(v, idx, \"index\") * dt) / 400000))")
                || code.contains(
                    "y <- (y + ((rr_index1_read(.arg_v, idx, \"index\") * .arg_dt) / 400000))",
                )
                || code.contains("y <- (y + dy)");
            assert!(
                has_particle_loads && has_u_advection && has_v_advection,
                "expected compiled tesseract output to preserve particle advection state threading for {}",
                flag
            );
            assert!(
                ((code.contains("} else if ((x < 0)) {")
                    && code.contains("} else if ((y < 0)) {")
                    && code.contains("} else if ((f < 1)) {"))
                    || (code.contains("if ((x > 1)) {")
                        && code.contains("if ((x < 0)) {")
                        && code.contains("if ((y > 1)) {")
                        && code.contains("if ((y < 0)) {")
                        && code.contains("if ((f > 6)) {")
                        && code.contains("if ((f < 1)) {"))),
                "expected particle wrap/update guards to remain structurally sound for {}",
                flag
            );
            assert!(
                (extract_r_function(&code, "Sym_186")
                    .as_deref()
                    .unwrap_or("")
                    .contains("for (i in seq_len(1000)) {")
                    || extract_r_function(&code, "Sym_186")
                        .as_deref()
                        .unwrap_or("")
                        .contains("for (i in seq_len((1000))) {"))
                    && !extract_r_function(&code, "Sym_186")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(i <= (1000))) break")
                    && !extract_r_function(&code, "Sym_186")
                        .as_deref()
                        .unwrap_or("")
                        .contains("i <- (i + 1)"),
                "expected final Sym_186 particle loop to lower to a canonical for-loop for {}",
                flag
            );
            assert!(
                extract_r_function(&code, "Sym_222")
                    .as_deref()
                    .unwrap_or("")
                    .contains("for (y in seq_len(H)) {")
                    && extract_r_function(&code, "Sym_222")
                        .as_deref()
                        .unwrap_or("")
                        .contains("for (x in seq_len(W)) {")
                    && !extract_r_function(&code, "Sym_222")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(y <= H)) break")
                    && !extract_r_function(&code, "Sym_222")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(x <= W)) break"),
                "expected final Sym_222 morphogenesis seed loops to lower to canonical for-loops for {}",
                flag
            );
            assert!(
                {
                    let sym_222 = extract_r_function(&code, "Sym_222").unwrap_or_default();
                    (sym_222.contains("for (i in seq_len(SIZE)) {")
                        && !sym_222.contains("if (!(i <= SIZE)) break")
                        && !sym_222.contains("i <- (i + 1)"))
                        || (sym_222.contains("repeat {")
                            && sym_222.contains("if (!(i <= SIZE)) break")
                            && sym_222.contains("i <- (i + 1)"))
                },
                "expected final Sym_222 cell update loop to remain structurally sound for {}",
                flag
            );
            assert!(
                extract_r_function(&code, "Sym_287")
                    .as_deref()
                    .unwrap_or("")
                    .contains("for (i in seq_len(size)) {")
                    && !extract_r_function(&code, "Sym_287")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(i <= size)) break")
                    && !extract_r_function(&code, "Sym_287")
                        .as_deref()
                        .unwrap_or("")
                        .contains("i <- (i + 1)"),
                "expected final Sym_287 thermodynamics loop to lower to a canonical for-loop for {}",
                flag
            );
            assert!(
                !code.contains(
                    "y <- (y + ((rr_index1_read(.arg_v, rr_idx_cube_vec_i(f, floor(gx), floor(gy), .arg_N), \"index\") * .arg_dt) / 400000))"
                ),
                "unexpected duplicate particle y update reappeared in {}",
                flag
            );
        }
        assert!(
            !code.contains("p_check <- Sym_89("),
            "stale particle state placeholder should not remain in compiled output for {}",
            flag
        );
        assert!(
            !code.contains("return(seq_len(.arg_n))"),
            "expected alloc_particles lowering to return the mutated particle buffer for {}",
            flag
        );
        if enable_experimental_shape_checks && flag != "-O0" {
            assert!(
                !code.contains("lat <- ((.__rr_cse_13 - 1) * 45)"),
                "expected direct get_lat lowering to avoid branch-local temp leakage for {}",
                flag
            );
            if code.contains("Sym_37 <- function(f, x, y, size)") {
                assert!(
                    code.contains("if ((f == 6)) {\n    lat <- ((-(45)) - ((1 - (((u * u) + (v * v)) * 0.25)) * 45))")
                        && !code.contains(".__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)")
                        && !code.contains(".__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)"),
                    "expected get_lat polar branch to reuse u/v directly rather than temp chains for {}",
                    flag
                );
            }
            assert!(
                !code.contains("grid_sq <- (40 * N)"),
                "expected grid_sq to remain N*N-derived rather than partially constant-folded for {}",
                flag
            );
            assert!(
                code.contains("x_curr <- (floor((rem / N)) + 1)")
                    && code.contains("y_curr <- ((rem %% N) + 1)")
                    && !code.contains("x_curr <- (floor((rem / 40)) + 1)")
                    && !code.contains("y_curr <- ((rem %% 40) + 1)"),
                "expected tesseract grid index recovery to stay N-derived rather than baking in 40 for {}",
                flag
            );
            assert!(
                code.contains("inlined_39_u <- (((x_curr / N) + (x_curr / N)) - 1)")
                    && code.contains("inlined_39_v <- (((y_curr / N) + (y_curr / N)) - 1)")
                    && (code.contains("f_curr <- (floor((k0 / grid_sq)) + 1)")
                        || code.contains("if (((floor((k0 / grid_sq)) + 1) == 1)) {"))
                    && (code.contains("if ((f_curr == 6)) {")
                        || code.contains("if (((floor((k0 / grid_sq)) + 1) == 6)) {"))
                    && code.contains(
                        "inlined_39_lat <- ((-(45)) - ((1 - (((inlined_39_u * inlined_39_u) + (inlined_39_v * inlined_39_v)) * 0.25)) * 45))"
                    )
                    && !code.contains(".__rr_cse_642 <- (x_curr / N)")
                    && !code.contains(".__rr_cse_648 <- (y_curr / N)")
                    && !code.contains(".__rr_cse_644 <- (.__rr_cse_642 + .__rr_cse_642)")
                    && !code.contains(".__rr_cse_650 <- (.__rr_cse_648 + .__rr_cse_648)"),
                "expected inlined get_lat lowering in tesseract_main to reuse inlined_39_u/v directly for {}",
                flag
            );
            assert!(
                (!code.contains("Sym_244 <- function") && code.contains("adv_u <- (ifelse((u > 0),"))
                    || (!code.contains(".__rr_cse_22 <- (3 * v_c)")
                        && code.contains(
                            "b1 <- (((1.0833 * ((v_m2 - (2 * v_m1)) + v_c)) * ((v_m2 - (2 * v_m1)) + v_c))"
                        )
                        && code.contains(
                            "b3 <- (((1.0833 * ((v_c - (2 * v_p1)) + v_p2)) * ((v_c - (2 * v_p1)) + v_p2)) + ((0.25 * (((3 * v_c) - (4 * v_p1)) + v_p2)) * (((3 * v_c) - (4 * v_p1)) + v_p2)))"
                        )),
                "expected Sym_244 stencil math to stay flattened rather than regressing back to temp-heavy helper form for {}",
                flag
            );
            assert!(
                !contains_line(&code, "ii <- i"),
                "expected immediate loop index alias ii <- i to be eliminated for {}",
                flag
            );
            assert_eq!(
                code.matches("coriolis <- Sym_17(TOTAL, 0, 3)").count()
                    + code.matches("coriolis <- (rep.int(0, TOTAL))").count()
                    + code.matches("coriolis <- rep.int(0, TOTAL)").count()
                    + code.matches("coriolis <- qr").count()
                    + code.matches("coriolis <- h_trn").count(),
                1,
                "expected coriolis to be allocated once and not reset after initialization for {}",
                flag
            );
            assert!(
                code.contains("qg <- qr") && !code.contains("qg <- qs"),
                "expected qg to flatten directly to qr instead of a transitive alias for {}",
                flag
            );
            assert!(
                code.matches("adj_ll <- Sym_17(TOTAL, 0, 2)").count()
                    + code.matches("adj_ll <- (rep.int(0, TOTAL))").count()
                    + code.matches("adj_ll <- rep.int(0, TOTAL)").count()
                    <= 1,
                "expected adj_ll to avoid duplicate zero-allocation replays for {}",
                flag
            );
            assert!(
                code.matches("adj_rr <- Sym_17(TOTAL, 0, 2)").count()
                    + code.matches("adj_rr <- (rep.int(0, TOTAL))").count()
                    + code.matches("adj_rr <- rep.int(0, TOTAL)").count()
                    <= 1,
                "expected adj_rr to avoid duplicate zero-allocation replays for {}",
                flag
            );
            assert!(
                code.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))")
                    && code.contains("adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))")
                    && !code.contains(
                        "adj_ll <- rr_assign_slice(adj_ll, i, TOTAL, rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL)))))"
                    )
                    && !code.contains(
                        "adj_rr <- rr_assign_slice(adj_rr, i, TOTAL, rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:TOTAL)))))"
                    ),
                "expected WENO second-neighbor tables to collapse to direct gather topology for {}",
                flag
            );
            assert!(
                (code.contains("rs_old <- (sum((r[seq_len(size)] * r[seq_len(size)])))")
                    || code.contains("rs_old <- sum((r[seq_len(size)] * r[seq_len(size)]))")
                    || code.contains("rs_old <- (sum((b[seq_len(size)] * b[seq_len(size)])))")
                    || code.contains("rs_old <- sum((b[seq_len(size)] * b[seq_len(size)]))"))
                    && (code.contains("p_Ap <- (sum((p[seq_len(size)] * Ap[seq_len(size)])))")
                        || code.contains("p_Ap <- sum((p[seq_len(size)] * Ap[seq_len(size)]))"))
                    && !code.contains("Sym_117 <- function")
                    && !code
                        .contains("rs_old <- Sym_117(rep.int(0, size), rep.int(0, size), size)"),
                "expected solve_cg dot_product helpers to inline to direct sum expressions for {}",
                flag
            );
            assert!(
                !code.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"),
                "expected solve_cg tail writeback to be eliminated for {}",
                flag
            );
            assert!(
                !code.contains("x <- (rep.int(0, size))"),
                "expected solve_cg invalid-rs_old guard to avoid re-emitting the same zero-vector x init for {}",
                flag
            );
            assert!(
                !code.contains("qc <- q_c[i]")
                    && !code.contains("qv <- q_v[i]")
                    && !code.contains("qs <- q_s[i]")
                    && !code.contains("qg <- q_g[i]"),
                "expected Sym_287 to inline direct q_*[i] scalar reads across the warm-cloud, ice, and melt branches for {}",
                flag
            );
            assert!(
                code.contains("if ((q_c[i] > 0.0001)) {")
                    && code.contains("if ((q_v[i] > 0.01)) {")
                    && code.contains("if ((q_s[i] > 0)) {")
                    && code.contains("if ((q_g[i] > 0)) {")
                    && !code.contains("rate <- (0.01 * q_c[i])")
                    && !code.contains("melt_rate <- 0")
                    && !code.contains("melt_rate <- (q_s[i] * 0.05)")
                    && !code.contains("melt_rate <- (melt_rate + (q_g[i] * 0.02))")
                    && code.contains("tendency_T <- (tendency_T - ((q_s[i] * 0.05) * L_f))")
                    && code.contains("tendency_T <- (tendency_T - ((q_g[i] * 0.02) * L_f))"),
                "expected Sym_287 to use direct q_*[i] scalar reads and direct melt heat-sink updates inside its thermal phase guards for {}",
                flag
            );
            assert!(
                code.contains("Sym_287 <- function(temp, q_v, q_c, q_s, q_g, size)")
                    && !code
                        .contains("Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)")
                    && code.contains("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
                    && !code.contains("heat <- Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL)"),
                "expected Sym_287 to drop unused middle helper params and callsite args for {}",
                flag
            );
            assert!(
                {
                    let sym_222 = extract_r_function(&code, "Sym_222").unwrap_or_default();
                    let outer_for = sym_222.contains("for (t in seq_len(20)) {")
                        && !sym_222.contains("if (!(t < (20))) break")
                        && !sym_222.contains("t <- (t + 1)")
                        && !contains_line(&sym_222, "t <- 0");
                    let outer_repeat = sym_222.contains("repeat {")
                        && sym_222.contains("if (!(t < (20))) break")
                        && sym_222.contains("t <- (t + 1)");
                    let inner_for = sym_222.contains("for (i in seq_len(SIZE)) {");
                    let inner_repeat = sym_222.contains("if (!(i <= SIZE)) break")
                        && sym_222.contains("i <- (i + 1)");
                    (outer_for || outer_repeat)
                        && (inner_for || inner_repeat)
                        && sym_222.contains("lapA <-")
                        && sym_222.contains("lapB <-")
                },
                "expected morphogenesis timestep loop to remain structurally sound for {}",
                flag
            );
            assert!(
                !code.contains("if ((u_new[i] > u_new[i])) {"),
                "expected max_u tracking guard to compare against max_u rather than self-compare for {}",
                flag
            );
            assert!(
                code.contains("particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N)")
                    && code.contains("p_f <- particles[[\"pf\"]]")
                    && (code.contains("for (i in seq_len(TOTAL)) {")
                        || (code.contains("repeat {")
                            && code.contains("if (!(i <= TOTAL)) break")))
                    && code.matches("p_x <- particles[[\"px\"]]").count() == 1
                    && code.matches("p_y <- particles[[\"py\"]]").count() == 1
                    && code.matches("p_f <- particles[[\"pf\"]]").count() == 1,
                "expected tesseract_main to keep a structurally sound first RK3 scalar loop for {}",
                flag
            );
            assert!(
                code.matches("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
                    .count()
                    == 1
                    && (code.contains("for (steps in seq_len(5)) {")
                        || (code.contains("repeat {")
                            && (code.contains("if (!(.__rr_i < (5))) break")
                                || code.contains("if (!(steps < (5))) break"))))
                    && !code.contains("rr_index_vec_floor(rr_index_vec_floor(")
                    && (!code.contains("if (!(steps < (5))) break") || code.contains("repeat {"))
                    && (!code.contains("steps <- (steps + 1)") || code.contains("repeat {"))
                    && (!code.contains("\nsteps <- 0\n") || code.contains("repeat {")),
                "expected tesseract_main to keep a structurally sound outer RK timestep loop for {}",
                flag
            );
            assert!(
                (code.contains("f_curr <- (floor((k0 / grid_sq)) + 1)")
                    && ((code.contains("} else if ((f_curr == 6)) {")
                        && code.contains("} else if ((f_curr < 5)) {"))
                        || (code.contains("if ((f_curr == 6)) {")
                            && code.contains("if ((f_curr < 5)) {"))))
                    || (code.contains("floor((k0 / grid_sq)) + 1")
                        && code.contains("if (((floor((k0 / grid_sq)) + 1) == 5)) {")
                        && code.contains("if (((floor((k0 / grid_sq)) + 1) == 6)) {")
                        && code.contains("if (((floor((k0 / grid_sq)) + 1) < 5)) {")),
                "expected tesseract_main environment-init latitude branches to remain structurally sound for {}",
                flag
            );
            assert!(
                code.contains("for (k in seq_len(TOTAL)) {")
                    && !code.contains("if (!(k <= TOTAL)) break")
                    && !code.contains("k <- (k + 1)")
                    && !code.contains("\nk <- 1\n"),
                "expected tesseract_main to lower the environment-init sweep to a canonical for-loop for {}",
                flag
            );
            assert!(
                (code.contains("max_u <- (-(1000))") || code.contains("max_u <- -1000L"))
                    && (code.contains("for (i in seq_len(TOTAL)) {\n      rr_mark(1586, 13);")
                        || code
                            .contains("for (i in seq_len(TOTAL)) {\n      rr_mark(1586L, 13L);")
                        || (code.contains("repeat {")
                            && code.contains("if (!(i <= TOTAL)) break"))
                        || (code.contains("repeat {")
                            && code.contains("if (!(.__rr_i < (5))) break")))
                    && code.contains("max_u <- u_new[i]")
                    && code.contains("tmp_u <- u")
                    && code.contains("u <- u_new")
                    && code.contains("u_new <- tmp_u")
                    && code.contains("print(max_u)")
                    && !code.contains("print(u_new[i])"),
                "expected tesseract_main to keep a structurally sound final RK/max_u sweep while preserving the u/u_new swap for {}",
                flag
            );
            assert!(
                code.contains("p_x <- Sym_183(1000)")
                    && code.contains("p_y <- p_x")
                    && !code.contains("p_y <- Sym_183(1000)"),
                "expected deterministic drifter seeds to reuse the first Sym_183 result for {}",
                flag
            );
            assert!(
                code.contains("u_stage <- qr")
                    && code.contains("u_new <- u_stage")
                    && !code.contains("u_new <- qr"),
                "expected tesseract RK stage buffers to reuse the first qr alias for {}",
                flag
            );
            assert!(
                !code.contains("p_x <- rr_field_get(particles, \"px\")")
                    && code.matches("p_x <- particles[[\"px\"]]").count() >= 1,
                "expected p_x particle field extraction to stay on direct base indexing for {}",
                flag
            );
            assert!(
                !code.contains("p_y <- rr_field_get(particles, \"py\")")
                    && code.matches("p_y <- particles[[\"py\"]]").count() >= 1,
                "expected p_y particle field extraction to stay on direct base indexing for {}",
                flag
            );
            assert!(
                !code.contains("p_f <- rr_field_get(particles, \"pf\")")
                    && code.matches("p_f <- particles[[\"pf\"]]").count() >= 1,
                "expected p_f particle field extraction to stay on direct base indexing for {}",
                flag
            );
            assert!(
                code.contains("px[i] <- x")
                    && code.contains("py[i] <- y")
                    && code.contains("pf[i] <- f")
                    && !code.contains("px[rr_index1_write(i, \"index\")] <- x")
                    && !code.contains("py[rr_index1_write(i, \"index\")] <- y")
                    && !code.contains("pf[rr_index1_write(i, \"index\")] <- f"),
                "expected particle writeback loop to use direct base indexing once i is proven safe for {}",
                flag
            );
            assert!(
                extract_r_function(&code, "Sym_83")
                    .as_deref()
                    .unwrap_or("")
                    .contains("for (f in seq_len(6)) {")
                    && extract_r_function(&code, "Sym_83")
                        .as_deref()
                        .unwrap_or("")
                        .contains("for (x in seq_len(size)) {")
                    && !extract_r_function(&code, "Sym_83")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(f <= 6)) break")
                    && !extract_r_function(&code, "Sym_83")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(x <= size)) break"),
                "expected final Sym_83 topology setup loops to lower to canonical for-loops for {}",
                flag
            );
            assert!(
                extract_r_function(&code, "Sym_123")
                    .as_deref()
                    .unwrap_or("")
                    .contains("for (iter in seq_len(20)) {")
                    && !extract_r_function(&code, "Sym_123")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(iter <= 20)) break")
                    && !extract_r_function(&code, "Sym_123")
                        .as_deref()
                        .unwrap_or("")
                        .contains("iter <- (iter + 1)"),
                "expected final Sym_123 CG loop to lower to a canonical for-loop for {}",
                flag
            );
            assert!(
                extract_r_function(&code, "Sym_183")
                    .as_deref()
                    .unwrap_or("")
                    .contains("for (i in seq_len(n)) {")
                    && !extract_r_function(&code, "Sym_183")
                        .as_deref()
                        .unwrap_or("")
                        .contains("if (!(i <= n)) break")
                    && !extract_r_function(&code, "Sym_183")
                        .as_deref()
                        .unwrap_or("")
                        .contains("i <- (i + 1)"),
                "expected final Sym_183 RNG loop to lower to a canonical for-loop for {}",
                flag
            );
            assert!(
                ((code.contains("dx <- ((u[idx] * dt) / 400000)")
                    && code.contains("dy <- ((v[idx] * dt) / 400000)"))
                    || (code
                        .contains("dx <- ((u[rr_idx_cube_vec_i(f, ix, iy, N)] * dt) / 400000)")
                        && code.contains(
                            "dy <- ((v[rr_idx_cube_vec_i(f, ix, iy, N)] * dt) / 400000)"
                        ))
                    || (code.contains(
                        "dx <- ((u[rr_idx_cube_vec_i(f, ix, floor(gy), N)] * dt) / 400000)"
                    ) && code.contains(
                        "dy <- ((v[rr_idx_cube_vec_i(f, ix, floor(gy), N)] * dt) / 400000)"
                    ))
                    || (code.contains(
                        "dx <- ((u[rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N)] * dt) / 400000)"
                    ) && code.contains(
                        "dy <- ((v[rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N)] * dt) / 400000)"
                    ))
                    || (code.contains(
                        "x <- (x + ((u[rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N)] * dt) / 400000))"
                    ) && code.contains(
                        "y <- (y + ((v[rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N)] * dt) / 400000))"
                    )))
                    && !code.contains("rr_index1_read(u, idx, \"index\")")
                    && !code.contains("rr_index1_read(v, idx, \"index\")"),
                "expected particle advection reads to use direct base indexing once idx is proven safe for {}",
                flag
            );
            assert!(
                !code.contains("qr <- q_r[i]")
                    && !code.contains("qi <- q_i[i]")
                    && !code.contains("es_ice <- ")
                    && !code.contains("L_v <- 2500000"),
                "expected dead scalar locals inside Sym_287 to stay pruned for {}",
                flag
            );
            assert!(
                !code.contains("T <- temp[i]"),
                "expected single-use named scalar reads like T <- temp[i] inside Sym_287 to inline away for {}",
                flag
            );
            assert!(
                !code.contains("id <- rr_wrap_index_vec_i(x, y, W, H)")
                    && !code.contains("center_idx <- rr_wrap_index_vec_i(32, 32, W, H)")
                    && !code.contains("side_idx <- rr_wrap_index_vec_i(40, 32, W, H)"),
                "expected single-use named pure calls like rr_wrap_index_vec_i(...) to inline away for {}",
                flag
            );
            assert!(
                code.contains("B[rr_wrap_index_vec_i(x, y, W, H)] <- 1")
                    && code.contains("print(B[rr_wrap_index_vec_i(32, 32, W, H)])")
                    && !code.contains(
                        "B[rr_index1_write(rr_wrap_index_vec_i(x, y, W, H), \"index\")] <- 1"
                    )
                    && !code.contains(
                        "print(rr_index1_read(B, rr_wrap_index_vec_i(32, 32, W, H), \"index\"))"
                    ),
                "expected wrap-index scalar access helpers to collapse to direct base indexing for {}",
                flag
            );
            assert!(
                !code.contains("heat2 <- Sym_287(") && !code.contains("heat3 <- Sym_287("),
                "expected dead repeated Sym_287 stage calls to stay pruned for {}",
                flag
            );
            assert!(
                !code.contains("Sym_156 <- function")
                    && !code.contains("Sym_171 <- function")
                    && !code.contains("visc <- Sym_156(")
                    && !code.contains("du1 <- Sym_171(")
                    && !code.contains("visc2 <- Sym_156(")
                    && !code.contains("du2 <- Sym_171(")
                    && !code.contains("visc3 <- Sym_156(")
                    && !code.contains("du3 <- Sym_171("),
                "expected straight-line Sym_156/Sym_171 helper calls to inline away in final tesseract output for {}",
                flag
            );
        }
    }
}

#[test]
fn tesseract_raw_o2_recovers_sym17_whole_range_replays_before_peephole() {
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
        "adj_ll <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL))))",
        "adj_rr <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:TOTAL))))",
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
    assert!(
        raw_code.contains("probe_vec <- Sym_49(c(1.0, 2.0, 3.0, 4.0), c(4.0, 3.0, 2.0, 1.0))")
            && raw_code.contains("probe_energy <- mean(abs(probe_vec))")
            && !raw_code.contains("Sym_51 <- function(a, b)")
            && !raw_code.contains("probe_energy <- Sym_51("),
        "expected raw tesseract output to reuse the earlier probe_vec call when forming probe_energy and drop the now-unreachable Sym_51 helper"
    );
    let raw_sym_183 = extract_r_function(&raw_code, "Sym_183")
        .unwrap_or_else(|| panic!("raw tesseract output should contain Sym_183"));
    assert!(
        raw_sym_183.contains("p <- seq_len(n)")
            && raw_sym_183.contains("seed <- 12345.0")
            && raw_sym_183.contains("repeat {")
            && raw_sym_183.contains("if (!(i <= n)) break")
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
            && raw_sym_287.contains("if (!(i <= size)) break")
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
            && raw_sym_83.contains("if (licm_28) {")
            && raw_sym_83.contains("if (licm_59) {"),
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
fn tesseract_runs_at_o2() {
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
fn tesseract_runtime_markers_match_between_o1_and_o2() {
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
fn tesseract_preserve_all_defs_keeps_helper_definitions_sound() {
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
    );
    assert!(
        code.contains("Sym_64 <- function(f, x, size)")
            || code.contains("Sym_64 <- function(f, x, ys, size)")
    );
    assert!(
        code.contains("Sym_66 <- function(f, x, size)")
            || code.contains("Sym_66 <- function(f, x, ys, size)")
    );
    assert!(
        code.contains("Sym_72 <- function(f, x, size)")
            || code.contains("Sym_72 <- function(f, x, ys, size)")
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

    assert!(code.contains("Sym_186 <- function(px, py, pf, u, v, dt, N)"));
    assert!(code.contains("px[i] <- x"));
    assert!(code.contains("py[i] <- y"));
    assert!(code.contains("pf[i] <- f"));
    assert!(!code.contains("rr_index1_write(i, \"index\")"));

    assert!(code.contains("Sym_287 <- function(temp, q_v, q_c, q_s, q_g, size)"));
    assert!(
        code.contains("heat[i] <- (tendency_T / 1004)")
            || code.contains("heat[i] <- (tendency_T / cp)")
    );
    assert!(code.contains("(0.005 * 2800000)") || code.contains("L_s <- 2800000"));
    assert!(!code.contains("cp <- 1004") || code.contains("heat[i] <- (tendency_T / cp)"));

    assert!(code.contains("Sym_49__typed_impl <- function(a, b)"));
    assert!(code.contains(
        "return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))"
    ));
}

#[test]
fn tesseract_o2_preserves_cg_recurrence_and_rk_buffer_swap() {
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
        .unwrap_or_else(|| panic!("expected Sym_123 in tesseract output"));
    let cg_loop = code[cg_fn..]
        .find("for (iter in seq_len(20)) {")
        .or_else(|| code[cg_fn..].find("repeat {"))
        .map(|idx| cg_fn + idx)
        .unwrap_or_else(|| panic!("expected CG loop in Sym_123"));
    let cg_ap = code[cg_loop..]
        .find("Ap <- ")
        .map(|idx| cg_loop + idx)
        .unwrap_or_else(|| panic!("expected Ap recompute inside Sym_123 loop"));
    assert!(cg_ap > cg_loop, "expected Ap to stay inside the CG loop");
    assert!(
        code[cg_loop..].contains("rs_new <- rs_old"),
        "expected rs_new fallback to restore rs_old inside CG loop"
    );
    assert!(
        code[cg_loop..].contains("p <- (r + (beta * p))"),
        "expected CG search direction update to remain in Sym_123"
    );
    assert!(
        code[cg_loop..].contains("rs_old <- rs_new"),
        "expected CG residual carry update to remain in Sym_123"
    );

    let rk_fn = code
        .find("Sym_303 <- function")
        .unwrap_or_else(|| panic!("expected Sym_303 in tesseract output"));
    assert!(
        code[rk_fn..].contains("tmp_u <- u")
            && code[rk_fn..].contains("u <- u_new")
            && code[rk_fn..].contains("u_new <- tmp_u"),
        "expected RK loop to preserve the u/u_new buffer swap"
    );
}

#[test]
fn tesseract_o1_proof_does_not_rebroadcast_vector_values() {
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
        .unwrap_or_else(|| panic!("expected Sym_123 in proof-enabled tesseract output"));
    assert!(
        !cg.contains("r <- rep.int(rr_index1_read_vec("),
        "proof-enabled Sym_123 must not rebroadcast an already-vector residual seed:\n{cg}"
    );
    assert!(
        !cg.contains("Ap <- (rep.int("),
        "proof-enabled Sym_123 must not rebroadcast an already-vector Ap expression:\n{cg}"
    );
    assert!(
        !cg.contains("x <- rep.int(.tachyon_exprmap"),
        "proof-enabled Sym_123 must not rebroadcast vector x updates:\n{cg}"
    );
    assert!(
        !cg.contains("r <- rep.int(.tachyon_exprmap"),
        "proof-enabled Sym_123 must not rebroadcast vector r updates:\n{cg}"
    );
    assert!(
        cg.contains("Ap <- ((4.0001 * p) -")
            && cg.contains("x <- (x + (alpha * p))")
            && cg.contains("r <- (r - (alpha * Ap))")
            && cg.contains("p <- (r + (beta * p))"),
        "proof-enabled Sym_123 must keep the CG vector recurrence intact:\n{cg}"
    );

    let sym_222 = extract_r_function(&code, "Sym_222")
        .unwrap_or_else(|| panic!("expected Sym_222 in proof-enabled tesseract output"));
    assert!(
        !sym_222.contains("lapA <- (rep.int(") && !sym_222.contains("lapB <- (rep.int("),
        "proof-enabled Sym_222 must not rebroadcast whole-grid laplacians:\n{sym_222}"
    );

    let rk = extract_r_function(&code, "Sym_303")
        .unwrap_or_else(|| panic!("expected Sym_303 in proof-enabled tesseract output"));
    assert!(
        !rk.contains("adj_ll <- rep.int(rr_gather(")
            && !rk.contains("adj_rr <- rep.int(rr_gather("),
        "proof-enabled Sym_303 must not rebroadcast topology gather vectors:\n{rk}"
    );
    assert!(
        !rk.contains("visc <- (rep.int(") && !rk.contains("adv_u <- (ifelse(rep.int("),
        "proof-enabled Sym_303 must not rebroadcast full-grid stencil vectors:\n{rk}"
    );
}
