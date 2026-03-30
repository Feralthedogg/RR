mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn compiler_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping compiler direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("compiler_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let sample_r = out_dir.join("sample_runtime.R");
    let sample_rc = out_dir.join("sample_runtime.Rc");

    let src = r#"
import r default from "compiler"

fn add1(x) {
  return x + 1.0
}

fn use_compiler() -> int {
  let old = compiler.enableJIT(0)
  let pkg_old = compiler.compilePKGS(false)
  let opt = compiler.getCompilerOption("optimize")
  let suppress_all = compiler.getCompilerOption("suppressAll")
  let suppress_undefined = compiler.getCompilerOption("suppressUndefined")
  let suppress_no_super = compiler.getCompilerOption("suppressNoSuperAssignVar")
  let prev = compiler.setCompilerOptions(optimize = 2)
  let prev_opt = prev.optimize
  let prev2 = compiler.setCompilerOptions(suppressAll = true)
  let prev_suppress = prev2.suppressAll
  let prev3 = compiler.setCompilerOptions(suppressUndefined = c("x", "y"))
  let prev_undefined = prev3.suppressUndefined
  let prev4 = compiler.setCompilerOptions(suppressNoSuperAssignVar = true)
  let prev_no_super = prev4.suppressNoSuperAssignVar
  let prev_both = compiler.setCompilerOptions(optimize = 2, suppressAll = true)
  let prev_both_opt = prev_both.optimize
  let prev_both_suppress = prev_both.suppressAll
  let bc = compiler.compile(add1)
  let compiled_fn = compiler.cmpfun(add1)
  let dis = compiler.disassemble(bc)
  let cmp_res = compiler.cmpfile("__SAMPLE_R__", "__SAMPLE_RC__")
  let load_res = compiler.loadcmp("__SAMPLE_RC__")
  print(old)
  print(pkg_old)
  print(opt)
  print(suppress_all)
  print(suppress_undefined)
  print(suppress_no_super)
  print(length(prev))
  print(prev_opt)
  print(prev_suppress)
  print(length(prev3))
  print(prev_undefined)
  print(prev_no_super)
  print(length(prev_both))
  print(prev_both_opt)
  print(prev_both_suppress)
  print(length(bc))
  print(length(compiled_fn))
  print(length(dis))
  print(length(cmp_res))
  print(length(load_res))
  compiler.setCompilerOptions(optimize = opt)
  compiler.setCompilerOptions(suppressAll = prev_suppress)
  compiler.setCompilerOptions(suppressUndefined = prev_undefined)
  compiler.setCompilerOptions(suppressNoSuperAssignVar = prev_no_super)
  compiler.setCompilerOptions(optimize = prev_both_opt, suppressAll = prev_both_suppress)
  compiler.compilePKGS(pkg_old)
  compiler.enableJIT(old)
  return length(bc)
}

print(use_compiler())
"#
    .replace("__SAMPLE_R__", &sample_r.to_string_lossy())
    .replace("__SAMPLE_RC__", &sample_rc.to_string_lossy());

    let rr_path = out_dir.join("compiler_direct_interop.rr");
    let o0 = out_dir.join("compiler_direct_interop_o0.R");
    let o2 = out_dir.join("compiler_direct_interop_o2.R");

    fs::write(&sample_r, "f <- function(x) x + 1\n")
        .expect("failed to write sample compiler input");
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let run_o0 = run_rscript(&rscript, &o0);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(run_o0.status, 0, "O0 runtime failed:\n{}", run_o0.stderr);
    assert_eq!(run_o2.status, 0, "O2 runtime failed:\n{}", run_o2.stderr);
    assert_eq!(
        normalize(&run_o0.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&run_o0.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch O0 vs O2"
    );

    let meta = fs::metadata(&sample_rc).expect("expected compiled Rc output");
    assert!(meta.len() > 0, "expected non-empty Rc output");
}

#[test]
fn compiler_compilepkgs_stays_on_direct_surface() {
    let src = r#"
import r default from "compiler"

fn touch_pkg_compile() -> int {
  let old = compiler.compilePKGS(false)
  print(old)
  compiler.compilePKGS(old)
  return 0
}

print(touch_pkg_compile())
"#;

    let (ok, stdout, stderr) = run_compile_case(
        "compiler_compilepkgs_direct_surface",
        src,
        "compiler_compilepkgs_direct_surface.rr",
        "-O1",
        &[],
    );

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "compiler::compilePKGS should stay on the direct surface, got stderr:\n{stderr}"
    );
}
