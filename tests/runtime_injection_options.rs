use rr::compiler::{
    CompileOutputOptions, OptLevel, compile_with_configs, compile_with_configs_with_options,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn unique_dir(root: &std::path::Path, name: &str) -> PathBuf {
    static UNIQUE_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let seq = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    root.join(format!("{}_{}_{}", name, std::process::id(), seq))
}

#[test]
fn no_runtime_flag_emits_pure_r_without_runtime_prelude() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_root = root
        .join("target")
        .join("tests")
        .join("runtime_injection_options");
    fs::create_dir_all(&out_root).expect("failed to create output dir");
    let proj = unique_dir(&out_root, "no_runtime");
    fs::create_dir_all(&proj).expect("failed to create sandbox dir");

    let rr_path = proj.join("main.rr");
    let out_path = proj.join("main.R");
    fs::write(
        &rr_path,
        r#"
fn add(a, b) {
  return a + b

}

print(add(1, 2))

"#,
    )
    .expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O0")
        .status()
        .expect("failed to run RR compiler");
    assert!(status.success(), "compile failed for {}", rr_path.display());

    let generated = fs::read_to_string(&out_path).expect("failed to read generated R");
    assert!(
        !generated.contains("rr_set_source(\""),
        "runtime bootstrap should be omitted when --no-runtime is set"
    );
    assert!(
        !generated.contains("rr_set_native_roots(c("),
        "compile-time runtime anchors should be omitted when --no-runtime is set"
    );
    assert!(
        generated.contains("# --- RR runtime (auto-generated) ---"),
        "helper library should remain available for generated code"
    );
    assert!(
        generated.contains("# --- RR generated code (from user RR source) ---"),
        "helper-only output should clearly separate generated RR code from runtime helpers"
    );
    assert!(
        generated.contains("# --- RR synthesized entrypoints (auto-generated) ---"),
        "helper-only output should clearly separate synthesized entrypoints"
    );
    assert!(
        !generated.contains("rr_set_type_mode <- function"),
        "unused runtime configuration helper definitions should stay omitted"
    );
    assert!(
        !generated.contains("rr_assign_slice <- function"),
        "unused runtime helpers should be omitted from helper-only output"
    );
    assert!(
        !generated.contains("rr_parallel_typed_vec_call <- function"),
        "unrelated runtime helpers should not be injected"
    );
    assert!(
        !generated.contains("rr_array3_shift_assign <- function"),
        "large unused array helpers should not be injected"
    );
    assert!(
        !generated.contains("rr_set_source <- function"),
        "unused source bootstrap helpers should not be injected in helper-only mode"
    );
    assert!(
        !generated.contains("rr_set_native_roots <- function"),
        "unused native-root helpers should not be injected in helper-only mode"
    );
    assert!(
        generated.contains("Sym_top_0 <- function"),
        "top-level function should still be emitted"
    );
    assert!(
        generated.contains("Sym_top_0()"),
        "top-level call should still be emitted for helper-only output"
    );
}

#[test]
fn runtime_injection_does_not_embed_compile_time_native_roots() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("runtime_injection_options");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "native_roots");
    fs::create_dir_all(&proj).expect("failed to create sandbox dir");

    let rr_path = proj.join("native_case.rr");
    let src = r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y

}

print(addv(c(1.0, 2.0), c(3.0, 4.0)))

"#;
    fs::write(&rr_path, src).expect("failed to write RR source");

    let compiled = compile_with_configs(
        rr_path.to_str().expect("non-utf8 path"),
        src,
        OptLevel::O2,
        rr::compiler::default_type_config(),
        rr::compiler::default_parallel_config(),
    )
    .expect("compile should succeed")
    .0;

    let sandbox_root = fs::canonicalize(&proj)
        .expect("failed to canonicalize sandbox dir")
        .to_string_lossy()
        .replace('\\', "/");
    assert!(
        compiled.contains(".rr_env$native_anchor_roots <- character(0);"),
        "runtime-injected output should avoid embedding compile-time native roots"
    );
    assert!(
        compiled.contains("# --- RR generated code (from user RR source) ---"),
        "runtime-injected output should clearly separate generated RR code from runtime helpers"
    );
    assert!(
        compiled.contains("# --- RR synthesized entrypoints (auto-generated) ---"),
        "runtime-injected output should clearly separate synthesized entrypoints"
    );
    assert!(
        !compiled.contains(&sandbox_root),
        "runtime-injected output should not anchor native roots to the temporary sandbox dir"
    );
}

