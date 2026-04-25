mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn commit_metadata_check_rejects_missing_sections_and_bad_prefix() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "commit_meta");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let changed = sandbox.join("changed.txt");
    fs::write(&changed, "src/compiler/incremental.rs\n")
        .expect("failed to write changed file list");
    let bad_subjects = sandbox.join("bad_subjects.txt");
    fs::write(&bad_subjects, "misc fix\n").expect("failed to write bad commit subjects");

    let bad_title = sandbox.join("bad_title.txt");
    let bad_body = sandbox.join("bad_body.txt");
    fs::write(&bad_title, "misc fix\n").expect("failed to write bad title");
    fs::write(&bad_body, "## Why\nNeeded\n").expect("failed to write bad body");

    let script = root.join("scripts").join("check_commit_metadata.pl");
    let bad = Command::new("perl")
        .arg(&script)
        .arg("--changed-files-file")
        .arg(&changed)
        .arg("--title-file")
        .arg(&bad_title)
        .arg("--body-file")
        .arg(&bad_body)
        .arg("--subjects-file")
        .arg(&bad_subjects)
        .current_dir(&root)
        .output()
        .expect("failed to run bad metadata check");
    assert!(
        !bad.status.success(),
        "bad metadata should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );

    let good_title = sandbox.join("good_title.txt");
    let good_body = sandbox.join("good_body.txt");
    let good_subjects = sandbox.join("good_subjects.txt");
    fs::write(
        &good_subjects,
        "incremental: tighten cache validation\nMerge pull request #9 from Feralthedogg/dev\n",
    )
    .expect("failed to write good commit subjects");
    fs::write(&good_title, "incremental: tighten cache validation\n")
        .expect("failed to write good title");
    fs::write(
        &good_body,
        r#"## Why
Incremental cache behavior changed.

## Risk
Touches incremental output reuse and strict verify.

## Test Plan
Ran cache equivalence and contributing audit checks.

## Performance Impact
No expected runtime regression; validation-only changes.

## Subsystems
incremental

## Verification
Ran `cargo test -q --test cache_equivalence_matrix`.

## Benchmark Evidence
Not performance-sensitive.

## Dependency Impact
No dependency changes.

## Exceptions
None.
"#,
    )
    .expect("failed to write good body");

    let good = Command::new("perl")
        .arg(&script)
        .arg("--changed-files-file")
        .arg(&changed)
        .arg("--title-file")
        .arg(&good_title)
        .arg("--body-file")
        .arg(&good_body)
        .arg("--subjects-file")
        .arg(&good_subjects)
        .current_dir(&root)
        .output()
        .expect("failed to run good metadata check");
    assert!(
        good.status.success(),
        "good metadata should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );
}

#[test]
fn subsystem_ownership_check_reports_touched_subsystems() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "ownership");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let changed = sandbox.join("changed.txt");
    fs::write(
        &changed,
        "src/compiler/incremental.rs\nsrc/mir/opt/poly/isl.rs\n",
    )
    .expect("failed to write changed files");

    let script = root.join("scripts").join("check_subsystem_ownership.pl");
    let output = Command::new("perl")
        .arg(&script)
        .arg("--changed-files-file")
        .arg(&changed)
        .arg("--print-plan")
        .current_dir(&root)
        .output()
        .expect("failed to run subsystem ownership check");
    assert!(
        output.status.success(),
        "ownership check should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("subsystem=incremental"));
    assert!(stdout.contains("subsystem=parallel-native"));
}

