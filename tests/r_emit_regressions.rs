mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use rr::compiler::{
    CompileOutputOptions, OptLevel, compile_with_configs_with_options, default_parallel_config,
    default_type_config,
};
use std::fs;
use std::path::PathBuf;

fn compile_helper_only(tag: &str, src: &str) -> String {
    compile_with_configs_with_options(
        tag,
        src,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        CompileOutputOptions {
            inject_runtime: false,
            ..Default::default()
        },
    )
    .expect("compile should succeed")
    .0
}

fn compile_helper_only_preserve_defs(tag: &str, src: &str) -> String {
    compile_with_configs_with_options(
        tag,
        src,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        CompileOutputOptions {
            inject_runtime: false,
            preserve_all_defs: true,
            ..Default::default()
        },
    )
    .expect("compile should succeed")
    .0
}

fn strip_trailing_null(stdout: &str) -> String {
    let normalized = normalize(stdout);
    normalized
        .strip_suffix("NULL\n")
        .map(str::to_string)
        .unwrap_or(normalized)
}

#[test]
fn emitted_r_preserves_safe_source_function_names() {
    let src = r#"
fn helper_name(input_value, scale) {
  let adjusted_value = input_value + scale
  print(adjusted_value)
  adjusted_value
}

export fn compute_total(seed_value) {
  let local_total = helper_name(seed_value, 2)
  local_total
}

compute_total(40)
"#;

    let code = compile_helper_only_preserve_defs("preserve_function_names.rr", src);
    assert!(
        code.contains("helper_name <- function(input_value, scale)"),
        "expected helper function to keep its source name:\n{code}"
    );
    assert!(
        code.contains("compute_total <- function(seed_value)"),
        "expected exported function to keep its source name:\n{code}"
    );
    assert!(
        code.contains("local_total <- helper_name(seed_value, 2L)")
            || code.contains("return((helper_name(40L, 2L)))"),
        "expected user-function call sites to use the source name:\n{code}"
    );
    assert!(
        !code.contains("Sym_1 <- function(input_value, scale)")
            && !code.contains("Sym_6 <- function(seed_value)"),
        "safe source functions should not be emitted under generated Sym_* names:\n{code}"
    );
}

#[test]
fn emitted_r_keeps_generated_name_for_builtin_shadowing_function() {
    let src = r#"
fn floor(x) {
  x + 1
}

floor(4)
"#;

    let code = compile_helper_only_preserve_defs("preserve_names_builtin_shadow.rr", src);
    assert!(
        !code.contains("floor <- function(x)"),
        "builtin-shadowing functions should not replace the base R binding:\n{code}"
    );
    assert!(
        code.contains("<- function(x)"),
        "expected the function definition to remain emitted under a generated safe name:\n{code}"
    );
}

#[test]
fn emitted_r_restores_readonly_arg_alias_names_after_unsafe_read_blocks() {
    let src = r#"
export fn inspect_value(x) {
  unsafe r(read) {
    print(x)
  }
  x + 1
}

inspect_value(4)
"#;

    let code = compile_helper_only_preserve_defs("preserve_readonly_arg_alias.rr", src);
    assert!(
        code.contains("inspect_value <- function(x)"),
        "expected function to keep its source name:\n{code}"
    );
    assert!(
        !code.contains(".arg_x"),
        "read-only parameter alias should be restored to the source variable name:\n{code}"
    );
    assert!(
        code.contains("print(x)") && code.contains("return((x + 1L))"),
        "expected unsafe-read body and later expression to use the source variable name:\n{code}"
    );
}

#[test]
fn same_var_non_finite_guard_is_emitted_without_is_na_or_chain() {
    let src = r#"
fn inspect(x) {
  if (is.na(x) | !(is.finite(x))) {
    print(1L)
  } else {
    print(0L)
  }
}
inspect(1.0)
"#;

    let code = compile_helper_only("same_var_non_finite_guard.rr", src);
    assert!(
        code.contains("if (!(is.finite(x))) {") || code.contains("if (!(is.finite(.arg_x))) {"),
        "expected direct non-finite guard in emitted R:\n{code}"
    );
    assert!(
        !code.contains("is.na(x) | !(is.finite(x))")
            && !code.contains("is.na(.arg_x) | !(is.finite(.arg_x))"),
        "same-var non-finite guard should be simplified before raw text rewrites:\n{code}"
    );
}

#[test]
fn not_finite_or_zero_guard_is_emitted_without_wrapped_inner_not_parens() {
    let src = r#"
fn inspect(x) {
  if (!(is.finite(x)) | (x == 0.0)) {
    print(0L)
  } else {
    print(1L)
  }
}
inspect(4.0)
"#;

    let code = compile_helper_only("not_finite_or_zero_guard.rr", src);
    assert!(
        code.contains("(!(is.finite(x)) | (x == 0))")
            || code.contains("(!(is.finite(x)) | (x == 0.0))")
            || code.contains("(!(is.finite(.arg_x)) | (.arg_x == 0))")
            || code.contains("(!(is.finite(.arg_x)) | (.arg_x == 0.0))"),
        "expected compact non-finite-or-zero guard in emitted R:\n{code}"
    );
    assert!(
        !code.contains("((!(is.finite(x))) | (x == 0.0))")
            && !code.contains("((!(is.finite(x))) | (x == 0))")
            && !code.contains("((!(is.finite(.arg_x))) | (.arg_x == 0.0))")
            && !code.contains("((!(is.finite(.arg_x))) | (.arg_x == 0))"),
        "wrapped inner not-finite parens should be eliminated upstream:\n{code}"
    );
}

#[test]
fn upstream_guard_simplification_preserves_runtime_output() {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping r_emit regression runtime check: Rscript unavailable.");
            return;
        }
    };

    let src = r#"
fn inspect(x) {
  if (is.na(x) | !(is.finite(x))) {
    print("bad")
  } else if (!(is.finite(x)) | (x == 0.0)) {
    print("zero-or-bad")
  } else {
    print("ok")
  }
}

inspect(0.0)
inspect(4.0)
"#;
    let ref_r = r#"
main <- function() {
  inspect <- function(x) {
    if (!(is.finite(x))) {
      print("bad")
    } else if ((!(is.finite(x)) | (x == 0.0))) {
      print("zero-or-bad")
    } else {
      print("ok")
    }
  }

  inspect(0.0)
  inspect(4.0)
}

main()
"#;

    let code = compile_helper_only("guard_runtime_equivalence.rr", src);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("r_emit_regressions");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "guard_runtime");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let compiled_path = proj_dir.join("compiled.R");
    let ref_path = proj_dir.join("ref.R");
    fs::write(&compiled_path, code).expect("failed to write compiled R");
    fs::write(&ref_path, ref_r).expect("failed to write reference R");

    let compiled_run = run_rscript(&rscript, &compiled_path);
    let ref_run = run_rscript(&rscript, &ref_path);
    assert_eq!(
        compiled_run.status, 0,
        "compiled R failed: {}",
        compiled_run.stderr
    );
    assert_eq!(ref_run.status, 0, "reference R failed: {}", ref_run.stderr);
    assert_eq!(
        strip_trailing_null(&compiled_run.stdout),
        strip_trailing_null(&ref_run.stdout)
    );
    assert_eq!(normalize(&compiled_run.stderr), normalize(&ref_run.stderr));
}
