use super::common::*;

#[test]
fn does_not_inline_named_scalar_expr_inside_repeat_loop() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  repeat {\n\
if (!(time <= 5)) break\n\
vy <- (vy + (g * dt))\n\
y <- (y + (vy * dt))\n\
time <- (time + dt)\n\
  }\n\
  return(y)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("vy <- (vy + (g * dt))"), "{out}");
    assert!(out.contains("y <- (y + (vy * dt))"), "{out}");
}

#[test]
fn rewrites_adjacent_duplicate_pure_call_assignments_to_alias() {
    let input = "\
Sym_303 <- function() \n\
{\n\
  p_x <- Sym_183(1000)\n\
  p_y <- Sym_183(1000)\n\
  return(p_y)\n\
}\n";
    let pure = FxHashSet::from_iter([String::from("Sym_183")]);
    let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
    assert!(out.contains("p_x <- Sym_183(1000)"), "{out}");
    assert!(out.contains("p_y <- p_x"), "{out}");
    assert!(!out.contains("p_y <- Sym_183(1000)"), "{out}");
}

#[test]
fn rewrites_adjacent_duplicate_symbol_assignments_to_alias() {
    let input = "\
Sym_123 <- function(b) \n\
{\n\
  r <- b\n\
  p <- b\n\
  return(p)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("r <- b") || out.contains("return(r)"), "{out}");
    assert!(out.contains("p <- r") || out.contains("return(r)"), "{out}");
    assert!(
        !out.contains("p <- b") || out.contains("return(r)"),
        "{out}"
    );
}

#[test]
fn prunes_dead_zero_loop_seeds_before_for() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
t <- 0\n\
  rr_mark(1031, 5);\n\
  print(\"  Watching the pattern emerge...\")\n\
for (t in seq_len(20)) {\n\
  print(t)\n\
}\n\
steps <- 0\n\
dt <- 0.1\n\
for (steps in seq_len(5)) {\n\
  print(steps)\n\
}\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("t <- 0"), "{out}");
    assert!(!out.contains("steps <- 0"), "{out}");
    assert!(!out.contains("k <- 1"), "{out}");
    assert!(out.contains("for (t in seq_len(20)) {"), "{out}");
    assert!(out.contains("for (steps in seq_len(5)) {"), "{out}");
}

#[test]
fn simplifies_same_var_is_na_or_not_finite_guards() {
    let input = "\
Sym_123 <- function() \n\
{\n\
  if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {\n\
rs_old <- 0.0000001\n\
  }\n\
  if ((is.na(alpha) | (!(is.finite(alpha))))) {\n\
alpha <- 0\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("!(is.finite(rs_old))") && out.contains("(rs_old == 0)"),
        "{out}"
    );
    assert!(out.contains("!(is.finite(alpha))"), "{out}");
    assert!(!out.contains("is.na(rs_old)"), "{out}");
    assert!(!out.contains("is.na(alpha)"), "{out}");
}

#[test]
fn simplifies_not_finite_or_zero_guard_parens() {
    let input = "\
Sym_123 <- function() \n\
{\n\
  if (((!(is.finite(rs_old))) | (rs_old == 0))) {\n\
rs_old <- 0.0000001\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("if ((!(is.finite(rs_old)) | (rs_old == 0))) {"),
        "{out}"
    );
    assert!(
        !out.contains("if (((!(is.finite(rs_old))) | (rs_old == 0))) {"),
        "{out}"
    );
}

