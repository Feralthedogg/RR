//! Regression tests for late peephole cleanup and control-flow rewrites.
use super::*;

#[test]
pub(crate) fn strips_empty_else_blocks() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  if (flag) {\n\
x <- 1\n\
  } else {\n\
  }\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("} else {\n}"));
    assert!(!out.contains("} else {\n  }"));
    assert!(out.contains("if (flag) {"));
    assert!(out.contains("x <- 1"));
}

#[test]
pub(crate) fn collapses_common_if_else_tail_assignments() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  if ((f == 5)) {\n\
lat <- 1\n\
arg_f <- f\n\
  } else {\n\
arg_f <- f\n\
  }\n\
  if ((arg_f == 6)) {\n\
lat <- 2\n\
  }\n\
    }\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.matches("arg_f <- f").count() <= 1);
}

#[test]
pub(crate) fn collapses_common_if_else_tail_assignment_sequences() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  if (flag) {\n\
x <- 1\n\
a <- foo\n\
b <- bar\n\
  } else {\n\
a <- foo\n\
b <- bar\n\
  }\n\
  return(b)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.matches("a <- foo").count() <= 1);
    assert!(out.matches("b <- bar").count() <= 1);
    assert!(out.contains("return(bar)") || out.contains("b <- bar"));
}

#[test]
pub(crate) fn collapses_common_if_else_tail_assignments_for_sym287_shape() {
    let input = "\
Sym_287 <- function() \n\
{\n\
  if ((T_c < (-(5)))) {\n\
if ((qc > 0.0001)) {\n\
  rate <- (0.01 * qc)\n\
  tendency_T <- (rate * L_f)\n\
}\n\
.__pc_src_tmp0 <- (temp[i] - 273.15)\n\
.__pc_src_tmp1 <- temp[i]\n\
.__pc_src_tmp2 <- q_v[i]\n\
.__pc_src_tmp3 <- q_s[i]\n\
.__pc_src_tmp4 <- q_g[i]\n\
T_c <- .__pc_src_tmp0\n\
T <- .__pc_src_tmp1\n\
qs <- .__pc_src_tmp3\n\
qg <- .__pc_src_tmp4\n\
  } else {\n\
.__pc_src_tmp0 <- (temp[i] - 273.15)\n\
.__pc_src_tmp1 <- temp[i]\n\
.__pc_src_tmp2 <- q_v[i]\n\
.__pc_src_tmp3 <- q_s[i]\n\
.__pc_src_tmp4 <- q_g[i]\n\
T_c <- .__pc_src_tmp0\n\
T <- .__pc_src_tmp1\n\
qs <- .__pc_src_tmp3\n\
qg <- .__pc_src_tmp4\n\
  }\n\
  return(qg)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.matches(".__pc_src_tmp0 <- (temp[i] - 273.15)").count() <= 1);
    assert!(out.matches(".__pc_src_tmp1 <- temp[i]").count() <= 1);
    assert!(out.matches(".__pc_src_tmp2 <- q_v[i]").count() <= 1);
    assert!(out.matches(".__pc_src_tmp3 <- q_s[i]").count() <= 1);
    assert!(out.matches(".__pc_src_tmp4 <- q_g[i]").count() <= 1);
    assert!(out.matches("T_c <- .__pc_src_tmp0").count() <= 1);
    assert!(out.matches("T <- .__pc_src_tmp1").count() <= 1);
    assert!(out.matches("qs <- .__pc_src_tmp3").count() <= 1);
    assert!(out.matches("qg <- .__pc_src_tmp4").count() <= 1);
    assert!(
        out.contains("return(qg)") || out.contains("return(q_g[i])"),
        "{out}"
    );
}

