use super::common::*;

#[test]
fn keeps_repeat_preheader_and_induction_assignments() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= n)) break\n\
ii <- i\n\
out[ii] <- x[ii]\n\
i <- (i + 1)\n\
next\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("out[i] <- x[i]") || out.contains("out[ii] <- x[ii]"),
        "{out}"
    );
    assert!(
        out.contains("i <- 1")
            || out.contains("i <- 1L")
            || out.contains("for (i in seq_len(n)) {"),
        "{out}"
    );
    assert!(
        out.contains("i <- (i + 1)") || out.contains("for (i in seq_len(n)) {"),
        "{out}"
    );
}

#[test]
fn keeps_generated_i_temp_induction_update_inside_repeat() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 10\n\
  clean <- rep.int(0, n)\n\
  score <- rep.int(1, n)\n\
  i_9 <- 1L\n\
  repeat {\n\
if (!(i_9 <= n)) break\n\
if ((score[i_9] > 0.4)) {\n\
  clean[i_9] <- sqrt((score[i_9] + 0.1))\n\
} else {\n\
  clean[i_9] <- ((score[i_9] * 0.55) + 0.03)\n\
}\n\
i_9 <- (i_9 + 1L)\n\
next\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("i_9 <- (i_9 + 1L)")
            || out.contains("clean <- ifelse(")
            || out.contains("for (i_9 in seq_len(n)) {")
            || (out.contains("n <- 10") && !out.contains("clean")),
        "{out}"
    );
}

#[test]
fn keeps_nested_loop_reseed_inside_outer_repeat() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  steps <- 0\n\
  repeat {\n\
if (!(steps < 3)) break\n\
i <- 1\n\
repeat {\n\
  if (!(i <= n)) break\n\
  ii <- i\n\
  out[ii] <- x[ii]\n\
  i <- (i + 1)\n\
  next\n\
}\n\
steps <- (steps + 1)\n\
next\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert_eq!(out.matches("i <- 1").count(), 2);
    assert!(out.contains("steps <- (steps + 1)"));
    assert!(out.contains("out[i] <- x[i]"));
}

#[test]
fn removes_dead_parenthesized_eval_lines() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  x <- 1\n\
  (floor(((x - 1) / 4)) + 1)\n\
  ((x + x) - 1)\n\
  rr_mark(10, 1);\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("(floor(((x - 1) / 4)) + 1)"));
    assert!(!out.contains("((x + x) - 1)"));
    assert!(out.contains("rr_mark(10, 1);"));
    assert!(out.contains("return(x)"));
}

#[test]
fn removes_dead_plain_identifier_eval_lines() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  x <- 1\n\
  x\n\
  rr_mark(10, 1);\n\
  return(x)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("\n  x\n"));
    assert!(out.contains("rr_mark(10, 1);"));
    assert!(out.contains("return(x)"));
}

#[test]
fn removes_noop_self_assignments() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- .__rr_cse_1\n\
  x <- x\n\
  y <- (x + 1)\n\
  return(y)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains(".__rr_cse_1 <- .__rr_cse_1"));
    assert!(!out.contains("x <- x"));
    assert!(
        out.contains("y <- (x + 1)") || out.contains("return((x + 1))"),
        "{out}"
    );
}

#[test]
fn forwards_exact_scalar_expr_into_following_if_chain() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  u <- ((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1)\n\
  if ((face == 5)) {\n\
lat <- (45 + ((1 - (((((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1) * ((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1))) * 0.25)) * 45))\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("u <- ((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1)")
            || out.contains("lat <- (45 + ((1 - (((u * u)")
            || out.contains("lat <- (45 + ((1 - (((((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)")
            || out.contains("if ((face == 5)) {\n}"),
        "{out}"
    );
    assert!(
        out.contains("lat <- (45 + ((1 - (((u * u)")
            || out
                .contains("lat <- (45 + ((1 - (((((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)")
            || out.contains("if ((face == 5)) {\n}"),
        "{out}"
    );
}

#[test]
fn exact_expr_reuse_does_not_rewrite_same_lhs_reassignment_to_self_copy() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- (x_curr / N)\n\
  if ((flag == 1)) {\n\
.__rr_cse_1 <- (x_curr / N)\n\
z <- ((.__rr_cse_1 + .__rr_cse_1) - 1)\n\
  }\n\
    }\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains(".__rr_cse_1 <- .__rr_cse_1"));
    assert!(out.matches(".__rr_cse_1 <- (x_curr / N)").count() <= 1);
}

#[test]
fn exact_expr_reuse_does_not_leak_branch_local_temp_into_sibling_branch() {
    let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  lat <- 0\n\
  if ((f == 6)) {\n\
.__rr_cse_11 <- (y / size)\n\
.__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
lat <- ((-(45)) - ((1 - ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 45))\n\
  }\n\
  if ((f < 5)) {\n\
lat <- ((.__rr_cse_13 - 1) * 45)\n\
  }\n\
  return(lat)\n\
    }\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("lat <- ((.__rr_cse_13 - 1) * 45)"));
}

