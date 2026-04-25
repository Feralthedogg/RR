use super::common::*;

#[test]
fn rewrites_literal_field_get_calls_to_base_indexing() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  return(rr_field_get(particles, \"pf\"))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("return(particles[[\"pf\"]])"), "{out}");
    assert!(!out.contains("rr_field_get(particles, \"px\")"), "{out}");
    assert!(!out.contains("rr_field_get(particles, \"pf\")"), "{out}");
}

#[test]
fn rewrites_literal_named_list_calls_to_base_list() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  return(rr_named_list(\"px\", px, \"py\", py, \"pf\", pf))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("return(list(px = px, py = py, pf = pf))"),
        "{out}"
    );
    assert!(
        !out.contains("rr_named_list(\"px\", px, \"py\", py, \"pf\", pf)"),
        "{out}"
    );
}

#[test]
fn rewrites_later_literal_helpers_when_earlier_literal_candidate_is_dynamic() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  a <- rr_field_get(particles, dynamic_name) + rr_field_get(particles, \"px\")\n\
  return(c(a, rr_named_list(dynamic_name, px), rr_named_list(\"py\", py)))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("rr_field_get(particles, dynamic_name) + particles[[\"px\"]]"),
        "{out}"
    );
    assert!(
        out.contains("rr_named_list(dynamic_name, px), list(py = py)"),
        "{out}"
    );
}

#[test]
fn rewrites_safe_loop_index_write_calls_to_base_indexing() {
    let input = "\
Sym_1 <- function(n, xs) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= n)) break\n\
xs[rr_index1_write(i, \"index\")] <- 0\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(xs)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("xs[i] <- 0"), "{out}");
    assert!(!out.contains("rr_index1_write(i, \"index\")"), "{out}");
}

#[test]
fn keeps_index_write_helper_when_loop_index_is_reassigned_non_canonically() {
    let input = "\
Sym_1 <- function(n, xs) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= n)) break\n\
xs[rr_index1_write(i, \"index\")] <- 0\n\
i <- (i * 2)\n\
next\n\
  }\n\
  return(xs)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("rr_index1_write(i, \"index\")"), "{out}");
}

#[test]
fn rewrites_safe_named_index_read_calls_to_base_indexing() {
    let input = "\
Sym_1 <- function(u, f, ix, iy, N) \n\
{\n\
  idx <- rr_idx_cube_vec_i(f, ix, iy, N)\n\
  dx <- ((rr_index1_read(u, idx, \"index\") * 0.1) / 4)\n\
  return(dx)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("dx <- ((u[idx] * 0.1) / 4)")
            || out.contains("dx <- ((u[rr_idx_cube_vec_i(f, ix, iy, N)] * 0.1) / 4)"),
        "{out}"
    );
    assert!(!out.contains("rr_index1_read(u, idx, \"index\")"), "{out}");
}

#[test]
fn rewrites_later_safe_index_read_when_earlier_call_is_unsafe() {
    let input = "\
Sym_1 <- function(samples, draws, f, ix, iy, n, raw_idx) \n\
{\n\
  idx <- rr_idx_cube_vec_i(f, ix, iy, n)\n\
  score <- (rr_index1_read(draws, raw_idx, \"index\") + rr_index1_read(samples, idx, \"index\"))\n\
  return(score)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("score <- (rr_index1_read(draws, raw_idx, \"index\") + samples[idx])")
            || out.contains(
                "score <- (rr_index1_read(draws, raw_idx, \"index\") + samples[rr_idx_cube_vec_i(f, ix, iy, n)])"
            ),
        "{out}"
    );
    assert!(
        !out.contains("rr_index1_read(samples, idx, \"index\")"),
        "{out}"
    );
}

#[test]
fn rewrites_floor_clamped_named_index_reads_to_base_indexing() {
    let input = "\
Sym_1 <- function(samples, draws, n, inner, resample) \n\
{\n\
  idx <- (pmin(pmax((1 + floor((rr_index1_read(draws, (((resample - 1) * n) + inner), \"index\") * n))), 1), n))\n\
  s <- (0 + rr_index1_read(samples, idx, \"index\"))\n\
  return(s)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("s <- (0 + samples[idx])")
            || out.contains("s <- (samples[idx])")
            || out.contains("return((0 + samples[idx]))")
            || out.contains("return(samples[idx])"),
        "{out}"
    );
    assert!(
        !out.contains("rr_index1_read(samples, idx, \"index\")"),
        "{out}"
    );
}

