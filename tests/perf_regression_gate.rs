mod common;

use common::unique_dir;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn build_perf_program() -> String {
    let mut src = String::new();

    for f in 0..40 {
        src.push_str(&format!("fn h{}(x) {{\n", f));
        src.push_str("  let t = x\n");
        for i in 0..10 {
            src.push_str(&format!("  let v{} = (t + {}L) * 2L\n", i, (i + f) % 9));
        }
        src.push_str("  return t + 1L\n");
        src.push_str("}\n\n");
    }

    src.push_str("fn giant(n) {\n");
    src.push_str("  let acc = 0L\n");
    src.push_str("  let i = 1L\n");
    src.push_str("  while (i <= n) {\n");
    src.push_str("    let t0 = i\n");
    for f in 0..40 {
        src.push_str(&format!("    let t{} = h{}(t{})\n", f + 1, f, f));
    }
    src.push_str("    acc = acc + t40\n");
    src.push_str("    i = i + 1L\n");
    src.push_str("  }\n");
    src.push_str("  return acc\n");
    src.push_str("}\n\n");

    src.push_str("fn main() {\n");
    src.push_str("  print(giant(64L))\n");
    src.push_str("}\n\n");
    src.push_str("main()\n");

    src
}

fn build_trait_sroa_perf_program() -> String {
    let mut src = String::from(
        r#"
trait Add {
  fn add(self: Self, rhs: Self) -> Self
}
trait Neg {
  fn neg(self: Self) -> Self
}
trait Transformable {
  fn translate(self: Self, offset: Self) -> Self
  fn scale(self: Self, factor: float) -> Self
}

impl Add for Vec2 {
  fn add(self: Vec2, rhs: Vec2) -> Vec2 {
    {x: self.x + rhs.x, y: self.y + rhs.y}
  }
}
impl Neg for Vec2 {
  fn neg(self: Vec2) -> Vec2 {
    {x: 0.0 - self.x, y: 0.0 - self.y}
  }
}
impl Transformable for Vec2 {
  fn translate(self: Vec2, offset: Vec2) -> Vec2 {
    self + offset
  }
  fn scale(self: Vec2, factor: float) -> Vec2 {
    {x: self.x * factor, y: self.y * factor}
  }
}

fn simulate_rebound<T>(entity: T, velocity: T, time: float) -> T
  where T: Add + Neg + Transformable
{
  let moved = entity + velocity
  let rebounded_vel = -velocity
  moved.translate(rebounded_vel).scale(time)
}

fn main() {
  let scratch = seq_len(8L)
  let acc = 0.0
"#,
    );

    for i in 0..24 {
        src.push_str(&format!(
            r#"
  let p{i}: Vec2 = {{x: {x}.0, y: {y}.0}}
  let v{i}: Vec2 = {{x: 2.0, y: -3.0}}
  let r{i} = simulate_rebound(p{i}, v{i}, 1.5)
  scratch[1L] = r{i}.x
  acc = acc + r{i}.y + scratch[1L]
"#,
            x = 10 + i,
            y = 15 + i
        ));
    }

    src.push_str(
        r#"
  print(acc)
}

main()
"#,
    );
    src
}

fn compile_elapsed_ms(rr_bin: &PathBuf, input: &PathBuf, output: &PathBuf, level: &str) -> u128 {
    let start = Instant::now();
    let status = Command::new(rr_bin)
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("--no-runtime")
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(status.success(), "compile failed for {}", level);
    start.elapsed().as_millis()
}

#[test]
fn compile_time_regression_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("perf_regression_gate");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join("perf_case.rr");
    let out_o1 = proj_dir.join("perf_o1.R");
    let out_o2 = proj_dir.join("perf_o2.R");
    fs::write(&rr_path, build_perf_program()).expect("failed to write perf case");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o1_ms = compile_elapsed_ms(&rr_bin, &rr_path, &out_o1, "-O1");
    let o2_ms = compile_elapsed_ms(&rr_bin, &rr_path, &out_o2, "-O2");

    let budget_ms: u128 = env::var("RR_PERF_GATE_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(45_000);
    let ratio_limit: f64 = env::var("RR_PERF_O2_O1_RATIO")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(12.0);
    let o2_vs_o1_limit = (o1_ms as f64 * ratio_limit + 1_500.0) as u128;

    assert!(
        o2_ms <= budget_ms,
        "compile-time budget exceeded: O2={}ms > budget={}ms (set RR_PERF_GATE_MS to tune)",
        o2_ms,
        budget_ms
    );
    assert!(
        o2_ms <= o2_vs_o1_limit,
        "O2 slowdown regression: O1={}ms, O2={}ms, limit={}ms (ratio={}, slack=1500ms)",
        o1_ms,
        o2_ms,
        o2_vs_o1_limit,
        ratio_limit
    );
}

#[test]
fn trait_sroa_compile_shape_regression_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("perf_regression_gate");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "trait_sroa");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join("trait_sroa_perf.rr");
    let out_o2 = proj_dir.join("trait_sroa_o2.R");
    fs::write(&rr_path, build_trait_sroa_perf_program()).expect("failed to write trait SROA case");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o2_ms = compile_elapsed_ms(&rr_bin, &rr_path, &out_o2, "-O2");
    let output = fs::read_to_string(&out_o2).expect("failed to read trait SROA output");

    let budget_ms: u128 = env::var("RR_PERF_TRAIT_SROA_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(45_000);
    assert!(
        o2_ms <= budget_ms,
        "trait/SROA compile-time budget exceeded: O2={}ms > budget={}ms (set RR_PERF_TRAIT_SROA_MS to tune)",
        o2_ms,
        budget_ms
    );

    let list_count = output.matches("list(").count();
    let sroa_temp_count = output.matches("__rr_sroa_").count();
    let max_output_bytes = 24 * 1024;
    assert!(
        output.len() <= max_output_bytes,
        "trait/SROA output shape grew unexpectedly: bytes={} > max={max_output_bytes}\n{output}",
        output.len()
    );
    assert_eq!(
        list_count, 0,
        "trait/SROA cross-call scalarization should remove hot-path record allocations; list_count={list_count}\n{output}"
    );
    assert!(
        sroa_temp_count >= 8,
        "trait/SROA benchmark should expose helper-local scalar replacement temps; sroa_temp_count={sroa_temp_count}\n{output}"
    );
    assert!(
        !output.contains("$translate") && !output.contains("$scale"),
        "trait/SROA benchmark must keep static dispatch in optimized output:\n{output}"
    );
    assert!(
        output.contains("r0__rr_sroa_ret_x")
            && output.contains("r0__rr_sroa_ret_y")
            && !output.contains("Sym_35(")
            && !output.contains("[[\"x\"]]")
            && !output.contains("[[\"y\"]]"),
        "trait/SROA benchmark should inline scalarized record-return fields across the helper boundary:\n{output}"
    );
}
