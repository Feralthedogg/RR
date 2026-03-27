use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out_path: &Path) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O0")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_src.display()
    );
}

fn test_out_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("codegen_binding_regression");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    out_dir
}

#[test]
fn assign_then_use_does_not_recompute_rng_expression() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("rng_recompute.rr");
    let r_path = out_dir.join("rng_recompute.R");

    let src = r#"
let alloc_particles <- function(n) {
  let p <- seq_len(n)
  let i <- 1L
  let seed <- 12345L
  while (i <= n) {
    seed = (seed * 1103515245L + 12345L) % 2147483648L
    p[i] = seed / 2147483648L
    i = i + 1L
  }
  p
}

alloc_particles(4L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("p[rr_index1_write(i, \"index\")] <- (seed /")
            || generated.contains("p[i] <- (seed /"),
        "expected assignment to reuse seed variable"
    );
    assert!(
        !generated.contains("p[rr_index1_write(i, \"index\")] <- ((((seed * 1103515245"),
        "found stale bug pattern: RNG expression was recomputed on p[i] assignment"
    );
}

#[test]
fn assign_then_print_uses_updated_variable_not_reexpanded_expr() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("print_off_by_one.rr");
    let r_path = out_dir.join("print_off_by_one.R");

    let src = r#"
let main <- function() {
  let t <- 0L
  let u <- 0L
  while (t < 3L) {
    t = t + 1L
    u = t
  }
  u
}

main()
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("u <- t") || generated.contains("u <- (t + 1L)"),
        "expected direct use of updated variable after assignment"
    );
    assert!(
        !generated.contains("u <- (t + 2L)"),
        "found stale bug pattern: assignment emitted with re-expanded expression"
    );
}

#[test]
fn if_else_codegen_does_not_reexpand_pre_if_assignment_on_else_path() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("if_else_reexpand.rr");
    let r_path = out_dir.join("if_else_reexpand.R");

    let src = r#"
let step <- function(x, dx) {
  x = x + dx
  if (x > 1L) {
    x = x - 1L
  }
  x
}

step(0L, 1L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    let count = generated.matches("(.arg_x + .arg_dx)").count()
        + generated.matches("(.arg_x + dx)").count()
        + generated.matches("(x + dx)").count();
    assert_eq!(
        count, 1,
        "expected exactly one x <- x + dx evaluation; found {}",
        count
    );
}

#[test]
fn loop_tail_eval_statement_is_not_dropped() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("loop_tail_eval.rr");
    let r_path = out_dir.join("loop_tail_eval.R");

    let src = r#"
let main <- function() {
  let B <- seq_len(10L)
  let t <- 0L
  let side_idx <- 0L
  while (t < 3L) {
    t = t + 1L
    side_idx = 3L
    print("Wave")
    print(B[side_idx])
  }
}

main()
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("print(\"Wave\")"),
        "missing first print in loop body"
    );
    assert!(
        generated.contains("print(rr_index1_read(B, side_idx, \"index\"))")
            || generated.contains("print(B[side_idx])"),
        "missing tail print statement in loop body (guarded or direct index form)"
    );
}

#[test]
fn swap_assign_preserves_temporary_source() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("swap_assign.rr");
    let r_path = out_dir.join("swap_assign.R");

    let src = r#"
let main <- function() {
  let u = 1.0
  let u_new = 2.0
  let tmp_u = u
  u = u_new
  u_new = tmp_u
  print(u)
  print(u_new)
}

main()
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("tmp_u <- u") || generated.contains("tmp_u <- 1"),
        "expected temporary copy to preserve original source"
    );
    assert!(
        generated.contains("u <- u_new") || generated.contains("u <- 2"),
        "expected swap first half to assign new buffer"
    );
    assert!(
        generated.contains("u_new <- tmp_u"),
        "expected swap second half to use temporary source"
    );
    assert!(
        !generated.contains("u_new <- u_new"),
        "found stale bug pattern: swap collapsed into self-assignment"
    );
}

#[test]
fn branch_carried_scalar_update_uses_merged_value() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("branch_carried_scalar.rr");
    let r_path = out_dir.join("branch_carried_scalar.R");

    let src = r#"
fn dot2(a, b) {
  return a + b
}

let main <- function() {
  let rs_old = 1.0
  let rs_new = 0.0
  let beta = 0.0
  rs_new = dot2(2.0, 3.0)
  if (is.na(rs_new) || !is.finite(rs_new)) {
    rs_new = rs_old
  }
  beta = rs_new / rs_old
  rs_old = rs_new
  print(beta)
  print(rs_old)
}

main()
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("beta <- (rs_new / rs_old)"),
        "expected post-branch recurrence to use merged rs_new value"
    );
    assert!(
        generated.contains("rs_old <- rs_new"),
        "expected old residual to update from new residual"
    );
    assert!(
        !generated.contains("beta <- rs_old"),
        "found stale bug pattern: beta collapsed to wrong source"
    );
    assert!(
        !generated.contains("beta <- (rs_old / rs_old)"),
        "found stale bug pattern: beta lost merged rs_new source"
    );
    assert!(
        !generated.contains("rs_old <- rs_old"),
        "found stale bug pattern: loop-carried update collapsed to self-assignment"
    );
}

