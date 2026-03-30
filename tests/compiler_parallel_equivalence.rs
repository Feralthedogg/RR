use RR::compiler::{
    CompileOutputOptions, CompilerParallelConfig, CompilerParallelMode, OptLevel,
    compile_with_configs_with_options_and_compiler_parallel, default_parallel_config,
    default_type_config,
};
use RR::error::RRException;

fn serial_compiler_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::Off,
        ..CompilerParallelConfig::default()
    }
}

fn enabled_compiler_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 2,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 2,
    }
}

fn compile_equivalence_fixture(
    entry_path: &str,
    source: &str,
    compiler_parallel_cfg: CompilerParallelConfig,
    output_opts: CompileOutputOptions,
) -> Result<(String, Vec<RR::codegen::mir_emit::MapEntry>), RRException> {
    compile_with_configs_with_options_and_compiler_parallel(
        entry_path,
        source,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        compiler_parallel_cfg,
        output_opts,
    )
}

fn compile_error_signature(err: &RRException) -> String {
    fn push_signature(err: &RRException, out: &mut String) {
        out.push_str(err.code.as_str());
        out.push('|');
        out.push_str(&format!("{:?}", err.stage));
        out.push('|');
        out.push_str(err.message.as_ref());
        out.push('|');
        out.push_str(&format!("{:?}", err.span));
        out.push('|');
        out.push_str(&format!("{:?}", err.labels));
        out.push('|');
        out.push_str(&format!("{:?}", err.fixes));
        out.push('|');
        out.push_str(&format!("{:?}", err.notes));
        out.push('|');
        out.push_str(&format!("{:?}", err.helps));
        out.push('|');
        out.push_str(&format!("{:?}", err.stacktrace));
        out.push('|');
        out.push_str(&err.related.len().to_string());
        out.push(';');
        for related in err.related.iter() {
            out.push('[');
            push_signature(related, out);
            out.push(']');
        }
    }

    let mut out = String::new();
    push_signature(err, &mut out);
    out
}

#[test]
fn compiler_parallel_output_and_source_map_match_serial() {
    let source = r#"
fn sq(x) {
  return x * x
}

fn sumsq(a, b, c, d) {
  let acc = 0.0
  acc = acc + sq(a)
  acc = acc + sq(b)
  acc = acc + sq(c)
  acc = acc + sq(d)
  return acc
}

fn series(n) {
  let out = numeric(n)
  let i = 1L
  while (i <= n) {
    out[i] = sumsq(i, i + 1L, i + 2L, i + 3L)
    i = i + 1L
  }
  return out
}

print(series(8L))
"#;
    let entry_path = "compiler_parallel_equivalence.rr";
    let output_opts = CompileOutputOptions::default();
    let (serial_code, serial_map) = compile_equivalence_fixture(
        entry_path,
        source,
        serial_compiler_parallel_cfg(),
        output_opts,
    )
    .expect("serial compile failed");
    let (parallel_code, parallel_map) = compile_equivalence_fixture(
        entry_path,
        source,
        enabled_compiler_parallel_cfg(),
        output_opts,
    )
    .expect("parallel compile failed");

    assert_eq!(serial_code, parallel_code);
    assert_eq!(serial_map, parallel_map);
}

#[test]
fn compiler_parallel_repeated_runs_are_deterministic() {
    let source = r#"
fn sq(x) {
  return x * x
}

fn cube(x) {
  return x * x * x
}

fn mix(a, b) {
  return sq(a) + cube(b)
}

fn main() {
  let out = numeric(6L)
  let i = 1L
  while (i <= 6L) {
    out[i] = mix(i, i + 1L)
    i = i + 1L
  }
  print(out)
}
main()
"#;
    let entry_path = "compiler_parallel_determinism.rr";
    let output_opts = CompileOutputOptions::default();
    let (expected_code, expected_map) = compile_equivalence_fixture(
        entry_path,
        source,
        enabled_compiler_parallel_cfg(),
        output_opts,
    )
    .expect("baseline parallel compile failed");

    for iteration in 0..8 {
        let (code, map) = compile_equivalence_fixture(
            entry_path,
            source,
            enabled_compiler_parallel_cfg(),
            output_opts,
        )
        .expect("repeat parallel compile failed");
        assert_eq!(
            expected_code, code,
            "parallel code drifted on iteration {}",
            iteration
        );
        assert_eq!(
            expected_map, map,
            "parallel source map drifted on iteration {}",
            iteration
        );
    }
}

