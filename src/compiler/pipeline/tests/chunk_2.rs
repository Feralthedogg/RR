use super::*;

#[test]
pub(crate) fn raw_emitted_mountain_dx_temp_inlines_into_dist_expr() {
    let input = [
        "Sym_303 <- function() ",
        "{",
        "  dx_m <- (x_curr - 20)",
        "  dist <- ((dx_m * dx_m) + ((y_curr - 20) * (y_curr - 20)))",
        "  return(dist)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_mountain_dx_temp_in_raw_emitted_r(&input);

    assert!(!out.contains("dx_m <- (x_curr - 20)"), "{out}");
    assert!(
        out.contains(
            "dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
        ),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_mountain_dx_dy_temps_inline_into_dist_expr() {
    let input = [
        "Sym_303 <- function() ",
        "{",
        "  dx_m <- (x_curr - 20)",
        "  dy_m <- (y_curr - 20)",
        "  .__rr_cse_205 <- (rem / N)",
        "  .__rr_cse_206 <- floor(.__rr_cse_205)",
        "  .__rr_cse_209 <- (rem %% N)",
        "  dist <- ((dx_m * dx_m) + (dy_m * dy_m))",
        "  return(dist)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_mountain_dx_temp_in_raw_emitted_r(&input);

    assert!(!out.contains("dx_m <- (x_curr - 20)"), "{out}");
    assert!(!out.contains("dy_m <- (y_curr - 20)"), "{out}");
    assert!(
        out.contains(
            "dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
        ),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_dead_zero_seed_ii_prunes_when_never_used() {
    let input = [
    "Sym_303 <- function() ",
    "{",
    "  i <- 1",
    "  ii <- 0",
    "  .tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL))))",
    "  return(.tachyon_exprmap0_0)",
    "}",
    "",
]
.join("\n");

    let out = strip_dead_zero_seed_ii_in_raw_emitted_r(&input);

    assert!(!out.contains("ii <- 0"), "{out}");
    assert!(out.contains(".tachyon_exprmap0_0 <- rr_gather("), "{out}");
}

#[test]
pub(crate) fn raw_emitted_trivial_fill_helper_calls_inline_to_rep_int() {
    let input = [
        "Sym_17 <- function(n, val) ",
        "{",
        "  return(rep.int(val, n))",
        "}",
        "",
        "Sym_303 <- function(TOTAL) ",
        "{",
        "  h <- Sym_17(TOTAL, 8000)",
        "  qv <- Sym_17(TOTAL, 0.015)",
        "  p_f <- Sym_17(1000, 1)",
        "  return(h)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_trivial_fill_helper_calls_in_raw_emitted_r(&input);

    assert!(out.contains("h <- rep.int(8000, TOTAL)"), "{out}");
    assert!(out.contains("qv <- rep.int(0.015, TOTAL)"), "{out}");
    assert!(out.contains("p_f <- rep.int(1, 1000)"), "{out}");
    assert!(!out.contains("h <- Sym_17(TOTAL, 8000)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_identical_zero_fill_pairs_rewrite_to_aliases() {
    let input = [
        "Sym_303 <- function(TOTAL) ",
        "{",
        "  adj_ll <- rep.int(0, TOTAL)",
        "  adj_rr <- rep.int(0, TOTAL)",
        "  u_stage <- rep.int(0, TOTAL)",
        "  u_new <- rep.int(0, TOTAL)",
        "  return(adj_rr)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&input);

    assert!(out.contains("adj_ll <- rep.int(0, TOTAL)"), "{out}");
    assert!(out.contains("adj_rr <- adj_ll"), "{out}");
    assert!(out.contains("u_stage <- rep.int(0, TOTAL)"), "{out}");
    assert!(out.contains("u_new <- u_stage"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_identical_zero_fill_pairs_do_not_alias_when_buffers_diverge_later() {
    let input = [
        "Sym_303 <- function(TOTAL) ",
        "{",
        "  u_stage <- rep.int(0, TOTAL)",
        "  u_new <- rep.int(0, TOTAL)",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= TOTAL)) break",
        "    u_new[i] <- i",
        "    i <- (i + 1)",
        "  }",
        "  return(u_new)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&input);

    assert!(out.contains("u_stage <- rep.int(0, TOTAL)"), "{out}");
    assert!(out.contains("u_new <- rep.int(0, TOTAL)"), "{out}");
    assert!(!out.contains("u_new <- u_stage"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_zero_fill_alias_chain_rewrites_to_root_alias() {
    let input = [
        "Sym_303 <- function(TOTAL, u, dt, du1, adv_u) ",
        "{",
        "  v <- rep.int(0, TOTAL)",
        "  u_stage <- v",
        "  u_new <- u_stage",
        "  u_stage <- (u + (dt * (du1 - adv_u)))",
        "  return(u_new)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&input);

    assert!(out.contains("u_stage <- v"), "{out}");
    assert!(out.contains("u_new <- v"), "{out}");
    assert!(
        out.contains("u_stage <- (u + (dt * (du1 - adv_u)))"),
        "{out}"
    );
    assert!(!out.contains("u_new <- u_stage"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_duplicate_sym183_calls_reuse_first_result() {
    let input = [
        "Sym_303 <- function() ",
        "{",
        "  p_x <- Sym_183(1000)",
        "  p_y <- Sym_183(1000)",
        "  return(p_y)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_duplicate_sym183_calls_in_raw_emitted_r(&input);

    assert!(out.contains("p_x <- Sym_183(1000)"), "{out}");
    assert!(out.contains("p_y <- p_x"), "{out}");
    assert!(!out.contains("p_y <- Sym_183(1000)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_prunes_dead_zero_loop_seeds_before_for() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
steps <- 0\n\
dt <- 0.1\n\
for (steps in seq_len(5)) {\n\
  print(steps)\n\
}\n\
k <- 1\n\
for (k in seq_len(TOTAL)) {\n\
  print(k)\n\
}\n\
}\n";
    let out = strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(input);
    assert!(!out.contains("steps <- 0"), "{out}");
    assert!(!out.contains("k <- 1"), "{out}");
    assert!(out.contains("for (steps in seq_len(5)) {"), "{out}");
    assert!(out.contains("for (k in seq_len(TOTAL)) {"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_duplicate_pure_call_assignments_reuse_first_result() {
    let input = [
        "Sym_303 <- function(temp, qv, qc, qs, qg, TOTAL) ",
        "{",
        "  heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)",
        "  heat2 <- Sym_287(temp, qv, qc, qs, qg, TOTAL)",
        "  return(heat2)",
        "}",
        "",
    ]
    .join("\n");
    let pure = FxHashSet::from_iter([String::from("Sym_287")]);

    let out = rewrite_duplicate_pure_call_assignments_in_raw_emitted_r(&input, &pure);

    assert!(
        out.contains("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)"),
        "{out}"
    );
    assert!(out.contains("heat2 <- heat"), "{out}");
    assert!(
        !out.contains("heat2 <- Sym_287(temp, qv, qc, qs, qg, TOTAL)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_adjacent_duplicate_symbol_assignments_reuse_first_result() {
    let input = [
        "Sym_123 <- function(b) ",
        "{",
        "  r <- b",
        "  p <- b",
        "  return(p)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_adjacent_duplicate_symbol_assignments_in_raw_emitted_r(&input);

    assert!(out.contains("r <- b"), "{out}");
    assert!(out.contains("p <- r"), "{out}");
    assert!(!out.contains("p <- b"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_weno_full_range_gather_replay_after_fill_inline_collapses() {
    let input = [
    "Sym_303 <- function(TOTAL, adj_l, adj_r) ",
    "{",
    "  adj_ll <- qr",
    "  adj_rr <- adj_ll",
    "  i <- 1",
    "",
    "  # rr-cse-pruned",
    "  .tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:((6 * N) * N)))))",
    "  .tachyon_exprmap1_0 <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:((6 * N) * N)))))",
    "  adj_ll <- rr_assign_slice(adj_ll, i, ((6 * N) * N), .tachyon_exprmap0_0)",
    "  adj_rr <- rr_assign_slice(adj_rr, i, ((6 * N) * N), .tachyon_exprmap1_0)",
    "  return(adj_ll)",
    "}",
    "",
]
.join("\n");

    let out = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&input);

    assert!(
        out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(
        out.contains("adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"),
        "{out}"
    );
    assert!(!out.contains(".tachyon_exprmap0_0 <-"), "{out}");
    assert!(!out.contains(".tachyon_exprmap1_0 <-"), "{out}");
    assert!(!out.contains("rr_assign_slice(adj_ll"), "{out}");
    assert!(!out.contains("rr_assign_slice(adj_rr"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_weno_full_range_gather_replay_after_fill_inline_collapses_with_following_lines()
 {
    let input = [
    "Sym_303 <- function(TOTAL, adj_l, adj_r) ",
    "{",
    "  print(\"  Building Extended Topology (WENO-5 Order)...\")",
    "  adj_ll <- qr",
    "  adj_rr <- qr",
    "  i <- 1",
    "",
    "  # rr-cse-pruned",
    "  .tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:((6 * N) * N)))))",
    "  .tachyon_exprmap1_0 <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:((6 * N) * N)))))",
    "  adj_ll <- rr_assign_slice(adj_ll, i, ((6 * N) * N), .tachyon_exprmap0_0)",
    "  adj_rr <- rr_assign_slice(adj_rr, i, ((6 * N) * N), .tachyon_exprmap1_0)",
    "  h_trn <- qr",
    "  coriolis <- qr",
    "  return(adj_ll)",
    "}",
    "",
]
.join("\n");

    let out = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&input);

    assert!(
        out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(
        out.contains("adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"),
        "{out}"
    );
    assert!(!out.contains(".tachyon_exprmap0_0 <-"), "{out}");
    assert!(!out.contains(".tachyon_exprmap1_0 <-"), "{out}");
    assert!(!out.contains("rr_assign_slice(adj_ll"), "{out}");
    assert!(!out.contains("rr_assign_slice(adj_rr"), "{out}");
    assert!(out.contains("h_trn <- qr"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_weno_full_range_gather_replay_collapses_numeric_folded_shape() {
    let input = [
        "print(\"  Building Extended Topology (WENO-5 Order)...\")",
        "adj_ll <- rep.int(0.0, TOTAL)",
        "adj_rr <- adj_ll",
        "i <- 1.0",
        ".tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:9600.0))))",
        ".tachyon_exprmap1_0 <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:9600.0))))",
        "adj_ll <- rr_assign_slice(adj_ll, i, 9600.0, .tachyon_exprmap0_0)",
        "adj_rr <- rr_assign_slice(adj_rr, i, 9600.0, .tachyon_exprmap1_0)",
        "h_trn <- v",
        "coriolis <- v",
        "",
    ]
    .join("\n");

    let out = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&input);

    assert!(
        out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(
        out.contains("adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"),
        "{out}"
    );
    assert!(!out.contains(".tachyon_exprmap0_0 <-"), "{out}");
    assert!(!out.contains("rr_assign_slice(adj_ll"), "{out}");
    assert!(out.contains("h_trn <- v"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_weno_full_range_gather_replay_skips_dead_ii_seed() {
    let input = [
        "print(\"  Building Extended Topology (WENO-5 Order)...\")",
        "adj_ll <- rep.int(0.0, TOTAL)",
        "adj_rr <- adj_ll",
        "i <- 1.0",
        "ii <- 0.0",
        ".tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:9600.0))))",
        ".tachyon_exprmap1_0 <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:9600.0))))",
        "adj_ll <- rr_assign_slice(adj_ll, i, 9600.0, .tachyon_exprmap0_0)",
        "adj_rr <- rr_assign_slice(adj_rr, i, 9600.0, .tachyon_exprmap1_0)",
        "h_trn <- v",
        "",
    ]
    .join("\n");

    let out = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&input);

    assert!(
        out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(
        out.contains("adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"),
        "{out}"
    );
    assert!(out.contains("ii <- 0.0"), "{out}");
    assert!(!out.contains(".tachyon_exprmap0_0 <-"), "{out}");
    assert!(!out.contains("rr_assign_slice(adj_ll"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_dot_product_helper_calls_inline_to_direct_sum_exprs() {
    let input = [
        "Sym_117 <- function(a, b, n) ",
        "{",
        "  return(sum((a[seq_len(n)] * b[seq_len(n)])))",
        "}",
        "",
        "Sym_123 <- function(r, p, Ap, size) ",
        "{",
        "  rs_old <- Sym_117(r, r, size)",
        "  p_Ap <- Sym_117(p, Ap, size)",
        "  rs_new <- Sym_117((r - (alpha * Ap)), (r - (alpha * Ap)), size)",
        "  return(rs_new)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_dot_product_helper_calls_in_raw_emitted_r(&input);

    assert!(
        out.contains("rs_old <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
        "{out}"
    );
    assert!(
        out.contains("p_Ap <- sum((p[seq_len(size)] * Ap[seq_len(size)]))"),
        "{out}"
    );
    assert!(
        out.contains(
            "rs_new <- sum(((r - (alpha * Ap))[seq_len(size)] * (r - (alpha * Ap))[seq_len(size)]))"
        ),
        "{out}"
    );
    assert!(!out.contains("rs_old <- Sym_117(r, r, size)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_sym119_helper_calls_inline_to_direct_gather_expr() {
    let input = [
    "Sym_119 <- function(x, n_l, n_r, n_d, n_u) ",
    "{",
    "  n_d <- rr_index_vec_floor(n_d)",
    "  n_l <- rr_index_vec_floor(n_l)",
    "  n_r <- rr_index_vec_floor(n_r)",
    "  n_u <- rr_index_vec_floor(n_u)",
    "  y <- ((4.0001 * x) - (((rr_gather(x, n_l) + rr_gather(x, n_r)) + rr_gather(x, n_d)) + rr_gather(x, n_u)))",
    "  return(y)",
    "}",
    "",
    "Sym_123 <- function(p, n_l, n_r, n_d, n_u) ",
    "{",
    "  Ap <- Sym_119(p, n_l, n_r, n_d, n_u)",
    "  return(Ap)",
    "}",
    "",
]
.join("\n");

    let out = rewrite_sym119_helper_calls_in_raw_emitted_r(&input);

    assert!(
    out.contains(
        "Ap <- ((4.0001 * p) - (((rr_gather(p, rr_index_vec_floor(n_l)) + rr_gather(p, rr_index_vec_floor(n_r))) + rr_gather(p, rr_index_vec_floor(n_d))) + rr_gather(p, rr_index_vec_floor(n_u))))"
    ),
    "{out}"
);
    assert!(
        !out.contains("Ap <- Sym_119(p, n_l, n_r, n_d, n_u)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_seq_len_full_overwrite_init_rewrites_to_zero_init() {
    let input = [
        "Sym_183 <- function(n) ",
        "{",
        "  p <- seq_len(n)",
        "  i <- 1",
        "  seed <- 12345",
        "  repeat {",
        "    if (!(i <= n)) break",
        "    seed <- (((seed * 1103515245) + 12345) %% 2147483648)",
        "    p[i] <- (seed / 2147483648)",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(p)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_seq_len_full_overwrite_inits_in_raw_emitted_r(&input);

    assert!(out.contains("p <- rep.int(0, n)"), "{out}");
    assert!(!out.contains("p <- seq_len(n)"), "{out}");
    assert!(out.contains("p[i] <- (seed / 2147483648)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_dead_seq_len_local_keeps_seed_when_rhs_reads_previous_value() {
    let input = [
    "Sym_1 <- function(n) ",
    "{",
    "  y <- seq_len(n)",
    "  y <- rr_call_map_slice_auto(y, 2L, n, \"abs\", 16L, c(1L), rr_index1_read_vec((seq_len(n) - 5L), rr_index_vec_floor(2L:n)))",
    "  return(y)",
    "}",
    "",
]
.join("\n");

    let out = strip_dead_seq_len_locals_in_raw_emitted_r(&input);

    assert!(out.contains("y <- seq_len(n)"), "{out}");
    assert!(
        out.contains("y <- rr_call_map_slice_auto(y, 2L, n"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_helper_expr_reuse_calls_rewrite_probe_energy_to_probe_vec() {
    let input = [
        "Sym_49 <- function(a, b) ",
        "{",
        "  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))",
        "}",
        "",
        "Sym_51 <- function(a, b) ",
        "{",
        "  mix <- Sym_49(a, b)",
        "  return(mean(abs(mix)))",
        "}",
        "",
        "Sym_303 <- function() ",
        "{",
        "  probe_vec <- Sym_49(c(1, 2, 3, 4), c(4, 3, 2, 1))",
        "  probe_energy <- Sym_51(c(1, 2, 3, 4), c(4, 3, 2, 1))",
        "  return(probe_energy)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_helper_expr_reuse_calls_in_raw_emitted_r(&input);

    assert!(
        out.contains("probe_vec <- Sym_49(c(1, 2, 3, 4), c(4, 3, 2, 1))"),
        "{out}"
    );
    assert!(
        out.contains("probe_energy <- mean(abs(probe_vec))"),
        "{out}"
    );
    assert!(
        !out.contains("probe_energy <- Sym_51(c(1, 2, 3, 4), c(4, 3, 2, 1))"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_cg_loop_carried_updates_restore_after_fused_rs_new_shape() {
    let input = [
        "Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) ",
        "{",
        "  x <- rep.int(0, size)",
        "  r <- b",
        "  p <- b",
        "  iter <- 1",
        "  repeat {",
        "    if (!(iter <= 20)) break",
        "    x <- (x + (alpha * p))",
        "",
        "    rs_new <- Sym_117((r - (alpha * Ap)), (r - (alpha * Ap)), size)",
        "    if ((is.na(rs_new) | (!(is.finite(rs_new))))) {",
        "      rs_new <- rs_old",
        "    }",
        "    beta <- (rs_new / rs_old)",
        "    if ((is.na(beta) | (!(is.finite(beta))))) {",
        "      beta <- 0",
        "    }",
        "    iter <- (iter + 1)",
        "    next",
        "  }",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");

    let out = restore_cg_loop_carried_updates_in_raw_emitted_r(&input);

    assert!(out.contains("r <- (r - (alpha * Ap))"), "{out}");
    assert!(
        out.contains("rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
        "{out}"
    );
    assert!(out.contains("p <- (r + (beta * p))"), "{out}");
    assert!(out.contains("rs_old <- rs_new"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_cg_loop_carried_updates_restore_after_direct_sum_rs_new_shape() {
    let input = [
    "Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) ",
    "{",
    "  x <- rep.int(0, size)",
    "  r <- b",
    "  p <- b",
    "  iter <- 1",
    "  repeat {",
    "    if (!(iter <= 20)) break",
    "    x <- (x + (alpha * p))",
    "",
    "    rs_new <- sum(((r - (alpha * Ap))[seq_len(size)] * (r - (alpha * Ap))[seq_len(size)]))",
    "    if ((is.na(rs_new) | (!(is.finite(rs_new))))) {",
    "      rs_new <- rs_old",
    "    }",
    "    beta <- (rs_new / rs_old)",
    "    if ((is.na(beta) | (!(is.finite(beta))))) {",
    "      beta <- 0",
    "    }",
    "    iter <- (iter + 1)",
    "    next",
    "  }",
    "  return(x)",
    "}",
    "",
]
.join("\n");

    let out = restore_cg_loop_carried_updates_in_raw_emitted_r(&input);

    assert!(out.contains("r <- (r - (alpha * Ap))"), "{out}");
    assert!(
        out.contains("rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
        "{out}"
    );
    assert!(out.contains("p <- (r + (beta * p))"), "{out}");
    assert!(out.contains("rs_old <- rs_new"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_cg_loop_carried_updates_restore_current_repeat_shape() {
    let input = [
    "Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) ",
    "{",
    "  x <- rep.int(0, size)",
    "  r <- b",
    "  p <- r",
    "  rs_old <- sum((r[seq_len(size)] * r[seq_len(size)]))",
    "  iter <- 1",
    "  repeat {",
    "    if (!(iter <= 20)) break",
    "    Ap <- ((4.0001 * p) - (((rr_gather(p, rr_index_vec_floor(n_l)) + rr_gather(p, rr_index_vec_floor(n_r))) + rr_gather(p, rr_index_vec_floor(n_d))) + rr_gather(p, rr_index_vec_floor(n_u))))",
    "    p_Ap <- sum((p[seq_len(size)] * Ap[seq_len(size)]))",
    "    alpha <- (rs_old / p_Ap)",
    "    x <- (x + (alpha * p))",
    "    r <- (r - (alpha * Ap))",
    "    rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))",
    "    if (rr_truthy1((!(is.finite(rs_new))), \"condition\")) {",
    "",
    "    } else {",
    "      rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))",
    "    }",
    "    beta <- (rs_new / rs_old)",
    "    if (!(is.finite(beta))) {",
    "      beta <- 0",
    "    }",
    "    iter <- (iter + 1)",
    "  }",
    "  return(x)",
    "}",
    "",
]
.join("\n");

    let out = restore_cg_loop_carried_updates_in_raw_emitted_r(&input);

    assert!(out.contains("r <- (r - (alpha * Ap))"), "{out}");
    assert!(
        out.contains("rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
        "{out}"
    );
    assert!(out.contains("rs_new <- rs_old"), "{out}");
    assert!(out.contains("p <- (r + (beta * p))"), "{out}");
    assert!(out.contains("rs_old <- rs_new"), "{out}");
    assert!(
        !out.contains("} else {\n      rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_cg_loop_carried_updates_drops_direct_helper_else_recompute() {
    let input = [
        "Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) ",
        "{",
        "  x <- rep.int(0, size)",
        "  r <- b",
        "  p <- r",
        "  rs_old <- Sym_117(b, b, size)",
        "  iter <- 1",
        "  repeat {",
        "    if (!(iter <= 20)) break",
        "    alpha <- (rs_old / p_Ap)",
        "    x <- (x + (alpha * p))",
        "    r <- (r - (alpha * Ap))",
        "    rs_new <- Sym_117(r, r, size)",
        "    if (rr_truthy1(!(is.finite(rs_new)), \"condition\")) {",
        "      rs_new <- rs_old",
        "    } else {",
        "      rs_new <- Sym_117(r, r, size)",
        "    }",
        "    beta <- (rs_new / rs_old)",
        "    iter <- (iter + 1)",
        "  }",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");

    let out = restore_cg_loop_carried_updates_in_raw_emitted_r(&input);

    assert!(out.contains("rs_new <- Sym_117(r, r, size)"), "{out}");
    assert!(out.contains("rs_new <- rs_old"), "{out}");
    assert!(
        !out.contains("} else {\n      rs_new <- Sym_117(r, r, size)"),
        "{out}"
    );
    assert_eq!(
        out.matches("rs_new <- Sym_117(r, r, size)").count(),
        1,
        "{out}"
    );
    assert!(out.contains("p <- (r + (beta * p))"), "{out}");
    assert!(out.contains("rs_old <- rs_new"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_restores_temp_buffer_swap_after_repeat_update_loop() {
    let input = [
        "Sym_303 <- function() ",
        "{",
        "  repeat {",
        "    if (!(steps < 5)) break",
        "    i <- 1",
        "    repeat {",
        "      if (!(i <= TOTAL)) break",
        "      u_new[i] <- step_u[i]",
        "      i <- (i + 1)",
        "    }",
        "    tmp_u <- u",
        "    print(max_u)",
        "    steps <- (steps + 1)",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(&input);

    assert!(out.contains("tmp_u <- u"), "{out}");
    assert!(out.contains("u <- u_new"), "{out}");
    assert!(out.contains("u_new <- tmp_u"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_branch_local_scalar_assigns_hoist_before_later_uses() {
    let input = [
        "Sym_287 <- function(temp, q_v, q_s, q_g, size) ",
        "{",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= size)) break",
        "    T_c <- (temp[i] - 273.15)",
        "    if ((T_c < (-(5)))) {",
        "      qv <- q_v[i]",
        "      qs <- q_s[i]",
        "      qg <- q_g[i]",
        "    }",
        "    if ((T_c < (-(15)))) {",
        "      if ((qv > 0.01)) {",
        "        print(T_c)",
        "      }",
        "    }",
        "    if ((T_c > 0)) {",
        "      if ((qs > 0)) {",
        "        print(qs)",
        "      }",
        "      if ((qg > 0)) {",
        "        print(qg)",
        "      }",
        "    }",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(0)",
        "}",
        "",
    ]
    .join("\n");

    let out = hoist_branch_local_pure_scalar_assigns_used_after_branch_in_raw_emitted_r(&input);

    assert!(out.contains("qv <- q_v[i]"), "{out}");
    assert!(out.contains("qs <- q_s[i]"), "{out}");
    assert!(out.contains("qg <- q_g[i]"), "{out}");
    let qv_idx = out.find("qv <- q_v[i]").expect("{out}");
    let qs_idx = out.find("qs <- q_s[i]").expect("{out}");
    let qg_idx = out.find("qg <- q_g[i]").expect("{out}");
    let warm_idx = out.find("if ((T_c < (-(5)))) {").expect("{out}");
    assert!(qv_idx < warm_idx, "{out}");
    assert!(qs_idx < warm_idx, "{out}");
    assert!(qg_idx < warm_idx, "{out}");
}

#[test]
pub(crate) fn raw_emitted_immediate_single_use_named_scalar_exprs_inline_before_peephole() {
    let input = [
        "Sym_287 <- function(q_c, i) ",
        "{",
        "  if ((q_c[i] > 0.0001)) {",
        "    rate <- (0.01 * q_c[i])",
        "    tendency_T <- (rate * L_f)",
        "  }",
        "  return(tendency_T)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(!out.contains("rate <- (0.01 * q_c[i])"), "{out}");
    assert!(
        out.contains("tendency_T <- ((0.01 * q_c[i]) * L_f)")
            || out.contains("tendency_T <- (rate * L_f)")
            || out.contains("tendency_T <- (((0.01 * q_c[i]) * L_f))"),
        "{out}"
    );
}
