use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out_path: &Path) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out_path)
        .arg("--no-runtime")
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
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("p[rr_index1_write(i, \"index\")] <- (seed /"),
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
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("u <- t"),
        "expected direct use of updated variable after assignment"
    );
    assert!(
        !generated.contains("u <- (t + 1L)"),
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
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    let count =
        generated.matches("(.arg_x + .arg_dx)").count() + generated.matches("(x + dx)").count();
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
"#;
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &r_path);

    let generated = fs::read_to_string(&r_path).expect("failed to read generated R");
    assert!(
        generated.contains("tmp_u <- u"),
        "expected temporary copy to preserve original source"
    );
    assert!(
        generated.contains("u <- u_new"),
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
