use super::*;

#[test]
pub(crate) fn rewrites_whole_slice_patterns() {
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
pub(crate) fn rewrites_nested_vector_helper_subcalls() {
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
pub(crate) fn o3_aggressive_gather_cse_hoists_profitable_repeats_and_strips_marks() {
    let input = "\
Sym_1 <- function(x, idx) \n\
{\n\
  rr_mark(10L, 3L);\n\
  y <- (((rr_gather(x, rr_index_vec_floor(idx)) + rr_gather(x, rr_index_vec_floor(idx))) * 2.0) + rr_gather(x, rr_index_vec_floor(idx)))\n\
  return(y)\n\
}\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::O3),
    );
    assert!(!out.contains("rr_mark("), "{out}");
    assert!(out.contains("idx_idx <- rr_index_vec_floor(idx)"), "{out}");
    assert!(out.contains("x_idx <- rr_gather(x, idx_idx)"), "{out}");
    assert_eq!(out.matches("rr_gather(").count(), 1, "{out}");
    assert_eq!(out.matches("rr_index_vec_floor(idx)").count(), 1, "{out}");
}

#[test]
pub(crate) fn o3_aggressive_gather_cse_respects_code_size_budget() {
    let input = "\
Sym_1 <- function(x, i) \n\
{\n\
  y <- (rr_gather(x, i) + rr_gather(x, i))\n\
  return(y)\n\
}\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::O3),
    );
    assert!(!out.contains(".__rr_cse_"), "{out}");
    assert_eq!(out.matches("rr_gather(").count(), 2, "{out}");
}

#[test]
pub(crate) fn o2_keeps_debug_marks_and_skips_o3_gather_cse() {
    let input = "\
Sym_1 <- function(x, idx) \n\
{\n\
  rr_mark(10L, 3L);\n\
  y <- (((rr_gather(x, rr_index_vec_floor(idx)) + rr_gather(x, rr_index_vec_floor(idx))) * 2.0) + rr_gather(x, rr_index_vec_floor(idx)))\n\
  return(y)\n\
}\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::O2),
    );
    assert!(out.contains("rr_mark("), "{out}");
    assert!(!out.contains(".__rr_cse_"), "{out}");
    assert_eq!(out.matches("rr_gather(").count(), 3, "{out}");
}

#[test]
pub(crate) fn o3_hoists_repeated_index_vec_floor_across_straight_line_region() {
    let out = hoist_o3_repeated_index_vec_floor_calls_across_lines(vec![
        "  a <- rr_gather(x, rr_index_vec_floor(adj_r))".to_string(),
        "  b <- rr_gather(y, rr_index_vec_floor(adj_r))".to_string(),
        "  c <- rr_gather(z, rr_index_vec_floor(adj_r))".to_string(),
    ])
    .join("\n");
    assert_eq!(out.matches("rr_index_vec_floor(adj_r)").count(), 1, "{out}");
    assert!(
        out.contains("rr_gather(x, .__rr_cse_")
            && out.contains("rr_gather(y, .__rr_cse_")
            && out.contains("rr_gather(z, .__rr_cse_"),
        "{out}"
    );
}

#[test]
pub(crate) fn o3_index_vec_floor_hoist_sees_through_debug_marks() {
    let out = hoist_o3_repeated_index_vec_floor_calls_across_lines(vec![
        "  field_n_r_rr <- rr_gather(field, rr_index_vec_floor(rr_gather(n_r, rr_index_vec_floor(n_rr))))".to_string(),
        "  rr_mark(1227L, 9L);".to_string(),
        "  field_rr <- rr_gather(field, rr_index_vec_floor(n_rr))".to_string(),
    ])
    .join("\n");
    assert_eq!(out.matches("rr_index_vec_floor(n_rr)").count(), 1, "{out}");
    assert!(out.contains(".__rr_cse_"), "{out}");
    assert!(out.contains("rr_gather(n_r, .__rr_cse_"), "{out}");
    assert!(out.contains("rr_gather(field, .__rr_cse_"), "{out}");
}

