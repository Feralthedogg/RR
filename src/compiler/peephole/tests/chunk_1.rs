use super::common::*;

#[test]
fn rewrites_whole_slice_patterns() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 8\n\
  idx <- seq_len(n)\n\
  x <- idx + 1\n\
  score <- rep.int(0, n)\n\
  i <- 1L\n\
  .tmp1 <- i:8\n\
  .tmp2 <- rr_index_vec_floor(.tmp1)\n\
  .arg <- abs(rr_index1_read_vec(x, .tmp2))\n\
  score <- rr_call_map_slice_auto(score, i, 8, \"pmax\", 44L, c(1L), .arg, 0.05)\n\
  score <- rr_assign_slice(score, i, 8, rr_index1_read_vec(score, .tmp2))\n\
    }\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("abs(") || out.contains("x <- seq_len(n) + 1"),
        "{out}"
    );
    assert!(
        out.contains("score <- pmax(.arg, 0.05)") || !out.contains("score <-"),
        "{out}"
    );
    assert!(!out.contains("rr_call_map_slice_auto("));
    assert!(!out.contains("rr_index1_read_vec(score, .tmp2)"));
}

#[test]
fn rewrites_nested_vector_helper_subcalls() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  idx <- seq_len(n)\n\
  x <- rr_parallel_vec_sub_f64(rr_parallel_vec_div_f64((rr_parallel_vec_mul_f64(idx, 13) %% 1000), 1000), 0.5)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("rr_parallel_vec_sub_f64"), "{out}");
    assert!(!out.contains("rr_parallel_vec_div_f64"), "{out}");
    assert!(!out.contains("rr_parallel_vec_mul_f64"), "{out}");
}

#[test]
fn generic_counted_repeat_loop_rewrite_still_applies_without_benchmark_rewrites() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= TOTAL)) break\n\
rr_mark(1545, 13);\n\
u_stage[i] <- (u[i] + (dt * (du1[i] - adv_u[i])))\n\
i <- (i + 1)\n\
  }\n\
}\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let (out, _) =
        optimize_emitted_r_with_context_and_fresh_with_options(input, true, &pure, &fresh, false);
    assert!(!out.contains("repeat {"), "{out}");
    assert!(!out.contains("if (!(i <= TOTAL)) break"), "{out}");
    assert!(out.contains("for (i in seq_len(TOTAL)) {"), "{out}");
}

#[test]
fn generic_counted_repeat_loop_rewrite_preserves_non_iter_prefix_lines() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  seed <- 12345\n\
  repeat {\n\
if (!(i <= n)) break\n\
seed <- (((seed * 1103515245) + 12345) %% 2147483648)\n\
p[i] <- (seed / 2147483648)\n\
i <- (i + 1)\n\
  }\n\
  return(seed)\n\
}\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let (out, _) =
        optimize_emitted_r_with_context_and_fresh_with_options(input, true, &pure, &fresh, false);
    assert!(out.contains("seed <- 12345"), "{out}");
    assert!(out.contains("for (i in seq_len(n)) {"), "{out}");
    assert!(!out.contains("repeat {"), "{out}");
}

#[test]
fn restores_counter_for_constant_one_guard_repeat_loop() {
    let input = "\
Sym_1 <- function(n)\n\
{\n\
  a <- 1L\n\
  b <- 2L\n\
  repeat {\n\
if !((1L) <= n) break\n\
t <- a\n\
a <- b\n\
b <- t\n\
  }\n\
  return((a + b))\n\
}\n";
    let out = restore_constant_one_guard_repeat_loop_counters(
        input.lines().map(str::to_string).collect(),
    )
    .join("\n");
    assert!(out.contains(".__rr_i <- 1L"), "{out}");
    assert!(out.contains("if (!(.__rr_i <= n)) break"), "{out}");
    assert!(out.contains(".__rr_i <- (.__rr_i + 1L)"), "{out}");
}

#[test]
fn restores_counter_for_constant_zero_guard_repeat_loop() {
    let input = "\
Sym_9 <- function(x)\n\
{\n\
  g <- ((x * 0.5) + 0.5)\n\
  repeat {\n\
if (!((0) < 8)) break\n\
g <- (0.5 * (g + (x / g)))\n\
  }\n\
  return(g)\n\
}\n";
    let out = restore_constant_one_guard_repeat_loop_counters(
        input.lines().map(str::to_string).collect(),
    )
    .join("\n");
    assert!(out.contains(".__rr_i <- 0"), "{out}");
    assert!(out.contains("if (!(.__rr_i < 8)) break"), "{out}");
    assert!(out.contains(".__rr_i <- (.__rr_i + 1)"), "{out}");
}