#[test]
fn rewrites_flat_positive_loop_index_reads_to_base_indexing() {
    let input = "\
Sym_1 <- function(draws, n, resamples) \n\
{\n\
  resample <- 1\n\
  repeat {\n\
if (!(resample <= resamples)) break\n\
inner <- 1\n\
repeat {\n\
  if (!(inner <= n)) break\n\
  draw <- rr_index1_read(draws, (((resample - 1) * n) + inner), \"index\")\n\
  inner <- (inner + 1)\n\
  next\n\
}\n\
resample <- (resample + 1)\n\
next\n\
  }\n\
  return(draw)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("draw <- draws[(((resample - 1) * n) + inner)]"),
        "{out}"
    );
    assert!(
        !out.contains("rr_index1_read(draws, (((resample - 1) * n) + inner), \"index\")"),
        "{out}"
    );
}

#[test]
fn rewrites_same_len_tail_scalar_reads_to_base_indexing() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 8\n\
  clean <- rep.int(0, n)\n\
  clean <- ifelse((clean > 0.4), sqrt((clean + 0.1)), ((clean * 0.55) + 0.03))\n\
  print(rr_index1_read(clean, n, \"index\"))\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("print(clean[n])"), "{out}");
    assert!(
        !out.contains("rr_index1_read(clean, n, \"index\")"),
        "{out}"
    );
}

#[test]
fn rewrites_safe_interior_loop_neighbor_reads_to_base_indexing() {
    let input = "\
Sym_1 <- function(n, a, next_a) \n\
{\n\
  i <- 2\n\
  repeat {\n\
if (!(i < n)) break\n\
lap_a <- ((rr_index1_read(a, (i - 1), \"index\") - (2 * rr_index1_read(a, i, \"index\"))) + rr_index1_read(a, (i + 1), \"index\"))\n\
next_a[rr_index1_write(i, \"index\")] <- lap_a\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(next_a)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("lap_a <- ((a[(i - 1)] - (2 * a[i])) + a[(i + 1)])")
            || out.contains("next_a[i] <- ((a[(i - 1)] - (2 * a[i])) + a[(i + 1)])"),
        "{out}"
    );
    assert!(
        out.contains("next_a[i] <- lap_a")
            || out.contains("next_a[i] <- ((a[(i - 1)] - (2 * a[i])) + a[(i + 1)])"),
        "{out}"
    );
    assert!(!out.contains("rr_index1_read(a, i, \"index\")"), "{out}");
    assert!(!out.contains("rr_index1_write(i, \"index\")"), "{out}");
}

#[test]
fn keeps_identical_rr_field_get_rebind_when_particles_change_inside_loop() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= TOTAL)) break\n\
particles <- rr_named_list(\"px\", p_x, \"py\", p_y, \"pf\", p_f)\n\
i <- (i + 1)\n\
next\n\
  }\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  return(p_x)\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("particles <- rr_named_list(\"px\", p_x, \"py\", p_y, \"pf\", p_f)")
            || out.contains("particles <- list(px = p_x, py = p_y, pf = p_f)"),
        "{out}"
    );
    assert!(
        out.contains("return(rr_field_get(particles, \"px\"))")
            || out
                .matches("p_x <- rr_field_get(particles, \"px\")")
                .count()
                == 2
            || out.contains("return(particles[[\"px\"]])")
            || out.matches("p_x <- particles[[\"px\"]]").count() >= 1,
        "{out}"
    );
}

#[test]
fn does_not_inline_singleton_assign_slice_as_whole_range_copy() {
    let input = "\
Sym_78 <- function(f, ys) \n\
{\n\
  rot <- rep.int(0, length(ys))\n\
  if ((f == 2)) {\n\
rot <- rr_assign_slice(rot, 1, 1, rep.int(1, 1))\n\
return(rot)\n\
  } else {\n\
return(rot)\n\
  }\n\
}\n";
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        true,
        &FxHashSet::default(),
        &FxHashSet::default(),
        true,
    );
    assert!(out.contains("rot <- replace(rot, 1, 1)"));
    assert!(!out.contains("rot <- rep.int(1, 1)"));
    assert!(!out.contains("rot <- rr_assign_slice(rot, 1, 1, rep.int(1, 1))"));
}

