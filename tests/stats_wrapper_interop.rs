mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_wrapper_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats wrapper runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_wrapper_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r { mean } from "base"

fn inspect_wrapper() -> float {
  let nums = c(1.0, 2.0, 3.0, 4.0)
  let groups = c("a", "a", "b", "b")
  let df = base.data.frame(x = nums)
  let agg_df = stats.aggregate.data.frame(df, by = base.list(g = groups), FUN = mean)
  let tsx = stats.ts(nums, frequency = 4.0)
  let agg_ts = stats.aggregate.ts(tsx, nfrequency = 2.0, FUN = mean)
  let named = stats.setNames(c(1.0, 2.0), c("left", "right"))
  let med = stats.median.default(nums)
  print(agg_df)
  print(agg_ts)
  print(named)
  print(med)
  return agg_ts[1L] + named[1L] + med
}

print(inspect_wrapper())
"#;

    let rr_path = out_dir.join("stats_wrapper_interop.rr");
    let o0 = out_dir.join("stats_wrapper_interop_o0.R");
    let o2 = out_dir.join("stats_wrapper_interop_o2.R");

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
fn stats_wrapper_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r { mean } from "base"

fn inspect_wrapper() -> int {
  let nums = c(1.0, 2.0, 3.0, 4.0)
  let groups = c("a", "a", "b", "b")
  let df = base.data.frame(x = nums)
  print(stats.aggregate.data.frame(df, by = base.list(g = groups), FUN = mean))
  print(stats.aggregate.ts(stats.ts(nums, frequency = 4.0), nfrequency = 2.0, FUN = mean))
  print(stats.setNames(c(1.0, 2.0), c("left", "right")))
  print(stats.median.default(nums))
  return 1L
}

print(inspect_wrapper())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_wrapper_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats wrapper helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
