use super::common::*;
use super::*;

#[test]
pub(crate) fn prune_dead_cse_temps_removes_unused_chain() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  .__rr_cse_1 <- (a + b)",
        "  .__rr_cse_2 <- (.__rr_cse_1 * c)",
        "  x <- 1",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(!output.contains(".__rr_cse_1 <-"));
    assert!(!output.contains(".__rr_cse_2 <-"));
    assert!(output.contains("# rr-cse-pruned"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_keeps_live_temp() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  .__rr_cse_1 <- (a + b)",
        "  x <- .__rr_cse_1",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(output.contains(".__rr_cse_1 <- (a + b)"));
    assert!(!output.contains("# rr-cse-pruned"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_removes_unused_tachyon_callmap_temp() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  .tachyon_callmap_arg0_0 <- abs((x + y))",
        "  score <- pmax(abs((x + y)), 0.05)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(!output.contains(".tachyon_callmap_arg0_0 <-"));
    assert!(output.contains("# rr-cse-pruned"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_removes_unused_loop_seed_before_whole_assign() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  i_9 <- 1L",
        "  clean <- ifelse((score > 0.4), sqrt((score + 0.1)), ((score * 0.55) + 0.03))",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(!output.contains("i_9 <- 1L"));
    assert!(output.contains("# rr-cse-pruned"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_keeps_loop_seed_used_by_following_slice_assign() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  i <- 1L",
        "  out <- rr_assign_slice(out, i, length(xs), xs)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(output.contains("i <- 1L"));
    assert!(!output.contains("# rr-cse-pruned"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_keeps_seq_len_seed_used_by_call_map_slice() {
    let mut output = [
        "Sym <- function(src, idx) ",
        "{",
        "  out <- seq_len(length(idx))",
        "  i <- 2L",
        "  .tachyon_callmap_arg0_0 <- rr_gather(src, rr_index1_read_vec(idx, rr_index_vec_floor(i:length(out))))",
        "  out <- rr_call_map_slice_auto(out, i, length(out), \"abs\", 14L, c(1L), .tachyon_callmap_arg0_0)",
        "  return(out)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(output.contains("out <- seq_len(length(idx))"), "{output}");
    assert!(
        output.contains("out <- rr_call_map_slice_auto(out,"),
        "{output}"
    );
}

#[test]
pub(crate) fn strip_dead_seq_len_locals_keeps_call_map_slice_seed() {
    let mut output = [
        "Sym <- function(src, idx) ",
        "{",
        "  out <- seq_len(length(idx))",
        "  out <- rr_call_map_slice_auto(out, 2L, length(out), \"abs\", 14L, c(1L), rr_gather(src, rr_index1_read_vec(idx, rr_index_vec_floor(2L:length(out)))))",
        "  return(out)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::strip_dead_seq_len_locals(&mut output);
    assert!(output.contains("out <- seq_len(length(idx))"), "{output}");
    assert!(
        output.contains("out <- rr_call_map_slice_auto(out,"),
        "{output}"
    );
}

#[test]
pub(crate) fn final_emit_cleanup_keeps_call_map_slice_seed() {
    let mut output = [
        "Sym_1 <- function(src, idx) ",
        "{",
        "  .arg_src <- src",
        "  .arg_idx <- idx",
        "  out <- seq_len(length(.arg_idx))",
        "  i <- 2L",
        "  .tachyon_callmap_arg0_0 <- rr_gather(.arg_src, rr_index1_read_vec(.arg_idx, rr_index_vec_floor(i:length(out))))",
        "  out <- rr_call_map_slice_auto(out, i, length(out), \"abs\", 14L, c(1L), .tachyon_callmap_arg0_0)",
        "  return(out)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    RBackend::rewrite_safe_scalar_loop_index_helpers(&mut output);
    RBackend::rewrite_branch_local_identical_alloc_rebinds(&mut output);
    RBackend::hoist_branch_local_pure_scalar_assigns_used_after_branch(&mut output);
    RBackend::rewrite_single_use_scalar_index_aliases(&mut output);
    RBackend::rewrite_immediate_and_guard_named_scalar_exprs(&mut output);
    RBackend::rewrite_two_use_named_scalar_exprs(&mut output);
    RBackend::rewrite_small_multiuse_scalar_index_aliases(&mut output);
    RBackend::rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(&mut output);
    RBackend::rewrite_named_scalar_pure_call_aliases(&mut output);
    RBackend::rewrite_loop_index_alias_ii(&mut output);
    RBackend::strip_dead_zero_seed_ii(&mut output);
    RBackend::rewrite_slice_bound_aliases(&mut output);
    RBackend::rewrite_particle_idx_alias(&mut output);
    RBackend::rewrite_adjacent_duplicate_symbol_assignments(&mut output);
    RBackend::rewrite_duplicate_pure_call_assignments(&mut output, &FxHashSet::default());
    RBackend::strip_noop_self_assignments(&mut output);
    RBackend::rewrite_temp_uses_after_named_copy(&mut output);
    RBackend::strip_noop_temp_copy_roundtrips(&mut output);
    RBackend::strip_dead_simple_scalar_assigns(&mut output);
    RBackend::strip_shadowed_simple_scalar_seed_assigns(&mut output);
    RBackend::strip_dead_seq_len_locals(&mut output);
    RBackend::strip_redundant_branch_local_vec_fill_rebinds(&mut output);
    RBackend::strip_unused_raw_arg_aliases(&mut output);
    RBackend::rewrite_readonly_raw_arg_aliases(&mut output);
    RBackend::strip_empty_else_blocks(&mut output);
    RBackend::collapse_nested_else_if_blocks(&mut output);
    RBackend::rewrite_guard_scalar_literals(&mut output);
    RBackend::rewrite_loop_guard_scalar_literals(&mut output);
    RBackend::rewrite_single_assignment_loop_seed_literals(&mut output);
    RBackend::rewrite_sym210_loop_seed(&mut output);
    RBackend::rewrite_seq_len_full_overwrite_inits(&mut output);
    assert!(
        output.contains("out <- seq_len(length(idx))")
            || output.contains("out <- seq_len(length(.arg_idx))"),
        "{output}"
    );
    assert!(
        output.contains("out <- rr_call_map_slice_auto(out,"),
        "{output}"
    );
}

#[test]
pub(crate) fn duplicate_pure_call_reuse_stops_at_indexed_store_to_dep() {
    let mut output = [
        "Sym <- function(u_stage, v, TOTAL) ",
        "{",
        "  visc2 <- Sym_156(u_stage, v, TOTAL)",
        "  i <- 1L",
        "  repeat {",
        "    if (!(i <= TOTAL)) break",
        "    u_stage[i] <- (u_stage[i] + 1)",
        "    i <- (i + 1L)",
        "  }",
        "  visc3 <- Sym_156(u_stage, v, TOTAL)",
        "  return(visc3)",
        "}",
        "",
    ]
    .join("\n");
    let pure = FxHashSet::from_iter([String::from("Sym_156")]);

    RBackend::rewrite_duplicate_pure_call_assignments(&mut output, &pure);

    assert!(
        output.contains("visc3 <- Sym_156(u_stage, v, TOTAL)"),
        "{output}"
    );
    assert!(!output.contains("visc3 <- visc2"), "{output}");
}

#[test]
pub(crate) fn prune_dead_cse_temps_removes_straight_line_dead_init_before_overwrite() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  y <- Sym_17(n, 0, 2)",
        "  tmp <- 0",
        "  y <- (a + b)",
        "  return(y)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(!output.contains("y <- Sym_17(n, 0, 2)"));
    assert!(output.contains("# rr-cse-pruned"));
    assert!(output.contains("y <- (a + b)"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_keeps_init_when_overwrite_is_not_straight_line() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  x <- rep.int(0, n)",
        "  if ((flag == 1)) {",
        "    x <- vals",
        "  } else {",
        "  }",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(output.contains("x <- rep.int(0, n)"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_keeps_init_when_overwrite_reads_previous_value() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  x <- rep.int(0, n)",
        "  x <- (x + p)",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(output.contains("x <- rep.int(0, n)"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_removes_globally_unused_scalar_init() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  ii <- 0",
        "  x <- y",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(!output.contains("ii <- 0"));
    assert!(output.contains("# rr-cse-pruned"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_keeps_scalar_init_that_is_later_used() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  ii <- 0",
        "  x <- (ii + 1)",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(output.contains("ii <- 0"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_compacts_adjacent_pruned_markers() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  a <- 0",
        "  b <- 0",
        "  c <- 0",
        "  x <- 1",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert_eq!(output.matches("# rr-cse-pruned").count(), 1);
}

#[test]
pub(crate) fn prune_dead_cse_temps_does_not_treat_other_function_uses_as_live() {
    let mut output = [
        "Sym_a <- function() ",
        "{",
        "  ii <- 0",
        "  x <- 1",
        "  return(x)",
        "}",
        "",
        "Sym_b <- function() ",
        "{",
        "  ii <- 0",
        "  ii <- 1",
        "  return(ii)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(
        !output.contains("Sym_a <- function() \n{\n  ii <- 0"),
        "dead init in first function should be pruned even if the same symbol is used in a later function"
    );
    assert!(
        output.contains("Sym_b <- function() \n{\n  ii <- 0")
            || output.contains("Sym_b <- function() \n{\n  # rr-cse-pruned\n  ii <- 1"),
        "second function should remain structurally intact"
    );
}

#[test]
pub(crate) fn prune_dead_cse_temps_removes_dead_pre_loop_init_overwritten_in_loop() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  i <- 1",
        "  x <- 0",
        "  repeat {",
        "    if (!(i <= n)) break",
        "    x <- vals[i]",
        "    y <- x",
        "    i <- (i + 1)",
        "    next",
        "  }",
        "  return(y)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(!output.contains("  x <- 0\n"));
    assert!(output.contains("# rr-cse-pruned"));
}

#[test]
pub(crate) fn prune_dead_cse_temps_keeps_pre_loop_init_used_after_loop() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  i <- 1",
        "  x <- 0",
        "  repeat {",
        "    if (!(i <= n)) break",
        "    x <- vals[i]",
        "    break",
        "  }",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");
    RBackend::prune_dead_cse_temps(&mut output);
    assert!(output.contains("  x <- 0\n"));
}

#[test]
pub(crate) fn restore_missing_generated_poly_loop_steps_reinserts_repeat_increments() {
    let mut output = [
            "Sym <- function() ",
            "{",
            ".__poly_gen_iv_tile_2_r <- 1L",
            ".__poly_gen_iv_tile_2_c <- .__poly_gen_iv_tile_2_r",
            ".__poly_gen_iv_2_r <- 1L",
            ".__poly_gen_iv_2_c <- .__poly_gen_iv_2_r",
            "  repeat {",
            "    if (!rr_truthy1((.__poly_gen_iv_tile_2_r <= n), \"condition\")) break",
            ".__poly_gen_iv_tile_2_c <- 1L",
            "    repeat {",
            "      if (!rr_truthy1((.__poly_gen_iv_tile_2_c <= m), \"condition\")) break",
            ".__poly_gen_iv_2_r <- .__poly_gen_iv_tile_2_r",
            "      repeat {",
            "        if (!rr_truthy1(((.__poly_gen_iv_2_r <= n) & (.__poly_gen_iv_2_r <= (.__poly_gen_iv_tile_2_r + 3L))), \"condition\")) break",
            ".__poly_gen_iv_2_c <- .__poly_gen_iv_tile_2_c",
            "        repeat {",
            "          if (!rr_truthy1(((.__poly_gen_iv_2_c <= m) & (.__poly_gen_iv_2_c <= (.__poly_gen_iv_tile_2_c + 3L))), \"condition\")) break",
            "          out[.__poly_gen_iv_2_r, .__poly_gen_iv_2_c] <- 0L",
            "        }",
            "      }",
            "    }",
            "  }",
            "}",
            "",
        ]
        .join("\n");
    RBackend::restore_missing_generated_poly_loop_steps(&mut output);
    assert!(
        output.contains(".__poly_gen_iv_tile_2_r <- (.__poly_gen_iv_tile_2_r + 4L)"),
        "{output}"
    );
    assert!(
        output.contains(".__poly_gen_iv_tile_2_c <- (.__poly_gen_iv_tile_2_c + 4L)"),
        "{output}"
    );
    assert!(
        output.contains(".__poly_gen_iv_2_r <- (.__poly_gen_iv_2_r + 1L)"),
        "{output}"
    );
    assert!(
        output.contains(".__poly_gen_iv_2_c <- (.__poly_gen_iv_2_c + 1L)"),
        "{output}"
    );
}

#[test]
pub(crate) fn restore_missing_generated_poly_loop_steps_inserts_before_terminal_next() {
    let mut output = [
        "Sym <- function() ",
        "{",
        "  .__poly_gen_iv_2_r <- 1L",
        "  repeat {",
        "    if (!rr_truthy1((.__poly_gen_iv_2_r <= n), \"condition\")) break",
        "    out[.__poly_gen_iv_2_r] <- src[.__poly_gen_iv_2_r]",
        "    next",
        "  }",
        "}",
        "",
    ]
    .join("\n");

    RBackend::restore_missing_generated_poly_loop_steps(&mut output);
    RBackend::strip_terminal_repeat_nexts(&mut output);

    assert!(
        output.contains(
            "    out[.__poly_gen_iv_2_r] <- src[.__poly_gen_iv_2_r]\n    .__poly_gen_iv_2_r <- (.__poly_gen_iv_2_r + 1L)\n  }"
        ),
        "{output}"
    );
    assert!(
        !output.contains("    next\n    .__poly_gen_iv_2_r <-"),
        "{output}"
    );
}

#[test]
pub(crate) fn invalidate_emitted_cse_temps_drops_stale_binding() {
    let mut backend = RBackend::new();
    backend.note_var_write(".__rr_cse_7");
    backend.bind_value_to_var(7, ".__rr_cse_7");
    backend
        .emit_scratch
        .emitted_temp_names
        .push(".__rr_cse_7".to_string());

    assert_eq!(
        backend.resolve_bound_value(7).as_deref(),
        Some(".__rr_cse_7")
    );

    backend.invalidate_emitted_cse_temps();

    assert!(backend.resolve_bound_value(7).is_none());
    assert!(backend.emit_scratch.emitted_temp_names.is_empty());
}

#[test]
pub(crate) fn stale_cse_temp_still_rewrites_full_range_alias_reads() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Const(Lit::Int(8)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Range { start: 1, end: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Int, true),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .loop_analysis
        .known_full_end_exprs
        .insert("x".to_string(), "8L".to_string());
    backend
        .loop_analysis
        .known_full_end_exprs
        .insert("p".to_string(), "8L".to_string());
    backend.note_var_write(".__rr_cse_218");
    backend.bind_var_to_value(".__rr_cse_218", 2);
    backend.note_var_write(".__rr_cse_218");

    let expr =
        "(rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))";
    let rewritten = backend.rewrite_known_one_based_full_range_alias_reads(expr, &values, &[]);

    assert_eq!(rewritten, "(x + (alpha * p))");
}

#[test]
pub(crate) fn stale_cse_temp_allows_direct_full_range_read_elision() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Const(Lit::Int(8)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Range { start: 1, end: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Int, true),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Load {
                var: "x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, true),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Load {
                var: ".__rr_cse_218".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some(".__rr_cse_218".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Int, true),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .loop_analysis
        .known_full_end_exprs
        .insert("x".to_string(), "8L".to_string());
    backend.note_var_write(".__rr_cse_218");
    backend.bind_var_to_value(".__rr_cse_218", 2);
    backend.note_var_write(".__rr_cse_218");

    let rendered = backend.resolve_call_expr(
        &values[0],
        "rr_index1_read_vec",
        &[3, 4],
        &[None, None],
        &values,
        &[],
    );

    assert_eq!(rendered, "x");
}
