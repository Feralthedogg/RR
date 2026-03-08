use RR::compiler::{OptLevel, compile_with_configs};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(root: &std::path::Path, name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.join(format!("{}_{}_{}", name, std::process::id(), ts))
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
        generated.contains("rr_assign_slice <- function"),
        "helper-only emission should still include runtime helper definitions"
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
fn runtime_injection_embeds_compile_time_native_roots() {
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
        RR::compiler::type_config_from_env(),
        RR::compiler::parallel_config_from_env(),
    )
    .expect("compile should succeed")
    .0;

    let expected_root = fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
        .expect("failed to canonicalize repo root")
        .to_string_lossy()
        .replace('\\', "/");
    let sandbox_root = fs::canonicalize(&proj)
        .expect("failed to canonicalize sandbox dir")
        .to_string_lossy()
        .replace('\\', "/");
    assert!(
        compiled.contains("rr_set_native_roots(c("),
        "runtime-injected output should embed compile-time native roots"
    );
    assert!(
        compiled.contains(&expected_root),
        "runtime-injected output should include compile-time project root"
    );
    assert!(
        !compiled.contains(&sandbox_root),
        "runtime-injected output should not anchor native roots to the temporary sandbox dir"
    );
}