#[test]
pub(crate) fn o3_late_index_vec_floor_hoist_runs_after_mark_strip() {
    let input = "\
Sym_1 <- function(field, n_r, n_rr) \n\
{\n\
  rr_mark(10L, 3L);\n\
  field_n_r_rr <- rr_gather(field, rr_index_vec_floor(rr_gather(n_r, rr_index_vec_floor(n_rr))))\n\
  rr_mark(11L, 3L);\n\
  field_rr <- rr_gather(field, rr_index_vec_floor(n_rr))\n\
  return(field_n_r_rr + field_rr)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  field <- rep.int(1.0, 4)\n\
  n_r <- seq_len(4)\n\
  n_rr <- c(4.0, 3.0, 2.0, 1.0)\n\
  return(Sym_1(field, n_r, n_rr))\n\
}\n\
Sym_top_0()\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::O3),
    );
    assert!(!out.contains("rr_mark("), "{out}");
    assert_eq!(out.matches("rr_index_vec_floor(n_rr)").count(), 1, "{out}");
    assert!(out.contains("Sym_1 <- function(field, n_r, n_rr)"), "{out}");
    assert!(out.contains("Sym_1(field, n_r, n_rr)"), "{out}");
    assert!(out.contains("idx_rr <- rr_index_vec_floor(n_rr)"), "{out}");
    assert!(out.contains("rr_gather(n_r, idx_rr)"), "{out}");
}

#[test]
pub(crate) fn o3_materializes_exact_gather_index_temps() {
    let out = materialize_o3_exact_gather_index_temps(vec![
        "  field_r <- rr_gather(field, rr_index_vec_floor(n_r))".to_string(),
        "  idx_rr <- rr_index_vec_floor(n_rr)".to_string(),
        "  field_n_r_rr <- rr_gather(field, rr_index_vec_floor(rr_gather(n_r, idx_rr)))"
            .to_string(),
    ])
    .join("\n");
    assert!(out.contains("idx_r <- rr_index_vec_floor(n_r)"), "{out}");
    assert!(out.contains("field_r <- rr_gather(field, idx_r)"), "{out}");
    assert!(out.contains("n_r_rr <- rr_gather(n_r, idx_rr)"), "{out}");
    assert!(
        out.contains("idx_r_rr <- rr_index_vec_floor(n_r_rr)"),
        "{out}"
    );
    assert!(
        out.contains("field_n_r_rr <- rr_gather(field, idx_r_rr)"),
        "{out}"
    );
    assert!(!out.contains("rr_index_vec_floor(rr_gather("), "{out}");
}

#[test]
pub(crate) fn o3_repairs_missing_semantic_gather_temp_before_index_temp() {
    let (lines, _) = define_missing_o3_semantic_index_temps_with_map(vec![
        "Sym_1 <- function(field, n_r, n_rr)".to_string(),
        "{".to_string(),
        "  idx_r_rr <- rr_index_vec_floor(n_r_rr)".to_string(),
        "  field_r_rr <- rr_gather(field, idx_r_rr)".to_string(),
        "}".to_string(),
    ]);
    let out = lines.join("\n");
    assert!(out.contains("n_r_rr <- rr_gather(n_r, n_rr)"), "{out}");
    assert!(
        out.contains("idx_r_rr <- rr_index_vec_floor(n_r_rr)"),
        "{out}"
    );
}

#[test]
pub(crate) fn o3_repairs_missing_semantic_index_temp_from_defined_adjacent_vector() {
    let (lines, _) = define_missing_o3_semantic_index_temps_with_map(vec![
        "Sym_1 <- function()".to_string(),
        "{".to_string(),
        "  adj_l <- c(1.0, 2.0)".to_string(),
        "  u_l <- rr_gather(u, idx_l_3)".to_string(),
        "}".to_string(),
    ]);
    let out = lines.join("\n");
    assert!(
        out.contains("idx_l_3 <- rr_index_vec_floor(adj_l)"),
        "{out}"
    );
    assert!(out.contains("u_l <- rr_gather(u, idx_l_3)"), "{out}");
}

