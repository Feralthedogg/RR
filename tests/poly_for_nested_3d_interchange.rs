use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_for_nested_3d_interchange(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create nested 3d for interchange test dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_PULSE_JSON_PATH", &stats_path)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).expect("failed to read pulse stats json");
    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    (stats, emitted)
}

#[test]
fn poly_applies_for_nested_3d_interchange_map() {
    let (stats, emitted) = compile_for_nested_3d_interchange(
        "poly_for_nested_3d_interchange_map",
        r#"
import r * as base from "base"

fn poly_for_nested_3d_interchange_map(a, b, out, n, m, p) {
  for (i in 1..n) {
    for (j in 1..m) {
      for (k in 1..p) {
        out[i, j, k] = a[i, j, k] + b[i, j, k]
      }
    }
  }
  return out
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let out = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_for_nested_3d_interchange_map(a, b, out, 3, 3, 3))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected one applied interchange poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_array3_binop_cube_assign("),
        "expected for-loop 3d interchange map to rebuild generic loops without cube helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_nested_3d_interchange_fused_reduction() {
    let (stats, emitted) = compile_for_nested_3d_interchange(
        "poly_for_nested_3d_interchange_multi_reduce",
        r#"
import r * as base from "base"

fn poly_for_nested_3d_interchange_multi_reduce(a, b, n, m, p) {
  let sum_acc = 0
  let prod_acc = 1
  for (i in 1..n) {
    for (j in 1..m) {
      for (k in 1..p) {
        sum_acc = sum_acc + a[i, j, k]
        prod_acc = prod_acc * b[i, j, k]
      }
    }
  }
  return sum_acc + prod_acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_for_nested_3d_interchange_multi_reduce(a, b, 3, 3, 3))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected one applied interchange poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_array3_reduce_cube("),
        "expected fused for-loop 3d interchange reductions to rebuild generic loops without cube helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_nested_3d_interchange_fused_map() {
    let (stats, emitted) = compile_for_nested_3d_interchange(
        "poly_for_nested_3d_interchange_multi_map",
        r#"
import r * as base from "base"

fn poly_for_nested_3d_interchange_multi_map(a, b, y, z, n, m, p) {
  for (i in 1..n) {
    for (j in 1..m) {
      for (k in 1..p) {
        y[i, j, k] = a[i, j, k] + 1
        z[i, j, k] = b[i, j, k] + 2
      }
    }
  }
  return y
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let y = base.array(rep.int(0, 27), base.c(3, 3, 3))
let z = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_for_nested_3d_interchange_multi_map(a, b, y, z, 3, 3, 3))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected one applied interchange poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_array3_binop_cube_assign("),
        "expected fused for-loop 3d interchange maps to rebuild generic loops without cube helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_nested_3d_interchange_fused_minmax_reduction() {
    let (stats, emitted) = compile_for_nested_3d_interchange(
        "poly_for_nested_3d_interchange_multi_minmax_reduce",
        r#"
import r * as base from "base"

fn poly_for_nested_3d_interchange_multi_minmax_reduce(a, n, m, p) {
  let min_acc = 100
  let max_acc = 0
  for (i in 1..n) {
    for (j in 1..m) {
      for (k in 1..p) {
        min_acc = min(min_acc, a[i, j, k])
        max_acc = max(max_acc, a[i, j, k])
      }
    }
  }
  return min_acc + max_acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_for_nested_3d_interchange_multi_minmax_reduce(a, 3, 3, 3))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected one applied interchange poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_array3_reduce_cube("),
        "expected fused for-loop 3d interchange minmax reductions to rebuild generic loops without cube helpers, got:\n{}",
        emitted
    );
    assert!(
        emitted.contains("min(") && emitted.contains("max("),
        "expected min/max aggregation in emitted code, got:\n{}",
        emitted
    );
}
