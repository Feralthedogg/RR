use super::*;

#[test]
fn emitted_program_round_trips_function_body() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  x <- 1".to_string(),
        "  return(x)".to_string(),
        "}".to_string(),
        "Sym_1()".to_string(),
    ];
    let out = EmittedProgram::parse(&lines).into_lines();
    assert_eq!(out, lines);
}

#[test]
fn strip_terminal_repeat_nexts_ir_removes_repeat_tail_next() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  repeat {".to_string(),
        "    x <- 1".to_string(),
        "    next".to_string(),
        "  }".to_string(),
        "}".to_string(),
    ];
    let out = strip_terminal_repeat_nexts_ir(lines).join("\n");
    assert!(!out.contains("\n    next\n"));
}

#[test]
fn strip_empty_else_blocks_ir_collapses_empty_else() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  if ((x > 0)) {".to_string(),
        "    y <- 1".to_string(),
        "  } else {".to_string(),
        "".to_string(),
        "  }".to_string(),
        "}".to_string(),
    ];
    let out = strip_empty_else_blocks_ir(lines).join("\n");
    assert!(!out.contains("} else {"));
}

#[test]
fn strip_dead_simple_eval_lines_ir_removes_dead_eval_lines() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  temp".to_string(),
        "  (tmp2)".to_string(),
        "  return(x)".to_string(),
        "}".to_string(),
    ];
    let out = strip_dead_simple_eval_lines_ir(lines).join("\n");
    assert!(!out.contains("\n  temp\n"), "{out}");
    assert!(!out.contains("\n  (tmp2)\n"), "{out}");
    assert!(out.contains("return(x)"), "{out}");
}

#[test]
fn strip_noop_self_assignments_ir_removes_noop_assign() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  x <- x".to_string(),
        "  y <- z".to_string(),
        "}".to_string(),
    ];
    let out = strip_noop_self_assignments_ir(lines).join("\n");
    assert!(!out.contains("\n  x <- x\n"), "{out}");
    assert!(out.contains("y <- z"), "{out}");
}

#[test]
fn rewrite_dead_zero_loop_seeds_before_for_ir_drops_unused_seed() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  i <- 0".to_string(),
        "  for (i in seq_len(n)) {".to_string(),
        "    x <- xs[i]".to_string(),
        "  }".to_string(),
        "}".to_string(),
    ];
    let out = rewrite_dead_zero_loop_seeds_before_for_ir(lines).join("\n");
    assert!(!out.contains("i <- 0"));
    assert!(out.contains("for (i in seq_len(n)) {"));
}

#[test]
fn collapse_trivial_dot_product_wrappers_ir_rewrites_wrapper() {
    let lines = vec![
        "Sym_117 <- function(a, b, n) ".to_string(),
        "{".to_string(),
        "  sum <- 0".to_string(),
        "  i <- 1".to_string(),
        "  repeat {".to_string(),
        "if (!(i <= n)) break".to_string(),
        "sum <- (sum + (a[i] * b[i]))".to_string(),
        "i <- (i + 1)".to_string(),
        "next".to_string(),
        "  }".to_string(),
        "  return(sum)".to_string(),
        "}".to_string(),
    ];
    let out = collapse_trivial_dot_product_wrappers_ir(lines).join("\n");
    assert!(
        out.contains("return(sum((a[seq_len(n)] * b[seq_len(n)])))")
            || out.contains("return((sum((a[seq_len(n)] * b[seq_len(n)]))))"),
        "{out}"
    );
}

#[test]
fn collapse_singleton_assign_slice_scalar_edits_ir_rewrites_singleton_slice() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  x <- rr_assign_slice(x, i, i, 7)".to_string(),
        "  return(x)".to_string(),
        "}".to_string(),
    ];
    let out = collapse_singleton_assign_slice_scalar_edits_ir(lines).join("\n");
    assert!(out.contains("x <- replace(x, i, 7)"), "{out}");
}

#[test]
fn collapse_inlined_copy_vec_sequences_ir_rewrites_alias_swap() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  inlined_9_n <- length(temp)".to_string(),
        "  inlined_9_out <- rep.int(0, inlined_9_n)".to_string(),
        "  inlined_9_i <- 1".to_string(),
        "  inlined_9_out <- temp".to_string(),
        "  next_temp <- inlined_9_out".to_string(),
        "  repeat {".to_string(),
        "if (!(i < n)) break".to_string(),
        "next_temp[i] <- temp[i]".to_string(),
        "i <- (i + 1)".to_string(),
        "next".to_string(),
        "  }".to_string(),
        "  temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)".to_string(),
        "  return(temp)".to_string(),
        "}".to_string(),
    ];
    let out = collapse_inlined_copy_vec_sequences_ir(lines).join("\n");
    assert!(out.contains("next_temp <- temp"), "{out}");
    assert!(out.contains("temp <- next_temp"), "{out}");
}