#[test]
pub(crate) fn o3_repairs_missing_semantic_index_temp_prefers_neighbor_param_over_shadow() {
    let (lines, _) = define_missing_o3_semantic_index_temps_with_map(vec![
        "Sym_1 <- function(p, n_r)".to_string(),
        "{".to_string(),
        "  r <- p".to_string(),
        "  Ap <- rr_gather(p, idx_r)".to_string(),
        "}".to_string(),
    ]);
    let out = lines.join("\n");
    assert!(out.contains("idx_r <- rr_index_vec_floor(n_r)"), "{out}");
    assert!(!out.contains("idx_r <- rr_index_vec_floor(r)"), "{out}");
}

#[test]
pub(crate) fn o3_materializes_repeated_large_arithmetic_subexpressions() {
    let repeated = "((1.0833 * ((left - (2.0 * mid)) + right)) * ((left - (2.0 * mid)) + right))";
    let rhs = [repeated; 12].join(" + ");
    let input = format!("  score <- {rhs}");
    let out =
        materialize_o3_large_repeated_arithmetic_subexpressions(vec![input.clone()]).join("\n");
    assert!(out.contains(".__rr_cse_"), "{out}");
    assert!(out.contains("score <-"), "{out}");
    assert!(out.len() < input.len(), "{out}");
}

#[test]
pub(crate) fn o3_materializes_large_ifelse_branches() {
    let branch = (0..24)
        .map(|idx| format!("((x + {idx}.0) * (y - {idx}.0))"))
        .collect::<Vec<_>>()
        .join(" + ");
    let input = format!("  flux <- ifelse(cond, (u * ({branch})), fallback)");
    let out = materialize_o3_large_ifelse_branches(vec![input]).join("\n");
    assert!(out.contains(".__rr_cse_"), "{out}");
    assert!(out.contains("flux <- ifelse(cond, .__rr_cse_"), "{out}");
}

#[test]
pub(crate) fn o3_materializes_large_root_arithmetic_branches() {
    let left = (0..8)
        .map(|idx| format!("((a + {idx}.0) * (b - {idx}.0))"))
        .collect::<Vec<_>>()
        .join(" + ");
    let right = (8..16)
        .map(|idx| format!("((a + {idx}.0) * (b - {idx}.0))"))
        .collect::<Vec<_>>()
        .join(" + ");
    let input = format!("  flux_pos <- (u_vel * (({left}) - ({right})))");
    let out = materialize_o3_large_repeated_arithmetic_subexpressions(vec![input]).join("\n");
    assert!(out.contains(".__rr_cse_"), "{out}");
    assert!(out.contains("flux_pos <- (u_vel * .__rr_cse_"), "{out}");
    assert!(
        out.lines().all(|line| line.len() < 430),
        "expected root split to cap long lines:\n{out}"
    );
}

pub(crate) fn assert_generated_cse_refs_are_defined(code: &str) {
    let mut defs = FxHashSet::default();
    for line in code.lines() {
        if let Some(lhs) = assign_re()
            .and_then(|re| re.captures(line.trim()))
            .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
            && is_generated_split_temp(&lhs)
        {
            defs.insert(lhs);
        }
    }

    for temp in expr_idents(code)
        .into_iter()
        .filter(|ident| is_generated_split_temp(ident))
    {
        assert!(
            defs.contains(&temp),
            "undefined generated temp {temp} in:\n{code}"
        );
    }
}

pub(crate) fn is_generated_split_temp(name: &str) -> bool {
    name.starts_with(".__rr_cse_") || name.starts_with(".__rr_expr_")
}

#[test]
pub(crate) fn o3_cse_lhs_root_splitting_keeps_generated_temp_defs() {
    let repeated = "((1.0833 * ((left - (2.0 * mid)) + right)) * ((left - (2.0 * mid)) + right))";
    let rhs = [repeated; 12].join(" + ");
    let input = format!("  .__rr_cse_0 <- (u_vel * ({rhs}))");
    let out = materialize_o3_large_repeated_arithmetic_subexpressions(vec![input]).join("\n");
    assert_generated_cse_refs_are_defined(&out);
}

