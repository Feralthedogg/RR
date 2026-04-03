use super::common::*;

#[test]
fn raw_emitted_trivial_clamp_helper_calls_inline_before_peephole() {
    let input = [
        "Sym_1 <- function() ",
        "{",
        "  next_a[i] <- Sym_20(next_a_cell, 0, 1)",
        "  next_b[i] <- Sym_20(next_b_cell, 0, 1)",
        "}",
        "",
        "Sym_20 <- function(x, lo, hi) ",
        "{",
        "  .arg_x <- x",
        "  .arg_lo <- lo",
        "  .arg_hi <- hi",
        "  y <- .arg_x",
        "  if ((y < .arg_lo)) {",
        "    y <- .arg_lo",
        "  } else {",
        "  }",
        "  if ((y > .arg_hi)) {",
        "    y <- hi",
        "  } else {",
        "  }",
        "  return(y)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(&input);

    assert!(out.contains("next_a[i] <- (pmin(pmax(next_a_cell, 0), 1))"));
    assert!(out.contains("next_b[i] <- (pmin(pmax(next_b_cell, 0), 1))"));
    assert!(!out.contains("Sym_20(next_a_cell, 0, 1)"));
    assert!(!out.contains("Sym_20(next_b_cell, 0, 1)"));
    assert!(out.contains("Sym_20 <- function(x, lo, hi)"));
}

#[test]
fn raw_emitted_branch_local_identical_alloc_rebinds_are_pruned_before_peephole() {
    let input = [
        "Sym_123 <- function(b, size) ",
        "{",
        "x <- rep.int(0, size)",
        "rs_old <- (sum((b[seq_len(size)] * b[seq_len(size)])))",
        "if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {",
        "rs_old <- 0.0000001",
        "x <- Sym_17(size, 0)",
        "}",
        "return(x)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(&input);

    assert!(out.contains("x <- rep.int(0, size)"));
    assert!(out.contains("rs_old <- 0.0000001"));
    assert!(!out.contains("x <- Sym_17(size, 0)"));
    assert!(out.contains("return(x)"));
}

#[test]
fn raw_emitted_branch_local_identical_scalar_rebinds_prune_before_alias_inline() {
    let input = [
        "Sym_287 <- function(temp, q_v, size) ",
        "{",
        "  i <- 1",
        "  ii <- i",
        "  T_c <- (temp[ii] - 273.15)",
        "  if ((T_c < (-(5)))) {",
        "    qv <- q_v[ii]",
        "    ii <- i",
        "  }",
        "  if ((T_c < (-(15)))) {",
        "    if ((qv > 0.01)) {",
        "      print(qv)",
        "    }",
        "  }",
        "  return(T_c)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(
        &rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(&input),
    );

    assert!(!out.contains("    ii <- i\n  }"), "{out}");
    assert!(!out.contains("qv <- q_v[ii]"), "{out}");
    assert!(out.contains("if ((q_v[ii] > 0.01)) {"), "{out}");
}

#[test]
fn raw_emitted_single_use_scalar_index_aliases_inline_before_peephole() {
    let input = [
        "Sym_287 <- function(temp, q_v, q_c, size) ",
        "{",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= size)) break",
        "    T_c <- (temp[i] - 273.15)",
        "    qv <- q_v[i]",
        "    qc <- q_c[i]",
        "    if ((qv > 0.01)) {",
        "      if ((qc > 0.0001)) {",
        "        print(T_c)",
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

    let out = rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(&input);

    assert!(!out.contains("qv <- q_v[i]"), "{out}");
    assert!(!out.contains("qc <- q_c[i]"), "{out}");
    assert!(
        out.contains("if ((q_v[i] > 0.01)) {") || out.contains("if (((q_v[i]) > 0.01)) {"),
        "{out}"
    );
    assert!(
        out.contains("if ((q_c[i] > 0.0001)) {") || out.contains("if (((q_c[i]) > 0.0001)) {"),
        "{out}"
    );
    assert!(out.contains("print(T_c)"), "{out}");
}

#[test]
fn raw_emitted_small_multiuse_scalar_index_aliases_inline_into_adjacent_assignments() {
    let input = [
        "Sym_210 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
        "{",
        "  a <- A[i]",
        "  b <- B[i]",
        "  new_a <- (a + ((((DA * lapA[i]) - (a * b) * b) + (f * (1 - a))) * dt))",
        "  new_b <- (b + ((((DB * lapB[i]) + (a * b) * b) - ((k + f) * b)) * dt))",
        "  return(new_b)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
        &input,
    );

    assert!(!out.contains("a <- A[i]"), "{out}");
    assert!(!out.contains("b <- B[i]"), "{out}");
    assert!(
        out.contains(
            "new_a <- (A[i] + ((((DA * lapA[i]) - (A[i] * B[i]) * B[i]) + (f * (1 - A[i]))) * dt))"
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "new_b <- (B[i] + ((((DB * lapB[i]) + (A[i] * B[i]) * B[i]) - ((k + f) * B[i])) * dt))"
        ),
        "{out}"
    );
}

#[test]
fn raw_emitted_small_multiuse_scalar_index_aliases_do_not_inline_past_adjacent_region() {
    let input = [
        "Sym_1 <- function(A, i) ",
        "{",
        "  a <- A[i]",
        "  keep <- (a + 1)",
        "  skip <- 0",
        "  out <- (a + 2)",
        "  return(out)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
        &input,
    );

    assert!(out.contains("a <- A[i]"), "{out}");
    assert!(out.contains("out <- (a + 2)"), "{out}");
}

#[test]
fn raw_emitted_floor_fed_particle_clamp_pair_collapses_gx_gy() {
    let input = [
        "Sym_186 <- function(x, y, N) ",
        "{",
        "  gx <- ((x * N) + 1)",
        "  gy <- ((y * N) + 1)",
        "  if ((gx < 1)) {",
        "    gx <- 1",
        "  }",
        "  if ((gx > N)) {",
        "    gx <- N",
        "  }",
        "  if ((gy < 1)) {",
        "    gy <- 1",
        "  }",
        "  if ((gy > N)) {",
        "    gy <- N",
        "  }",
        "  return(rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N))",
        "}",
        "",
    ]
    .join("\n");

    let out = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&input);

    assert!(
        out.contains("gx <- (pmin(pmax((x * N) + 1, 1), N))"),
        "{out}"
    );
    assert!(
        out.contains("gy <- (pmin(pmax((y * N) + 1, 1), N))"),
        "{out}"
    );
    assert!(!out.contains("if ((gx < 1)) {"), "{out}");
    assert!(!out.contains("if ((gy > N)) {"), "{out}");
}