#[test]
fn simplifies_wrapped_not_finite_parens() {
    let input = "\
Sym_123 <- function() \n\
{\n\
  if ((!(is.finite(alpha)))) {\n\
alpha <- 0\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("if (!(is.finite(alpha))) {"), "{out}");
    assert!(!out.contains("if ((!(is.finite(alpha)))) {"), "{out}");
}

#[test]
fn strips_terminal_repeat_nexts_without_touching_inner_if_nexts() {
    let input = "\
Sym_83 <- function() \n\
{\n\
  repeat {\n\
if ((flag)) {\n\
  next\n\
}\n\
x <- (x + 1)\n\
next\n\
  }\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("if ((flag)) {\nnext\n}") || out.contains("if ((flag)) {\n  next\n}"),
        "{out}"
    );
    assert!(
        !out.contains("x <- (x + 1)\nnext\n}") && !out.contains("x <- (x + 1)\n  next\n}"),
        "{out}"
    );
}

#[test]
fn inlines_single_use_named_scalar_pure_calls() {
    let input = "\
Sym_222 <- function() \n\
{\n\
  id <- rr_wrap_index_vec_i(x, y, W, H)\n\
  rr_mark(1, 1);\n\
  B[rr_index1_write(id, \"index\")] <- 1\n\
  center_idx <- rr_wrap_index_vec_i(32, 32, W, H)\n\
  print(rr_index1_read(B, center_idx, \"index\"))\n\
}\n";
    let out = optimize_emitted_r(input, true);
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
fn rewrites_wrap_index_scalar_access_helpers_to_base_indexing() {
    let input = "\
Sym_top_0 <- function() \n\
{\n\
  B[rr_index1_write(rr_wrap_index_vec_i(x, y, W, H), \"index\")] <- 1\n\
  return(rr_index1_read(B, rr_wrap_index_vec_i(32, 32, W, H), \"index\"))\n\
}\n\
Sym_top_0()\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("B[rr_wrap_index_vec_i(x, y, W, H)] <- 1"),
        "{out}"
    );
    assert!(
        out.contains("return(B[rr_wrap_index_vec_i(32, 32, W, H)])"),
        "{out}"
    );
    assert!(
        !out.contains("rr_index1_write(rr_wrap_index_vec_i("),
        "{out}"
    );
    assert!(
        !out.contains("rr_index1_read(B, rr_wrap_index_vec_i("),
        "{out}"
    );
}

#[test]
fn rewrites_straight_line_sym156_helper_call() {
    let input = "\
Sym_156 <- function(u, v, n_l, n_r, n_d, n_u, size)\n\
{\n\
  .arg_n_l <- rr_index_vec_floor(n_l)\n\
  .arg_n_r <- rr_index_vec_floor(n_r)\n\
  .arg_n_d <- rr_index_vec_floor(n_d)\n\
  .arg_n_u <- rr_index_vec_floor(n_u)\n\
  Cs <- 0.15\n\
  DX <- 10000\n\
  mix_sq <- ((Cs * DX) * (Cs * DX))\n\
  visc <- (mix_sq * sqrt((((((rr_gather(u, .arg_n_r) - rr_gather(u, .arg_n_l)) / (2 * DX)) - ((rr_gather(v, .arg_n_u) - rr_gather(v, .arg_n_d)) / (2 * DX))) * (((rr_gather(u, .arg_n_r) - rr_gather(u, .arg_n_l)) / (2 * DX)) - ((rr_gather(v, .arg_n_u) - rr_gather(v, .arg_n_d)) / (2 * DX)))) + ((((rr_gather(u, .arg_n_u) - rr_gather(u, .arg_n_d)) / (2 * DX)) + ((rr_gather(v, .arg_n_r) - rr_gather(v, .arg_n_l)) / (2 * DX))) * (((rr_gather(u, .arg_n_u) - rr_gather(u, .arg_n_d)) / (2 * DX)) + ((rr_gather(v, .arg_n_r) - rr_gather(v, .arg_n_l)) / (2 * DX)))))))\n\
  return(visc)\n\
}\n\
Sym_1 <- function()\n\
{\n\
  visc <- Sym_156(u, v, adj_l, adj_r, adj_d, adj_u, TOTAL)\n\
  return(visc)\n\
}\n";
    let lines: Vec<String> = input.lines().map(str::to_string).collect();
    let mut bindings = rustc_hash::FxHashMap::default();
    for line in lines.iter().skip(2).take(8) {
        let trimmed = line.trim();
        let caps = assign_re()
            .and_then(|re| re.captures(trimmed))
            .expect("expected assignment in helper body");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let expanded = substitute_helper_expr(rhs, &bindings);
        bindings.insert(lhs.to_string(), expanded);
    }
    let expanded_return = substitute_helper_expr("visc", &bindings);
    assert!(
        expanded_return.contains("rr_index_vec_floor(n_l)"),
        "{expanded_return}"
    );
    assert!(
        expanded_return.contains("rr_gather(u,"),
        "{expanded_return}"
    );
    let helpers = collect_simple_expr_helpers(&lines, &FxHashSet::default());
    assert!(helpers.contains_key("Sym_156"), "{helpers:#?}");
    let out = optimize_emitted_r(input, true);
    assert!(
        !out.contains("visc <- Sym_156(u, v, adj_l, adj_r, adj_d, adj_u, TOTAL)"),
        "{out}"
    );
    assert!(out.contains("return("), "{out}");
}