#[test]
pub(crate) fn o3_weno_leaf_inline_budget_keeps_large_kernel_boundary() {
    let input = "\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2) \n\
{\n\
  .__rr_cse_22 <- (3.0 * v_c)\n\
  b1 <- (((1.0833 * ((v_m2 - (2.0 * v_m1)) + v_c)) * ((v_m2 - (2.0 * v_m1)) + v_c)) + ((0.25 * ((v_m2 - (4.0 * v_m1)) + .__rr_cse_22)) * ((v_m2 - (4.0 * v_m1)) + .__rr_cse_22)))\n\
  b2 <- (((1.0833 * ((v_m1 - (2.0 * v_c)) + v_p1)) * ((v_m1 - (2.0 * v_c)) + v_p1)) + ((0.25 * (v_m1 - v_p1)) * (v_m1 - v_p1)))\n\
  b3 <- (((1.0833 * ((v_c - (2.0 * v_p1)) + v_p2)) * ((v_c - (2.0 * v_p1)) + v_p2)) + ((0.25 * ((.__rr_cse_22 - (4.0 * v_p1)) + v_p2)) * ((.__rr_cse_22 - (4.0 * v_p1)) + v_p2)))\n\
  d1 <- 0.1\n\
  d2 <- 0.6\n\
  d3 <- 0.3\n\
  eps <- 0.000001\n\
  a1 <- (d1 / ((eps + b1) * (eps + b1)))\n\
  a2 <- (d2 / ((eps + b2) * (eps + b2)))\n\
  a3 <- (d3 / ((eps + b3) * (eps + b3)))\n\
  sum_a <- ((a1 + a2) + a3)\n\
  w1 <- (a1 / sum_a)\n\
  w2 <- (a2 / sum_a)\n\
  w3 <- (a3 / sum_a)\n\
  return((((w1 * (((0.3333 * v_m2) - (1.1666 * v_m1)) + (1.8333 * v_c))) + (w2 * (((-0.1666 * v_m1) + (0.8333 * v_c)) + (0.3333 * v_p1)))) + (w3 * (((0.3333 * v_c) + (0.8333 * v_p1)) - (0.1666 * v_p2)))))\n\
}\n\
Sym_268 <- function(field, u_vel, n_l, n_r, n_ll, n_rr)\n\
{\n\
  field_r <- rr_gather(field, rr_index_vec_floor(n_r))\n\
  field_l <- rr_gather(field, rr_index_vec_floor(n_l))\n\
  field_n_r_rr <- rr_gather(field, rr_index_vec_floor(rr_gather(n_r, rr_index_vec_floor(n_rr))))\n\
  field_ll <- rr_gather(field, rr_index_vec_floor(n_ll))\n\
  field_rr <- rr_gather(field, rr_index_vec_floor(n_rr))\n\
  flux <- ifelse((u_vel > 0.0), (u_vel * (Sym_244(field_l, field, field_r, field_rr, field_n_r_rr) - Sym_244(field_ll, field_l, field, field_r, field_rr))), ((u_vel * (field_r - field_l)) * 0.5))\n\
  return(flux)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  field <- rep.int(1.0, 8)\n\
  idx <- seq_len(8)\n\
  print(Sym_268(field, field, idx, idx, idx, idx))\n\
}\n\
Sym_top_0()\n";
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        input,
        &FxHashSet::default(),
        &FxHashSet::default(),
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::O3),
    );
    assert_generated_cse_refs_are_defined(&out);
    assert!(out.contains("Sym_244(field_l"), "{out}");
    assert!(
        !out.contains(".__rr_expr_"),
        "large WENO leaf inline should stay behind the helper call boundary:\n{out}"
    );
}

