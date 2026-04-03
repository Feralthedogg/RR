use super::common::*;

#[test]
fn prune_dead_cse_temps_removes_unused_chain() {
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
fn prune_dead_cse_temps_keeps_live_temp() {
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
fn prune_dead_cse_temps_removes_unused_tachyon_callmap_temp() {
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
fn prune_dead_cse_temps_removes_unused_loop_seed_before_whole_assign() {
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
fn prune_dead_cse_temps_keeps_loop_seed_used_by_following_slice_assign() {
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
fn prune_dead_cse_temps_removes_straight_line_dead_init_before_overwrite() {
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
fn prune_dead_cse_temps_keeps_init_when_overwrite_is_not_straight_line() {
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
fn prune_dead_cse_temps_keeps_init_when_overwrite_reads_previous_value() {
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
fn prune_dead_cse_temps_removes_globally_unused_scalar_init() {
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
fn prune_dead_cse_temps_keeps_scalar_init_that_is_later_used() {
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
fn prune_dead_cse_temps_compacts_adjacent_pruned_markers() {
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
fn prune_dead_cse_temps_does_not_treat_other_function_uses_as_live() {
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
fn prune_dead_cse_temps_removes_dead_pre_loop_init_overwritten_in_loop() {
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
fn prune_dead_cse_temps_keeps_pre_loop_init_used_after_loop() {
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
fn restore_missing_generated_poly_loop_steps_reinserts_repeat_increments() {
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
fn invalidate_emitted_cse_temps_drops_stale_binding() {
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
fn stale_cse_temp_still_rewrites_full_range_alias_reads() {
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
fn stale_cse_temp_allows_direct_full_range_read_elision() {
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
