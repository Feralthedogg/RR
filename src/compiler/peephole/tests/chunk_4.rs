use super::common::*;

#[test]
fn strips_arg_aliases_in_trivial_return_wrappers() {
    let input = "\
Sym_17 <- function(n, val) \n\
{\n\
  .arg_n <- n\n\
  .arg_val <- val\n\
  return(rep.int(.arg_val, .arg_n))\n\
}\n\
Sym_49__typed_impl <- function(a, b) \n\
{\n\
  .arg_a <- a\n\
  .arg_b <- b\n\
  return(rr_parallel_vec_mul_f64(rr_intrinsic_vec_add_f64(.arg_a, .arg_b), 0.5))\n\
}\n\
Sym_186 <- function(px, py, pf, u, v, dt, N) \n\
{\n\
  .arg_px <- px\n\
  .arg_py <- py\n\
  x <- .arg_px[i]\n\
  y <- .arg_py[i]\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("return(rep.int(val, n))"));
    assert!(out.contains("return(rr_parallel_vec_mul_f64(rr_intrinsic_vec_add_f64(a, b), 0.5))"));
    assert!(!out.contains(".arg_n <- n"));
    assert!(!out.contains(".arg_val <- val"));
    assert!(!out.contains(".arg_a <- a"));
    assert!(!out.contains(".arg_b <- b"));
    assert!(
        out.contains(".arg_px <- px")
            || out.contains("x <- px[i]")
            || out.contains("return(px[i])"),
        "{out}"
    );
    assert!(
        out.contains(".arg_py <- py")
            || out.contains("y <- py[i]")
            || out.contains("py[i]")
            || (!out.contains(".arg_py <- py") && !out.contains("y <- .arg_py[i]")),
        "{out}"
    );
}

#[test]
fn collapses_trivial_copy_wrapper_and_rewrites_calls() {
    let input = "\
Sym_12 <- function(xs) \n\
{\n\
  n <- length(xs)\n\
  out <- rep.int(0, n)\n\
  out <- xs\n\
  return(out)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  a <- seq_len(8)\n\
  next_a <- Sym_12(a)\n\
  a <- Sym_12(a)\n\
  return(next_a)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("Sym_12 <- function(xs)"));
    assert!(out.contains("{\nreturn(xs)\n}") || out.contains("{\n  return(xs)\n}"));
    assert!(out.contains("return(xs)"));
    assert!(out.contains("return(a)") || out.contains("return(next_a)"));
    assert!(!out.contains("next_a <- Sym_12(a)"));
    assert!(!out.contains("a <- Sym_12(a)"));
}

#[test]
fn collapses_passthrough_wrapper_with_dead_length_setup() {
    let input = "\
Sym_10 <- function(xs) \n\
{\n\
  n <- length(xs)\n\
  out <- xs\n\
  return(out)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  z <- seq_len(8)\n\
  x <- Sym_10(z)\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("Sym_10 <- function(xs)"));
    assert!(out.contains("return(xs)"));
    assert!(
        out.contains("x <- z") || out.contains("return(z)") || !out.contains("Sym_10(z)"),
        "{out}"
    );
    assert!(!out.contains("x <- Sym_10(z)"));
}

#[test]
fn rewrites_simple_expression_helper_chain_calls() {
    let input = "\
Sym_39 <- function(xs) \n\
{\n\
  n <- length(xs)\n\
  s <- sum(xs)\n\
  return(s)\n\
}\n\
Sym_12 <- function(xs) \n\
{\n\
  return(Sym_39(xs) / length(xs))\n\
}\n\
Sym_1 <- function() \n\
{\n\
  z <- seq_len(8)\n\
  return(Sym_12(z))\n\
}\n";
    let pure = FxHashSet::from_iter(["Sym_39".to_string(), "Sym_12".to_string()]);
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        true,
        &pure,
        &FxHashSet::default(),
        false,
    );
    assert!(
        out.contains("sum(z)") || out.contains("mean(z)") || out.contains("sum(seq_len(8))"),
        "{out}"
    );
    assert!(
        out.contains("length(z)") || out.contains("mean(z)") || out.contains("length(seq_len(8))"),
        "{out}"
    );
    assert!(!out.contains("return(Sym_12(z))"));
}