#[test]
fn raw_emitted_gray_scott_clamp_pair_collapses_new_a_new_b() {
    let input = [
        "Sym_222 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
        "{",
        "  new_a <- (A[i] + ((((DA * lapA[i]) - (A[i] * B[i]) * B[i]) + (f * (1 - A[i]))) * dt))",
        "  new_b <- (B[i] + ((((DB * lapB[i]) + (A[i] * B[i]) * B[i]) - ((k + f) * B[i])) * dt))",
        "  if ((new_a < 0)) {",
        "    new_a <- 0",
        "  }",
        "  if ((new_a > 1)) {",
        "    new_a <- 1",
        "",
        "  }",
        "  if ((new_b < 0)) {",
        "    new_b <- 0",
        "  }",
        "  if ((new_b > 1)) {",
        "    new_b <- 1",
        "  }",
        "  return(new_b)",
        "}",
        "",
    ]
    .join("\n");

    let out = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&input);

    assert!(out.contains("new_a <- (pmin(pmax(A[i] + ((((DA * lapA[i]) - (A[i] * B[i]) * B[i]) + (f * (1 - A[i]))) * dt), 0), 1))"), "{out}");
    assert!(out.contains("new_b <- (pmin(pmax(B[i] + ((((DB * lapB[i]) + (A[i] * B[i]) * B[i]) - ((k + f) * B[i])) * dt), 0), 1))"), "{out}");
    assert!(!out.contains("if ((new_a < 0)) {"), "{out}");
    assert!(!out.contains("if ((new_b > 1)) {"), "{out}");
}