#[test]
fn floor_index_substitution_keeps_position_variables() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("floor_index_substitution.rr");
    let r_path = out_dir.join("floor_index_substitution.R");

    let src = r#"
let sample <- function(px, py, p, N) {
  let gx = px[p] * N
  let gy = py[p] * N
  let ix = floor(gx)
  let iy = floor(gy)
  print(ix)
  print(iy)
}

sample(c(0.1, 0.2), c(0.3, 0.4), 1L, 10L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("gx <-")
            && generated.contains("gy <-")
            && generated.contains("ix <- floor(gx)")
            && generated.contains("iy <- floor(gy)"),
        "expected floor index substitution to preserve gx/gy intermediates"
    );
    assert!(
        !generated.contains("ix <- floor(N)"),
        "found stale bug pattern: gx was replaced by N"
    );
    assert!(
        !generated.contains("iy <- floor(N)"),
        "found stale bug pattern: gy was replaced by N"
    );
}

#[test]
fn call_expression_is_not_rewritten_to_later_alias_name() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("call_alias_order.rr");
    let r_path = out_dir.join("call_alias_order.R");

    let src = r#"
fn main(n) {
  let x = seq_len(n) - 4L
  let y = seq_len(n)
  let i = 1L
  while (i <= length(x)) {
    y[i] = abs(x[i])
    i = i + 1L
  }
  y
}

main(4L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("x <- (seq_len(.arg_n) - 4L)")
            || generated.contains("x <- (seq_len(n) - 4L)"),
        "expected call expression to stay explicit in earlier assignment"
    );
    assert!(
        !generated.contains("x <- (y - 4L)"),
        "found stale bug pattern: earlier call expression rewritten to later alias"
    );
}

#[test]
fn pure_helper_call_is_reused_across_guarded_updates() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("pure_helper_reuse.rr");
    let r_path = out_dir.join("pure_helper_reuse.R");

    let src = r#"
fn matvec(p, n) {
  return p + n
}

fn main(p, n, size) {
  let Ap = matvec(p, n)
  let denom = 1.0
  if (is.na(denom) || !is.finite(denom)) {
    denom = 1.0
  }
  let alpha = 1.0 / denom
  let idx = 1..size
  let out = p[idx] - alpha * matvec(p, n)[idx]
  out
}

main(c(1.0, 2.0), c(3.0, 4.0), 2L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&r_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    let helper_call_count = generated.matches("Sym_1(p, n)").count()
        + generated.matches("matvec(p, n)").count()
        + generated.matches("Sym_1(.arg_p, .arg_n)").count()
        + generated.matches("matvec(.arg_p, .arg_n)").count();
    assert!(
        generated.contains("Ap <- Sym_")
            || generated.contains("Ap <- matvec(")
            || generated.contains("Ap <- ((p + n))")
            || generated.contains("Ap <- (.arg_p + .arg_n)")
            || helper_call_count == 1,
        "expected helper result to bind once to Ap or collapse to a single direct call"
    );
    assert!(
        generated.contains("rr_index1_read_vec(Ap, idx)")
            || generated.contains("rr_index1_read(Ap, idx, \"index\")")
            || generated.contains("rr_index1_read(Ap, (1L:size), \"index\")")
            || generated.contains("Ap[idx]")
            || generated.contains("rr_index1_read(Sym_1(p, n), idx, \"index\")")
            || generated.contains("rr_index1_read(matvec(p, n), idx, \"index\")"),
        "expected indexed use to avoid duplicating helper calls"
    );
    assert!(
        helper_call_count <= 1
            && !generated.contains("rr_index1_read_vec(matvec(")
            && !generated.contains("rr_index1_read(matvec(")
            && generated.matches("rr_index1_read_vec(Sym_").count() <= 1,
        "found stale bug pattern: helper call was recomputed under index read"
    );
}

#[test]
fn return_assign_slice_is_split_into_assign_then_return_var() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("return_assign_slice.rr");
    let r_path = out_dir.join("return_assign_slice.R");

    let src = r#"
fn main(n) {
  let x = rep.int(0.0, n)
  let y = rep.int(1.0, n)
  let i = 1L
  x = y
  x
}

main(4L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&r_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("return(x)"),
        "expected return of assigned variable"
    );
    assert!(
        !generated.contains("return(rr_assign_slice("),
        "expected nested return(assign_slice(...)) to be split"
    );
}

#[test]
fn conditional_return_of_assign_slice_emits_assignment_before_return() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("conditional_return_assign_slice.rr");
    let r_path = out_dir.join("conditional_return_assign_slice.R");

    let src = r#"
fn main(f, ys) {
  let rot = rep.int(0.0, length(ys))
  if (f == 2.0) {
    return rr_assign_slice(rot, 1.0, 1.0, rep.int(1.0, 1.0))
  }
  if (f == 4.0) {
    return rr_assign_slice(rot, 1.0, 1.0, rep.int(3.0, 1.0))
  }
  return rot
}