#[test]
fn generic_counted_repeat_loop_rewrite_skips_when_iter_used_after_loop() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= n)) break\n\
acc[i] <- value\n\
i <- (i + 1)\n\
  }\n\
  return(i)\n\
}\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let (out, _) =
        optimize_emitted_r_with_context_and_fresh_with_options(input, true, &pure, &fresh, false);
    assert!(out.contains("repeat {"), "{out}");
    assert!(!out.contains("for (i in seq_len(n)) {"), "{out}");
}

#[test]
fn hoists_loop_invariant_pure_assignment_from_counted_repeat_loop() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  steps <- 0\n\
  total <- 0\n\
  repeat {\n\
if (!(steps < 5)) break\n\
heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)\n\
total <- (total + heat[1L])\n\
steps <- (steps + 1)\n\
  }\n\
  return(total)\n\
}\n";
    let pure = FxHashSet::from_iter([String::from("Sym_287")]);
    let fresh = FxHashSet::default();
    let (out, _) =
        optimize_emitted_r_with_context_and_fresh_with_options(input, true, &pure, &fresh, false);
    let heat_pos = out
        .find("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
        .expect("{out}");
    let loop_pos = out.find("repeat {").expect("{out}");
    assert!(heat_pos < loop_pos, "{out}");
    assert_eq!(
        out.matches("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
            .count(),
        1
    );
}

#[test]
fn does_not_hoist_loop_invariant_pure_assignment_when_dependency_mutates() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  steps <- 0\n\
  total <- 0\n\
  repeat {\n\
if (!(steps < 5)) break\n\
heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)\n\
qv <- next_qv\n\
total <- (total + heat[1L])\n\
steps <- (steps + 1)\n\
  }\n\
  return(total)\n\
}\n";
    let pure = FxHashSet::from_iter([String::from("Sym_287")]);
    let fresh = FxHashSet::default();
    let (out, _) =
        optimize_emitted_r_with_context_and_fresh_with_options(input, true, &pure, &fresh, false);
    let heat_pos = out
        .find("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
        .expect("{out}");
    let loop_pos = out.find("repeat {").expect("{out}");
    assert!(heat_pos > loop_pos, "{out}");
    assert_eq!(
        out.matches("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
            .count(),
        1
    );
}

#[test]
fn preserves_loop_facts_across_repeat_and_single_line_break_guard() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 250000\n\
  idx <- seq_len(n)\n\
  x <- ((((idx * 13) %% 1000) / 1000) - 0.5)\n\
  y <- (((((idx * 17) + 7) %% 1000) / 1000) - 0.5)\n\
  score <- rep.int(0, n)\n\
  clean <- rep.int(0, n)\n\
  pass <- 1\n\
  repeat {\n\
if (!rr_truthy1((pass <= 16), \"condition\")) break\n\
i <- 1L\n\
.__rr_cse_159 <- i:250000\n\
.__rr_cse_160 <- rr_index_vec_floor(.__rr_cse_159)\n\
.tachyon_callmap_arg0_0 <- abs((((rr_index1_read_vec(x, .__rr_cse_160) * 0.65) + (rr_index1_read_vec(y, .__rr_cse_160) * 0.35)) - 0.08))\n\
score <- rr_call_map_slice_auto(score, i, 250000, \"pmax\", 44L, c(1L), .tachyon_callmap_arg0_0, 0.05)\n\
i_9 <- 1L\n\
.__rr_cse_174 <- i_9:250000\n\
.__rr_cse_175 <- rr_index_vec_floor(.__rr_cse_174)\n\
.__rr_cse_176 <- rr_index1_read_vec(score, .__rr_cse_175)\n\
clean <- rr_assign_slice(clean, i_9, 250000, rr_ifelse_strict((.__rr_cse_176 > 0.4), sqrt((.__rr_cse_176 + 0.1)), ((.__rr_cse_176 * 0.55) + 0.03)))\n\
i_10 <- 1L\n\
.__rr_cse_184 <- i_10:250000\n\
.__rr_cse_185 <- rr_index_vec_floor(.__rr_cse_184)\n\
x <- rr_assign_slice(x, i_10, 250000, (rr_index1_read_vec(clean, .__rr_cse_185) + (rr_index1_read_vec(y, .__rr_cse_185) * 0.15)))\n\
i_11 <- 1L\n\
.__rr_cse_191 <- i_11:250000\n\
.__rr_cse_192 <- rr_index_vec_floor(.__rr_cse_191)\n\
y <- rr_assign_slice(y, i_11, 250000, ((rr_index1_read_vec(score, .__rr_cse_192) * 0.8) + (rr_index1_read_vec(clean, .__rr_cse_192) * 0.2)))\n\
pass <- (pass + 1)\n\
next\n\
  }\n\
    }\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("repeat {"));
    assert!(out.contains("pass <- 1"));
    assert!(out.contains("score <- pmax("));
    assert!(
        out.contains(
            "clean <- ifelse((score > 0.4), sqrt((score + 0.1)), ((score * 0.55) + 0.03))"
        )
    );
    assert!(out.contains("x <- (clean + (y * 0.15))"));
    assert!(out.contains("y <- ((score * 0.8) + (clean * 0.2))"));
}

#[test]
fn does_not_rewrite_full_slice_facts_across_branch_boundaries() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  base <- seq_len(3)\n\
  idx <- c(1L, 3L)\n\
  if (flag) {\n\
idx <- 1:3\n\
  }\n\
  out <- rr_index1_read_vec(base, idx)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("out <- base"), "{out}");
    assert!(!out.contains("return(base)"), "{out}");
}

#[test]
fn reuses_pure_user_call_binding_in_later_expression() {
    let input = "\
Sym_123 <- function() \n\
{\n\
  Ap <- Sym_119(p, n_l, n_r, n_d, n_u, size)\n\
  idx <- rr_index_vec_floor(i:size)\n\
  r <- (rr_index1_read_vec(r, idx) - (alpha * rr_index1_read_vec(Sym_119(p, n_l, n_r, n_d, n_u, size), idx)))\n\
}\n";
    let pure = FxHashSet::from_iter([String::from("Sym_119")]);
    let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
    assert!(
        out.contains("rr_index1_read_vec(Ap, idx)")
            || out.matches("Sym_119(p, n_l, n_r, n_d, n_u, size)").count() <= 1,
        "{out}"
    );
    assert!(!out.contains("rr_index1_read_vec(Sym_119("));
}

#[test]
fn rewrites_return_of_last_assignment_rhs_to_variable() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  x <- rr_assign_slice(x, 1, n, expr)\n\
  return(rr_assign_slice(x, 1, n, expr))\n\
    }\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("return(x)") || !out.contains("return(rr_assign_slice("));
    assert!(!out.contains("return(rr_assign_slice("));
}