#[test]
pub(crate) fn rewrites_if_truthy_scalar_guards() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  f <- 5\n\
  if (rr_truthy1((f == 5), \"condition\")) {\n\
x <- 1\n\
  }\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("if ((f == 5)) {"));
    assert!(!out.contains("if (rr_truthy1("));
}

#[test]
pub(crate) fn forwards_simple_alias_into_following_guards_only() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  arg_f <- f_curr\n\
  if ((arg_f == 6)) {\n\
lat <- 1\n\
  }\n\
  if ((arg_f < 5)) {\n\
lat <- 2\n\
  }\n\
  return(lat)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("arg_f <- f_curr"));
    assert!(out.contains("if ((f_curr == 6)) {"));
    assert!(
        out.contains("if ((f_curr < 5)) {") || out.contains("if ((f_curr <= 4)) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn removes_dead_pure_helper_call_assignment() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  rot_l <- Sym_91(1, N)\n\
  used <- 1\n\
  return(used)\n\
}\n";
    let pure = FxHashSet::from_iter([String::from("Sym_91")]);
    let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
    assert!(!out.contains("rot_l <- Sym_91(1, N)"));
    assert!(out.contains("used <- 1"));
}

#[test]
pub(crate) fn removes_simple_init_overwritten_before_first_read() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  k0 <- 0\n\
  rem <- 0\n\
  k0 <- (k - 1)\n\
  rem <- (k0 %% grid_sq)\n\
  return(rem)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("k0 <- 0"));
    assert!(!out.contains("rem <- 0"));
    assert!(
        out.contains("k0 <- (k - 1)") || out.contains("return(((k - 1) %% grid_sq))"),
        "{out}"
    );
    assert!(
        out.contains("rem <- (k0 %% grid_sq)") || out.contains("return(((k - 1) %% grid_sq))"),
        "{out}"
    );
}

#[test]
pub(crate) fn indexed_write_invalidates_stale_return_rhs_rewrite() {
    let input = "\
Sym_183 <- function(n) \n\
{\n\
  p <- seq_len(n)\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= n)) break\n\
p[i] <- (seed / 2147483648)\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(p)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("return(p)"));
    assert!(!out.contains("return(seq_len(n))"));
}

#[test]
pub(crate) fn nested_field_write_invalidates_alias_before_later_helper_call() {
    let input = "\
Sym_1 <- function(state) \n\
{\n\
  marked <- state\n\
  marked[[\"marks\"]] <- c(marked[[\"marks\"]], marked[[\"used\"]])\n\
  out <- rr_field_set(NULL, \"state\", marked)\n\
  out <- rr_field_set(out, \"mark\", marked[[\"used\"]])\n\
  return(out)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("out <- rr_field_set(NULL, \"state\", marked)"),
        "{out}"
    );
    assert!(
        !out.contains("out <- rr_field_set(NULL, \"state\", state)"),
        "{out}"
    );
}

#[test]
pub(crate) fn removes_branch_local_init_overwritten_before_first_read_in_loop() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  dist <- 0\n\
  repeat {\n\
if (!(k <= n)) break\n\
if ((flag == 1)) {\n\
  dist <- ((dx * dx) + (dy * dy))\n\
  out[k] <- dist\n\
}\n\
k <- (k + 1)\n\
next\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("dist <- 0"));
    assert!(
        out.contains("dist <- ((dx * dx) + (dy * dy))")
            || out.contains("out[k] <- ((dx * dx) + (dy * dy))"),
        "{out}"
    );
    assert!(
        out.contains("out[k] <- dist") || out.contains("out[k] <- ((dx * dx) + (dy * dy))"),
        "{out}"
    );
}

#[test]
pub(crate) fn keeps_loop_accumulator_init_used_after_inner_repeat() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  sum1 <- 0\n\
  count1 <- 0\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= n)) break\n\
if ((flag == 1)) {\n\
  sum1 <- (sum1 + x)\n\
  count1 <- (count1 + 1)\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  if ((count1 > 0)) {\n\
out <- (sum1 / count1)\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("sum1 <- 0"));
    assert!(out.contains("count1 <- 0"));
    assert!(
        out.contains("out <- (sum1 / count1)")
            || out.contains("return((sum1 / count1))")
            || out.contains("if ((count1 > 0)) {\n}"),
        "{out}"
    );
}

#[test]
pub(crate) fn removes_redundant_tail_assign_slice_after_non_empty_repeat() {
    let input = "\
Sym_123 <- function() \n\
{\n\
  x <- rep.int(0, n)\n\
  iter <- 1\n\
  repeat {\n\
if (!(iter <= 20)) break\n\
i <- 1\n\
.tachyon_exprmap0_1 <- expr\n\
x <- rr_assign_slice(x, i, n, expr)\n\
iter <- (iter + 1)\n\
next\n\
  }\n\
  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)"));
    assert!(out.contains("return(x)"));
}

