use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_src.display(),
        level
    );
}

fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    Some("Rscript".to_string())
}

fn rscript_available(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_rscript(path: &str, script: &Path) -> (i32, String) {
    let output = Command::new(path)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

#[test]
fn vectorization_supports_reduction_conditional_and_recurrence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_extended");
    fs::create_dir_all(&out_dir).expect("failed to create test output dir");

    let rr_src = r#"
fn red(n) {
  let x = seq_len(n)

  let s = 0

  for (i in 1..length(x)) {
    s = s + x[i]

  }
  return s

}

fn red_plus_const(n, k) {
  let x = seq_len(n)

  let s = 0

  for (i in 1..length(x)) {
    s = s + (x[i] + k)

  }
  return s

}

fn cond_map(n, k) {
  let b = seq_len(n)

  let c = seq_len(n) + 100

  let a = seq_len(n)

  for (i in 1..length(a)) {
    if (i > k) {
      a[i] = b[i]

    } else {
      a[i] = c[i]

    }
  }
  return a

}

fn cond_map_data(n, k) {
  let b = seq_len(n)

  let a = seq_len(n)

  for (i in 1..length(a)) {
    if (b[i] > k) {
      a[i] = b[i]

    } else {
      a[i] = 0

    }
  }
  return a

}

fn gt(v, k) {
  return v > k

}

fn cond_map_call(n, k) {
  let b = seq_len(n)

  let a = seq_len(n)

  for (i in 1..length(a)) {
    if (gt(b[i], k)) {
      a[i] = b[i]

    } else {
      a[i] = 0

    }
  }
  return a

}

fn recur(n) {
  let a = seq_len(n)

  for (i in 2..n) {
    a[i] = a[i - 1] + 1

  }
  return a

}

fn recur_sub(n) {
  let a = seq_len(n)

  for (i in 2..n) {
    a[i] = a[i - 1] - 2

  }
  return a

}

fn shift_back(n) {
  let x = seq_len(n)

  let y = seq_len(n)

  for (i in 2..length(x)) {
    y[i] = x[i - 1]

  }
  return y

}

fn call_abs(n) {
  let x = seq_len(n) - 4

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = abs(x[i])

  }
  return y

}

fn call_pmax(n) {
  let x = seq_len(n)

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = pmax(x[i], 3)

  }
  return y

}

fn call_pmax_param(n, k) {
  let x = seq_len(n)

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = pmax(x[i], k)

  }
  return y

}

fn call_pmax_zip(n) {
  let x = seq_len(n)

  let z = (seq_len(n) * 2) - 3

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = pmax(x[i], z[i])

  }
  return y

}

fn call_pmax_params(x, z) {
  let y = seq_len(length(x))

  for (i in 1..length(y)) {
    y[i] = pmax(x[i], z[i])

  }
  return y

}

fn call_log10(n) {
  let x = seq_len(n)

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = log10(x[i])

  }
  return y

}

fn call_atan2(n) {
  let x = seq_len(n)

  let z = seq_len(n) + 1

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = atan2(x[i], z[i])

  }
  return y

}

fn call_abs_nested(n) {
  let x = seq_len(n)

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = abs(x[i] + 1)

  }
  return y

}

fn call_pmax_nested(n) {
  let x = seq_len(n)

  let z = seq_len(n) + 5

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = pmax(x[i] + 1, z[i] * 2)

  }
  return y

}

fn call_pmax_chain(n) {
  let x = seq_len(n)

  let z = seq_len(n) + 5

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = pmax(log(x[i] + 1), sqrt(z[i]))

  }
  return y

}

fn mat_row(n, m) {
  let a = matrix(seq_len(n * m), n, m)

  let b = matrix(seq_len(n * m), n, m)

  let c = matrix(seq_len(n * m), n, m)

  for (i in 1..n) {
    for (j in 1..m) {
      c[i, j] = a[i, j] + b[i, j]

    }
  }
  return c

}