#[test]
fn rewrites_readonly_param_aliases_and_index_only_mutated_param_shadows() {
    let input = "\
Sym_60 <- function(f, x, ys, size) \n\
{\n\
  .arg_f <- f\n\
  .arg_x <- x\n\
  .arg_ys <- ys\n\
  .arg_size <- size\n\
  if ((.arg_f == 1)) {\n\
return(rr_idx_cube_vec_i(rep.int(4, length(ys)), rep.int(.arg_size, length(ys)), .arg_ys, .arg_size))\n\
  }\n\
  return(rr_idx_cube_vec_i(rep.int(.arg_f, length(ys)), rep.int((.arg_x - 1), length(ys)), .arg_ys, .arg_size))\n\
}\n\
Sym_186 <- function(px, py, pf, u, v, dt, N) \n\
{\n\
  .arg_px <- px\n\
  .arg_py <- py\n\
  .arg_dt <- dt\n\
  x <- .arg_px[i]\n\
  .arg_px[i] <- x\n\
  return(.arg_dt)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains(".arg_f <- f"));
    assert!(!out.contains(".arg_x <- x"));
    assert!(!out.contains(".arg_ys <- ys"));
    assert!(!out.contains(".arg_size <- size"));
    assert!(out.contains("if ((f == 1)) {"));
    assert!(out.contains("rep.int(size, length(ys))"));
    assert!(out.contains(
        "return(rr_idx_cube_vec_i(rep.int(f, length(ys)), rep.int((x - 1), length(ys)), ys, size))"
    ), "{out}");
    assert!(!out.contains(".arg_px"));
    assert!(!out.contains(".arg_py"));
    assert!(!out.contains(".arg_dt"));
}

#[test]
fn does_not_rewrite_mutated_param_shadow_aliases() {
    let input = "\
Sym_13 <- function(n, acc) \n\
{\n\
  .arg_n <- n\n\
  .arg_acc <- acc\n\
  repeat {\n\
if ((.arg_n <= 0L)) break\n\
.__pc_src_tmp0 <- (.arg_n - 1L)\n\
.__pc_src_tmp1 <- (.arg_acc + .arg_n)\n\
.arg_n <- .__pc_src_tmp0\n\
.arg_acc <- .__pc_src_tmp1\n\
next\n\
  }\n\
  return(.arg_acc)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains(".arg_n <- n"));
    assert!(out.contains(".arg_n <- .__pc_src_tmp0"));
    assert!(out.contains("if ((.arg_n <= 0L)) break"));
}

#[test]
fn readonly_param_alias_rewrite_keeps_mutated_tco_shadow_aliases() {
    let input = vec![
        "Sym_13 <- function(n, acc) ".to_string(),
        "{".to_string(),
        "  .arg_n <- n".to_string(),
        "  .arg_acc <- acc".to_string(),
        "  repeat {".to_string(),
        "    if ((.arg_n <= 0L)) break".to_string(),
        "    .__pc_src_tmp0 <- (.arg_n - 1L)".to_string(),
        "    .__pc_src_tmp1 <- (.arg_acc + .arg_n)".to_string(),
        "    .arg_n <- .__pc_src_tmp0".to_string(),
        "    .arg_acc <- .__pc_src_tmp1".to_string(),
        "    next".to_string(),
        "  }".to_string(),
        "  return(.arg_acc)".to_string(),
        "}".to_string(),
    ];
    let out = rewrite_readonly_param_aliases(input).join("\n");
    assert!(out.contains(".arg_n <- n"));
    assert!(out.contains("if ((.arg_n <= 0L)) break"));
    assert!(out.contains(".arg_n <- .__pc_src_tmp0"));
}

#[test]
fn strip_unused_arg_aliases_keeps_mutated_tco_shadow_aliases() {
    let input = vec![
        "Sym_13 <- function(n, acc) ".to_string(),
        "{".to_string(),
        "  .arg_n <- n".to_string(),
        "  .arg_acc <- acc".to_string(),
        "  repeat {".to_string(),
        "    if ((.arg_n <= 0L)) break".to_string(),
        "    .__pc_src_tmp0 <- (.arg_n - 1L)".to_string(),
        "    .__pc_src_tmp1 <- (.arg_acc + .arg_n)".to_string(),
        "    .arg_n <- .__pc_src_tmp0".to_string(),
        "    .arg_acc <- .__pc_src_tmp1".to_string(),
        "    next".to_string(),
        "  }".to_string(),
        "  return(.arg_acc)".to_string(),
        "}".to_string(),
    ];
    let out = strip_unused_arg_aliases(input).join("\n");
    assert!(out.contains(".arg_n <- n"));
    assert!(out.contains(".arg_n <- .__pc_src_tmp0"));
}