#[test]
pub(crate) fn o3_inlines_small_physics_kernels_but_keeps_weno_boundary() {
    let input = "\
Sym_156 <- function(u, v, n_l, n_r, n_d, n_u)\n\
{\n\
  u_l <- rr_gather(u, n_l)\n\
  u_r <- rr_gather(u, n_r)\n\
  return((u_r - u_l))\n\
}\n\
Sym_171 <- function(u, v, h, h_trn, coriolis, visc, n_l, n_r, n_d, n_u)\n\
{\n\
  n_l <- rr_index_vec_floor(n_l)\n\
  n_r <- rr_index_vec_floor(n_r)\n\
  h_r <- rr_gather(h, n_r)\n\
  h_l <- rr_gather(h, n_l)\n\
  return((visc + (h_r - h_l)))\n\
}\n\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2)\n\
{\n\
  return((((v_m2 + v_m1) + v_c) + (v_p1 + v_p2)))\n\
}\n\
Sym_268 <- function(field, n_l, n_r, n_rr)\n\
{\n\
  return(Sym_244(rr_gather(field, n_l), field, rr_gather(field, n_r), rr_gather(field, n_rr), rr_gather(field, n_rr)))\n\
}\n\
Sym_1 <- function(u, v, h, h_trn, coriolis, n_l, n_r, n_d, n_u, n_rr)\n\
{\n\
  visc <- Sym_156(u, v, n_l, n_r, n_d, n_u)\n\
  du1 <- Sym_171(u, v, h, h_trn, coriolis, visc, n_l, n_r, n_d, n_u)\n\
  adv_u <- Sym_268(u, n_l, n_r, n_rr)\n\
  return((du1 - adv_u))\n\
}\n";
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        input,
        &FxHashSet::default(),
        &FxHashSet::default(),
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::O3),
    );
    assert!(!out.contains("visc <- Sym_156("), "{out}");
    assert!(!out.contains("du1 <- Sym_171("), "{out}");
    assert!(out.contains("visc <- ((rr_gather(u, n_r)"), "{out}");
    assert!(out.contains("du1 <- ((visc +"), "{out}");
}

#[test]
pub(crate) fn o3_semanticizes_index_and_gather_cse_names() {
    let input = vec![
        "  .__rr_cse_0 <- rr_index_vec_floor(adj_r)".to_string(),
        "  .__rr_cse_1 <- rr_gather(u, .__rr_cse_0)".to_string(),
        "  out <- (.__rr_cse_1 + .__rr_cse_1)".to_string(),
    ];
    let out = semanticize_o3_vector_cse_temp_names(input).join("\n");
    assert!(out.contains("idx_r <- rr_index_vec_floor(adj_r)"), "{out}");
    assert!(out.contains("u_r <- rr_gather(u, idx_r)"), "{out}");
    assert!(out.contains("out <- (u_r + u_r)"), "{out}");
    assert!(!out.contains(".__rr_cse_"), "{out}");
}

#[test]
pub(crate) fn o3_materializes_single_use_gathers_with_semantic_names_on_large_rhs() {
    let input = vec![(
        "  flux <- ifelse((u_vel > 0.0), (u_vel * (Sym_244(rr_gather(field, rr_index_vec_floor(n_l)), field, rr_gather(field, rr_index_vec_floor(n_r)), rr_gather(field, rr_index_vec_floor(n_rr)), rr_gather(field, rr_index_vec_floor(n_rr))) - Sym_244(rr_gather(field, rr_index_vec_floor(n_ll)), rr_gather(field, rr_index_vec_floor(n_l)), field, rr_gather(field, rr_index_vec_floor(n_r)), rr_gather(field, rr_index_vec_floor(n_rr))))), 0.0)"
    ).to_string()];
    let out = materialize_o3_semantic_gather_subexpressions(input).join("\n");
    assert!(
        out.contains("field_l <- rr_gather(field, rr_index_vec_floor(n_l))"),
        "{out}"
    );
    assert!(
        out.contains("field_r <- rr_gather(field, rr_index_vec_floor(n_r))"),
        "{out}"
    );
    assert!(
        out.contains("field_rr <- rr_gather(field, rr_index_vec_floor(n_rr))"),
        "{out}"
    );
    assert!(
        out.contains("field_ll <- rr_gather(field, rr_index_vec_floor(n_ll))"),
        "{out}"
    );
    assert!(out.contains("flux <- ifelse"), "{out}");
}