main(2.0, c(0.0, 0.0))
"#;
    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&r_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("rot <- rr_assign_slice(rot, 1, 1, rep.int(1, 1))\n    return(rot)")
            || generated.contains(
                "rot <- rr_assign_slice(rot, 1.0, 1.0, rep.int(1.0, 1.0))\n    return(rot)"
            )
            || generated
                .contains("rot <- rr_assign_slice(rot, 1, 1, rep.int(3, 1))\n      return(rot)")
            || generated.contains(
                "rot <- rr_assign_slice(rot, 1.0, 1.0, rep.int(3.0, 1.0))\n      return(rot)"
            )
            || generated.contains("rot <- replace(rot, 1, 1)")
            || generated.contains("rot <- replace(rot, 1.0, 1.0)")
            || generated.contains("rot <- replace(rot, 1, 3)")
            || generated.contains("rot <- replace(rot, 1.0, 3.0)"),
        "expected branch return(assign_slice(...)) to emit assignment before return"
    );
    assert!(
        !generated.contains("if (rr_truthy1((.arg_f == 2), \"condition\")) {\n    return(rot)"),
        "stale bug: branch returned base binding without assignment"
    );
}

#[test]
fn fresh_helper_aliases_are_materialized_as_distinct_inits() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("fresh_alias_init.rr");
    let r_path = out_dir.join("fresh_alias_init.R");

    let src = r#"
fn zeros(n) {
  return rep.int(0.0, n)
}

fn main(n) {
  let qr = zeros(n)
  let qi = qr
  let qs = qr
  print(length(qi) + length(qs))
}

main(4L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&r_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        !generated.contains("qi <- qr") && !generated.contains("qs <- qr"),
        "fresh helper aliases should be emitted as distinct initializations, not raw aliases"
    );
    assert!(
        generated.contains("qi <- Sym_1(.arg_n)")
            || generated.contains("qi <- Sym_1(n)")
            || generated.contains("qi <- zeros(.arg_n)")
            || generated.contains("qi <- zeros(n)")
            || generated.matches("rep.int(0, n)").count() >= 2
            || generated.matches("rep.int(0, 4L)").count() >= 2
            || generated.matches("rep.int(0.0, n)").count() >= 2
            || generated.matches("rep.int(0.0, 4L)").count() >= 2
            || generated.contains("length(((rep.int(0.0, n)))) + length(((rep.int(0.0, n))))")
            || generated.contains("length(((rep.int(0.0, 4L)))) + length(((rep.int(0.0, 4L))))"),
        "expected qi init to duplicate the fresh helper expression"
    );
    assert!(
        generated.contains("qs <- Sym_1(.arg_n)")
            || generated.contains("qs <- Sym_1(n)")
            || generated.contains("qs <- zeros(.arg_n)")
            || generated.contains("qs <- zeros(n)")
            || generated.matches("rep.int(0, n)").count() >= 2
            || generated.matches("rep.int(0, 4L)").count() >= 2
            || generated.matches("rep.int(0.0, n)").count() >= 2
            || generated.matches("rep.int(0.0, 4L)").count() >= 2
            || generated.contains("length(((rep.int(0.0, n)))) + length(((rep.int(0.0, n))))")
            || generated.contains("length(((rep.int(0.0, 4L)))) + length(((rep.int(0.0, 4L))))"),
        "expected qs init to duplicate the fresh helper expression"
    );
}

#[test]
fn fresh_helper_aliases_before_loop_are_materialized_as_distinct_inits() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("fresh_alias_loop_init.rr");
    let r_path = out_dir.join("fresh_alias_loop_init.R");

    let src = r#"
fn zeros(n) {
  return rep.int(0.0, n)
}

fn main(n) {
  let qr = zeros(n)
  let u_stage = qr
  let u_new = qr
  let i = 1.0
  while (i <= n) {
    u_new[i] = i
    i = i + 1.0
  }
  print(length(u_stage))
  print(length(u_new))
}

main(4L)
"#;
    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&r_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        (!generated.contains("u_stage <- qr") && !generated.contains("u_new <- qr"))
            || generated.contains("(qr) <- rr_assign_slice((qr), (1), n, (1):n)")
            || generated.contains("(qr) <- rr_assign_slice((qr), (1.0), n, (1.0):n)"),
        "fresh aliases before loops should be materialized as distinct fresh initializations"
    );
}

#[test]
fn vectorized_return_does_not_duplicate_whole_assign_slice() {
    let out_dir = test_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("vectorized_return_dedup.rr");
    let r_path = out_dir.join("vectorized_return_dedup.R");

    let src = r#"
fn f(x) {
  let y = rep.int(0.0, length(x))
  let i = 1.0
  while (i <= length(x)) {
    y[i] = abs(x[i])
    i = i + 1.0
  }
  return y
}

print(f(c(1.0, -2.0, 3.0)))
"#;
    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&r_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    let assign_count = generated.matches("y <- rr_assign_slice(").count();
    assert!(
        assign_count <= 1,
        "vectorized whole assignment should not be emitted twice\n{}",
        generated
    );
}
