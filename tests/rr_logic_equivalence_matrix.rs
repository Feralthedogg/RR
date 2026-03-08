mod common;

use RR::compiler::{OptLevel, compile_with_config};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::PathBuf;

struct EqCase {
    name: &'static str,
    rr_src: &'static str,
    ref_r_src: &'static str,
}

fn opt_tag(level: OptLevel) -> &'static str {
    match level {
        OptLevel::O0 => "o0",
        OptLevel::O1 => "o1",
        OptLevel::O2 => "o2",
    }
}

#[test]
fn rr_compiled_r_matches_reference_logic_across_mode_matrix() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping RR logic equivalence matrix: Rscript unavailable.");
            return;
        }
    };

    let cases = [
        EqCase {
            name: "loop_branch_sum",
            rr_src: r#"
fn kernel(n, k) {
  let x = seq_len(n)

  let i = 1L

  let s = 0L

  while (i <= length(x)) {
    if (x[i] > k) {
      s = s + (x[i] * 2L)

    } else {
      s = s + (x[i] - 1L)

    }
    i = i + 1L

  }
  return s

}
print(kernel(12L, 7L))

"#,
            ref_r_src: r#"
kernel <- function(n, k) {
  x <- seq_len(n)
  i <- 1L
  s <- 0L
  while (i <= length(x)) {
    if (x[i] > k) {
      s <- s + (x[i] * 2L)
    } else {
      s <- s + (x[i] - 1L)
    }
    i <- i + 1L
  }
  s
}
print(kernel(12L, 7L))
"#,
        },
        EqCase {
            name: "matrix_row_col_summary",
            rr_src: r#"
fn main() {
  let v = seq_len(6L)

  let m = matrix(v, 2L, 3L)

  let r = rowSums(m)

  let c = colSums(m)

  print(sum(r))

  print(sum(c))

  print(m[2L, 3L])

  return sum(r) + sum(c) + m[2L, 3L]

}
print(main())

"#,
            ref_r_src: r#"
main <- function() {
  v <- seq_len(6L)
  m <- matrix(v, 2L, 3L)
  r <- rowSums(m)
  c <- colSums(m)
  print(sum(r))
  print(sum(c))
  print(m[2L, 3L])
  sum(r) + sum(c) + m[2L, 3L]
}
print(main())
"#,
        },
        EqCase {
            name: "na_semantics_bundle",
            rr_src: r#"
fn main() {
  let x = c(1L, NA, 3L)

  let l = c(TRUE, NA, FALSE)

  print(x + 2L)

  print(l & TRUE)

  print(l | FALSE)

  print(x[NA])

  return 0L

}
print(main())

"#,
            ref_r_src: r#"
main <- function() {
  x <- c(1L, NA, 3L)
  l <- c(TRUE, NA, FALSE)
  print(x + 2L)
  print(l & TRUE)
  print(l | FALSE)
  print(x[NA])
  0L
}
print(main())
"#,
        },
        EqCase {
            name: "typed_numeric_reduce",
            rr_src: r#"
fn score(n: int) -> float {
  let x = seq_len(n)

  let y = abs((x * 3L) - 7L)

  let s = sum(y)

  let m = mean(y)

  return s + m

}
print(score(10L))

"#,
            ref_r_src: r#"
score <- function(n) {
  x <- seq_len(n)
  y <- abs((x * 3L) - 7L)
  s <- sum(y)
  m <- mean(y)
  s + m
}
print(score(10L))
"#,
        },
        EqCase {
            name: "closure_and_apply",
            rr_src: r#"
fn apply_twice(f, x) {
  return f(f(x))

}

fn main() {
  let seed = 5L

  let add_seed = fn(v) { return v + seed
 }

  let r1 = apply_twice(add_seed, 10L)

  let r2 = (fn(z) { return z * 2L
 })(7L)

  print(r1)

  print(r2)

  return r1 + r2

}
print(main())

"#,
            ref_r_src: r#"
apply_twice <- function(f, x) {
  f(f(x))
}

main <- function() {
  seed <- 5L
  add_seed <- function(v) { v + seed }
  r1 <- apply_twice(add_seed, 10L)
  r2 <- (function(z) { z * 2L })(7L)
  print(r1)
  print(r2)
  r1 + r2
}
print(main())
"#,
        },
    ];

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("rr_logic_equivalence_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let opt_levels = [OptLevel::O0, OptLevel::O1, OptLevel::O2];
    let type_modes = [TypeMode::Strict, TypeMode::Gradual];
    let native_backends = [NativeBackend::Off, NativeBackend::Optional];

    for case in &cases {
        let ref_path = proj_dir.join(format!("{}_ref.R", case.name));
        fs::write(&ref_path, case.ref_r_src).expect("failed to write reference R source");
        let reference = run_rscript(&rscript, &ref_path);
        assert_eq!(
            reference.status, 0,
            "reference R failed for case {}\nstdout:\n{}\nstderr:\n{}",
            case.name, reference.stdout, reference.stderr
        );

        for opt in opt_levels {
            for mode in type_modes {
                for native in native_backends {
                    let cfg = TypeConfig {
                        mode,
                        native_backend: native,
                    };
                    let input_name = format!(
                        "{}_{}_{}_{}.rr",
                        case.name,
                        opt_tag(opt),
                        mode.as_str(),
                        native.as_str()
                    );
                    let (compiled_code, _source_map) =
                        compile_with_config(&input_name, case.rr_src, opt, cfg).unwrap_or_else(
                            |e| {
                                panic!(
                                    "compile failed for case={} opt={} type={} native={}: {:?}",
                                    case.name,
                                    opt_tag(opt),
                                    mode.as_str(),
                                    native.as_str(),
                                    e
                                )
                            },
                        );
                    let compiled_path = proj_dir.join(format!(
                        "{}_{}_{}_{}.R",
                        case.name,
                        opt_tag(opt),
                        mode.as_str(),
                        native.as_str()
                    ));
                    fs::write(&compiled_path, compiled_code)
                        .expect("failed to write compiled output");
                    let compiled = run_rscript(&rscript, &compiled_path);

                    assert_eq!(
                        reference.status,
                        compiled.status,
                        "exit status mismatch case={} opt={} type={} native={}",
                        case.name,
                        opt_tag(opt),
                        mode.as_str(),
                        native.as_str()
                    );
                    assert_eq!(
                        normalize(&reference.stdout),
                        normalize(&compiled.stdout),
                        "stdout mismatch case={} opt={} type={} native={}\nref:\n{}\ncompiled:\n{}",
                        case.name,
                        opt_tag(opt),
                        mode.as_str(),
                        native.as_str(),
                        reference.stdout,
                        compiled.stdout
                    );
                    assert_eq!(
                        normalize(&reference.stderr),
                        normalize(&compiled.stderr),
                        "stderr mismatch case={} opt={} type={} native={}\nref:\n{}\ncompiled:\n{}",
                        case.name,
                        opt_tag(opt),
                        mode.as_str(),
                        native.as_str(),
                        reference.stderr,
                        compiled.stderr
                    );
                }
            }
        }
    }
}