#[test]
fn warning_baseline_check_flags_unexpected_warning() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "warnings");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let baseline = sandbox.join("baseline.txt");
    let log = sandbox.join("audit.log");
    fs::write(
        &baseline,
        "current-dir-review|^src/compiler/incremental\\.rs$\n",
    )
    .expect("failed to write baseline");
    fs::write(
        &log,
        "warn[current-dir-review] src/compiler/incremental.rs:1: ok\nwarn[hash-order-review] src/mir/opt.rs:10: new\n",
    )
    .expect("failed to write log");

    let script = root.join("scripts").join("check_new_warnings.pl");
    let bad = Command::new("perl")
        .arg(&script)
        .arg("--baseline")
        .arg(&baseline)
        .arg("--log")
        .arg(&log)
        .current_dir(&root)
        .output()
        .expect("failed to run warning baseline check");
    assert!(
        !bad.status.success(),
        "unexpected warning should fail baseline\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
}

#[test]
fn subsystem_matrix_prints_expected_plan() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "matrix");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let changed = sandbox.join("changed.txt");
    fs::write(
        &changed,
        "src/compiler/incremental.rs\nsrc/codegen/mir_emit.rs\n",
    )
    .expect("failed to write changed files");

    let script = root.join("scripts").join("ci_subsystem_matrix.pl");
    let output = Command::new("perl")
        .arg(&script)
        .arg("--changed-files-file")
        .arg(&changed)
        .arg("--print-plan")
        .current_dir(&root)
        .output()
        .expect("failed to run subsystem matrix planner");
    assert!(
        output.status.success(),
        "matrix planner should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("subsystem=incremental"));
    assert!(stdout.contains("subsystem=codegen"));
    assert!(stdout.contains("plan="));
}