#[test]
pub(crate) fn o3_materializes_repeated_short_gathers_in_large_rhs() {
    let input = vec![(
        "  visc <- (mix_sq * sqrt((((rr_gather(u, n_r) - rr_gather(u, n_l)) * (rr_gather(u, n_r) - rr_gather(u, n_l))) + ((rr_gather(v, n_u) - rr_gather(v, n_d)) * (rr_gather(v, n_u) - rr_gather(v, n_d))))))"
    ).to_string()];
    let out = materialize_o3_semantic_gather_subexpressions(input).join("\n");
    assert!(out.contains("u_r <- rr_gather(u, n_r)"), "{out}");
    assert!(out.contains("u_l <- rr_gather(u, n_l)"), "{out}");
    assert!(out.contains("v_u <- rr_gather(v, n_u)"), "{out}");
    assert!(out.contains("v_d <- rr_gather(v, n_d)"), "{out}");
    assert_eq!(out.matches("rr_gather(u, n_r)").count(), 1, "{out}");
}

#[test]
pub(crate) fn o3_materializes_gathers_inside_large_return_expr() {
    let input = vec![
        "Sym_210 <- function(field, w, h) ".to_string(),
        "{".to_string(),
        "  i <- 1.0".to_string(),
        "  return((rr_gather(field, rr_wrap_index_vec_i(i - 1.0, i, w, h)) + rr_gather(field, rr_wrap_index_vec_i(i + 1.0, i, w, h)) + rr_gather(field, rr_wrap_index_vec_i(i, i - 1.0, w, h)) + rr_gather(field, rr_wrap_index_vec_i(i, i + 1.0, w, h))))".to_string(),
        "}".to_string(),
    ];
    let out = materialize_o3_semantic_gather_subexpressions(input).join("\n");
    assert!(
        out.contains("field_wrap <- rr_gather(field, rr_wrap_index_vec_i("),
        "{out}"
    );
    assert!(out.contains("field_wrap_2 <- rr_gather(field"), "{out}");
    let return_line = out
        .lines()
        .find(|line| line.contains("return("))
        .unwrap_or("");
    assert!(return_line.contains("field_wrap"), "{out}");
    assert!(!return_line.contains("rr_gather("), "{out}");
    assert!(
        out.lines()
            .filter(|line| line.contains("return("))
            .all(|line| line.len() < 160),
        "{out}"
    );
}

#[test]
pub(crate) fn o3_hoists_loop_invariant_indexed_gathers_from_proven_nonzero_scalar_loop() {
    let input = vec![
        "  size <- 4.0".to_string(),
        "  i <- 1.0".to_string(),
        "  repeat {".to_string(),
        "    if (!(i <= size)) break".to_string(),
        "    r <- rr_index1_read_idx(n_r, i, \"index\")".to_string(),
        "    l <- rr_index1_read_idx(n_l, i, \"index\")".to_string(),
        "    out[i] <- (rr_index1_read(u, r, \"index\") - rr_index1_read(u, l, \"index\"))"
            .to_string(),
        "    i <- (i + 1.0)".to_string(),
        "  }".to_string(),
    ];
    let out = hoist_loop_invariant_indexed_gathers(input).join("\n");
    assert!(out.contains("u_r <- rr_gather(u, n_r)"), "{out}");
    assert!(out.contains("u_l <- rr_gather(u, n_l)"), "{out}");
    assert!(out.contains("out[i] <- (u_r[i] - u_l[i])"), "{out}");
    assert!(!out.contains("rr_index1_read(u, r"), "{out}");
}

#[test]
pub(crate) fn o3_does_not_hoist_loop_gathers_when_loop_can_be_zero_trip() {
    let input = vec![
        "  size <- 0.0".to_string(),
        "  i <- 1.0".to_string(),
        "  repeat {".to_string(),
        "    if (!(i <= size)) break".to_string(),
        "    r <- rr_index1_read_idx(n_r, i, \"index\")".to_string(),
        "    out[i] <- rr_index1_read(u, r, \"index\")".to_string(),
        "    i <- (i + 1.0)".to_string(),
        "  }".to_string(),
    ];
    let out = hoist_loop_invariant_indexed_gathers(input).join("\n");
    assert!(!out.contains("rr_gather(u, n_r)"), "{out}");
    assert!(out.contains("rr_index1_read(u, r"), "{out}");
}