#[test]
fn boundary_alias_rewrite_does_not_corrupt_following_if_condition() {
    let input = "\
Sym_20 <- function(x, lo, hi) \n\
{\n\
  .arg_x <- x\n\
  .arg_lo <- lo\n\
  .arg_hi <- hi\n\
  y <- .arg_x\n\
  if ((y < .arg_lo)) {\n\
y <- lo\n\
  }\n\
  if ((y > .arg_hi)) {\n\
y <- hi\n\
  }\n\
  return(y)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("if ((y > hi)) {")
            || out.contains("if ((y > .arg_hi)) {")
            || out.contains("return(pmin(pmax(x, lo), hi))"),
        "{out}"
    );
    assert!(!out.contains("if ((lo > hi)) {"));
}

#[test]
fn exact_expr_reuse_tolerates_prologue_arg_aliases_for_same_rhs_dep_write() {
    let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  if ((f < 5)) {\n\
.__rr_cse_11 <- (.arg_y / .arg_size)\n\
lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("lat <- (v * 45)")
            || out.contains("lat <- ((((y / size) + (y / size)) - 1) * 45)")
            || out.contains("lat <- ((((.arg_y / .arg_size) + (.arg_y / .arg_size)) - 1) * 45)")
            || !out.contains(".__rr_cse_11 <- (.arg_y / .arg_size)"),
        "{out}"
    );
}

#[test]
fn inlines_immediate_single_use_scalar_temp_into_following_assignment() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_642 <- (x_curr / N)\n\
  inlined_39_u <- ((.__rr_cse_642 + .__rr_cse_642) - 1)\n\
  .__rr_cse_648 <- (y_curr / N)\n\
  inlined_39_v <- ((.__rr_cse_648 + .__rr_cse_648) - 1)\n\
  return(inlined_39_v)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains(".__rr_cse_642 <- (x_curr / N)"));
    assert!(!out.contains(".__rr_cse_648 <- (y_curr / N)"));
    assert!(
        out.contains("inlined_39_u <- (((x_curr / N) + (x_curr / N)) - 1)")
            || !out.contains("inlined_39_u <-"),
        "{out}"
    );
    assert!(
        out.contains("inlined_39_v <- (((y_curr / N) + (y_curr / N)) - 1)")
            || out.contains("return((((y_curr / N) + (y_curr / N)) - 1))"),
        "{out}"
    );
}

#[test]
fn inlines_immediate_single_use_index_temp_into_following_assignment() {
    let input = "\
Sym_1 <- function(size) \n\
{\n\
  i <- 1\n\
  .__rr_cse_65 <- rr_index_vec_floor(i:size)\n\
  y <- rr_assign_slice(y, i, size, (rr_index1_read_vec(x, .__rr_cse_65) + rr_index1_read_vec(z, .__rr_cse_65)))\n\
  return(y)\n\
    }\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains(".__rr_cse_65 <- rr_index_vec_floor(i:size)"));
    assert!(out.contains("(x + z)"), "{out}");
}