#[test]
pub(crate) fn removes_redundant_tail_assign_slice_for_sym123_shape() {
    let input = "\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
if (!(iter <= 20)) break\n\
Ap <- Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size)\n\
p_Ap <- Sym_117(p, Ap, .arg_size)\n\
alpha <- (rs_old / p_Ap)\n\
i <- 1\n\
.tachyon_exprmap0_1 <- (rr_index1_read_vec(x, rr_index_vec_floor(i:.arg_size)) + (alpha * rr_index1_read_vec(p, rr_index_vec_floor(i:.arg_size))))\n\
x <- rr_assign_slice(x, i, .arg_size, (rr_index1_read_vec(x, rr_index_vec_floor(i:.arg_size)) + (alpha * rr_index1_read_vec(p, rr_index_vec_floor(i:.arg_size)))))\n\
r <- rr_assign_slice(r, i, .arg_size, (rr_index1_read_vec(r, rr_index_vec_floor(i:.arg_size)) - (alpha * rr_index1_read_vec(Ap, rr_index_vec_floor(i:.arg_size)))))\n\
rs_new <- Sym_117(r, r, .arg_size)\n\
beta <- (rs_new / rs_old)\n\
i <- 1\n\
p <- rr_assign_slice(p, i, .arg_size, (rr_index1_read_vec(r, rr_index_vec_floor(i:.arg_size)) + (beta * rr_index1_read_vec(p, rr_index_vec_floor(i:.arg_size)))))\n\
rs_old <- rs_new\n\
iter <- (iter + 1)\n\
next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
    assert!(out.contains("return(x)"));
    assert!(
        out.contains("rs_old <- (sum((r[seq_len(size)] * r[seq_len(size)])))")
            || out.contains("rs_old <- sum((r[seq_len(size)] * r[seq_len(size)]))")
            || out.contains("rs_old <- Sym_117(r, r, size)")
    );
    assert!(!out.contains("rs_old <- Sym_117(rep.int(0, size), rep.int(0, size), size)"));
}

#[test]
pub(crate) fn removes_redundant_tail_assign_slice_for_actual_sym123_raw_shape() {
    let input = "\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
if (!(iter <= 20)) break\n\
p_Ap <- Sym_117(p, Ap, .arg_size)\n\
alpha <- (rs_old / p_Ap)\n\
if ((is.na(alpha) | (!(is.finite(alpha))))) {\n\
  alpha <- 0\n\
} else {\n\
}\n\
i <- 1\n\
.__rr_cse_217 <- i:.arg_size\n\
.__rr_cse_218 <- rr_index_vec_floor(.__rr_cse_217)\n\
.tachyon_exprmap0_1 <- (rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))\n\
.tachyon_exprmap1_1 <- (rr_index1_read_vec(r, .__rr_cse_218) - (alpha * rr_index1_read_vec(Ap, .__rr_cse_218)))\n\
x <- rr_assign_slice(x, i, .arg_size, .tachyon_exprmap0_1)\n\
r <- rr_assign_slice(r, i, .arg_size, .tachyon_exprmap1_1)\n\
rs_new <- Sym_117(r, r, .arg_size)\n\
beta <- (rs_new / rs_old)\n\
.__rr_cse_231 <- 1:.arg_size\n\
.__rr_cse_232 <- rr_index_vec_floor(.__rr_cse_231)\n\
p <- rr_assign_slice(p, 1, .arg_size, (rr_index1_read_vec(r, .__rr_cse_232) + (beta * rr_index1_read_vec(p, .__rr_cse_232))))\n\
rs_old <- rs_new\n\
iter <- (iter + 1)\n\
next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
    assert!(out.contains("return(x)"));
}

