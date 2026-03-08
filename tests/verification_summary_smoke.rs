mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn verification_summary_collects_triage_reports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("verification_summary_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let artifacts_root = common::unique_dir(&sandbox_root, "artifacts");
    let out_dir = artifacts_root.join("nightly-soak");
    fs::create_dir_all(&out_dir).expect("failed to create nightly out dir");

    let diff_dir = artifacts_root.join("differential-triage");
    let pass_dir = artifacts_root.join("pass-verify-triage");
    let fuzz_dir = artifacts_root.join("fuzz-triage");
    fs::create_dir_all(&diff_dir).expect("failed to create diff dir");
    fs::create_dir_all(&pass_dir).expect("failed to create pass dir");
    fs::create_dir_all(&fuzz_dir).expect("failed to create fuzz dir");

    fs::write(
        diff_dir.join("summary.json"),
        r#"{
  "schema": "rr-triage-report",
  "version": 1,
  "kind": "differential",
  "failure_bundles": 2,
  "invalid_bundles": 1,
  "rust_regression_skeletons": 2,
  "cases": [
    {"case": "diff_case_1", "opt": "O2", "reference_status": 0, "compiled_status": 1, "case_dir": "x"},
    {"case": "diff_case_2", "opt": "O1", "reference_status": 0, "compiled_status": 1, "case_dir": "y"}
  ]
}
"#,
    )
    .expect("failed to write differential summary");
    fs::create_dir_all(diff_dir.join("diff_case_1")).expect("failed to create diff case dir");
    fs::write(
        diff_dir.join("diff_case_1").join("regression.rs"),
        "// regression",
    )
    .expect("failed to write diff regression");
    fs::write(
        diff_dir.join("diff_case_1").join("bundle.manifest"),
        "schema: rr-triage-bundle\nversion: 1\nkind: differential\ncase: diff_case_1\nopt: O2\nreference_status: 0\ncompiled_status: 1\n",
    )
    .expect("failed to write diff manifest");

    fs::write(
        pass_dir.join("summary.json"),
        r#"{
  "schema": "rr-triage-report",
  "version": 1,
  "kind": "pass-verify",
  "failure_bundles": 1,
  "invalid_bundles": 0,
  "cases": [
    {"case": "verify_case_1", "case_dir": "z"}
  ]
}
"#,
    )
    .expect("failed to write pass-verify summary");
    fs::create_dir_all(pass_dir.join("verify_case_1")).expect("failed to create pass case dir");
    fs::write(
        pass_dir.join("verify_case_1").join("regression.rs"),
        "// regression",
    )
    .expect("failed to write pass regression");
    fs::write(
        pass_dir.join("verify_case_1").join("bundle.manifest"),
        "schema: rr-triage-bundle\nversion: 1\nkind: pass-verify\ncase: verify_case_1\nstatus: verify-failed\n",
    )
    .expect("failed to write pass manifest");

    fs::write(
        fuzz_dir.join("summary.json"),
        r#"{
  "schema": "rr-triage-report",
  "version": 1,
  "kind": "fuzz",
  "artifacts": 3,
  "reproduced": 2,
  "minimized": 2,
  "rust_regression_skeletons": 1,
  "manual_replay_notes": 1,
  "cases": [
    {"target": "pipeline", "artifact": "a", "repro_status": 0, "tmin_status": 0, "skeleton_kind": "rust", "case_dir": "a"},
    {"target": "generated_pipeline", "artifact": "b", "repro_status": 1, "tmin_status": 1, "skeleton_kind": "manual", "case_dir": "b"}
  ]
}
"#,
    )
    .expect("failed to write fuzz summary");
    fs::create_dir_all(fuzz_dir.join("pipeline_crash_1")).expect("failed to create fuzz case dir");
    fs::write(
        fuzz_dir.join("pipeline_crash_1").join("regression.rs"),
        "// regression",
    )
    .expect("failed to write fuzz regression");
    fs::write(
        fuzz_dir.join("pipeline_crash_1").join("bundle.manifest"),
        "schema: rr-triage-bundle\nversion: 1\nkind: fuzz\ntarget: pipeline\nartifact: crash-1\nrepro_status: 0\ntmin_status: 0\nskeleton_kind: rust\n",
    )
    .expect("failed to write fuzz manifest");

    let script = root.join("scripts").join("verification_summary.sh");
    let output = Command::new("bash")
        .arg(&script)
        .arg(&artifacts_root)
        .arg(&out_dir)
        .env("GITHUB_RUN_ID", "123456789")
        .env("GITHUB_RUN_ATTEMPT", "2")
        .env("GITHUB_WORKFLOW", "RR Nightly Soak")
        .env("GITHUB_JOB", "verification-summary")
        .env("GITHUB_REF_NAME", "main")
        .env("GITHUB_SHA", "deadbeefcafebabe")
        .output()
        .expect("failed to execute verification summary script");
    assert!(
        output.status.success(),
        "verification summary script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json = fs::read_to_string(out_dir.join("verification-summary.json"))
        .expect("failed to read verification summary json");
    assert!(json.contains("\"schema\": \"rr-verification-summary\""));
    assert!(json.contains("\"version\": 1"));
    assert!(json.contains("\"differential_failure_bundles\": 2"));
    assert!(json.contains("\"pass_verify_failure_bundles\": 1"));
    assert!(json.contains("\"fuzz_artifacts\": 3"));
    assert!(json.contains("\"total_cases\": 5"));
    assert!(json.contains("\"promotion_candidate_count\": 3"));
    assert!(json.contains("\"kind\": \"differential\""));
    assert!(json.contains("\"kind\": \"pass-verify\""));
    assert!(json.contains("\"kind\": \"fuzz\""));
    assert!(json.contains("\"severity\": \"critical\""));
    assert!(json.contains("\"priority\": 300"));
    assert!(json.contains("\"priority\": 260"));
    assert!(json.contains("\"priority\": 180"));
    assert!(json.contains("\"github_run_id\": \"123456789\""));
    assert!(json.contains("\"github_workflow\": \"RR Nightly Soak\""));
    assert!(json.contains("\"differential_by_opt\": {"));
    assert!(json.contains("\"O1\": 1"));
    assert!(json.contains("\"O2\": 1"));
    assert!(json.contains("\"fuzz_by_target\": {"));
    assert!(json.contains("\"pipeline\": 1"));
    assert!(json.contains("\"generated_pipeline\": 1"));

    let md = fs::read_to_string(out_dir.join("verification-summary.md"))
        .expect("failed to read verification summary markdown");
    let top_md = fs::read_to_string(out_dir.join("top-promotion-candidates.md"))
        .expect("failed to read top promotion candidate markdown");
    let top_json = fs::read_to_string(out_dir.join("top-promotion-candidates.json"))
        .expect("failed to read top promotion candidate json");
    assert!(md.contains("differential failure bundles: `2`"));
    assert!(md.contains("pass-verify failure bundles: `1`"));
    assert!(md.contains("fuzz artifacts: `3`"));
    assert!(md.contains("total triaged cases: `5`"));
    assert!(md.contains("promotion candidates: `3`"));
    assert!(md.contains("github run id: `123456789`"));
    assert!(md.contains("github workflow: `RR Nightly Soak`"));
    assert!(md.contains("## Breakdown"));
    assert!(md.contains("differential by opt: `{'O1': 1, 'O2': 1}`"));
    assert!(md.contains("fuzz by target: `{'generated_pipeline': 1, 'pipeline': 1}`"));
    assert!(md.contains("## Top Promotion Candidates"));
    assert!(md.contains("critical` / `pass-verify` / priority `300`"));
    assert!(md.contains("critical` / `differential` / priority `260`"));
    assert!(md.contains("high` / `fuzz` / priority `180`"));
    assert!(md.contains("verifier failed after a compiler pass"));
    assert!(md.contains("optimized output changed runtime exit behavior"));
    assert!(md.contains("fuzz crash reproduces and minimizes cleanly"));
    assert!(md.contains("scripts/triage_driver.sh promote differential"));
    assert!(md.contains("scripts/triage_driver.sh promote pass-verify"));
    assert!(md.contains("scripts/triage_driver.sh promote fuzz"));
    assert!(top_md.contains("# Top Promotion Candidates"));
    assert!(top_md.contains("github run id: `123456789`"));
    assert!(top_md.contains("candidate count: `3`"));
    assert!(top_json.contains("\"schema\": \"rr-promotion-candidates\""));
    assert!(top_json.contains("\"github_run_id\": \"123456789\""));
    assert!(top_json.contains("\"priority\": 300"));
}
