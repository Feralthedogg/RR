use super::*;

#[test]
pub(crate) fn raw_emitted_single_use_named_scalar_pure_calls_inline_wrap_index_reads() {
    let input = [
        "Sym_222 <- function(B, x, y, W, H) ",
        "{",
        "  id <- rr_wrap_index_vec_i(x, y, W, H)",
        "  B[id] <- 1",
        "  center_idx <- rr_wrap_index_vec_i(32, 32, W, H)",
        "  print(B[center_idx])",
        "  return(B)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_single_use_named_scalar_pure_calls_in_raw_emitted_r(&input);

    assert!(
        !out.contains("id <- rr_wrap_index_vec_i(x, y, W, H)"),
        "{out}"
    );
    assert!(
        !out.contains("center_idx <- rr_wrap_index_vec_i(32, 32, W, H)"),
        "{out}"
    );
    assert!(
        out.contains("B[rr_wrap_index_vec_i(x, y, W, H)] <- 1"),
        "{out}"
    );
    assert!(
        out.contains("print(B[rr_wrap_index_vec_i(32, 32, W, H)])"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_trivial_dot_product_wrapper_collapses_to_direct_sum() {
    let input = [
        "Sym_117 <- function(a, b, n) ",
        "{",
        "  sum <- 0",
        "  i <- 1",
        "  # rr-cse-pruned",
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

    let out = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&input);

    assert!(
        out.contains("return(sum((a[seq_len(n)] * b[seq_len(n)])))"),
        "{out}"
    );
    assert!(!out.contains("sum <- 0"), "{out}");
    assert!(!out.contains("sum <- (sum + (a[i] * b[i]))"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_trivial_dot_product_wrapper_with_parenthesized_iter_collapses() {
    let input = [
        "Sym_117 <- function(a, b, n) ",
        "{",
        "  sum <- 0",
        "  i <- 1",
        "  # rr-cse-pruned",
        "  repeat {",
        "    if (!(i <= n)) break",
        "    sum <- (sum + (a[(i)] * b[(i)]))",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(sum)",
        "}",
        "",
    ]
    .join("\n");

    let out = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&input);

    assert!(
        out.contains("return(sum((a[seq_len(n)] * b[seq_len(n)])))"),
        "{out}"
    );
    assert!(!out.contains("sum <- (sum + (a[(i)] * b[(i)]))"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_two_use_named_scalar_pure_call_inlines_idx_into_dx_dy() {
    let input = [
        "Sym_186 <- function(f, gx, gy, N, u, v, dt) ",
        "{",
        "  idx <- rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)",
        "  dx <- ((u[idx] * dt) / 400000)",
        "  dy <- ((v[idx] * dt) / 400000)",
        "  return(dx)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_two_use_named_scalar_pure_calls_in_raw_emitted_r(&input);

    assert!(!out.contains("idx <- rr_idx_cube_vec_i"), "{out}");
    assert!(
        out.contains("dx <- ((u[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"),
        "{out}"
    );
    assert!(
        out.contains("dy <- ((v[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_particle_idx_alias_rewrites_into_dx_dy() {
    let input = [
        "Sym_186 <- function(f, gx, gy, N, u, v, dt) ",
        "{",
        "  idx <- rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)",
        "  dx <- ((u[idx] * dt) / 400000)",
        "  dy <- ((v[idx] * dt) / 400000)",
        "  return(dx)",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_particle_idx_alias_in_raw_emitted_r(&input);

    assert!(!out.contains("idx <- rr_idx_cube_vec_i"), "{out}");
    assert!(
        out.contains("dx <- ((u[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"),
        "{out}"
    );
    assert!(
        out.contains("dy <- ((v[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_static_record_scalarization_splits_list_temps() {
    let input = [
        "Sym_29 <- function() ",
        "{",
        "  .__rr_inline_expr_0 <- list(x = (10.0 + 2.0), y = (15.0 + -3.0))",
        "  .__rr_inline_expr_1 <- list(x = (0.0 - 2.0), y = (0.0 - -3.0))",
        "  .__rr_inline_expr_2 <- list(x = (.__rr_inline_expr_0[[\"x\"]] + (.__rr_inline_expr_1)[[\"x\"]]), y = (.__rr_inline_expr_0[[\"y\"]] + (.__rr_inline_expr_1)[[\"y\"]]))",
        "  final_state <- list(x = (.__rr_inline_expr_2[[\"x\"]] * 1.5), y = (.__rr_inline_expr_2[[\"y\"]] * 1.5))",
        "  return(final_state[[\"x\"]])",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_static_record_scalarization_in_raw_emitted_r(&input);

    assert!(
        out.contains(".__rr_inline_expr_0__rr_sroa_x <- (10.0 + 2.0)"),
        "{out}"
    );
    assert!(
        out.contains(
            ".__rr_inline_expr_2__rr_sroa_x <- (.__rr_inline_expr_0__rr_sroa_x + .__rr_inline_expr_1__rr_sroa_x)"
        ),
        "{out}"
    );
    assert!(out.contains("return(final_state__rr_sroa_x)"), "{out}");
    assert!(!out.contains("<- list("), "{out}");
}

#[test]
pub(crate) fn raw_emitted_static_record_scalarization_keeps_impure_literal_projection() {
    let input = [
        "Sym_29 <- function() ",
        "{",
        "  return(list(x = print(1), y = 2)[[\"y\"]])",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_static_record_scalarization_in_raw_emitted_r(&input);

    assert!(
        out.contains("return(list(x = print(1), y = 2)[[\"y\"]])"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_static_record_scalarization_skips_self_dependent_record() {
    let input = [
        "Sym_29 <- function() ",
        "{",
        "  p <- list(x = p[[\"x\"]], y = 2)",
        "  return(p[[\"x\"]])",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_static_record_scalarization_in_raw_emitted_r(&input);

    assert!(out.contains("p <- list(x = p[[\"x\"]], y = 2)"), "{out}");
    assert!(out.contains("return(p[[\"x\"]])"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_static_record_scalarization_skips_field_write_records() {
    let input = [
        "Sym_29 <- function() ",
        "{",
        "  cfg <- list(alpha = 1L, beta = 2L)",
        "  cfg[[\"alpha\"]] <- 7L",
        "  return(cfg[[\"alpha\"]])",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_static_record_scalarization_in_raw_emitted_r(&input);

    assert!(out.contains("cfg <- list(alpha = 1L, beta = 2L)"), "{out}");
    assert!(out.contains("cfg[[\"alpha\"]] <- 7L"), "{out}");
    assert!(out.contains("return(cfg[[\"alpha\"]])"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_loop_index_alias_ii_rewrites_to_i() {
    let input = [
        "Sym_186 <- function(u, du1, adv_u, TOTAL) ",
        "{",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= TOTAL)) break",
        "    ii <- i",
        "    u_stage[ii] <- (u[ii] + (du1[ii] - adv_u[ii]))",
        "    if ((u_stage[ii] > max_u)) {",
        "      max_u <- u_stage[ii]",
        "    }",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_loop_index_alias_ii_in_raw_emitted_r(&input);

    assert!(!out.contains("ii <- i"), "{out}");
    assert!(
        out.contains("u_stage[i] <- (u[i] + (du1[i] - adv_u[i]))"),
        "{out}"
    );
    assert!(out.contains("if ((u_stage[i] > max_u)) {"), "{out}");
    assert!(out.contains("max_u <- u_stage[i]"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_loop_index_alias_ii_keeps_alias_if_used_after_i_changes() {
    let input = [
        "Sym_1 <- function(x, n) ",
        "{",
        "  i <- 1",
        "  repeat {",
        "    if (!(i <= n)) break",
        "    ii <- i",
        "    out[ii] <- x[ii]",
        "    i <- (i + 1)",
        "    y <- ii",
        "    next",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_loop_index_alias_ii_in_raw_emitted_r(&input);

    assert!(out.contains("ii <- i"), "{out}");
    assert!(out.contains("out[i] <- x[i]"), "{out}");
    assert!(out.contains("y <- ii"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_blank_line_runs_compact() {
    let input = "Sym_1 <- function() \n{\n\n\n  x <- 1\n\n\n  return(x)\n}\n";
    let out = compact_blank_lines_in_raw_emitted_r(input);
    assert!(!out.contains("\n\n\n"), "{out}");
    assert!(out.contains("{\n\n  x <- 1\n\n  return(x)\n}"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_orphan_rr_cse_markers_before_repeat_prune() {
    let input = [
        "Sym_83 <- function(size) ",
        "{",
        "  f <- 1",
        "  # rr-cse-pruned",
        "  x <- 0",
        "  # rr-cse-pruned",
        "",
        "  repeat {",
        "    if (!(f <= size)) break",
        "    f <- (f + 1)",
        "    next",
        "  }",
        "  return(f)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_orphan_rr_cse_markers_before_repeat_in_raw_emitted_r(&input);

    assert!(!out.contains("# rr-cse-pruned"), "{out}");
    assert!(out.contains("f <- 1"), "{out}");
    assert!(out.contains("x <- 0"), "{out}");
    assert!(out.contains("repeat {"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_single_blank_spacers_prune_between_assignments_and_control() {
    let input = [
        "Sym_123 <- function() ",
        "{",
        "  x <- rep.int(0, size)",
        "",
        "  r <- b",
        "  iter <- 1",
        "",
        "  repeat {",
        "    if (!(iter <= 20)) break",
        "    rs_old <- 0.0000001",
        "",
        "  }",
        "  y <- 1",
        "",
        "  z <- 2",
        "  return(z)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

    assert!(!out.contains("x <- rep.int(0, size)\n\n  r <- b"), "{out}");
    assert!(!out.contains("iter <- 1\n\n  repeat {"), "{out}");
    assert!(!out.contains("rs_old <- 0.0000001\n\n  }"), "{out}");
    assert!(!out.contains("y <- 1\n\n  z <- 2"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_single_blank_spacers_prune_between_assignment_and_if() {
    let input = [
        "Sym_60 <- function(f, x, size) ",
        "{",
        "  ys <- seq_len(size)",
        "",
        "  if ((x > 1)) {",
        "    return(a)",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

    assert!(
        !out.contains("ys <- seq_len(size)\n\n  if ((x > 1)) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_single_blank_spacers_prune_after_control_open_before_returns() {
    let input = [
        "Sym_60 <- function(f, x, size) ",
        "{",
        "",
        "  if ((x > 1)) {",
        "",
        "    return(a)",
        "  } else if ((f == 1)) {",
        "",
        "    return(b)",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

    assert!(!out.contains("{\n\n  if ((x > 1)) {"), "{out}");
    assert!(!out.contains("if ((x > 1)) {\n\n    return(a)"), "{out}");
    assert!(
        !out.contains("} else if ((f == 1)) {\n\n    return(b)"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_single_blank_spacers_prune_between_closing_braces() {
    let input = [
        "Sym_60 <- function(f, x, size) ",
        "{",
        "  if ((x > 1)) {",
        "    return(a)",
        "  } else {",
        "    return(b)",
        "  }",
        "",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

    assert!(!out.contains("  }\n\n}"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_single_blank_spacers_prune_after_break_before_branch() {
    let input = [
        "Sym_83 <- function(dir, size) ",
        "{",
        "  repeat {",
        "    if (!(x <= size)) break",
        "",
        "    if ((dir == 1)) {",
        "      neighbors[i] <- 1",
        "    }",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

    assert!(
        !out.contains("if (!(x <= size)) break\n\n    if ((dir == 1)) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn raw_emitted_readonly_arg_aliases_rewrite_to_bare_params() {
    let input = [
        "Sym_83 <- function(dir, size) ",
        "{",
        "  .arg_dir <- dir",
        "  .arg_size <- size",
        "  ys <- seq_len(.arg_size)",
        "  if ((.arg_dir == 1)) {",
        "    return(ys)",
        "  }",
        "  return(seq_len(.arg_size))",
        "}",
        "",
    ]
    .join("\n");

    let out = rewrite_readonly_raw_arg_aliases_in_raw_emitted_r(&input);

    assert!(!out.contains(".arg_dir <- dir"), "{out}");
    assert!(!out.contains(".arg_size <- size"), "{out}");
    assert!(out.contains("ys <- seq_len(size)"), "{out}");
    assert!(out.contains("if ((dir == 1)) {"), "{out}");
    assert!(out.contains("return(seq_len(size))"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_noop_self_assignments_prune() {
    let input = [
        "Sym_287 <- function(temp) ",
        "{",
        "  T_c <- (temp[1] - 273.15)",
        "  T_c <- T_c",
        "  return(T_c)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_noop_self_assignments_in_raw_emitted_r(&input);

    assert!(out.contains("T_c <- (temp[1] - 273.15)"), "{out}");
    assert!(!out.contains("T_c <- T_c"), "{out}");
    assert!(out.contains("return(T_c)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_empty_else_blocks_prune() {
    let input = [
        "Sym_83 <- function(dir) ",
        "{",
        "  if ((dir == 1)) {",
        "    return(1)",
        "  } else {",
        "  }",
        "  return(0)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_empty_else_blocks_in_raw_emitted_r(&input);

    assert!(!out.contains("} else {\n  }"), "{out}");
    assert!(out.contains("  }\n  return(0)"), "{out}");
}

#[test]
pub(crate) fn raw_emitted_branch_local_vec_fill_rebinds_prune_before_peephole() {
    let input = [
        "Sym_123 <- function(size) ",
        "{",
        "  x <- rep.int(0, size)",
        "  if ((bad == 1)) {",
        "    x <- Sym_17(size, 0)",
        "  }",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_redundant_branch_local_vec_fill_rebinds_in_raw_emitted_r(&input);

    assert!(out.contains("x <- rep.int(0, size)"), "{out}");
    assert!(!out.contains("x <- Sym_17(size, 0)"), "{out}");
    assert!(out.contains("return(x)"), "{out}");
}

#[test]
pub(crate) fn emitted_r_cache_keys_partition_o3_and_oz() {
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let raw = "Sym_1 <- function(x) {\n  return(x + 1L)\n}\n";
    let preserve_all_defs = false;
    let compile_mode = CompileMode::Standard;
    let cache_options = |opt_level| OutputCacheKeyOptions {
        opt_level,
        direct_builtin_call_map: false,
        preserve_all_defs,
        compile_mode,
    };

    let peephole_o2 = peephole_output_cache_key(raw, &pure, &fresh, cache_options(OptLevel::O2));
    let peephole_o3 = peephole_output_cache_key(raw, &pure, &fresh, cache_options(OptLevel::O3));
    let peephole_oz = peephole_output_cache_key(raw, &pure, &fresh, cache_options(OptLevel::Oz));
    assert_ne!(peephole_o2, peephole_o3);
    assert_ne!(peephole_o2, peephole_oz);
    assert_ne!(peephole_o3, peephole_oz);

    let opt_fragment_o2 =
        optimized_fragment_output_cache_key(raw, &pure, &fresh, cache_options(OptLevel::O2));
    let opt_fragment_o3 =
        optimized_fragment_output_cache_key(raw, &pure, &fresh, cache_options(OptLevel::O3));
    let opt_fragment_oz =
        optimized_fragment_output_cache_key(raw, &pure, &fresh, cache_options(OptLevel::Oz));
    assert_ne!(opt_fragment_o2, opt_fragment_o3);
    assert_ne!(opt_fragment_o2, opt_fragment_oz);
    assert_ne!(opt_fragment_o3, opt_fragment_oz);

    let opt_assembly_o2 =
        optimized_assembly_cache_key(raw, &pure, &fresh, cache_options(OptLevel::O2));
    let opt_assembly_o3 =
        optimized_assembly_cache_key(raw, &pure, &fresh, cache_options(OptLevel::O3));
    let opt_assembly_oz =
        optimized_assembly_cache_key(raw, &pure, &fresh, cache_options(OptLevel::Oz));
    assert_ne!(opt_assembly_o2, opt_assembly_o3);
    assert_ne!(opt_assembly_o2, opt_assembly_oz);
    assert_ne!(opt_assembly_o3, opt_assembly_oz);

    let raw_rewrite_o2 =
        raw_rewrite_output_cache_key(raw, OptLevel::O2, &pure, preserve_all_defs, compile_mode);
    let raw_rewrite_o3 =
        raw_rewrite_output_cache_key(raw, OptLevel::O3, &pure, preserve_all_defs, compile_mode);
    let raw_rewrite_oz =
        raw_rewrite_output_cache_key(raw, OptLevel::Oz, &pure, preserve_all_defs, compile_mode);
    assert_ne!(raw_rewrite_o2, raw_rewrite_o3);
    assert_ne!(raw_rewrite_o2, raw_rewrite_oz);
    assert_ne!(raw_rewrite_o3, raw_rewrite_oz);
}

#[test]
pub(crate) fn raw_named_scalar_expr_inline_skips_function_literals() {
    assert!(!is_inlineable_raw_named_scalar_expr("function(x)"));
    assert!(!is_inlineable_raw_named_scalar_expr("function (x)"));
    assert!(is_inlineable_raw_named_scalar_expr("(x + 1L)"));
}

#[test]
pub(crate) fn raw_shadowed_scalar_seed_prune_respects_else_boundaries() {
    let input = [
        "Sym_4 <- function() ",
        "{",
        "  if ((cond)) {",
        "    m <- 10L",
        "  } else {",
        "    m <- 0L",
        "  }",
        "  print(m)",
        "  return(m)",
        "}",
        "",
    ]
    .join("\n");

    let out = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&input);

    assert!(out.contains("    m <- 10L"), "{out}");
    assert!(out.contains("    m <- 0L"), "{out}");
}