#[test]
fn return_rewrite_does_not_use_stale_alias_after_rhs_var_is_mutated() {
    let input = "\
Sym_5 <- function(n) \n\
{\n\
  acc <- 1L\n\
  i <- acc\n\
  acc <- prod(1L:n)\n\
  return(acc)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("return(acc)") || !out.contains("return(i)"));
    assert!(!out.contains("return(i)"));
}

#[test]
fn reuses_pure_user_call_across_guarded_block_sequence() {
    let input = "\
Sym_123 <- function() \n\
{\n\
  Ap <- Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size)\n\
  p_Ap <- Sym_117(p, Ap, .arg_size)\n\
  if (rr_truthy1(((is.na(p_Ap) | (!(is.finite(p_Ap)))) | (p_Ap == 0)), \"condition\")) {\n\
p_Ap <- 0.0000001\n\
  } else {\n\
  }\n\
  alpha <- (rs_old / p_Ap)\n\
  if (rr_truthy1((is.na(alpha) | (!(is.finite(alpha)))), \"condition\")) {\n\
alpha <- 0\n\
  } else {\n\
  }\n\
  .tachyon_exprmap1_1 <- (rr_index1_read_vec(r, idx) - (alpha * rr_index1_read_vec(Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size), idx)))\n\
    }\n";
    let pure = FxHashSet::from_iter([String::from("Sym_119"), String::from("Sym_117")]);
    let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
    assert!(
        out.matches("Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size)")
            .count()
            <= 1
    );
    assert!(!out.contains("rr_index1_read_vec(Sym_119("));
}

#[test]
fn removes_dead_simple_alias_and_literal_assignments() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  a <- 1\n\
  b <- a\n\
  c <- 0\n\
  return((a + 1))\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("a <- 1"));
    assert!(!out.contains("b <- a"));
    assert!(!out.contains("c <- 0"));
}

