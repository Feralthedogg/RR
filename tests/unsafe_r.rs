mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use rr::compiler::{OptLevel, compile};
use std::fs;

#[test]
fn unsafe_r_block_is_emitted_verbatim_and_marks_function_opaque() {
    let src = r#"
fn main() {
  let x = 2L
  unsafe r {
    y <- x + 40L
    f <- function(v) { paste0("{", v, "}") }
    # comment with braces should not close the RR unsafe block: { }
    z <- 1L; print(z)
    print(f(y))
  }
  return 0L
}

main()
"#;

    let (code, _map) = compile("unsafe_r_verbatim.rr", src, OptLevel::O2).expect("compile");
    assert!(code.contains("# rr-opaque-interop:"), "{code}");
    assert!(code.contains("y <- x + 40L"), "{code}");
    assert!(
        code.contains(r#"f <- function(v) { paste0("{", v, "}") }"#),
        "{code}"
    );
    assert!(
        code.contains("# comment with braces should not close the RR unsafe block: { }"),
        "{code}"
    );
    assert!(code.contains("z <- 1L; print(z)"), "{code}");
    assert!(code.contains("print(f(y))"), "{code}");
}

#[test]
fn unsafe_r_read_block_is_emitted_without_opaque_interop() {
    let src = r#"
fn main() {
  let x = 2L
  unsafe r(read) {
    print(x)
  }
  print(x + 1L)
  return 0L
}

main()
"#;

    let (code, _map) = compile("unsafe_r_read.rr", src, OptLevel::O2).expect("compile");
    assert!(code.contains("# rr-unsafe-r-read-begin"), "{code}");
    assert!(code.contains("print(x)"), "{code}");
    assert!(
        !code.contains("# rr-opaque-interop: unsafe R block"),
        "read-only unsafe R should not mark the whole function opaque:\n{code}"
    );
}

#[test]
fn unsafe_r_read_can_read_locals_under_o2_when_rscript_is_available() {
    let Some(rscript) = rscript_path() else {
        eprintln!("Skipping unsafe R read local smoke: Rscript path unavailable.");
        return;
    };
    if !rscript_available(&rscript) {
        eprintln!("Skipping unsafe R read local smoke: Rscript unavailable.");
        return;
    }

    let src = r#"
fn main() {
  let x = 2L
  unsafe r(read) {
    print(x)
  }
  print(x + 1L)
  return 0L
}

main()
"#;

    let (code, _map) = compile("unsafe_r_read_runtime.rr", src, OptLevel::O2).expect("compile");
    assert!(code.contains("# rr-unsafe-r-read-begin"), "{code}");
    assert!(
        !code.contains("# rr-opaque-interop: unsafe R block"),
        "read-only unsafe R should not mark the whole function opaque:\n{code}"
    );
    let dir = unique_dir(&std::env::temp_dir(), "rr_unsafe_r_read_runtime");
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    let script = dir.join("compiled.R");
    fs::write(&script, code).expect("failed to write compiled R");

    let run = run_rscript(&rscript, &script);
    assert_eq!(
        run.status, 0,
        "compiled unsafe R read failed\nstdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    let stdout = normalize(&run.stdout);
    assert!(
        stdout.contains("[1] 2") && stdout.contains("[1] 3"),
        "unsafe R read did not preserve local binding:\n{}",
        run.stdout
    );
}

#[test]
fn unsafe_r_is_statement_only_not_expression() {
    let src = r#"
fn main() {
  let x = unsafe r {
    1L
  }
  print(x)
}

main()
"#;

    let err = compile("unsafe_r_expression.rr", src, OptLevel::O0)
        .expect_err("unsafe r expression form must be rejected");
    assert!(
        err.message
            .contains("unsafe r blocks are statements and cannot be used as expressions"),
        "{}",
        err.message
    );
}

#[test]
fn unsafe_r_read_can_capture_lambda_locals_when_rscript_is_available() {
    let Some(rscript) = rscript_path() else {
        eprintln!("Skipping unsafe R read lambda capture smoke: Rscript path unavailable.");
        return;
    };
    if !rscript_available(&rscript) {
        eprintln!("Skipping unsafe R read lambda capture smoke: Rscript unavailable.");
        return;
    }

    let src = r#"
fn main() {
  let seed = 41L
  let f = fn() {
    unsafe r(read) {
      print(seed)
    }
    return seed + 1L
  }
  print(f())
}

main()
"#;

    let (code, _map) =
        compile("unsafe_r_read_lambda_capture.rr", src, OptLevel::O2).expect("compile");
    assert!(code.contains("# rr-unsafe-r-read-begin"), "{code}");
    assert!(
        !code.contains("# rr-opaque-interop: unsafe R block"),
        "read-only unsafe R should not force opaque interop:\n{code}"
    );
    let dir = unique_dir(&std::env::temp_dir(), "rr_unsafe_r_read_lambda");
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    let script = dir.join("compiled.R");
    fs::write(&script, code).expect("failed to write compiled R");

    let run = run_rscript(&rscript, &script);
    assert_eq!(
        run.status, 0,
        "compiled unsafe R read lambda failed\nstdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    let stdout = normalize(&run.stdout);
    assert!(
        stdout.contains("[1] 41") && stdout.contains("[1] 42"),
        "unsafe R read lambda did not capture seed correctly:\n{}",
        run.stdout
    );
}

#[test]
fn unsafe_r_block_runs_inside_compiled_function_when_rscript_is_available() {
    let Some(rscript) = rscript_path() else {
        eprintln!("Skipping unsafe R runtime smoke: Rscript path unavailable.");
        return;
    };
    if !rscript_available(&rscript) {
        eprintln!("Skipping unsafe R runtime smoke: Rscript unavailable.");
        return;
    }

    let src = r#"
fn main() {
  let x = 2L
  unsafe r {
    y <- x + 40L
    print(y)
  }
  unsafe r {
    x <- 5L
  }
  print(x)
  return 0L
}

main()
"#;

    let (code, _map) = compile("unsafe_r_runtime.rr", src, OptLevel::O2).expect("compile");
    let dir = unique_dir(&std::env::temp_dir(), "rr_unsafe_r_runtime");
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    let script = dir.join("compiled.R");
    fs::write(&script, code).expect("failed to write compiled R");

    let run = run_rscript(&rscript, &script);
    assert_eq!(
        run.status, 0,
        "compiled unsafe R failed\nstdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    let stdout = normalize(&run.stdout);
    assert!(
        stdout.contains("[1] 42"),
        "unexpected stdout:\n{}",
        run.stdout
    );
    assert!(
        stdout.contains("[1] 5"),
        "unsafe R assignment to RR local was not visible after the block:\n{}",
        run.stdout
    );
}

#[test]
fn unsafe_r_can_read_params_and_write_rr_locals_when_rscript_is_available() {
    let Some(rscript) = rscript_path() else {
        eprintln!("Skipping unsafe R capture smoke: Rscript path unavailable.");
        return;
    };
    if !rscript_available(&rscript) {
        eprintln!("Skipping unsafe R capture smoke: Rscript unavailable.");
        return;
    }

    let src = r#"
fn raw_probe(values: vector<float>) -> float {
  let energy = 0.0
  unsafe r {
    if (!is.numeric(values)) {
      stop("raw_probe expected a numeric vector")
    }
    energy <- sum(values * values)
  }
  return energy
}

print(raw_probe(c(1.0, 2.0, 3.0)))
"#;

    let (code, _map) = compile("unsafe_r_capture.rr", src, OptLevel::O2).expect("compile");
    assert!(code.contains("# rr-unsafe-r-begin"), "{code}");
    assert!(code.contains("energy <- sum(values * values)"), "{code}");

    let dir = unique_dir(&std::env::temp_dir(), "rr_unsafe_r_capture");
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    let script = dir.join("compiled.R");
    fs::write(&script, code).expect("failed to write compiled R");

    let run = run_rscript(&rscript, &script);
    assert_eq!(
        run.status, 0,
        "compiled unsafe R capture failed\nstdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(
        normalize(&run.stdout).contains("[1] 14"),
        "unsafe R did not read params and write RR locals:\n{}",
        run.stdout
    );
}

#[test]
fn unsafe_r_mutation_inside_branch_reaches_join_when_rscript_is_available() {
    let Some(rscript) = rscript_path() else {
        eprintln!("Skipping unsafe R branch smoke: Rscript path unavailable.");
        return;
    };
    if !rscript_available(&rscript) {
        eprintln!("Skipping unsafe R branch smoke: Rscript unavailable.");
        return;
    }

    let src = r#"
fn touch(flag: bool) {
  let x = 1L
  if flag {
    unsafe r {
      x <- 7L
    }
  }
  print(x)
}

touch(TRUE)
"#;

    let (code, _map) = compile("unsafe_r_branch.rr", src, OptLevel::O2).expect("compile");
    let dir = unique_dir(&std::env::temp_dir(), "rr_unsafe_r_branch");
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    let script = dir.join("compiled.R");
    fs::write(&script, code).expect("failed to write compiled R");

    let run = run_rscript(&rscript, &script);
    assert_eq!(
        run.status, 0,
        "compiled unsafe R branch failed\nstdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(
        normalize(&run.stdout).contains("[1] 7"),
        "unsafe R branch assignment was not visible after join:\n{}",
        run.stdout
    );
}

#[test]
fn unsafe_r_mutation_inside_loop_reaches_next_condition_when_rscript_is_available() {
    let Some(rscript) = rscript_path() else {
        eprintln!("Skipping unsafe R loop smoke: Rscript path unavailable.");
        return;
    };
    if !rscript_available(&rscript) {
        eprintln!("Skipping unsafe R loop smoke: Rscript unavailable.");
        return;
    }

    let src = r#"
fn count_to_three() {
  let x = 0L
  while x < 3L {
    unsafe r {
      x <- x + 1L
    }
  }
  print(x)
}

count_to_three()
"#;

    let (code, _map) = compile("unsafe_r_loop.rr", src, OptLevel::O2).expect("compile");
    let dir = unique_dir(&std::env::temp_dir(), "rr_unsafe_r_loop");
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    let script = dir.join("compiled.R");
    fs::write(&script, code).expect("failed to write compiled R");

    let run = run_rscript(&rscript, &script);
    assert_eq!(
        run.status, 0,
        "compiled unsafe R loop failed\nstdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(
        normalize(&run.stdout).contains("[1] 3"),
        "unsafe R loop assignment was not visible to the next condition:\n{}",
        run.stdout
    );
}