#[test]
fn inlines_single_use_scalar_temp_across_adjacent_temp_setup() {
    let input = "\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2) \n\
{\n\
  .__rr_cse_10 <- ((v_m2 - (2 * v_m1)) + v_c)\n\
  .__rr_cse_20 <- (v_m2 - (4 * v_m1))\n\
  .__rr_cse_22 <- (3 * v_c)\n\
  b1 <- (((1.0833 * .__rr_cse_10) * .__rr_cse_10) + ((0.25 * (.__rr_cse_20 + .__rr_cse_22)) * (.__rr_cse_20 + .__rr_cse_22)))\n\
  return(b1)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("b1 <-") || out.contains("return(("), "{out}");
    assert!(out.contains("1.0833"), "{out}");
    assert!(out.contains("0.25"), "{out}");
    assert!(out.contains("v_m2"), "{out}");
    assert!(out.contains("v_c"), "{out}");
}

#[test]
fn inlines_two_use_scalar_temp_within_straight_line_region() {
    let input = "\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2) \n\
{\n\
  .__rr_cse_22 <- (3 * v_c)\n\
  b1 <- ((0.25 * ((v_m2 - (4 * v_m1)) + .__rr_cse_22)) * ((v_m2 - (4 * v_m1)) + .__rr_cse_22))\n\
  b3 <- ((0.25 * ((.__rr_cse_22 - (4 * v_p1)) + v_p2)) * ((.__rr_cse_22 - (4 * v_p1)) + v_p2))\n\
  return(b3)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("b1 <-") || !out.contains("b1"), "{out}");
    assert!(out.contains("b3 <-") || out.contains("return(("), "{out}");
    assert!(out.contains("3 * v_c"), "{out}");
}

#[test]
fn strips_unused_arg_aliases_but_keeps_used_ones() {
    let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  u <- (((x / size) + (x / size)) - 1)\n\
  v <- (((y / size) + (y / size)) - 1)\n\
  lat <- (v * 45)\n\
  return(lat)\n\
}\n\
Sym_186 <- function(px, py, pf, u, v, dt, N) \n\
{\n\
  .arg_px <- px\n\
  .arg_py <- py\n\
  .arg_dt <- dt\n\
  x <- .arg_px[i]\n\
  y <- .arg_py[i]\n\
  dx <- (x * .arg_dt)\n\
  return(dx)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains(".arg_x <- x"));
    assert!(!out.contains(".arg_y <- y"));
    assert!(!out.contains(".arg_size <- size"));
    assert!(!out.contains(".arg_px"));
    assert!(!out.contains(".arg_py"));
    assert!(!out.contains(".arg_dt"));
}

#[test]
fn rewrites_immediate_ii_alias_to_i_in_loop_body() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  repeat {\n\
if (!(i <= n)) break\n\
ii <- i\n\
out[ii] <- (x[ii] + y[ii])\n\
if ((out[ii] > max_v)) {\n\
  max_v <- out[ii]\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("ii <- i"));
    assert!(out.contains("out[i] <- (x[i] + y[i])"));
    assert!(out.contains("if ((out[i] > max_v)) {"));
    assert!(out.contains("max_v <- out[i]"));
}

#[test]
fn rewrites_temp_uses_after_named_copy() {
    let input = "\
Sym_287 <- function() \n\
{\n\
  .__pc_src_tmp0 <- (temp[i] - 273.15)\n\
  .__pc_src_tmp1 <- temp[i]\n\
  .__pc_src_tmp3 <- q_s[i]\n\
  .__pc_src_tmp4 <- q_g[i]\n\
  T_c <- .__pc_src_tmp0\n\
  T <- .__pc_src_tmp1\n\
  qs <- .__pc_src_tmp3\n\
  qg <- .__pc_src_tmp4\n\
  if ((.__pc_src_tmp0 < (-(15)))) {\n\
T_c <- .__pc_src_tmp0\n\
  }\n\
  if ((.__pc_src_tmp3 > 0)) {\n\
melt_rate <- (qs * 0.05)\n\
  }\n\
  if ((.__pc_src_tmp4 > 0)) {\n\
melt_rate <- (melt_rate + (qg * 0.02))\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("if ((T_c < (-(15)))) {"));
    assert!(out.contains("if ((qs > 0)) {"));
    assert!(out.contains("if ((qg > 0)) {"));
    assert!(!out.contains("if ((.__pc_src_tmp0 < (-(15)))) {"));
    assert!(!out.contains("if ((.__pc_src_tmp3 > 0)) {"));
    assert!(!out.contains("if ((.__pc_src_tmp4 > 0)) {"));
}

#[test]
fn branch_local_scalar_hoist_does_not_corrupt_guard_self_compare() {
    let input = "\
Sym_303 <- function() \n\
{\n\
  i <- 1\n\
  max_u <- (-(1000))\n\
  repeat {\n\
if (!(i <= TOTAL)) break\n\
u_new[i] <- (u_new[i] + heat[i])\n\
if ((u_new[i] > max_u)) {\n\
  max_u <- u_new[i]\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(max_u)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("if ((u_new[i] > max_u)) {"), "{out}");
    assert!(!out.contains("if ((u_new[i] > u_new[i])) {"), "{out}");
    assert!(out.contains("max_u <- u_new[i]"), "{out}");
}

#[test]
fn branch_local_named_scalar_index_read_does_not_leak_past_if() {
    let input = "\
Sym_303 <- function() \n\
{\n\
  max_u <- (-(1000))\n\
  if ((u_new[i] > max_u)) {\n\
max_u <- u_new[i]\n\
  }\n\
  print(max_u)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("max_u <- u_new[i]"), "{out}");
    assert!(out.contains("print(max_u)"), "{out}");
    assert!(!out.contains("print(u_new[i])"), "{out}");
}

#[test]
fn simple_alias_guard_rewrite_does_not_leak_branch_local_alias_to_outer_guard() {
    let input = "\
Sym_207 <- function(x, y, w, h) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_w <- w\n\
  .arg_h <- h\n\
  xx <- .arg_x\n\
  yy <- .arg_y\n\
  if ((xx < 1)) {\n\
xx <- .arg_w\n\
  } else {\n\
  }\n\
  if ((xx > .arg_w)) {\n\
xx <- 1\n\
  } else {\n\
  }\n\
  if ((yy < 1)) {\n\
yy <- h\n\
  } else {\n\
  }\n\
  if ((yy > .arg_h)) {\n\
yy <- 1\n\
  } else {\n\
  }\n\
  return((((yy - 1) * .arg_w) + xx))\n\
}\n";
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        true,
        &FxHashSet::default(),
        &FxHashSet::default(),
        true,
    );
    assert!(out.contains("yy <- y"));
    assert!(out.contains("xx <- w"));
    assert!(out.contains("yy <- h"));
}
