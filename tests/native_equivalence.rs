mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript};
use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, compile_with_config};
use std::fs;
use std::path::PathBuf;

#[test]
fn off_and_optional_have_equivalent_output_without_native_library() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping native equivalence test: Rscript unavailable");
            return;
        }
    };

    let src = r#"
fn call_abs(n: int) {
  let x = seq_len(n) - 4

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = abs(x[i])

  }
  return y

}
print(call_abs(5L))

"#;

    let (off_code, _map1) = compile_with_config(
        "native_eq_off.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile off");
    let (opt_code, _map2) = compile_with_config(
        "native_eq_opt.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Optional,
        },
    )
    .expect("compile optional");

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("native_equivalence");
    fs::create_dir_all(&out_dir).expect("mkdir");
    let off_script = out_dir.join("off.R");
    let opt_script = out_dir.join("optional.R");
    fs::write(&off_script, off_code).expect("write off");
    fs::write(&opt_script, opt_code).expect("write optional");

    let off = run_rscript(&rscript, &off_script);
    let opt = run_rscript(&rscript, &opt_script);
    assert_eq!(off.status, 0, "off failed: {}", off.stderr);
    assert_eq!(opt.status, 0, "optional failed: {}", opt.stderr);
    assert_eq!(normalize(&off.stdout), normalize(&opt.stdout));
    assert_eq!(normalize(&off.stderr), normalize(&opt.stderr));
}
