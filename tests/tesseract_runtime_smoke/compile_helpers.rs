use std::fs;
use std::path::PathBuf;

use super::common::{compile_rr, unique_dir};

pub(crate) fn extract_numeric_series(stdout: &str, marker: &str) -> Vec<f64> {
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

pub(crate) fn assert_series_close(label: &str, a: &[f64], b: &[f64]) {
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

pub(crate) fn contains_line(haystack: &str, needle: &str) -> bool {
    haystack.lines().any(|line| line.trim() == needle)
}

pub(crate) fn tesseract_test_dir(name: &str) -> PathBuf {
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

pub(crate) fn extract_r_function(code: &str, name: &str) -> Option<String> {
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
pub(crate) fn tesseract_compiles_across_opt_levels() {
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
            (code.contains("particles <- Sym_") || code.contains("particles <- advect_particles("))
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
                    code.contains("probe_pair <- list(left = c(1.0, 2.0, 3.0, 4.0), right = c(4.0, 3.0, 2.0, 1.0))")
                        && code.contains("probe_pair[[\"left\"]]")
                        && code.contains("probe_pair[[\"right\"]]")
                        && !code.contains("FieldEnergy.energy")
                        && !code.contains("typed_trait_energy("),
                    "expected probe_energy to lower through the static FieldEnergy trait probe for {}",
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