#[test]
fn compiler_parallel_diagnostics_match_serial() {
    let source = r#"
fn bad_alpha() -> float {
  return "oops"
}

fn bad_beta() -> int {
  return "still nope"
}

bad_alpha()
bad_beta()
"#;
    let entry_path = "compiler_parallel_diagnostics.rr";
    let output_opts = CompileOutputOptions::default();

    let serial_err = compile_equivalence_fixture(
        entry_path,
        source,
        serial_compiler_parallel_cfg(),
        output_opts,
    )
    .expect_err("serial compile should fail");
    let parallel_err = compile_equivalence_fixture(
        entry_path,
        source,
        enabled_compiler_parallel_cfg(),
        output_opts,
    )
    .expect_err("parallel compile should fail");

    assert_eq!(
        compile_error_signature(&serial_err),
        compile_error_signature(&parallel_err)
    );
}

#[test]
fn compiler_parallel_keeps_quoted_entry_source_map_lines_stable() {
    let source = r#"fn main() {
  let a0 = 1L
  let a1 = a0 + 1L
  let a2 = a1 + 1L
  let a3 = a2 + 1L
  let a4 = a3 + 1L
  let a5 = a4 + 1L
  let a6 = a5 + 1L
  let a7 = a6 + 1L
  let a8 = a7 + 1L
  let a9 = a8 + 1L
  let a10 = a9 + 1L
  let a11 = a10 + 1L
  let a12 = a11 + 1L
  let a13 = a12 + 1L
  let a14 = a13 + 1L
  let a15 = a14 + 1L
  print("quoted-wrapper-line")
  print(a15)
}
main()
"#;
    let entry_path = "compiler_parallel_quoted_entry.rr";
    let output_opts = CompileOutputOptions {
        inject_runtime: false,
        ..CompileOutputOptions::default()
    };

    let (serial_code, serial_map) = compile_equivalence_fixture(
        entry_path,
        source,
        serial_compiler_parallel_cfg(),
        output_opts,
    )
    .expect("serial compile failed");
    let (parallel_code, parallel_map) = compile_equivalence_fixture(
        entry_path,
        source,
        enabled_compiler_parallel_cfg(),
        output_opts,
    )
    .expect("parallel compile failed");

    assert_eq!(serial_code, parallel_code);
    assert_eq!(serial_map, parallel_map);
    assert!(
        serial_code.contains(".__rr_body_"),
        "expected quoted body wrapper in emitted code:\n{}",
        serial_code
    );

    let rr_line = source
        .lines()
        .position(|line| line.contains("quoted-wrapper-line"))
        .map(|idx| idx as u32 + 1)
        .expect("expected quoted wrapper marker in RR source");
    let generated_code_header_line = serial_code
        .lines()
        .position(|line| line == "# --- RR generated code (from user RR source) ---")
        .map(|idx| idx as u32 + 1)
        .expect("expected generated-code header in emitted code");
    let mapped_lines: Vec<u32> = serial_map
        .iter()
        .filter(|entry| entry.rr_span.start_line == rr_line)
        .map(|entry| entry.r_line)
        .collect();

    assert!(
        mapped_lines
            .iter()
            .any(|line| *line > generated_code_header_line),
        "expected a shifted source map entry for RR line {} after generated-code header line {} but got {:?}\ncode:\n{}",
        rr_line,
        generated_code_header_line,
        mapped_lines,
        serial_code
    );
}