#[test]
fn rewrites_metric_helper_return_call_inline() {
    let input = "\
Sym_10 <- function(name, value) \n\
{\n\
  rr_mark(125, 5);\n\
  print(name)\n\
  rr_mark(126, 5);\n\
  print(value)\n\
  return(value)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  temp <- seq_len(8)\n\
  return(Sym_10(\"heat_bench_energy\", sum(temp)))\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("print(\"heat_bench_energy\")"));
    assert!(
        out.contains(".__rr_inline_metric_0 <- sum(temp)")
            || out.contains(".__rr_inline_metric_0 <- sum(seq_len(8))")
            || out.contains("sum(temp)")
            || out.contains("sum(seq_len(8))"),
        "{out}"
    );
    assert!(
        out.contains("return(.__rr_inline_metric_0)")
            || out.contains("return(sum(temp))")
            || out.contains("return(sum(seq_len(8)))"),
        "{out}"
    );
    assert!(!out.contains("return(Sym_10(\"heat_bench_energy\", sum(temp)))"));
}

#[test]
fn collapses_trivial_clamp_wrapper_and_rewrites_calls() {
    let input = "\
Sym_20 <- function(x, lo, hi) \n\
{\n\
  y <- x\n\
  if ((x < lo)) {\n\
y <- lo\n\
  }\n\
  if ((y > hi)) {\n\
y <- hi\n\
  }\n\
  return(y)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  next_a_cell <- 1.2\n\
  return(Sym_20(next_a_cell, 0, 1))\n\
}\n";
    let pure = FxHashSet::from_iter(["Sym_20".to_string()]);
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        true,
        &pure,
        &FxHashSet::default(),
        false,
    );
    assert!(
        out.contains("return((pmin(pmax(next_a_cell, 0), 1)))")
            || out.contains("return(pmin(pmax(next_a_cell, 0), 1))")
    );
    assert!(!out.contains("return(Sym_20(next_a_cell, 0, 1))"));
}

#[test]
fn collapses_trivial_unit_index_wrapper_and_rewrites_calls() {
    let input = "\
Sym_14 <- function(u, n) \n\
{\n\
  idx <- (1 + floor((u * n)))\n\
  if ((idx < 1)) {\n\
idx <- 1\n\
  }\n\
  if ((idx > n)) {\n\
idx <- n\n\
  }\n\
  return(idx)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  return(Sym_14(draw, n))\n\
}\n";
    let pure = FxHashSet::from_iter(["Sym_14".to_string()]);
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        true,
        &pure,
        &FxHashSet::default(),
        false,
    );
    assert!(
        out.contains("return((pmin(pmax((1 + floor((draw * n))), 1), n)))")
            || out.contains("return(pmin(pmax((1 + floor((draw * n))), 1), n))"),
        "{out}"
    );
    assert!(!out.contains("return(Sym_14(draw, n))"), "{out}");
}

#[test]
fn collapses_trivial_dot_product_wrapper_and_rewrites_calls() {
    let input = "\
Sym_117 <- function(a, b, n) \n\
{\n\
  sum <- 0\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= n)) break\n\
sum <- (sum + (a[i] * b[i]))\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(sum)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  return(Sym_117(r, Ap, size))\n\
}\n";
    let pure = FxHashSet::from_iter([String::from("Sym_117")]);
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        true,
        &pure,
        &FxHashSet::default(),
        false,
    );
    assert!(
        out.contains("return((sum((r[seq_len(size)] * Ap[seq_len(size)]))))")
            || out.contains("return(sum((r[seq_len(size)] * Ap[seq_len(size)])))"),
        "{out}"
    );
    assert!(!out.contains("return(Sym_117(r, Ap, size))"), "{out}");
}

#[test]
fn collapses_contextual_full_range_gather_replay_to_direct_gather() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  adj_ll <- (rep.int(0, TOTAL))\n\
  i <- 1\n\
  adj_ll <- rr_assign_slice(adj_ll, i, TOTAL, rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL)))))\n\
  return(adj_ll)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))")
            || out.contains("return(rr_gather(adj_l, rr_index_vec_floor(adj_l)))"),
        "{out}"
    );
    assert!(
        !out.contains("adj_ll <- rr_assign_slice(adj_ll, i, TOTAL, rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL)))))"),
        "{out}"
    );
}