#[test]
fn strip_unreachable_sym_helpers_ir_drops_unreachable_helper() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  return(Sym_10(\"x\", Sym_11(temp)))".to_string(),
        "}".to_string(),
        "Sym_top_0 <- function() ".to_string(),
        "{".to_string(),
        "  return(Sym_1())".to_string(),
        "}".to_string(),
        "Sym_7 <- function(xs) ".to_string(),
        "{".to_string(),
        "  return(xs)".to_string(),
        "}".to_string(),
        "Sym_11 <- function(xs) ".to_string(),
        "{".to_string(),
        "  return(sum(xs))".to_string(),
        "}".to_string(),
        "Sym_10 <- function(name, value) ".to_string(),
        "{".to_string(),
        "  print(name)".to_string(),
        "  return(value)".to_string(),
        "}".to_string(),
        "Sym_top_0()".to_string(),
    ];
    let out = strip_unreachable_sym_helpers_ir(lines).join("\n");
    assert!(out.contains("Sym_10 <- function"), "{out}");
    assert!(out.contains("Sym_11 <- function"), "{out}");
    assert!(!out.contains("Sym_7 <- function"), "{out}");
}

#[test]
fn strip_redundant_tail_assign_slice_return_ir_clears_tail_assign() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  i <- 1".to_string(),
        "  .tachyon_exprmap0_1 <- rr_map_int(x, f)".to_string(),
        "  repeat {".to_string(),
        "if (!(i <= n)) break".to_string(),
        "x <- rr_assign_slice(x, i, n, .tachyon_exprmap0_1)".to_string(),
        "next".to_string(),
        "  }".to_string(),
        "  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)".to_string(),
        "  return(x)".to_string(),
        "}".to_string(),
    ];
    let out = strip_redundant_tail_assign_slice_return_ir(lines).join("\n");
    assert!(
        !out.contains("\n  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)\n"),
        "{out}"
    );
    assert!(out.contains("return(x)"), "{out}");
}

#[test]
fn strip_redundant_nested_temp_reassigns_ir_drops_indented_duplicate_temp_assign() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  .__rr_cse_1 <- (x + y)".to_string(),
        "  if ((flag)) {".to_string(),
        "    .__rr_cse_1 <- (x + y)".to_string(),
        "  }".to_string(),
        "  return(.__rr_cse_1)".to_string(),
        "}".to_string(),
    ];
    let out = strip_redundant_nested_temp_reassigns_ir(lines).join("\n");
    assert_eq!(out.matches(".__rr_cse_1 <- (x + y)").count(), 1, "{out}");
}

#[test]
fn collapse_identical_if_else_tail_assignments_late_ir_hoists_shared_tail() {
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  if ((flag)) {".to_string(),
        "    x <- 1".to_string(),
        "    y <- z".to_string(),
        "  } else {".to_string(),
        "    y <- z".to_string(),
        "  }".to_string(),
        "}".to_string(),
    ];
    let out = collapse_identical_if_else_tail_assignments_late_ir(lines).join("\n");
    assert_eq!(out.matches("y <- z").count(), 1, "{out}");
}

#[test]
fn strip_redundant_identical_pure_rebinds_ir_drops_branch_local_duplicate() {
    let pure = FxHashSet::default();
    let lines = vec![
        "Sym_123 <- function(b, size) ".to_string(),
        "{".to_string(),
        "  x <- rep.int(0, size)".to_string(),
        "  if ((flag)) {".to_string(),
        "x <- (rep.int(0, size))".to_string(),
        "  } else {".to_string(),
        "  }".to_string(),
        "  return(x)".to_string(),
        "}".to_string(),
    ];
    let out = strip_redundant_identical_pure_rebinds_ir(lines, &pure).join("\n");
    assert!(out.contains("x <- rep.int(0, size)"));
    assert!(!out.contains("x <- (rep.int(0, size))"));
}

#[test]
fn rewrite_forward_exact_pure_call_reuse_ir_rewrites_nested_call() {
    let pure = FxHashSet::from_iter([String::from("rr_parallel_typed_vec_call")]);
    let lines = vec![
            "Sym_top_0 <- function() ".to_string(),
            "{".to_string(),
            "  probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(2, 1))".to_string(),
            "  probe_energy <- mean(abs(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(2, 1))))".to_string(),
            "  return(probe_energy)".to_string(),
            "}".to_string(),
        ];
    let out = rewrite_forward_exact_pure_call_reuse_ir(lines, &pure).join("\n");
    assert!(
        out.contains("probe_energy <- mean(abs(probe_vec))"),
        "{out}"
    );
}

#[test]
fn rewrite_forward_exact_pure_call_reuse_ir_stops_at_indexed_store_to_dep() {
    let pure = FxHashSet::from_iter([String::from("Sym_268")]);
    let lines = vec![
        "Sym_top_0 <- function() ".to_string(),
        "{".to_string(),
        "  adv_u2 <- Sym_268(u_stage, u_stage, adj_l, adj_r, adj_ll, adj_rr, TOTAL)".to_string(),
        "  u_stage[i] <- (u_stage[i] + 1.0)".to_string(),
        "  adv_u3 <- Sym_268(u_stage, u_stage, adj_l, adj_r, adj_ll, adj_rr, TOTAL)".to_string(),
        "  return(adv_u3)".to_string(),
        "}".to_string(),
    ];
    let out = rewrite_forward_exact_pure_call_reuse_ir(lines, &pure).join("\n");
    assert!(
        out.contains("adv_u3 <- Sym_268(u_stage, u_stage, adj_l, adj_r, adj_ll, adj_rr, TOTAL)"),
        "{out}"
    );
    assert!(!out.contains("adv_u3 <- adv_u2"), "{out}");
}