fn mat_col(n, m) {
  let a = matrix(seq_len(n * m), n, m)

  let b = matrix(seq_len(n * m), n, m)

  let c = matrix(seq_len(n * m), n, m)

  for (j in 1..m) {
    for (i in 1..n) {
      c[i, j] = a[i, j] + b[i, j]

    }
  }
  return c

}

fn mat_row_fixed(m) {
  let a = matrix(seq_len(3 * m), 3, m)

  let b = matrix(seq_len(3 * m), 3, m)

  let c = matrix(seq_len(3 * m), 3, m)

  for (j in 1..m) {
    c[1, j] = a[1, j] + b[1, j]

  }
  return c

}

fn mat_col_fixed(n) {
  let a = matrix(seq_len(n * 3), n, 3)

  let b = matrix(seq_len(n * 3), n, 3)

  let c = matrix(seq_len(n * 3), n, 3)

  for (i in 1..n) {
    c[i, 1] = a[i, 1] + b[i, 1]

  }
  return c

}

fn mat_row_param(m, r0) {
  let a = matrix(seq_len(3 * m), 3, m)

  let b = matrix(seq_len(3 * m), 3, m)

  let c = matrix(seq_len(3 * m), 3, m)

  for (j in 1..m) {
    c[r0, j] = a[r0, j] + b[r0, j]

  }
  return c

}

fn mat_col_param(n, c0) {
  let a = matrix(seq_len(n * 3), n, 3)

  let b = matrix(seq_len(n * 3), n, 3)

  let c = matrix(seq_len(n * 3), n, 3)

  for (i in 1..n) {
    c[i, c0] = a[i, c0] + b[i, c0]

  }
  return c

}

fn mat_row_tail(m) {
  let a = matrix(seq_len(3 * m), 3, m)

  let b = matrix(seq_len(3 * m), 3, m)

  let c = matrix(seq_len(3 * m), 3, m)

  for (j in 2..m) {
    c[1, j] = a[1, j] + b[1, j]

  }
  return c

}

fn mat_col_tail(n) {
  let a = matrix(seq_len(n * 3), n, 3)

  let b = matrix(seq_len(n * 3), n, 3)

  let c = matrix(seq_len(n * 3), n, 3)

  for (i in 2..n) {
    c[i, 1] = a[i, 1] + b[i, 1]

  }
  return c

}

fn mat_row_reduce(n, m, r0) {
  let a = matrix(seq_len(n * m), n, m)

  let s = 0

  for (j in 1..m) {
    s = s + a[r0, j]

  }
  return s

}

fn mat_col_reduce(n, m, c0) {
  let a = matrix(seq_len(n * m), n, m)

  let s = 0

  for (i in 1..n) {
    s = s + a[i, c0]

  }
  return s

}

print(red(10))

print(red_plus_const(10, 3))

print(cond_map(8, 3))

print(cond_map_data(8, 3))

print(cond_map_call(8, 3))

print(recur(8))

print(recur_sub(8))

print(shift_back(8))

print(call_abs(8))

print(call_pmax(8))

print(call_pmax_param(8, 4))

print(call_pmax_zip(8))

print(call_pmax_params(seq_len(8), seq_len(8) + 1))

print(call_log10(8))

print(call_atan2(8))

print(call_abs_nested(8))

print(call_pmax_nested(8))

print(call_pmax_chain(8))

print(mat_row_fixed(4))

print(mat_col_fixed(3))

print(mat_row_param(4, 2))

print(mat_col_param(3, 2))

print(mat_row_tail(4))

print(mat_col_tail(3))

print(mat_row_reduce(3, 4, 2))

print(mat_col_reduce(3, 4, 2))