#[test]
fn raw_emitted_gray_scott_clamp_pair_collapses_float_literal_bounds() {
    let input = [
        "Sym_222 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
        "{",
        "  new_a <- expr_a",
        "  new_b <- expr_b",
        "  if ((new_a < 0.0)) {",
        "    new_a <- 0.0",
        "  }",
        "  if ((new_a > 1.0)) {",
        "    new_a <- 1.0",
        "  }",
        "  if ((new_b < 0.0)) {",
        "    new_b <- 0.0",
        "  }",
        "  if ((new_b > 1.0)) {",
        "    new_b <- 1.0",
        "  }",
        "  return(new_b)",
        "}",
        "",
    ]
    .join("\n");

    let out = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&input);

    assert!(out.contains("new_a <- (pmin(pmax(expr_a, 0), 1))"), "{out}");
    assert!(out.contains("new_b <- (pmin(pmax(expr_b, 0), 1))"), "{out}");
    assert!(!out.contains("if ((new_a < 0.0)) {"), "{out}");
    assert!(!out.contains("if ((new_b > 1.0)) {"), "{out}");
}

#[test]
fn raw_emitted_temp_copy_roundtrip_strips_before_gray_scott_clamp_collapse() {
    let input = [
        "Sym_222 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
        "{",
        "  new_a <- (A[i] + ((((DA * lapA[i]) - (A[i] * B[i]) * B[i]) + (f * (1 - A[i]))) * dt))",
        "  new_b <- (B[i] + ((((DB * lapB[i]) + (A[i] * B[i]) * B[i]) - ((k + f) * B[i])) * dt))",
        "  if ((new_a < 0)) {",
        "    new_a <- 0",
        "  }",
        "  if ((new_a > 1)) {",
        "    new_a <- 1",
        "    .__pc_src_tmp0 <- new_b",
        "    new_b <- .__pc_src_tmp0",
        "  }",
        "  if ((new_b < 0)) {",
        "    new_b <- 0",
        "  }",
        "  if ((new_b > 1)) {",
        "    new_b <- 1",
        "  }",
        "  return(new_b)",
        "}",
        "",
    ]
    .join("\n");

    let out = collapse_gray_scott_clamp_pair_in_raw_emitted_r(
        &strip_noop_temp_copy_roundtrips_in_raw_emitted_r(&input),
    );

    assert!(!out.contains(".__pc_src_tmp0 <- new_b"), "{out}");
    assert!(!out.contains("new_b <- .__pc_src_tmp0"), "{out}");
    assert!(out.contains("new_a <- (pmin(pmax("), "{out}");
    assert!(out.contains("new_b <- (pmin(pmax("), "{out}");
}