#[test]
pub(crate) fn removes_redundant_tail_assign_slice_for_actual_sym123_raw_shape_with_context() {
    let input = "\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
if (!(iter <= 20)) break\n\
p_Ap <- Sym_117(p, Ap, .arg_size)\n\
alpha <- (rs_old / p_Ap)\n\
if ((is.na(alpha) | (!(is.finite(alpha))))) {\n\
  alpha <- 0\n\
} else {\n\
}\n\
i <- 1\n\
.__rr_cse_217 <- i:.arg_size\n\
.__rr_cse_218 <- rr_index_vec_floor(.__rr_cse_217)\n\
.tachyon_exprmap0_1 <- (rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))\n\
.tachyon_exprmap1_1 <- (rr_index1_read_vec(r, .__rr_cse_218) - (alpha * rr_index1_read_vec(Ap, .__rr_cse_218)))\n\
x <- rr_assign_slice(x, i, .arg_size, .tachyon_exprmap0_1)\n\
r <- rr_assign_slice(r, i, .arg_size, .tachyon_exprmap1_1)\n\
rs_new <- Sym_117(r, r, .arg_size)\n\
beta <- (rs_new / rs_old)\n\
.__rr_cse_231 <- 1:.arg_size\n\
.__rr_cse_232 <- rr_index_vec_floor(.__rr_cse_231)\n\
p <- rr_assign_slice(p, 1, .arg_size, (rr_index1_read_vec(r, .__rr_cse_232) + (beta * rr_index1_read_vec(p, .__rr_cse_232))))\n\
rs_old <- rs_new\n\
iter <- (iter + 1)\n\
next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
    let pure = FxHashSet::from_iter([String::from("Sym_117")]);
    let fresh = FxHashSet::from_iter([String::from("Sym_17")]);
    let (out, _) = optimize_emitted_r_with_context_and_fresh(input, true, &pure, &fresh);
    assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
    assert!(out.contains("return(x)"));
}

#[test]
pub(crate) fn removes_branch_local_identical_pure_rebind_after_same_outer_init() {
    let input = "\
Sym_123 <- function(b, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  rs_old <- (sum((b[seq_len(size)] * b[seq_len(size)])))\n\
  if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {\n\
rs_old <- 0.0000001\n\
x <- (rep.int(0, size))\n\
  } else {\n\
  }\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("x <- rep.int(0, size)"));
    assert!(!out.contains("x <- (rep.int(0, size))"), "{out}");
    assert!(out.contains("return(x)"));
}

#[test]
pub(crate) fn removes_redundant_tail_assign_after_runtime_trycatch_helper() {
    let input = "\
rr_native_try_load <- function() {\n\
  ok <- tryCatch({\n\
dyn.load(.rr_env$native_lib)\n\
TRUE\n\
  }, error = function(e) FALSE)\n\
  .rr_env$native_loaded <- isTRUE(ok)\n\
  isTRUE(ok)\n\
}\n\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
if (!(iter <= 20)) break\n\
p_Ap <- Sym_117(p, Ap, .arg_size)\n\
alpha <- (rs_old / p_Ap)\n\
i <- 1\n\
.__rr_cse_217 <- i:.arg_size\n\
.__rr_cse_218 <- rr_index_vec_floor(.__rr_cse_217)\n\
.tachyon_exprmap0_1 <- (rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))\n\
x <- rr_assign_slice(x, i, .arg_size, .tachyon_exprmap0_1)\n\
r <- rr_assign_slice(r, i, .arg_size, (rr_index1_read_vec(r, .__rr_cse_218) - (alpha * rr_index1_read_vec(Ap, .__rr_cse_218))))\n\
rs_new <- Sym_117(r, r, .arg_size)\n\
beta <- (rs_new / rs_old)\n\
.__rr_cse_231 <- 1:.arg_size\n\
.__rr_cse_232 <- rr_index_vec_floor(.__rr_cse_231)\n\
p <- rr_assign_slice(p, 1, .arg_size, (rr_index1_read_vec(r, .__rr_cse_232) + (beta * rr_index1_read_vec(p, .__rr_cse_232))))\n\
rs_old <- rs_new\n\
iter <- (iter + 1)\n\
next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
    assert!(out.contains("return(x)"));
}

#[test]
pub(crate) fn preserves_local_binding_used_by_inlined_helper_calls() {
    let input = "\
Sym_21 <- function(default_capacity)\n\
{\n\
  return(numeric(default_capacity))\n\
}\n\
Sym_54 <- function(default_capacity)\n\
{\n\
  return(rr_field_set(NULL, \"default_chunk_capacity\", default_capacity))\n\
}\n\
Sym_1 <- function(chunks)\n\
{\n\
  default_capacity <- length(chunks)\n\
  chunk_buf <- Sym_21(default_capacity)\n\
  chunk_meta <- Sym_54(default_capacity)\n\
  out <- rr_field_set(NULL, \"buffer\", chunk_buf)\n\
  out <- rr_field_set(out, \"meta\", chunk_meta)\n\
  return(out)\n\
}\n";

    let pure = FxHashSet::from_iter([String::from("Sym_21"), String::from("Sym_54")]);
    let fresh = FxHashSet::default();
    let (out, _) = optimize_emitted_r_with_context_and_fresh(input, true, &pure, &fresh);

    assert!(
        !out.contains("numeric(default_capacity)") || out.contains("default_capacity <-"),
        "helper-local binding was dropped while helper-call inlining kept the symbol live:\n{out}"
    );
    assert!(
        !out.contains("\"default_chunk_capacity\", default_capacity")
            || out.contains("default_capacity <-"),
        "field write still references an unbound helper local:\n{out}"
    );
}

