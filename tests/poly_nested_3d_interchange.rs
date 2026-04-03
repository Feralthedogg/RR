use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_nested_3d_interchange(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create nested 3d interchange test dir");

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
fn poly_applies_nested_3d_interchange_for_full_cube_map() {
    let (stats, emitted) = compile_nested_3d_interchange(
        "poly_nested_3d_interchange_map",
        r#"
import r * as base from "base"

fn poly_nested_3d_interchange_map(a, b, out) {
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        out[i, j, k] = a[i, j, k] + b[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return out
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let out = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_nested_3d_interchange_map(a, b, out))
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
        "expected nested 3d interchange map to rebuild generic loops without cube helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_nested_3d_interchange_for_full_cube_reduction() {
    let (stats, emitted) = compile_nested_3d_interchange(
        "poly_nested_3d_interchange_reduce",
        r#"
import r * as base from "base"

fn poly_nested_3d_interchange_reduce(a) {
  let acc = 0
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        acc = acc + a[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_nested_3d_interchange_reduce(a))
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
        "expected nested 3d interchange reduction to rebuild generic loops without cube helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_nested_3d_interchange_for_fused_full_cube_maps() {
    let (stats, emitted) = compile_nested_3d_interchange(
        "poly_nested_3d_interchange_multi_map",
        r#"
import r * as base from "base"

fn poly_nested_3d_interchange_multi_map(a, b, y, z) {
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        y[i, j, k] = a[i, j, k] + 1
        z[i, j, k] = b[i, j, k] + 2
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return y
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let y = base.array(rep.int(0, 27), base.c(3, 3, 3))
let z = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_nested_3d_interchange_multi_map(a, b, y, z))
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
        "expected fused 3d interchange maps to rebuild generic loops without cube helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_nested_3d_interchange_for_fused_full_cube_reductions() {
    let (stats, emitted) = compile_nested_3d_interchange(
        "poly_nested_3d_interchange_multi_reduce",
        r#"
import r * as base from "base"

fn poly_nested_3d_interchange_multi_reduce(a, b) {
  let sum_acc = 0
  let prod_acc = 1
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        sum_acc = sum_acc + a[i, j, k]
        prod_acc = prod_acc * b[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return sum_acc + prod_acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_nested_3d_interchange_multi_reduce(a, b))
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
        "expected fused 3d interchange reductions to rebuild generic loops without cube helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_nested_3d_interchange_for_fused_full_cube_minmax_reductions() {
    let (stats, emitted) = compile_nested_3d_interchange(
        "poly_nested_3d_interchange_multi_minmax_reduce",
        r#"
import r * as base from "base"

fn poly_nested_3d_interchange_multi_minmax_reduce(a) {
  let min_acc = 100
  let max_acc = 0
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        min_acc = min(min_acc, a[i, j, k])
        max_acc = max(max_acc, a[i, j, k])
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return min_acc + max_acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_nested_3d_interchange_multi_minmax_reduce(a))
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
        "expected fused 3d interchange min/max reductions to rebuild generic loops without cube helpers, got:\n{}",
        emitted
    );
    assert!(
        emitted.contains("min(") && emitted.contains("max("),
        "expected min/max aggregation in emitted code, got:\n{}",
        emitted
    );
}