#[test]
fn rewrites_selected_simple_expr_helper_calls_in_text() {
    let input = "\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2)\n\
{\n\
  return((v_c + v_p1))\n\
}\n\
Sym_268 <- function(field, u_vel, n_l, n_r, n_ll, n_rr)\n\
{\n\
  flux <- ifelse((u_vel > 0), (u_vel * (Sym_244(rr_gather(field, n_l), field, rr_gather(field, n_r), rr_gather(field, n_rr), rr_gather(field, n_rr)) - Sym_244(rr_gather(field, n_ll), rr_gather(field, n_l), field, rr_gather(field, n_r), rr_gather(field, n_rr)))), ((u_vel * (rr_gather(field, n_r) - rr_gather(field, n_l))) * 0.5))\n\
  return(flux)\n\
}\n\
Sym_999 <- function(x)\n\
{\n\
  return((x + 1))\n\
}\n\
Sym_303 <- function()\n\
{\n\
  adv_u <- Sym_268(u, u, adj_l, adj_r, adj_ll, adj_rr)\n\
  keep_me <- Sym_999(1)\n\
  return(adv_u)\n\
}\n";
    let out = rewrite_selected_simple_expr_helper_calls_in_text(input, &["Sym_244", "Sym_268"]);
    assert!(!out.contains("adv_u <- Sym_268("), "{out}");
    assert!(out.contains("ifelse((u > 0),"), "{out}");
    assert!(out.contains("rr_gather(u, adj_l)"), "{out}");
    assert!(out.contains("keep_me <- Sym_999(1)"), "{out}");
}

#[test]
fn restores_missing_scalar_loop_increment_for_repeat_guarded_index_loop() {
    let input = "\
Sym_1 <- function(n)\n\
{\n\
  y <- seq_len(n)\n\
  i <- 1L\n\
  repeat {\n\
if (!(i <= length(y))) break\n\
y[i] <- 0L\n\
  }\n\
  return(y)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("i <- (i + 1L)"), "{out}");
}

#[test]
fn restores_missing_scalar_loop_increment_for_repeat_guarded_reduction_loop() {
    let input = "\
Sym_1 <- function(n)\n\
{\n\
  acc <- 0L\n\
  i <- 1L\n\
  repeat {\n\
if (!(i <= n)) break\n\
acc <- (acc + (i * i))\n\
  }\n\
  return(acc)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("i <- (i + 1L)") || out.contains("for (i in seq_len(n)) {"),
        "{out}"
    );
}