"#;

    let rr_path = out_dir.join("vectorization_extended.rr");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o0 = out_dir.join("vectorization_extended_o0.R");
    let o1 = out_dir.join("vectorization_extended_o1.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o1, "-O1");

    let code = fs::read_to_string(&o1).expect("failed to read O1 output");
    assert!(
        code.contains("sum("),
        "expected reduction vectorization to use sum(...)"
    );
    assert!(
        code.contains("rr_ifelse_strict("),
        "expected conditional map vectorization to use rr_ifelse_strict(...)"
    );
    assert!(
        code.contains("rr_shift_assign("),
        "expected shifted map vectorization to use rr_shift_assign(...)"
    );
    assert!(
        code.contains("rr_recur_add_const("),
        "expected recurrence vectorization to use rr_recur_add_const(...)"
    );
    assert!(
        code.contains("(-2L)") || code.contains("-(2L)") || code.contains("-2"),
        "expected subtraction recurrence to preserve negative delta"
    );
    assert!(
        code.contains("abs("),
        "expected call-map vectorization to use abs(...)"
    );
    assert!(
        code.contains("pmax("),
        "expected multi-arg call-map vectorization to use pmax(...)"
    );
    assert!(
        code.contains("rr_same_or_scalar("),
        "expected invariant call-map args to be guarded by rr_same_or_scalar(...)"
    );
    assert!(
        code.contains("log10("),
        "expected extended call-map vectorization to use log10(...)"
    );
    assert!(
        code.contains("atan2("),
        "expected extended call-map vectorization to use atan2(...)"
    );
    assert!(
        code.contains("abs((x + 1L))")
            || code.contains("abs((x + i))")
            || code.contains(".tachyon_callmap_arg0_0 <- (x + 1L)")
            || code.contains(".tachyon_callmap_arg0_0 <- (x + i)"),
        "expected nested call-map vectorization for abs(x[i]+1) with optional arg hoisting"
    );
    assert!(
        code.contains("pmax((x + 1L),")
            || code.contains("pmax((x + i),")
            || code.contains(".tachyon_callmap_arg"),
        "expected nested call-map vectorization for pmax(x[i]+1, ...) with optional arg hoisting"
    );
    assert!(
        code.contains("log((x + 1L))")
            || code.contains("log((x + i))")
            || code.contains("pmax(log((x + i))"),
        "expected call-chain vectorization to preserve nested log(x[i]+1)"
    );
    assert!(
        code.contains("sqrt("),
        "expected call-chain vectorization to preserve nested sqrt(...)"
    );
    assert!(
        code.contains("rr_same_len("),
        "expected call-map vectorization to keep rr_same_len guard when lengths are not provably equal"
    );
    assert!(
        code.contains("rr_intrinsic_vec_abs_f64(")
            || code.contains("rr_intrinsic_vec_pmax_f64(")
            || code.contains("rr_intrinsic_vec_sum_f64(")
            || code.contains("rr_intrinsic_vec_mean_f64(")
            || code.contains("abs("),
        "expected intrinsic or builtin vectorized call emission"
    );
    assert!(
        code.contains("rr_row_binop_assign("),
        "expected row matrix vectorization to use rr_row_binop_assign(...)"
    );
    assert!(
        code.contains("rr_col_binop_assign("),
        "expected col matrix vectorization to use rr_col_binop_assign(...)"
    );
    assert!(
        code.contains("rr_row_sum_range("),
        "expected row-reduction vectorization to use rr_row_sum_range(...)"
    );
    assert!(
        code.contains("rr_col_sum_range("),
        "expected col-reduction vectorization to use rr_col_sum_range(...)"
    );
    assert!(
        code.contains("* length("),
        "expected sum-plus-const reduction to include c * length(x) style rewrite"
    );

    if let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) {
        let (s0, out0) = run_rscript(&rscript, &o0);
        let (s1, out1) = run_rscript(&rscript, &o1);
        assert_eq!(s0, 0, "O0 execution failed");
        assert_eq!(s1, 0, "O1 execution failed");
        assert_eq!(out0.replace("\r\n", "\n"), out1.replace("\r\n", "\n"));
    }
}