#[test]
pub(crate) fn o3_size_control_keeps_large_simple_expr_helper_calls() {
    let long_expr = (0..30)
        .map(|idx| format!("((x + {idx}.0) * (x + {idx}.0))"))
        .collect::<Vec<_>>()
        .join(" + ");
    let input = format!(
        "\
Sym_1 <- function(x) \n\
{{\n\
  return({long_expr})\n\
}}\n\
Sym_2 <- function(x) \n\
{{\n\
  y <- Sym_1(x)\n\
  return(y)\n\
}}\n"
    );
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        &input,
        &pure,
        &fresh,
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::O3),
    );
    assert!(out.contains("y <- Sym_1(x)"), "{out}");
    assert!(out.contains("Sym_1 <- function"), "{out}");
}

#[test]
pub(crate) fn generic_counted_repeat_loop_rewrite_still_applies_without_benchmark_rewrites() {
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
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true),
    );
    assert!(!out.contains("repeat {"), "{out}");
    assert!(!out.contains("if (!(i <= TOTAL)) break"), "{out}");
    assert!(out.contains("for (i in seq_len(TOTAL)) {"), "{out}");
}

#[test]
pub(crate) fn generic_counted_repeat_loop_rewrite_preserves_non_iter_prefix_lines() {
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
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true),
    );
    assert!(out.contains("seed <- 12345"), "{out}");
    assert!(out.contains("for (i in seq_len(n)) {"), "{out}");
    assert!(!out.contains("repeat {"), "{out}");
}

#[test]
pub(crate) fn restores_counter_for_constant_one_guard_repeat_loop() {
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
pub(crate) fn restores_counter_for_constant_zero_guard_repeat_loop() {
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
pub(crate) fn generic_counted_repeat_loop_rewrite_skips_when_iter_used_after_loop() {
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
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true),
    );
    assert!(out.contains("repeat {"), "{out}");
    assert!(!out.contains("for (i in seq_len(n)) {"), "{out}");
}

#[test]
pub(crate) fn hoists_loop_invariant_pure_assignment_from_counted_repeat_loop() {
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
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true),
    );
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
pub(crate) fn does_not_hoist_loop_invariant_pure_assignment_when_dependency_mutates() {
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
    let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true),
    );
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
pub(crate) fn preserves_loop_facts_across_repeat_and_single_line_break_guard() {
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
pub(crate) fn does_not_rewrite_full_slice_facts_across_branch_boundaries() {
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
pub(crate) fn reuses_pure_user_call_binding_in_later_expression() {
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
pub(crate) fn rewrites_return_of_last_assignment_rhs_to_variable() {
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
pub(crate) fn return_rewrite_does_not_use_stale_alias_after_rhs_var_is_mutated() {
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
pub(crate) fn reuses_pure_user_call_across_guarded_block_sequence() {
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
pub(crate) fn oz_materializes_indexed_vector_helper_snapshots_before_mutating_loop() {
    let input = "\
Sym_10 <- function(u, size) \n\
{\n\
  return((u + 1.0))\n\
}\n\
Sym_top_0 <- function(u, du2, size) \n\
{\n\
  i <- 1.0\n\
  repeat {\n\
    if (!(i <= size)) break\n\
    u[i] <- (du2[i] - Sym_10(u, size)[i])\n\
    i <- (i + 1.0)\n\
  }\n\
  return(u)\n\
}\n";
    let pure = FxHashSet::default();
    let fresh = FxHashSet::default();
    let ((out, _), _) = optimize_emitted_r_with_context_and_fresh_with_profile(
        input,
        &pure,
        &fresh,
        PeepholeOptions::new(true).opt_level(crate::compiler::OptLevel::Oz),
    );
    assert!(
        out.contains("adv_u2 <- Sym_10(u, size)") || out.contains("adv_u2 <- ((u + 1.0))"),
        "{out}"
    );
    assert!(out.contains("u[i] <- (du2[i] - adv_u2[i])"), "{out}");
    assert!(!out.contains("Sym_10(u, size)[i]"), "{out}");
}

#[test]
pub(crate) fn removes_dead_simple_alias_and_literal_assignments() {
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