#[test]
fn raw_emitted_dead_temp_alias_strips_without_roundtrip_use() {
    let input = [
        "Sym_222 <- function(A, B) ",
        "{",
        "  new_b <- (A + B)",
        "  .__pc_src_tmp0 <- new_b",
        "  if ((new_b > 1.0)) {",
        "    new_b <- 1.0",
        "  }",
        "  return(new_b)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_noop_temp_copy_roundtrips_in_raw_emitted_r(&input);

    assert!(!out.contains(".__pc_src_tmp0 <- new_b"), "{out}");
    assert!(out.contains("new_b <- (A + B)"), "{out}");
}

#[test]
fn raw_emitted_sym287_melt_rate_branch_collapses_to_direct_heat_sink_updates() {
    let input = [
        "Sym_287 <- function(temp, q_s, q_g) ",
        "{",
        "  if ((T_c > 0)) {",
        "    melt_rate <- 0",
        "    if ((q_s[i] > 0)) {",
        "      melt_rate <- (q_s[i] * 0.05)",
        "    }",
        "    if ((q_g[i] > 0)) {",
        "      melt_rate <- (melt_rate + (q_g[i] * 0.02))",
        "    }",
        "    tendency_T <- (tendency_T - (melt_rate * L_f))",
        "  }",
        "  return(tendency_T)",
        "}",
        "",
    ]
    .join("\n");

    let out = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&input);

    assert!(!out.contains("melt_rate <- 0"), "{out}");
    assert!(!out.contains("melt_rate <- (q_s[i] * 0.05)"), "{out}");
    assert!(
        !out.contains("melt_rate <- (melt_rate + (q_g[i] * 0.02))"),
        "{out}"
    );
    assert!(
        !out.contains("tendency_T <- (tendency_T - (melt_rate * L_f))"),
        "{out}"
    );
    assert!(
        out.contains("tendency_T <- (tendency_T - ((q_s[i] * 0.05) * L_f))"),
        "{out}"
    );
    assert!(
        out.contains("tendency_T <- (tendency_T - ((q_g[i] * 0.02) * L_f))"),
        "{out}"
    );
}

#[test]
fn raw_emitted_exact_safe_loop_index_write_calls_rewrite_to_base_indexing() {
    let input = [
        "Sym_210 <- function(A, B, heat, SIZE) ",
        "{",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= SIZE)) break",
        "    A[rr_index1_write(i, \"index\")] <- new_a",
        "    B[rr_index1_write(i, \"index\")] <- new_b",
        "    heat[rr_index1_write(i, \"index\")] <- out_v",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&input);

    assert!(!out.contains("rr_index1_write(i, \"index\")"), "{out}");
    assert!(out.contains("A[i] <- new_a"), "{out}");
    assert!(out.contains("B[i] <- new_b"), "{out}");
    assert!(out.contains("heat[i] <- out_v"), "{out}");
}