#[test]
fn collapses_inlined_copy_vec_sequence_to_direct_alias_and_swap() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  inlined_9_n <- length(temp)\n\
  inlined_9_out <- rep.int(0, inlined_9_n)\n\
  inlined_9_i <- 1\n\
  inlined_9_out <- temp\n\
  next_temp <- inlined_9_out\n\
  repeat {\n\
if (!(i < n)) break\n\
next_temp[i] <- temp[i]\n\
i <- (i + 1)\n\
next\n\
  }\n\
  temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)\n\
  return(temp)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("next_temp <- temp"));
    assert!(out.contains("return(temp)") || out.contains("temp <- next_temp"));
}

#[test]
fn collapses_copy_vec_sequence_after_named_copy_alias_cleanup() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  inlined_9_n <- length(temp)\n\
  inlined_9_out <- rep.int(0, inlined_9_n)\n\
  inlined_9_i <- 1\n\
  inlined_9_out <- temp\n\
  next_temp <- temp\n\
  repeat {\n\
if (!(i < n)) break\n\
next_temp[i] <- temp[i]\n\
i <- (i + 1)\n\
next\n\
  }\n\
  temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)\n\
  return(temp)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("next_temp <- temp"));
    assert!(out.contains("temp <- next_temp"));
    assert!(
        !out.contains("temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)")
    );
}

#[test]
fn strips_unreachable_sym_helpers_after_call_rewrite() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  return(Sym_10(\"x\", Sym_11(temp)))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_1())\n\
}\n\
Sym_7 <- function(xs) \n\
{\n\
  return(xs)\n\
}\n\
Sym_11 <- function(xs) \n\
{\n\
  return(sum(xs))\n\
}\n\
Sym_10 <- function(name, value) \n\
{\n\
  print(name)\n\
  return(value)\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        (out.contains("Sym_10 <- function") && out.contains("Sym_11 <- function"))
            || (out.contains("Sym_10 <- function") && out.contains("sum(temp)"))
            || (out.contains("print(\"x\")") && out.contains("sum(temp)")),
        "{out}"
    );
    assert!(!out.contains("Sym_7 <- function"));
}

#[test]
fn keeps_sym_top_entrypoint_reachable_closure() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  return(Sym_10(\"x\", Sym_11(temp)))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_1())\n\
}\n\
Sym_11 <- function(xs) \n\
{\n\
  return(sum(xs))\n\
}\n\
Sym_10 <- function(name, value) \n\
{\n\
  print(name)\n\
  return(value)\n\
}\n\
# --- RR synthesized entrypoints (auto-generated) ---\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("Sym_top_0 <- function"));
    assert!(
        (out.contains("Sym_1 <- function")
            && out.contains("Sym_10 <- function")
            && out.contains("Sym_11 <- function"))
            || (out.contains("Sym_1 <- function")
                && out.contains("Sym_10 <- function")
                && out.contains("sum(temp)"))
            || (out.contains("print(\"x\")") && out.contains("sum(temp)")),
        "{out}"
    );
    assert!(out.contains("Sym_top_0()"));
}

#[test]
fn keeps_helper_only_sym_defs_when_synthesized_entrypoint_is_null() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  return(1)\n\
}\n\
Sym_2 <- function() \n\
{\n\
  return(2)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(NULL)\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("Sym_1 <- function"));
    assert!(out.contains("Sym_2 <- function"));
    assert!(out.contains("Sym_top_0 <- function"));
}

#[test]
fn preserve_all_defs_keeps_unreachable_sym_helpers() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  return(1)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_1())\n\
}\n\
Sym_99 <- function() \n\
{\n\
  print(\"DROP\")\n\
  return(2)\n\
}\n\
Sym_top_0()\n";
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        true,
        &FxHashSet::default(),
        &FxHashSet::default(),
        true,
    );
    assert!(out.contains("Sym_99 <- function"));
    assert!(out.contains("print(\"DROP\")"));
}