#[test]
fn readonly_param_alias_rewrite_fully_rewrites_recursive_param_uses() {
    let input = vec![
        "Sym_13 <- function(n, acc) ".to_string(),
        "{".to_string(),
        "  .arg_n <- n".to_string(),
        "  .arg_acc <- acc".to_string(),
        "  if ((.arg_n <= 0L)) {".to_string(),
        "    return(.arg_acc)".to_string(),
        "  } else {".to_string(),
        "    return(Sym_13((.arg_n - 1L), (.arg_acc + .arg_n)))".to_string(),
        "  }".to_string(),
        "}".to_string(),
    ];
    let out = rewrite_readonly_param_aliases(input).join("\n");
    assert!(!out.contains(".arg_n <- n"));
    assert!(!out.contains(".arg_acc <- acc"));
    assert!(out.contains("if ((n <= 0L)) {"));
    assert!(out.contains("return(acc)"));
    assert!(out.contains("return(Sym_13((n - 1L), (acc + n)))"));
}

#[test]
fn shifted_square_reuse_collapses_recent_temp_chain_to_named_scalar() {
    let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .__rr_cse_5 <- (x / size)\n\
  u <- ((.__rr_cse_5 + .__rr_cse_5) - 1)\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  if ((f == 6)) {\n\
.__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
.__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
lat <- ((-(45)) - ((1 - ((((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1)) + ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 0.25)) * 45))\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("lat <-") || out.contains("if ((f == 6)) {\n}"),
        "{out}"
    );
    assert!(
        out.contains("* 45") || out.contains("if ((f == 6)) {\n}"),
        "{out}"
    );
    assert!(
        !out.contains(".__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)"),
        "{out}"
    );
    assert!(
        !out.contains(".__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)"),
        "{out}"
    );
}

#[test]
fn shifted_square_reuse_handles_actual_sym37_shape() {
    let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  .__rr_cse_5 <- (x / size)\n\
  u <- ((.__rr_cse_5 + .__rr_cse_5) - 1)\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  if ((f == 6)) {\n\
.__rr_cse_5 <- (.arg_x / .arg_size)\n\
.__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
.__rr_cse_11 <- (.arg_y / .arg_size)\n\
.__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
lat <- ((-(45)) - ((1 - ((((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1)) + ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 0.25)) * 45))\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("lat <-") || out.contains("if ((f == 6)) {\n}"),
        "{out}"
    );
    assert!(
        out.contains("* 45") || out.contains("if ((f == 6)) {\n}"),
        "{out}"
    );
    assert!(
        !out.contains(".__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)"),
        "{out}"
    );
    assert!(
        !out.contains(".__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)"),
        "{out}"
    );
}

#[test]
fn shifted_square_reuse_handles_raw_emitted_sym37_shape() {
    let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_f <- f\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  .__rr_cse_5 <- (.arg_x / .arg_size)\n\
  u <- ((.__rr_cse_5 + .__rr_cse_5) - 1)\n\
  .__rr_cse_11 <- (.arg_y / .arg_size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  lat <- 0\n\
  if ((.arg_f == 5)) {\n\
.__rr_cse_5 <- (.arg_x / .arg_size)\n\
.__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
.__rr_cse_11 <- (.arg_y / .arg_size)\n\
.__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
lat <- (45 + ((1 - (((u * u) + (v * v)) * 0.25)) * 45))\n\
  } else {\n\
  }\n\
  if ((.arg_f == 6)) {\n\
.__rr_cse_5 <- (.arg_x / .arg_size)\n\
.__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
.__rr_cse_11 <- (.arg_y / .arg_size)\n\
.__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
lat <- ((-(45)) - ((1 - ((((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1)) + ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 0.25)) * 45))\n\
  } else {\n\
  }\n\
  if ((.arg_f < 5)) {\n\
.__rr_cse_11 <- (.arg_y / .arg_size)\n\
lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)\n\
  } else {\n\
  }\n\
  return(lat)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("((u * u) + (v * v))"));
    assert!(!out.contains("((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1))"));
    assert!(!out.contains("((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))"));
    assert!(out.contains("lat <-"));
    assert!(out.contains("* 45)"));
}

#[test]
fn removes_redundant_identical_nested_temp_reassign() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- (x_curr / N)\n\
  if ((f_curr == 6)) {\n\
.__rr_cse_1 <- (x_curr / N)\n\
y <- ((.__rr_cse_1 + .__rr_cse_1) - 1)\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.matches(".__rr_cse_1 <- (x_curr / N)").count() <= 1);
    assert!(
        out.contains("y <-") || out.contains("return(") || out.contains("if ((f_curr == 6)) {\n}"),
        "{out}"
    );
}

#[test]
fn removes_redundant_temp_reassign_even_with_intermediate_self_copy() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- (x_curr / N)\n\
  if ((flag == 1)) {\n\
.__rr_cse_1 <- .__rr_cse_1\n\
.__rr_cse_1 <- (x_curr / N)\n\
z <- (.__rr_cse_1 + 1)\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.matches(".__rr_cse_1 <- (x_curr / N)").count() <= 1);
}