#[test]
fn raw_emitted_literal_named_list_calls_rewrite_to_base_list() {
    let input = [
        "Sym_186 <- function(px, py, pf) ",
        "{",
        "  return(rr_named_list(\"px\", px, \"py\", py, \"pf\", pf))",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_literal_named_list_calls_in_raw_emitted_r(&input);

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
fn raw_emitted_literal_field_get_calls_rewrite_to_base_indexing() {
    let input = [
        "Sym_303 <- function(particles) ",
        "{",
        "  p_x <- rr_field_get(particles, \"px\")",
        "  p_y <- rr_field_get(particles, \"py\")",
        "  p_f <- rr_field_get(particles, \"pf\")",
        "  return(p_f)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_literal_field_get_calls_in_raw_emitted_r(&input);

    assert!(out.contains("p_x <- particles[[\"px\"]]"), "{out}");
    assert!(out.contains("p_y <- particles[[\"py\"]]"), "{out}");
    assert!(out.contains("p_f <- particles[[\"pf\"]]"), "{out}");
    assert!(!out.contains("rr_field_get(particles, \"px\")"), "{out}");
}

#[test]
fn raw_emitted_particle_state_rebinds_are_restored_after_sym186_call() {
    let input = [
        "Sym_top_0 <- function() ",
        "{",
        "  particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N)",
        "  print(p_x[1L])",
        "  return(0L)",
        "}",
        "",
    ]
    .join("\n");

    let out = restore_particle_state_rebinds_in_raw_emitted_r(&input);

    assert!(out.contains("p_x <- particles[[\"px\"]]"), "{out}");
    assert!(out.contains("p_y <- particles[[\"py\"]]"), "{out}");
    assert!(out.contains("p_f <- particles[[\"pf\"]]"), "{out}");
}

#[test]
fn raw_emitted_single_use_scalar_index_alias_rewrite_keeps_match_phi_assignments() {
    let input = "\
Sym_17 <- function(v)\n\
{\n\
  .arg_v <- v\n\
  if (((((TRUE & rr_field_exists(.arg_v, \"a\")) & TRUE) & rr_field_exists(.arg_v, \"b\")) & TRUE)) {\n\
    x <- .arg_v[[\"a\"]]\n\
    y <- .arg_v[[\"b\"]]\n\
    .phi_30 <- (x + y)\n\
  } else {\n\
    if (((TRUE & rr_field_exists(.arg_v, \"a\")) & TRUE)) {\n\
      x_3 <- .arg_v[[\"a\"]]\n\
      .phi_30 <- x_3\n\
    } else {\n\
      .phi_30 <- 0L\n\
    }\n\
  }\n\
  return(.phi_30)\n\
}\n";
    let out = rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(input);
    assert!(
        out.contains(".phi_30 <- (.arg_v[[\"a\"]] + .arg_v[[\"b\"]])"),
        "{out}"
    );
    assert!(out.contains(".phi_30 <- .arg_v[[\"a\"]]"), "{out}");
    assert!(out.contains(".phi_30 <- 0L"), "{out}");
}

#[test]
fn raw_emitted_slice_bound_aliases_inline_into_neighbor_row_writes() {
    let input = [
        "Sym_83 <- function(dir, size) ",
        "{",
        "  start <- rr_idx_cube_vec_i(f, x, 1, size)",
        "  end <- rr_idx_cube_vec_i(f, x, size, size)",
        "  if ((dir == 1)) {",
        "    neighbors[start:end] <- Sym_60(f, x, ys, size)",
        "  }",
        "  if ((dir == 2)) {",
        "    neighbors[start:end] <- Sym_64(f, x, ys, size)",
        "  }",
        "  return(neighbors)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_slice_bound_aliases_in_raw_emitted_r(&input);

    assert!(!out.contains("start <- rr_idx_cube_vec_i"), "{out}");
    assert!(!out.contains("end <- rr_idx_cube_vec_i"), "{out}");
    assert!(
    out.contains(
        "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)"
    ),
    "{out}"
);
    assert!(
    out.contains(
        "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)"
    ),
    "{out}"
);
}

#[test]
fn raw_emitted_adjacent_dir_neighbor_row_branches_collapse_to_else_if_chain() {
    let input = [
    "Sym_83 <- function(dir, size) ",
    "{",
    "  if ((dir == 1)) {",
    "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)",
    "  }",
    "  if ((dir == 2)) {",
    "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)",
    "  }",
    "  if ((dir == 3)) {",
    "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, ys, size)",
    "  }",
    "  if ((dir == 4)) {",
    "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, ys, size)",
    "  }",
    "  return(neighbors)",
    "}",
    "",
]
.join("\n");

    let out = collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(&input);

    assert!(out.contains("} else if ((dir == 2)) {"), "{out}");
    assert!(out.contains("} else if ((dir == 3)) {"), "{out}");
    assert!(out.contains("} else if ((dir == 4)) {"), "{out}");
    assert!(!out.contains("  }\n  if ((dir == 2)) {"), "{out}");
}

#[test]
fn raw_emitted_single_assignment_loop_seed_literals_inline_into_next_vector_expr() {
    let input = [
        "Sym_210 <- function(field, w, h) ",
        "{",
        "  size <- (w * h)",
        "  i <- 1",
        "  lap <- (((i:size - 1) %% w) + ((i:size - 1) %% h))",
        "  return(lap)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&input);

    assert!(!out.contains("  i <- 1"), "{out}");
    assert!(
        out.contains("lap <- (((1:size - 1) %% w) + ((1:size - 1) %% h))"),
        "{out}"
    );
}

#[test]
fn raw_emitted_sym210_loop_seed_literal_inlines_into_laplacian_expr() {
    let input = [
    "Sym_210 <- function(field, w, h) ",
    "{",
    "  size <- (w * h)",
    "",
    "  i <- 1",
    "",
    "  lap <- ((((((rr_gather(field, rr_wrap_index_vec_i(((((i:size - 1) %% w) + 1) - 1), (floor(((i:size - 1) / w)) + 1), w, h)) + rr_gather(field, rr_wrap_index_vec_i(((((i:size - 1) %% w) + 1) + 1), (floor(((i:size - 1) / w)) + 1), w, h))) + rr_gather(field, rr_wrap_index_vec_i((((i:size - 1) %% w) + 1), ((floor(((i:size - 1) / w)) + 1) + 1), w, h))) + rr_gather(field, rr_wrap_index_vec_i((((i:size - 1) %% w) + 1), ((floor(((i:size - 1) / w)) + 1) - 1), w, h))) * 0.2) - field)",
    "  return(lap)",
    "}",
    "",
]
.join("\n");

    let out = rewrite_sym210_loop_seed_in_raw_emitted_r(&input);

    assert!(!out.contains("\n  i <- 1\n"), "{out}");
    assert!(out.contains("1:size - 1"), "{out}");
    assert!(!out.contains("i:size - 1"), "{out}");
}

#[test]
fn raw_emitted_unreachable_helper_definitions_prune_after_probe_energy_reuse() {
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
        "  probe_energy <- mean(abs(probe_vec))",
        "  return(probe_energy)",
        "}",
        "",
        "Sym_top_0 <- function() ",
        "{",
        "  return(Sym_303())",
        "}",
        "",
    ]
    .join("\n");

    let out = prune_unreachable_raw_helper_definitions(&input);

    assert!(out.contains("Sym_49 <- function(a, b)"), "{out}");
    assert!(!out.contains("Sym_51 <- function(a, b)"), "{out}");
    assert!(out.contains("Sym_303 <- function()"), "{out}");
    assert!(out.contains("Sym_top_0 <- function()"), "{out}");
}

#[test]
fn raw_emitted_unreachable_helper_prune_keeps_unquoted_symbol_references() {
    let input = [
    "Sym_49 <- function(a, b) ",
    "{",
    "  return(a + b)",
    "}",
    "",
    "Sym_49__typed_impl <- function(a, b) ",
    "{",
    "  return(a * b)",
    "}",
    "",
    "Sym_303 <- function() ",
    "{",
    "  probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(3, 4))",
    "  return(probe_vec)",
    "}",
    "",
    "Sym_top_0 <- function() ",
    "{",
    "  return(Sym_303())",
    "}",
    "",
]
.join("\n");

    let out = prune_unreachable_raw_helper_definitions(&input);

    assert!(!out.contains("Sym_49 <- function(a, b)"), "{out}");
    assert!(
        out.contains("Sym_49__typed_impl <- function(a, b)"),
        "{out}"
    );
    assert!(
        out.contains("rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"),
        "{out}"
    );
    assert!(out.contains("Sym_top_0 <- function()"), "{out}");
}

#[test]
fn raw_emitted_shadowed_simple_scalar_seed_assigns_prune_before_first_use() {
    let input = [
        "Sym_303 <- function(adj_l, adj_r, TOTAL) ",
        "{",
        "  i <- 1",
        "  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))",
        "  adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= TOTAL)) break",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(adj_ll)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&input);

    assert!(
        !out.contains("  i <- 1\n  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(
        out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(out.contains("  i <- 1\n  repeat {"), "{out}");
}

#[test]
fn raw_emitted_dead_weno_topology_seed_i_prunes_before_direct_adj_gather() {
    let input = [
        "Sym_303 <- function(adj_l, adj_r, TOTAL) ",
        "{",
        "  i <- 1",
        "  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))",
        "  adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))",
        "  h_trn <- rep.int(0, TOTAL)",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= TOTAL)) break",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(adj_ll)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(&input);

    assert!(
        !out.contains("  i <- 1\n  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(
        out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
        "{out}"
    );
    assert!(out.contains("  i <- 1\n  repeat {"), "{out}");
}