#[test]
fn rewrite_forward_exact_expr_reuse_ir_rewrites_branch_tail_use() {
    let lines = vec![
        "Sym_37 <- function(f, y, size) ".to_string(),
        "{".to_string(),
        "  .__rr_cse_11 <- (y / size)".to_string(),
        "  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)".to_string(),
        "  if ((f < 5)) {".to_string(),
        "    lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)".to_string(),
        "  }".to_string(),
        "}".to_string(),
    ];
    let out = rewrite_forward_exact_expr_reuse_ir(lines).join("\n");
    assert!(
        out.contains("lat <- (v * 45)")
            || out.contains("lat <- (((v) * 45))")
            || out.contains("lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)"),
        "{out}"
    );
}

#[test]
fn rewrite_forward_exact_pure_call_reuse_ir_does_not_turn_beta_reset_into_alpha() {
    let pure = FxHashSet::from_iter([String::from("Sym_117")]);
    let lines = vec![
        "Sym_1 <- function(b, size) ".to_string(),
        "{".to_string(),
        "  r <- b".to_string(),
        "  p <- r".to_string(),
        "  rs_old <- Sym_117(b, b, size)".to_string(),
        "  rs_new <- Sym_117(r, r, size)".to_string(),
        "  alpha <- (rs_old / p)".to_string(),
        "  beta <- (rs_new / rs_old)".to_string(),
        "  if (!(is.finite(beta))) {".to_string(),
        "    beta <- 0.0".to_string(),
        "  }".to_string(),
        "  p <- (r + (beta * p))".to_string(),
        "}".to_string(),
    ];
    let out = rewrite_forward_exact_pure_call_reuse_ir(lines, &pure).join("\n");
    assert!(!out.contains("beta <- alpha"), "{out}");
}

#[test]
fn rewrite_forward_exact_pure_call_reuse_ir_skips_cse_candidates() {
    let pure = FxHashSet::default();
    let lines = vec![
        "Sym_1 <- function() ".to_string(),
        "{".to_string(),
        "  .__rr_cse_8 <- (2.0 * v_m1)".to_string(),
        "  .__rr_cse_9 <- (v_m2 - .__rr_cse_8)".to_string(),
        "  .__rr_cse_10 <- ((v_m2 - .__rr_cse_8) + v_c)".to_string(),
        "}".to_string(),
    ];
    let out = rewrite_forward_exact_pure_call_reuse_ir(lines, &pure).join("\n");
    assert!(
        out.contains(".__rr_cse_10 <- ((v_m2 - .__rr_cse_8) + v_c)"),
        "{out}"
    );
    assert!(
        !out.contains(".__rr_cse_10 <- (.__rr_cse_9 + v_c)"),
        "{out}"
    );
}

#[test]
fn rewrite_forward_exact_expr_reuse_ir_keeps_larger_cse_temps_expanded() {
    let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  .__rr_cse_8 <- (2.0 * v_m1)".to_string(),
            "  .__rr_cse_9 <- (v_m2 - .__rr_cse_8)".to_string(),
            "  .__rr_cse_10 <- ((v_m2 - .__rr_cse_8) + v_c)".to_string(),
            "  .__rr_cse_19 <- (4.0 * v_m1)".to_string(),
            "  .__rr_cse_20 <- (v_m2 - .__rr_cse_19)".to_string(),
            "  .__rr_cse_22 <- (3.0 * v_c)".to_string(),
            "  .__rr_cse_23 <- ((v_m2 - .__rr_cse_19) + .__rr_cse_22)".to_string(),
            "  b1 <- (((1.0833 * ((v_m2 - .__rr_cse_8) + v_c)) * ((v_m2 - .__rr_cse_8) + v_c)) + ((0.25 * ((v_m2 - .__rr_cse_19) + .__rr_cse_22)) * ((v_m2 - .__rr_cse_19) + .__rr_cse_22)))".to_string(),
            "}".to_string(),
        ];
    let out = rewrite_forward_exact_expr_reuse_ir(lines).join("\n");
    assert!(
            out.contains("b1 <- (((1.0833 * (.__rr_cse_9 + v_c)) * (.__rr_cse_9 + v_c)) + ((0.25 * (.__rr_cse_20 + .__rr_cse_22)) * (.__rr_cse_20 + .__rr_cse_22)))"),
            "{out}"
        );
    assert!(out.contains("(.__rr_cse_9 + v_c)"), "{out}");
    assert!(out.contains("(.__rr_cse_20 + .__rr_cse_22)"), "{out}");
    assert!(!out.contains(".__rr_cse_10) * .__rr_cse_10"), "{out}");
    assert!(!out.contains(".__rr_cse_23) * .__rr_cse_23"), "{out}");
}