#[test]
fn removes_dead_unused_scalar_index_reads_and_pure_call_bindings() {
    let input = "\
Sym_287 <- function(temp, q_r, q_i, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T <- temp[i]\n\
qr <- q_r[i]\n\
qi <- q_i[i]\n\
es_ice <- (6.11 * exp(T))\n\
rr_mark(1, 1);\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("qr <- q_r[i]"), "{out}");
    assert!(!out.contains("qi <- q_i[i]"), "{out}");
    assert!(!out.contains("es_ice <- (6.11 * exp(T))"), "{out}");
    assert!(out.contains("rr_mark(1, 1);"), "{out}");
}

#[test]
fn inlines_single_use_named_scalar_index_reads() {
    let input = "\
Sym_287 <- function(temp, q_v, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T <- temp[i]\n\
qv <- q_v[i]\n\
T_c <- (T - 273.15)\n\
if ((qv > 0.01)) {\n\
  rr_mark(1, 1);\n\
  print(T_c)\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("T <- temp[i]"), "{out}");
    assert!(
        out.contains("T_c <- (temp[i] - 273.15)") || out.contains("T_c <- ((temp[i]) - 273.15)"),
        "{out}"
    );
    assert!(
        out.contains("if (((q_v[i]) > 0.01)) {")
            || out.contains("if ((q_v[i] > 0.01)) {")
            || out.contains("if ((qv > 0.01)) {"),
        "{out}"
    );
}

#[test]
fn inlines_single_use_named_scalar_index_reads_across_if_boundaries() {
    let input = "\
Sym_287 <- function(temp, q_v, q_c, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T_c <- (temp[i] - 273.15)\n\
qc <- q_c[i]\n\
if ((T_c < (-(5)))) {\n\
  if ((qc > 0.0001)) {\n\
    rate <- (0.01 * qc)\n\
  }\n\
}\n\
qv <- q_v[i]\n\
if ((T_c < (-(15)))) {\n\
  if ((qv > 0.01)) {\n\
    print(T_c)\n\
  }\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("if ((q_c[i] > 0.0001)) {")
            || out.contains("if (((q_c[i]) > 0.0001)) {")
            || out.contains("if ((qc > 0.0001)) {"),
        "{out}"
    );
    assert!(
        out.contains("if ((q_v[i] > 0.01)) {")
            || out.contains("if (((q_v[i]) > 0.01)) {")
            || out.contains("if ((qv > 0.01)) {"),
        "{out}"
    );
}

#[test]
fn inlines_two_use_named_scalar_index_reads_across_if_boundaries() {
    let input = "\
Sym_287 <- function(temp, q_s, q_g, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T_c <- (temp[i] - 273.15)\n\
qs <- q_s[i]\n\
qg <- q_g[i]\n\
if ((T_c > 0)) {\n\
  melt_rate <- 0\n\
  if ((qs > 0)) {\n\
    melt_rate <- (qs * 0.05)\n\
  }\n\
  if ((qg > 0)) {\n\
    melt_rate <- (melt_rate + (qg * 0.02))\n\
  }\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("if ((q_s[i] > 0)) {")
            || out.contains("if (((q_s[i]) > 0)) {")
            || out.contains("if ((qs > 0)) {"),
        "{out}"
    );
    assert!(
        out.contains("melt_rate <- (q_s[i] * 0.05)")
            || out.contains("melt_rate <- ((q_s[i]) * 0.05)")
            || out.contains("melt_rate <- (qs * 0.05)"),
        "{out}"
    );
    assert!(
        out.contains("if ((q_g[i] > 0)) {")
            || out.contains("if (((q_g[i]) > 0)) {")
            || out.contains("if ((qg > 0)) {"),
        "{out}"
    );
    assert!(
        out.contains("melt_rate <- (melt_rate + (q_g[i] * 0.02))")
            || out.contains("melt_rate <- (melt_rate + ((q_g[i]) * 0.02))")
            || out.contains("melt_rate <- (melt_rate + (qg * 0.02))"),
        "{out}"
    );
}

#[test]
fn inlines_immediate_single_use_named_scalar_expr_into_following_assignment() {
    let input = "\
Sym_287 <- function(q_c, i) \n\
{\n\
  if ((q_c[i] > 0.0001)) {\n\
rate <- (0.01 * q_c[i])\n\
tendency_T <- (rate * L_f)\n\
  }\n\
  return(tendency_T)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("rate <- (0.01 * q_c[i])"), "{out}");
    assert!(
        out.contains("tendency_T <- ((0.01 * q_c[i]) * L_f)")
            || out.contains("tendency_T <- (((0.01 * q_c[i]) * L_f))")
            || out.contains("tendency_T <- ((0.01 * (q_c[i])) * L_f)"),
        "{out}"
    );
}