#[test]
fn required_ci_contract_matches_workflow() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts").join("check_required_ci_jobs.pl");
    let output = Command::new("perl")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("failed to run required ci contract check");
    assert!(
        output.status.success(),
        "required ci contract should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn subsystem_review_check_requires_owner_approval() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "reviews");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let changed = sandbox.join("changed.txt");
    fs::write(&changed, "src/compiler/incremental.rs\n")
        .expect("failed to write changed file list");

    let missing_reviews = sandbox.join("missing_reviews.json");
    fs::write(
        &missing_reviews,
        r#"[{"state":"COMMENTED","user":{"login":"Feralthedogg"}}]"#,
    )
    .expect("failed to write missing reviews");

    let approved_reviews = sandbox.join("approved_reviews.json");
    fs::write(
        &approved_reviews,
        r#"[{"state":"APPROVED","user":{"login":"Feralthedogg"}}]"#,
    )
    .expect("failed to write approved reviews");

    let script = root.join("scripts").join("check_subsystem_reviews.pl");
    let bad = Command::new("perl")
        .arg(&script)
        .arg("--changed-files-file")
        .arg(&changed)
        .arg("--reviews-json-file")
        .arg(&missing_reviews)
        .current_dir(&root)
        .output()
        .expect("failed to run subsystem review check with missing approvals");
    assert!(
        !bad.status.success(),
        "missing owner approval should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );

    let good = Command::new("perl")
        .arg(&script)
        .arg("--changed-files-file")
        .arg(&changed)
        .arg("--reviews-json-file")
        .arg(&approved_reviews)
        .arg("--print-plan")
        .current_dir(&root)
        .output()
        .expect("failed to run subsystem review check with approval");
    assert!(
        good.status.success(),
        "owner approval should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );
    let stdout = String::from_utf8_lossy(&good.stdout);
    assert!(stdout.contains("subsystem=incremental"));
    assert!(stdout.contains("approved=feralthedogg"));
}

#[test]
fn commit_series_check_enforces_trailers_and_supports_buildability_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "series_repo");
    fs::create_dir_all(sandbox.join("src").join("compiler")).expect("failed to create src dir");
    fs::create_dir_all(sandbox.join("policy")).expect("failed to create policy dir");

    fs::copy(
        root.join("policy").join("subsystems.toml"),
        sandbox.join("policy").join("subsystems.toml"),
    )
    .expect("failed to copy subsystem policy");

    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&sandbox)
            .output()
            .expect("failed to run git");
        assert!(
            output.status.success(),
            "git command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    git(&["init"]);
    git(&["config", "user.email", "rr@example.com"]);
    git(&["config", "user.name", "RR Test"]);

    fs::write(
        sandbox.join("src").join("compiler").join("incremental.rs"),
        "pub fn base() -> usize { 1 }\n",
    )
    .expect("failed to write initial file");
    git(&["add", "."]);
    git(&["commit", "-m", "incremental: initial base"]);
    let base_sha = git(&["rev-parse", "HEAD"]);

    fs::write(
        sandbox.join("src").join("compiler").join("incremental.rs"),
        "pub fn base() -> usize { 2 }\n",
    )
    .expect("failed to update good file");
    git(&["add", "."]);
    git(&[
        "commit",
        "-m",
        "incremental: tighten cache validation",
        "-m",
        "Subsystem: incremental\nTested-by: perl -e 'exit 0'\nRisk: low\nPerf-impact: no expected regression\n",
    ]);

    let script = root.join("scripts").join("check_commit_series.pl");
    let good = Command::new("perl")
        .arg(&script)
        .arg("--repo")
        .arg(&sandbox)
        .arg("--base")
        .arg(&base_sha)
        .arg("--verify-buildability")
        .arg("--build-command")
        .arg("perl -e 'exit 0'")
        .current_dir(&root)
        .output()
        .expect("failed to run good commit series check");
    assert!(
        good.status.success(),
        "good commit series should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );

    let good_tip = git(&["rev-parse", "HEAD"]);
    fs::write(
        sandbox.join("src").join("compiler").join("incremental.rs"),
        "pub fn base() -> usize { 3 }\n",
    )
    .expect("failed to update bad file");
    git(&["add", "."]);
    git(&[
        "commit",
        "-m",
        "incremental: missing trailers",
        "-m",
        "Subsystem: incremental\nRisk: low\n",
    ]);

    let bad = Command::new("perl")
        .arg(&script)
        .arg("--repo")
        .arg(&sandbox)
        .arg("--base")
        .arg(&good_tip)
        .current_dir(&root)
        .output()
        .expect("failed to run bad commit series check");
    assert!(
        !bad.status.success(),
        "bad commit series should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("missing trailer 'Tested-by:'"));
}

#[test]
fn commit_series_check_can_skip_trailer_enforcement_for_push_style_runs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "series_skip_trailers");
    fs::create_dir_all(sandbox.join("src").join("compiler")).expect("failed to create src dir");
    fs::create_dir_all(sandbox.join("policy")).expect("failed to create policy dir");

    fs::copy(
        root.join("policy").join("subsystems.toml"),
        sandbox.join("policy").join("subsystems.toml"),
    )
    .expect("failed to copy subsystem policy");

    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&sandbox)
            .output()
            .expect("failed to run git");
        assert!(
            output.status.success(),
            "git command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };

    git(&["init"]);
    git(&["config", "user.email", "rr@example.com"]);
    git(&["config", "user.name", "RR Test"]);

    fs::write(
        sandbox.join("src").join("compiler").join("incremental.rs"),
        "pub fn base() -> usize { 1 }\n",
    )
    .expect("failed to write initial file");
    git(&["add", "."]);
    git(&["commit", "-m", "incremental: initial base"]);

    fs::write(
        sandbox.join("src").join("compiler").join("incremental.rs"),
        "pub fn base() -> usize { 2 }\n",
    )
    .expect("failed to update file");
    git(&["add", "."]);
    git(&[
        "commit",
        "-m",
        "Merge pull request #9 from Feralthedogg/dev",
        "-m",
        "incremental: push-safe change",
    ]);

    let script = root.join("scripts").join("check_commit_series.pl");
    let output = Command::new("perl")
        .arg(&script)
        .arg("--repo")
        .arg(&sandbox)
        .arg("--verify-buildability")
        .arg("--build-command")
        .arg("perl -e 'exit 0'")
        .arg("--skip-trailers")
        .current_dir(&root)
        .output()
        .expect("failed to run push-style commit series check");
    assert!(
        output.status.success(),
        "push-style commit series should pass without trailers\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn failure_bundle_builder_writes_summary_and_logs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "failure_bundle");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let changed = sandbox.join("changed.txt");
    let log = sandbox.join("semantic-audit.log");
    let bundle = sandbox.join("bundle");
    fs::write(
        &changed,
        "src/compiler/incremental.rs\nsrc/codegen/mir_emit.rs\n",
    )
    .expect("failed to write changed file list");
    fs::write(&log, "fake failure log\n").expect("failed to write fake log");

    let script = root.join("scripts").join("build_failure_bundle.pl");
    let output = Command::new("perl")
        .arg(&script)
        .arg("--bundle-dir")
        .arg(&bundle)
        .arg("--label")
        .arg("semantic-audit")
        .arg("--changed-files-file")
        .arg(&changed)
        .arg("--log")
        .arg(&log)
        .current_dir(&root)
        .output()
        .expect("failed to build failure bundle");
    assert!(
        output.status.success(),
        "failure bundle should build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let summary = fs::read_to_string(bundle.join("triage-summary.md"))
        .expect("failed to read triage summary");
    assert!(summary.contains("semantic-audit"));
    assert!(summary.contains("incremental"));
    assert!(summary.contains("codegen"));
    assert!(bundle.join("metadata.json").exists());
    assert!(bundle.join("logs").join("semantic-audit.log").exists());
    assert!(bundle.join("repro.sh").exists());
}

#[test]
fn perf_governance_can_emit_empty_report_for_filtered_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "perf_governance");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let report = sandbox.join("perf-report.json");
    let script = root.join("scripts").join("perf_governance.pl");
    let output = Command::new("perl")
        .arg(&script)
        .arg("--report")
        .arg(&report)
        .arg("--filter")
        .arg("__no_perf_case_should_match__")
        .current_dir(&root)
        .output()
        .expect("failed to run perf governance smoke");
    assert!(
        output.status.success(),
        "perf governance smoke should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report_text = fs::read_to_string(&report).expect("failed to read perf governance report");
    assert!(report_text.contains("\"results\":[]"));
}

#[test]
fn perf_report_compare_flags_regressions_against_thresholds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("process_gate_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "perf_compare");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox");

    let base = sandbox.join("base.json");
    let head = sandbox.join("head.json");
    let thresholds = sandbox.join("thresholds.txt");
    let output_json = sandbox.join("compare.json");

    fs::write(
        &base,
        r#"{"git_sha":"base","results":[{"name":"perf_regression_gate","elapsed_seconds":10.0},{"name":"commercial_determinism","elapsed_seconds":20.0}]}"#,
    )
    .expect("failed to write base report");
    fs::write(
        &head,
        r#"{"git_sha":"head","results":[{"name":"perf_regression_gate","elapsed_seconds":16.0},{"name":"commercial_determinism","elapsed_seconds":21.0}]}"#,
    )
    .expect("failed to write head report");
    fs::write(
        &thresholds,
        "perf_regression_gate|1.25|3\ncommercial_determinism|1.20|5\n",
    )
    .expect("failed to write thresholds");

    let script = root.join("scripts").join("compare_perf_reports.pl");
    let bad = Command::new("perl")
        .arg(&script)
        .arg("--base")
        .arg(&base)
        .arg("--head")
        .arg(&head)
        .arg("--threshold")
        .arg(&thresholds)
        .arg("--output-json")
        .arg(&output_json)
        .current_dir(&root)
        .output()
        .expect("failed to run perf compare check");
    assert!(
        !bad.status.success(),
        "perf compare should fail on regression\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("perf_regression_gate regressed"));
    let compare_json = fs::read_to_string(&output_json).expect("failed to read compare report");
    assert!(compare_json.contains("\"failures\""));
}
