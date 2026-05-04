use super::*;

#[test]
pub(crate) fn raw_emitted_immediate_single_use_named_scalar_exprs_do_not_inline_into_guard_only_use()
 {
    let input = [
        "Sym_123 <- function(rs_old, p_Ap, x, p) ",
        "{",
        "  alpha <- (rs_old / p_Ap)",
        "  if ((is.na(alpha) | (!(is.finite(alpha))))) {",
        "    alpha <- 0",
        "  }",
        "  x <- (x + (alpha * p))",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(out.contains("alpha <- (rs_old / p_Ap)"), "{out}");
    assert!(!out.contains("if ((is.na(rs_old / p_Ap)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_named_scalar_exprs_do_not_inline_inside_repeat_loop() {
    let input = [
        "Sym_1 <- function() ",
        "{",
        "  repeat {",
        "    if (!(time <= 5)) break",
        "    vy <- (vy + (g * dt))",
        "    y <- (y + (vy * dt))",
        "    time <- (time + dt)",
        "  }",
        "  return(y)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(out.contains("vy <- (vy + (g * dt))"), "{out}");
    assert!(out.contains("y <- (y + (vy * dt))"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_guard_only_named_scalar_exprs_inline_into_next_guard() {
    let input = [
        "Sym_222 <- function(x, y, cx, cy, r) ",
        "{",
        "  dx <- (x - cx)",
        "  dy <- (y - cy)",
        "  if ((((dx * dx) + (dy * dy)) < (r * r))) {",
        "    return(1)",
        "  }",
        "  return(0)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(!out.contains("dx <- (x - cx)"), "{out}");
    assert!(!out.contains("dy <- (y - cy)"), "{out}");
    assert!(
        out.contains("if (((((x - cx) * (x - cx)) + ((y - cy) * (y - cy))) < (r * r))) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_guard_only_named_scalar_exprs_do_not_inline_when_used_later() {
    let input = [
        "Sym_123 <- function(rs_old, p_Ap, x, p) ",
        "{",
        "  alpha <- (rs_old / p_Ap)",
        "  if ((is.na(alpha) | (!(is.finite(alpha))))) {",
        "    alpha <- 0",
        "  }",
        "  x <- (x + (alpha * p))",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(out.contains("alpha <- (rs_old / p_Ap)"), "{out}");
    assert!(!out.contains("if ((is.na(rs_old / p_Ap)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_immediate_single_use_named_scalar_exprs_inline_floor_index_alias() {
    let input = [
        "Sym_186 <- function(gx, gy, f, N) ",
        "{",
        "  ix <- floor(gx)",
        "",
        "  idx <- rr_idx_cube_vec_i(f, ix, floor(gy), N)",
        "  return(idx)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(!out.contains("ix <- floor(gx)"), "{out}");
    assert!(
        out.contains("idx <- rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)")
            || out.contains("idx <- rr_idx_cube_vec_i(f, (floor(gx)), floor(gy), N)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_guard_only_scalar_literals_inline_into_guard() {
    let input = [
        "Sym_186 <- function() ",
        "{",
        "  LIMIT <- 1000",
        "  if (!(i <= LIMIT)) break",
        "  return(i)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_guard_only_scalar_literals_in_raw_emitted_r(&input);

    assert!(!out.contains("LIMIT <- 1000"), "{out}");
    assert!(out.contains("if (!(i <= 1000)) break"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_loop_guard_scalar_literals_inline_through_repeat_header() {
    let input = [
        "Sym_186 <- function() ",
        "{",
        "  LIMIT <- 1000",
        "  # rr-cse-pruned",
        "",
        "  repeat {",
        "    if (!(i <= LIMIT)) break",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(i)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&input);

    assert!(!out.contains("LIMIT <- 1000"), "{out}");
    assert!(out.contains("if (!(i <= 1000)) break"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_loop_guard_scalar_literals_do_not_inline_induction_seed() {
    let input = [
        "Sym_123 <- function() ",
        "{",
        "  iter <- 1",
        "  repeat {",
        "    if (!(iter <= 20)) break",
        "    x <- iter",
        "    iter <- (iter + 1)",
        "    next",
        "  }",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&input);

    assert!(out.contains("iter <- 1"), "{out}");
    assert!(out.contains("if (!(iter <= 20)) break"), "{out}");
    assert!(!out.contains("if (!(1 <= 20)) break"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_loop_guard_scalar_literals_skip_rr_marked_induction_seed() {
    let input = [
        "Sym_1 <- function(n) ",
        "{",
        "  a <- 1L",
        "  b <- 2L",
        "  i <- 1L",
        "  repeat {",
        "    rr_mark(4, 5);",
        "    if (!(i <= n)) break",
        "    t <- a",
        "    a <- b",
        "    b <- t",
        "    i <- (i + 1L)",
        "    next",
        "  }",
        "  return((a + b))",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&input);

    assert!(out.contains("  i <- 1L"), "{out}");
    assert!(out.contains("if (!(i <= n)) break"), "{out}");
    assert!(out.contains("    i <- (i + 1L)"), "{out}");
    assert!(!out.contains("if (!((1L) <= n)) break"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_guard_only_named_scalar_exprs_skip_self_referential_increment() {
    let input = [
        "Sym_1 <- function() ",
        "{",
        "  i <- 0L",
        "  repeat {",
        "    if (!(i < 5L)) break",
        "    i <- (i + 1L)",
        "    next",
        "  }",
        "  return(i)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(out.contains("    i <- (i + 1L)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_restores_missing_lt_guard_counter_update() {
    let input = [
        "Sym_1 <- function(n) ",
        "{",
        "  x <- seq_len(n)",
        "  y <- x",
        "  i <- 1L",
        "  repeat {",
        "    rr_mark(8, 5);",
        "    if (!(i < length(x))) break",
        "    y[i] <- (x[i] + 10L)",
        "  }",
        "  return(y)",
        "}",
        "",
    ]
    .join("\n");

    let out = restore_missing_repeat_loop_counter_updates_in_raw_emitted_r(&input);

    assert!(out.contains("    i <- (i + 1L)"), "{out}");
    assert!(out.contains("    y[i] <- (x[i] + 10L)"), "{out}");
    assert!(out.contains("    if (!(i < length(x))) break"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_unused_middle_helper_params_trim_and_update_callsites() {
    let input = [
        "Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size) ",
        "{",
        "  heat <- rep.int(0, size)",
        "  if ((q_c[1] > 0)) {",
        "    heat[1] <- (q_c[1] + q_v[1])",
        "  }",
        "  if ((q_s[1] > 0)) {",
        "    heat[1] <- (heat[1] + q_g[1])",
        "  }",
        "  return(heat)",
        "}",
        "Sym_top_0 <- function() ",
        "{",
        "  heat <- Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL)",
        "  return(heat)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_unused_helper_params_in_raw_emitted_r(&input);

    assert!(
        out.contains("Sym_287 <- function(q_v, q_c, q_s, q_g, size)"),
        "{out}"
    );
    assert!(
        !out.contains("Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)"),
        "{out}"
    );
    assert!(
        out.contains("heat <- Sym_287(qv, qc, qs, qg, TOTAL)"),
        "{out}"
    );
    assert!(
        !out.contains("heat <- Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_helper_params_are_kept_when_helper_escapes_as_value() {
    let input = [
        "Sym_1 <- function(obj) ",
        "{",
        "  return(methods::standardGeneric(\"rr_tmp_generic\"))",
        "}",
        "Sym_top_0 <- function() ",
        "{",
        "  generic_name <- methods::setGeneric(\"rr_tmp_generic\", Sym_1)",
        "  return(generic_name)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_unused_helper_params_in_raw_emitted_r(&input);

    assert!(out.contains("Sym_1 <- function(obj)"), "{out}");
    assert!(!out.contains("Sym_1 <- function()"), "{out}");
    assert!(
        out.contains("methods::setGeneric(\"rr_tmp_generic\", Sym_1)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_terminal_repeat_nexts_prune_without_touching_inner_if_nexts() {
    let input = [
        "Sym_83 <- function() ",
        "{",
        "  repeat {",
        "    if ((flag)) {",
        "      next",
        "    }",
        "    x <- (x + 1)",
        "    next",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_terminal_repeat_nexts_in_raw_emitted_r(&input);

    assert!(out.contains("if ((flag)) {\n      next\n    }"), "{out}");
    assert!(!out.contains("x <- (x + 1)\n    next\n  }"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_same_var_is_na_or_not_finite_guards_simplify() {
    let input = [
        "Sym_123 <- function() ",
        "{",
        "  if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {",
        "    rs_old <- 0.0000001",
        "  }",
        "  if ((is.na(alpha) | (!(is.finite(alpha))))) {",
        "    alpha <- 0",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = simplify_same_var_is_na_or_not_finite_guards_in_raw_emitted_r(&input);

    assert!(
        out.contains("if (((!(is.finite(rs_old))) | (rs_old == 0))) {"),
        "{out}"
    );
    assert!(out.contains("if ((!(is.finite(alpha)))) {"), "{out}");
    assert!(!out.contains("is.na(rs_old)"), "{out}");
    assert!(!out.contains("is.na(alpha)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_not_finite_or_zero_guard_parens_simplify() {
    let input = [
        "Sym_123 <- function() ",
        "{",
        "  if (((!(is.finite(rs_old))) | (rs_old == 0))) {",
        "    rs_old <- 0.0000001",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = simplify_not_finite_or_zero_guard_parens_in_raw_emitted_r(&input);

    assert!(
        out.contains("if ((!(is.finite(rs_old)) | (rs_old == 0))) {"),
        "{out}"
    );
    assert!(
        !out.contains("if (((!(is.finite(rs_old))) | (rs_old == 0))) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_wrapped_not_finite_parens_simplify() {
    let input = [
        "Sym_123 <- function() ",
        "{",
        "  if ((!(is.finite(alpha)))) {",
        "    alpha <- 0",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = simplify_wrapped_not_finite_parens_in_raw_emitted_r(&input);

    assert!(out.contains("if (!(is.finite(alpha))) {"), "{out}");
    assert!(!out.contains("if ((!(is.finite(alpha)))) {"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_nested_else_if_blocks_collapse() {
    let input = [
        "Sym_60 <- function(f, x, size) ",
        "{",
        "  if ((x > 1)) {",
        "    return(a)",
        "  } else {",
        "    if ((f == 1)) {",
        "      return(b)",
        "    } else {",
        "      if ((f == 2)) {",
        "        return(c)",
        "      } else {",
        "        return(d)",
        "      }",
        "    }",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = collapse_nested_else_if_blocks_in_raw_emitted_r(&input);

    assert!(out.contains("} else if ((f == 1)) {"), "{out}");
    assert!(out.contains("} else if ((f == 2)) {"), "{out}");
    assert!(!out.contains("  } else {\n    if ((f == 1)) {"), "{out}");
    assert!(
        !out.contains("    } else {\n      if ((f == 2)) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_unused_arg_aliases_strip_after_dead_scalar_alias_prune() {
    let input = [
        "Sym_287 <- function(q_r, q_i, i) ",
        "{",
        "  .arg_q_r <- q_r",
        "  .arg_q_i <- q_i",
        "  qr <- .arg_q_r[i]",
        "  qi <- .arg_q_i[i]",
        "  return(0)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_unused_raw_arg_aliases_in_raw_emitted_r(
        &rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(&input),
    );

    assert!(!out.contains(".arg_q_r <- q_r"), "{out}");
    assert!(!out.contains(".arg_q_i <- q_i"), "{out}");
    assert!(!out.contains("qr <- .arg_q_r[i]"), "{out}");
    assert!(!out.contains("qi <- .arg_q_i[i]"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_dead_simple_scalar_alias_and_literal_assignments_prune() {
    let input = [
        "Sym_287 <- function(size) ",
        "{",
        "  keep <- size",
        "  dead_const <- 2500000",
        "  dead_alias <- keep",
        "  live <- keep",
        "  return(live)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

    assert!(out.contains("keep <- size"), "{out}");
    assert!(!out.contains("dead_const <- 2500000"), "{out}");
    assert!(!out.contains("dead_alias <- keep"), "{out}");
    assert!(out.contains("live <- keep"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_dead_licm_scalar_expr_assignments_prune() {
    let input = [
        "Sym_83 <- function(x) ",
        "{",
        "  licm_71 <- (x + 1)",
        "  keep <- x",
        "  return(keep)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

    assert!(!out.contains("licm_71 <- (x + 1)"), "{out}");
    assert!(out.contains("keep <- x"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_dead_pure_scalar_expr_assignments_prune() {
    let input = [
        "Sym_287 <- function(temp, size) ",
        "{",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= size)) break",
        "    T_c <- (temp[i] - 273.15)",
        "    es_ice <- (6.11 * exp(((22.5 * T_c) / (temp[i] + 273.15))))",
        "    keep <- (T_c + 1)",
        "    rr_mark(1, 1);",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(keep)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

    assert!(!out.contains("es_ice <- "), "{out}");
    assert!(out.contains("keep <- (T_c + 1)"), "{out}");
    assert!(out.contains("rr_mark(1, 1);"), "{out}");
    assert!(out.contains("return(keep)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_dead_pure_scalar_expr_assignments_keep_loop_induction_updates() {
    let input = [
        "Sym_117 <- function(a, b, n) ",
        "{",
        "  sum <- 0",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= n)) break",
        "    sum <- (sum + (a[i] * b[i]))",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(sum)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

    assert!(out.contains("i <- (i + 1)"), "{out}");
    assert!(out.contains("sum <- (sum + (a[i] * b[i]))"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_two_use_named_scalar_exprs_inline_into_adjacent_assignments() {
    let input = [
        "Sym_210 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
        "{",
        "  a <- A[i]",
        "  b <- B[i]",
        "  reaction <- ((a * b) * b)",
        "  new_a <- (a + ((((DA * lapA[i]) - reaction) + (f * (1 - a))) * dt))",
        "  new_b <- (b + ((((DB * lapB[i]) + reaction) - ((k + f) * b)) * dt))",
        "  return(new_b)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(&input);

    assert!(!out.contains("reaction <- ((a * b) * b)"), "{out}");
    assert!(
        out.contains("new_a <- (a + ((((DA * lapA[i]) - (a * b) * b) + (f * (1 - a))) * dt))"),
        "{out}"
    );
    assert!(
        out.contains("new_b <- (b + ((((DB * lapB[i]) + (a * b) * b) - ((k + f) * b)) * dt))"),
        "{out}"
    );
}
