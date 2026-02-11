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
alloc_particles <- function(n) {
  p <- seq_len(n)
  i <- 1L
  seed <- 12345L
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
main <- function() {
  t <- 0L
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
step <- function(x, dx) {
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
    let count = generated.matches("(.arg_x + .arg_dx)").count();
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
main <- function() {
  B <- seq_len(10L)
  t <- 0L
  while (t < 3L) {
    t = t + 1L
    side_idx <- 3L
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
        generated.contains("print(rr_index1_read(B, side_idx, \"index\"))"),
        "missing tail print statement in loop body"
    );
}
