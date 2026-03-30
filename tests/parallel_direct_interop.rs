mod common;

use common::{normalize, rscript_available, rscript_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn parallel_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping parallel direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("parallel_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "parallel"

fn run_parallel() -> int {
  let cores = parallel.detectCores()
  print(cores)
  let cl = parallel.makeCluster(2)
  parallel.clusterExport(cl, character())
  let evalq = parallel.clusterEvalQ(cl, 41.0 + 1.0)
  print(evalq)
  let out = parallel.parLapply(cl, c(1.0, 4.0, 9.0), function(v) {
    return(sqrt(v))
  })
  let applied = parallel.clusterApply(cl, list(1.0, 4.0), function(v) {
    return(sqrt(v))
  })
  let called = parallel.clusterCall(cl, function(v) {
    return(v + 1.0)
  }, 41.0)
  let zipped = parallel.clusterMap(cl, function(a, b) {
    return(a + b)
  }, c(1.0, 2.0), c(10.0, 20.0))
  let mc = parallel.mclapply(list(1.0, 4.0, 9.0), function(v) {
    return(sqrt(v))
  })
  let split = parallel.clusterSplit(cl, c(1.0, 2.0, 3.0, 4.0))
  let idx = parallel.splitIndices(10, 3)
  let lb = parallel.clusterApplyLB(cl, list(1.0, 4.0, 9.0), function(v) {
    return(sqrt(v))
  })
  let ps = parallel.parSapply(cl, c(1.0, 4.0, 9.0), function(v) {
    return(sqrt(v))
  })
  let pslb = parallel.parSapplyLB(cl, c(1.0, 4.0, 9.0), function(v) {
    return(sqrt(v))
  })
  let pa = parallel.parApply(cl, matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2), 1, function(row) {
    return(sum(row))
  })
  let job = parallel.mcparallel(sqrt(16.0))
  let collected = parallel.mccollect(job)
  print(applied)
  print(called)
  print(out)
  print(zipped)
  print(mc)
  print(split)
  print(idx)
  print(lb)
  print(ps)
  print(pslb)
  print(pa)
  print(length(collected))
  parallel.stopCluster(cl)
  return length(out)
}

print(run_parallel())
"#;

    let rr_path = out_dir.join("parallel_direct_interop.rr");
    let o0 = out_dir.join("parallel_direct_interop_o0.R");
    let o2 = out_dir.join("parallel_direct_interop_o2.R");

    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o0)
        .arg("-O0")
        .status()
        .expect("failed to compile O0");
    assert!(status.success(), "O0 compile failed");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o2)
        .arg("-O2")
        .status()
        .expect("failed to compile O2");
    assert!(status.success(), "O2 compile failed");

    let code_o2 = fs::read_to_string(&o2).expect("failed to read emitted O2 R");
    assert!(
        code_o2.contains("parallel::detectCores(")
            && code_o2.contains("parallel::makeCluster(")
            && code_o2.contains("parallel::clusterExport(")
            && code_o2.contains("parallel::clusterEvalQ(")
            && code_o2.contains("parallel::parLapply(")
            && code_o2.contains("parallel::clusterMap(")
            && code_o2.contains("parallel::clusterApply(")
            && code_o2.contains("parallel::clusterCall(")
            && code_o2.contains("parallel::mclapply(")
            && code_o2.contains("parallel::clusterSplit(")
            && code_o2.contains("parallel::splitIndices(")
            && code_o2.contains("parallel::clusterApplyLB(")
            && code_o2.contains("parallel::parSapply(")
            && code_o2.contains("parallel::parSapplyLB(")
            && code_o2.contains("parallel::parApply(")
            && code_o2.contains("parallel::mcparallel(")
            && code_o2.contains("parallel::mccollect(")
            && code_o2.contains("parallel::stopCluster(")
            && !code_o2.contains("# rr-opaque-interop:")
            && !code_o2.contains("# rr-hybrid-fallback:"),
        "expected parallel cluster workflow to stay on direct interop surface"
    );

    let run_o0 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o0)
        .output()
        .expect("failed to execute O0 Rscript");
    let run_o2 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o2)
        .output()
        .expect("failed to execute O2 Rscript");

    assert_eq!(
        run_o0.status.code().unwrap_or(-1),
        0,
        "O0 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o0.stderr)
    );
    assert_eq!(
        run_o2.status.code().unwrap_or(-1),
        0,
        "O2 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o2.stderr)
    );

    let stdout_o0 = normalize(&String::from_utf8_lossy(&run_o0.stdout));
    let stdout_o2 = normalize(&String::from_utf8_lossy(&run_o2.stdout));
    let stderr_o0 = normalize(&String::from_utf8_lossy(&run_o0.stderr));
    let stderr_o2 = normalize(&String::from_utf8_lossy(&run_o2.stderr));

    assert_eq!(stdout_o0, stdout_o2, "stdout mismatch O0 vs O2");
    assert_eq!(stderr_o0, stderr_o2, "stderr mismatch O0 vs O2");
}