#[test]
fn runtime_injection_keeps_only_helpers_used_by_generated_program() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("runtime_injection_options");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "helper_subset");
    fs::create_dir_all(&proj).expect("failed to create sandbox dir");

    let rr_path = proj.join("subset_case.rr");
    let src = r#"
fn pick(xs, i) {
  return xs[i]

}

print(pick(c(4.0, 5.0, 6.0), 2.0))

"#;
    fs::write(&rr_path, src).expect("failed to write RR source");

    let compiled = compile_with_configs(
        rr_path.to_str().expect("non-utf8 path"),
        src,
        OptLevel::O0,
        rr::compiler::default_type_config(),
        rr::compiler::default_parallel_config(),
    )
    .expect("compile should succeed")
    .0;

    assert!(
        compiled.contains("rr_index1_read <- function"),
        "used index-read helper should be injected"
    );
    assert!(
        compiled.contains("rr_index1_read_strict <- function"),
        "transitive dependency of used helper should be injected"
    );
    assert!(
        !compiled.contains("rr_parallel_typed_vec_call <- function"),
        "unused parallel helper should not be injected"
    );
    assert!(
        !compiled.contains("rr_assign_slice <- function"),
        "unused slice helper should not be injected"
    );
}

#[test]
fn preserve_all_defs_keeps_unreachable_top_level_functions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("runtime_injection_options");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "preserve_all_defs");
    fs::create_dir_all(&proj).expect("failed to create sandbox dir");

    let rr_path = proj.join("preserve_defs.rr");
    let src = r#"
fn kept() {
  return 1
}

fn dropped() {
  print("DROP")
  return 2
}

print(kept())
"#;
    fs::write(&rr_path, src).expect("failed to write RR source");

    let stripped = compile_with_configs_with_options(
        rr_path.to_str().expect("non-utf8 path"),
        src,
        OptLevel::O1,
        rr::compiler::default_type_config(),
        rr::compiler::default_parallel_config(),
        CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: false,
            ..Default::default()
        },
    )
    .expect("default compile should succeed")
    .0;

    let preserved = compile_with_configs_with_options(
        rr_path.to_str().expect("non-utf8 path"),
        src,
        OptLevel::O1,
        rr::compiler::default_type_config(),
        rr::compiler::default_parallel_config(),
        CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: true,
            ..Default::default()
        },
    )
    .expect("preserve-all-defs compile should succeed")
    .0;

    assert!(
        !stripped.contains("print(\"DROP\")"),
        "default whole-program lowering should strip unreachable top-level definitions"
    );
    assert!(
        preserved.contains("print(\"DROP\")"),
        "preserve-all-defs should keep otherwise unreachable top-level definitions"
    );
}

#[test]
fn cli_preserve_all_defs_flag_keeps_unreachable_top_level_functions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_root = root
        .join("target")
        .join("tests")
        .join("runtime_injection_options");
    fs::create_dir_all(&out_root).expect("failed to create output dir");
    let proj = unique_dir(&out_root, "cli_preserve_all_defs");
    fs::create_dir_all(&proj).expect("failed to create sandbox dir");

    let rr_path = proj.join("main.rr");
    let out_path = proj.join("main.R");
    fs::write(
        &rr_path,
        r#"
fn kept() {
  return 1
}

fn dropped() {
  print("DROP")
  return 2
}

print(kept())
"#,
    )
    .expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O0")
        .status()
        .expect("failed to run RR compiler");
    assert!(status.success(), "compile failed for {}", rr_path.display());

    let generated = fs::read_to_string(&out_path).expect("failed to read generated R");
    assert!(
        generated.contains("print(\"DROP\")"),
        "CLI --preserve-all-defs should keep unreachable top-level definitions"
    );
}