#[test]
pub(crate) fn hoists_repeated_vector_helper_calls_within_single_assignment_rhs() {
    let input = vec![String::from(
        "  .tachyon_exprmap0_0 <- (((rr_index1_read_vec(x, rr_index_vec_floor(i:n)) + rr_index1_read_vec(x, rr_index_vec_floor(i:n))) + rr_index1_read_vec(x, rr_index_vec_floor(i:n))) + ((rr_index1_read_vec(y, rr_index_vec_floor(i:n)) * rr_index1_read_vec(y, rr_index_vec_floor(i:n))) + rr_index1_read_vec(y, rr_index_vec_floor(i:n))))",
    )];
    let out = hoist_repeated_vector_helper_calls_within_lines(input);
    let joined = out.join("\n");
    assert!(
        joined.contains(".__rr_cse_0 <- rr_index1_read_vec(x, rr_index_vec_floor(i:n))"),
        "{joined}"
    );
    assert!(
        joined.contains(".__rr_cse_1 <- rr_index1_read_vec(y, rr_index_vec_floor(i:n))"),
        "{joined}"
    );
    assert_eq!(
        joined
            .matches("rr_index1_read_vec(x, rr_index_vec_floor(i:n))")
            .count(),
        1,
        "{joined}"
    );
    assert_eq!(
        joined
            .matches("rr_index1_read_vec(y, rr_index_vec_floor(i:n))")
            .count(),
        1,
        "{joined}"
    );
    assert!(joined.contains(".tachyon_exprmap0_0 <- (((.__rr_cse_0 + .__rr_cse_0) + .__rr_cse_0) + ((.__rr_cse_1 * .__rr_cse_1) + .__rr_cse_1))"), "{joined}");
}

#[test]
pub(crate) fn forward_exact_vector_helper_reuse_rewrites_later_lines() {
    let input = vec![
        String::from("  .__rr_cse_3 <- rr_index1_read_vec(a, idx)"),
        String::from("  .tachyon_exprmap0_0 <- (rr_index1_read_vec(a, idx) + 1)"),
    ];
    let out = rewrite_forward_exact_vector_helper_reuse(input);
    let joined = out.join("\n");
    assert!(
        joined.contains(".__rr_cse_3 <- rr_index1_read_vec(a, idx)"),
        "{joined}"
    );
    assert!(
        joined.contains(".tachyon_exprmap0_0 <- (.__rr_cse_3 + 1)"),
        "{joined}"
    );
}

#[test]
pub(crate) fn forward_temp_aliases_rewrite_later_uses_to_original_temp() {
    let input = vec![
        String::from("  .__rr_cse_0 <- rr_index1_read_vec(a, idx)"),
        String::from("  .__rr_cse_3 <- .__rr_cse_0"),
        String::from("  next <- (.__rr_cse_3 + 1)"),
    ];
    let out = rewrite_forward_temp_aliases(input);
    let joined = out.join("\n");
    assert!(joined.contains(".__rr_cse_3 <- .__rr_cse_0"), "{joined}");
    assert!(joined.contains("next <- (.__rr_cse_0 + 1)"), "{joined}");
}

#[test]
pub(crate) fn does_not_hoist_two_use_vector_helper_calls_within_single_assignment_rhs() {
    let input = vec![String::from(
        "  score <- (rr_index1_read_vec(x, rr_index_vec_floor(i:n)) + rr_index1_read_vec(x, rr_index_vec_floor(i:n)))",
    )];
    let out = hoist_repeated_vector_helper_calls_within_lines(input);
    let joined = out.join("\n");
    assert!(
        !joined.contains(".__rr_cse_0 <- rr_index1_read_vec(x, rr_index_vec_floor(i:n))"),
        "{joined}"
    );
    assert_eq!(
        joined
            .matches("rr_index1_read_vec(x, rr_index_vec_floor(i:n))")
            .count(),
        2,
        "{joined}"
    );
}