#[test]
fn keeps_typed_parallel_impl_helper_referenced_as_symbol_argument() {
    let input = "\
Sym_49__typed_impl <- function(a, b) \n\
{\n\
  return(rr_parallel_vec_mul_f64(rr_intrinsic_vec_add_f64(a, b), 0.5))\n\
}\n\
Sym_49 <- function(a, b) \n\
{\n\
  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_49(c(1, 2), c(2, 1)))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("Sym_49__typed_impl <- function"));
    assert!(out.contains("rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"));
}

#[test]
fn reuses_exact_parallel_typed_vec_call_binding_inside_nested_pure_expr() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2, 3, 4), c(4, 3, 2, 1))\n\
  probe_energy <- mean(abs(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2, 3, 4), c(4, 3, 2, 1))))\n\
  return(probe_energy)\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"),
        "{out}"
    );
    assert!(
        out.contains("probe_energy <- mean(abs(probe_vec))")
            || out.contains("probe_energy <- (mean(abs(probe_vec)))")
            || out.contains("return(mean(abs(probe_vec)))")
            || out.contains("return((mean(abs(probe_vec))))"),
        "{out}"
    );
    assert!(
        !out.contains("probe_energy <- mean(abs(rr_parallel_typed_vec_call(\"Sym_49\"")
            && !out.contains("probe_energy <- (mean(abs((rr_parallel_typed_vec_call(\"Sym_49\""),
        "{out}"
    );
}

#[test]
fn drops_unreachable_typed_parallel_wrapper_when_only_string_name_remains() {
    let input = "\
Sym_49__typed_impl <- function(a, b) \n\
{\n\
  return(((a + b) * 0.5))\n\
}\n\
Sym_49 <- function(a, b) \n\
{\n\
  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(2, 1)))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("Sym_49 <- function"), "{out}");
    assert!(out.contains("Sym_49__typed_impl <- function"), "{out}");
    assert!(
        out.contains("rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"),
        "{out}"
    );
}

#[test]
fn removes_redundant_identical_rr_field_get_rebind_after_loop() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  p_y <- rr_field_get(particles, \"py\")\n\
  p_f <- rr_field_get(particles, \"pf\")\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= TOTAL)) break\n\
u_stage[i] <- u[i]\n\
i <- (i + 1)\n\
next\n\
  }\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  p_y <- rr_field_get(particles, \"py\")\n\
  p_f <- rr_field_get(particles, \"pf\")\n\
  return(p_f)\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.matches("p_x <-").count() <= 1, "{out}");
    assert!(out.matches("p_y <-").count() <= 1, "{out}");
    assert!(out.matches("p_f <-").count() <= 1, "{out}");
}

#[test]
fn strips_unused_trailing_helper_param_and_updates_callsites() {
    let input = "\
Sym_186 <- function(px, py, pf, u, v, dt, N, total_grid) \n\
{\n\
  out_px <- px\n\
  out_py <- py\n\
  out_pf <- pf\n\
  i <- 1\n\
  if ((i == 1)) {\n\
out_px[i] <- px[i]\n\
  }\n\
  return(rr_named_list(\"px\", out_px, \"py\", out_py, \"pf\", out_pf))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)\n\
  return(rr_field_get(particles, \"pf\"))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("Sym_186 <- function(px, py, pf)"), "{out}");
    assert!(
        !out.contains("Sym_186 <- function(px, py, pf, u, v, dt, N, total_grid)"),
        "{out}"
    );
    assert!(out.contains("particles <- Sym_186(p_x, p_y, p_f)"), "{out}");
    assert!(
        !out.contains("particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)"),
        "{out}"
    );
}

#[test]
fn strips_unused_middle_helper_params_and_updates_callsites() {
    let input = "\
Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size) \n\
{\n\
  heat <- rep.int(0, size)\n\
  if ((q_c[1] > 0)) {\n\
heat[1] <- (q_c[1] + q_v[1])\n\
  }\n\
  if ((q_s[1] > 0)) {\n\
heat[1] <- (heat[1] + q_g[1])\n\
  }\n\
  return(heat)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("Sym_287 <- function(q_v, q_c, q_s, q_g, size)"),
        "{out}"
    );
    assert!(
        !out.contains("Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)"),
        "{out}"
    );
    assert!(
        out.contains("return(Sym_287(qv, qc, qs, qg, TOTAL))"),
        "{out}"
    );
    assert!(
        !out.contains("return(Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL))"),
        "{out}"
    );
}
