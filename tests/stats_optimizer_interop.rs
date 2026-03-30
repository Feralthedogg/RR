mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_optimizer_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats optimizer runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_optimizer_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as base from "base"

fn quad(p) -> float {
  return ((p[1L] - 2.0) * (p[1L] - 2.0)) + ((p[2L] + 1.0) * (p[2L] + 1.0))
}

fn grad(p) {
  return c(2.0 * (p[1L] - 2.0), 2.0 * (p[2L] + 1.0))
}

fn one(x) -> float {
  return ((x - 3.0) * (x - 3.0)) + 1.0
}

fn quad1(x) -> float {
  return ((x - 2.0) * (x - 2.0)) + 1.0
}

fn rootfn(x) -> float {
  return x - 2.0
}

fn inspect_optimizer() -> float {
  let init = c(0.0, 0.0)
  let ui = base.matrix(c(1.0, 0.0, 0.0, 1.0), nrow = 2L)
  let ci = c(-10.0, -10.0)
  let opt = stats.optim(init, quad)
  let hess = stats.optimHess(init, quad, grad)
  let op = stats.optimize(one, c(-10.0, 10.0))
  let ops = stats.optimise(one, c(-10.0, 10.0))
  let nl = stats.nlm(one, p = 0.0)
  let nb = stats.nlminb(init, objective = quad, gradient = grad)
  let co = stats.constrOptim(init, quad, grad, ui, ci)
  let ur = stats.uniroot(rootfn, c(0.0, 10.0))
  let ig = stats.integrate(quad1, 0.0, 1.0)
  print(opt.par)
  print(opt.value)
  print(opt.counts)
  print(opt.convergence)
  print(hess)
  print(op.minimum)
  print(op.objective)
  print(ops.minimum)
  print(ops.objective)
  print(nl.minimum)
  print(nl.estimate)
  print(nl.gradient)
  print(nl.code)
  print(nl.iterations)
  print(nb.par)
  print(nb.objective)
  print(nb.evaluations)
  print(nb.iterations)
  print(co.par)
  print(co.value)
  print(co.counts)
  print(ur.root)
  print(ur.iter)
  print(ig.value)
  print(ig.subdivisions)
  return opt.value + op.objective + ops.objective + nl.minimum + nb.objective + co.value + ur.root + ig.value
}

print(inspect_optimizer())
"#;

    let rr_path = out_dir.join("stats_optimizer_interop.rr");
    let o0 = out_dir.join("stats_optimizer_interop_o0.R");
    let o2 = out_dir.join("stats_optimizer_interop_o2.R");

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
}

#[test]
fn stats_optimizer_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"

fn quad(p) -> float {
  return ((p[1L] - 2.0) * (p[1L] - 2.0)) + ((p[2L] + 1.0) * (p[2L] + 1.0))
}

fn grad(p) {
  return c(2.0 * (p[1L] - 2.0), 2.0 * (p[2L] + 1.0))
}

fn one(x) -> float {
  return ((x - 3.0) * (x - 3.0)) + 1.0
}

fn quad1(x) -> float {
  return ((x - 2.0) * (x - 2.0)) + 1.0
}

fn rootfn(x) -> float {
  return x - 2.0
}

fn inspect_optimizer_helpers() -> int {
  let init = c(0.0, 0.0)
  let ui = base.matrix(c(1.0, 0.0, 0.0, 1.0), nrow = 2L)
  let ci = c(-10.0, -10.0)
  let opt = stats.optim(init, quad)
  let hess = stats.optimHess(init, quad, grad)
  let op = stats.optimize(one, c(-10.0, 10.0))
  let ops = stats.optimise(one, c(-10.0, 10.0))
  let nl = stats.nlm(one, p = 0.0)
  let nb = stats.nlminb(init, objective = quad, gradient = grad)
  let co = stats.constrOptim(init, quad, grad, ui, ci)
  let ur = stats.uniroot(rootfn, c(0.0, 10.0))
  let ig = stats.integrate(quad1, 0.0, 1.0)
  print(opt)
  print(hess)
  print(op)
  print(ops)
  print(nl)
  print(nb)
  print(co)
  print(ur)
  print(ig)
  return 1L
}

print(inspect_optimizer_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_optimizer_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats optimizer helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