#[test]
fn restores_missing_scalar_loop_increment_before_next_branch() {
    let input = "\
Sym_1 <- function(n)\n\
{\n\
s <- 0L\n\
i <- 1L\n\
  repeat {\n\
if (!(i <= n)) break\n\
if ((i == 3L)) {\n\
i <- (i + 1L)\n\
  next\n\
} else if ((i == 6L)) {\n\
  break\n\
} else {\n\
s <- (s + i)\n\
  next\n\
}\n\
  }\n\
  return(s)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("s <- (s + i)\ni <- (i + 1L)\nnext"), "{out}");
}

#[test]
fn restores_missing_scalar_loop_increment_after_nested_if() {
    let input = "\
Sym_11 <- function(n, k)\n\
{\n\
x <- seq_len(n)\n\
y <- x\n\
i <- 1L\n\
  repeat {\n\
if (!(i <= length(x))) break\n\
if ((x[i] > k)) {\n\
  y[i] <- x[i]\n\
} else {\n\
  y[i] <- 0L\n\
}\n\
  }\n\
  return(y)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("}\ni <- (i + 1L)\n}") || out.contains("for (i in seq_len(length(x))) {"),
        "{out}"
    );
}

#[test]
fn rewrites_known_length_calls_from_prior_vector_facts() {
    let mut vector_lens = FxHashMap::default();
    vector_lens.insert("y".to_string(), "n".to_string());
    assert_eq!(
        rewrite_known_length_calls("rep.int(0L, length(y))", &vector_lens),
        "rep.int(0L, n)"
    );
    assert_eq!(
        rewrite_known_length_calls("rep.int(i, length(y))", &vector_lens),
        "rep.int(i, n)"
    );
}

#[test]
fn restores_empty_singleton_list_match_arm() {
    let input = "\
Sym_9 <- function(v)\n\
{\n\
  if ((((length(v) >= 2L) & TRUE) & TRUE)) {\n\
.phi_32 <- ((v[1L] + v[2L]) + 1L)\n\
  } else {\n\
if (((length(v) == 1L) & TRUE)) {\n\
} else {\n\
.phi_32 <- 0L\n\
}\n\
  }\n\
  return(.phi_32)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains(".phi_32 <- v[1L]"), "{out}");
}

#[test]
fn restores_empty_single_field_record_match_arm() {
    let input = "\
Sym_17 <- function(v)\n\
{\n\
  if (((((TRUE & rr_field_exists(v, \"a\")) & TRUE) & rr_field_exists(v, \"b\")) & TRUE)) {\n\
.phi_30 <- (v[[\"a\"]] + v[[\"b\"]])\n\
  } else {\n\
if (((TRUE & rr_field_exists(v, \"a\")) & TRUE)) {\n\
} else {\n\
.phi_30 <- 0L\n\
}\n\
  }\n\
  return(.phi_30)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains(".phi_30 <- v[[\"a\"]]"), "{out}");
}

#[test]
fn simplifies_nested_index_vec_floor_calls_in_text() {
    let input = "\
Sym_303 <- function()\n\
{\n\
  visc <- rr_gather(u, rr_index_vec_floor(rr_index_vec_floor(adj_r)))\n\
  return(rr_index_vec_floor(rr_index_vec_floor(adj_l)))\n\
}\n";
    let out = simplify_nested_index_vec_floor_calls_in_text(input);
    assert!(
        !out.contains("rr_index_vec_floor(rr_index_vec_floor("),
        "{out}"
    );
    assert!(
        out.contains("rr_gather(u, rr_index_vec_floor(adj_r))"),
        "{out}"
    );
    assert!(out.contains("return(rr_index_vec_floor(adj_l))"), "{out}");
}

#[test]
fn keeps_loop_carried_assignments_that_are_read_elsewhere_in_function() {
    let input = "\
Sym_1 <- function() \n\
{\n\
  rs_old <- 1\n\
  repeat {\n\
if (!(iter <= 2)) break\n\
beta <- (rs_new / rs_old)\n\
rs_old <- rs_new\n\
iter <- (iter + 1)\n\
next\n\
  }\n\
  return(rs_old)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(out.contains("rs_old <- rs_new"));
}
