mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo_file(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).expect("failed to read repository file")
}

fn contributing_audit_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn fenced_code_block_after_marker(doc: &str, marker: &str) -> Vec<String> {
    let start = doc
        .find(marker)
        .unwrap_or_else(|| panic!("missing marker: {marker}"));
    let tail = &doc[start + marker.len()..];

    let fence = ["```bash\n", "```\n"]
        .into_iter()
        .find_map(|needle| tail.find(needle).map(|idx| (needle, idx)))
        .unwrap_or_else(|| panic!("missing fenced code block after marker: {marker}"));
    let after_fence = &tail[fence.1 + fence.0.len()..];
    let end = after_fence
        .find("\n```")
        .unwrap_or_else(|| panic!("missing closing fence after marker: {marker}"));
    after_fence[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[test]
fn contributing_audit_reports_static_violations_and_skips_cfg_test_tail() {
    let root = repo_root();
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("contributing_audit_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "audit");
    fs::create_dir_all(sandbox.join("src")).expect("failed to create sandbox src");

    let bad_file = sandbox.join("src").join("bad.rs");
    fs::write(
        &bad_file,
        r#"
fn bad() {
    dbg!(1);
    let _ = Some(1).expect("boom");
    unsafe { side_effect(); }
}
"#,
    )
    .expect("failed to write bad file");

    let script = root.join("scripts").join("contributing_audit.pl");
    let _audit_guard = contributing_audit_lock().lock().unwrap();
    let bad_output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&bad_file)
        .output()
        .expect("failed to execute contributing audit script for bad file");
    assert!(
        !bad_output.status.success(),
        "bad audit input should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad_output.stdout),
        String::from_utf8_lossy(&bad_output.stderr)
    );
    let bad_stdout = String::from_utf8_lossy(&bad_output.stdout);
    assert!(bad_stdout.contains("error[production-dbg]"));
    assert!(bad_stdout.contains("error[production-unwrap]"));
    assert!(bad_stdout.contains("error[unsafe-missing-safety]"));

    let warn_file = sandbox.join("src").join("mir").join("warn.rs");
    fs::create_dir_all(warn_file.parent().expect("warn parent"))
        .expect("failed to create warn dir");
    fs::write(
        &warn_file,
        r#"
fn warn_only() {
    // SAFETY: caller ensures the pointer is valid for this test helper.
    unsafe { side_effect(); }
}
"#,
    )
    .expect("failed to write warn file");

    let warn_output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&warn_file)
        .output()
        .expect("failed to execute contributing audit script for warn file");
    assert!(
        warn_output.status.success(),
        "warn-only audit input should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&warn_output.stdout),
        String::from_utf8_lossy(&warn_output.stderr)
    );
    let warn_stdout = String::from_utf8_lossy(&warn_output.stdout);
    assert!(warn_stdout.contains("warn[unsafe-safe-alt-review]"));
    assert!(warn_stdout.contains("warn[tests-review]"));
    assert!(warn_stdout.contains("confirm touched semantic areas such as cache behavior, fallback behavior, numeric semantics, and IR invariants still match intent"));

    let good_file = sandbox.join("src").join("good.rs");
    fs::write(
        &good_file,
        r#"
fn good() -> i32 {
    1
}

struct Parser;

impl Parser {
    fn expect(&self, _token: i32) {}
}

fn parser_style_expect_is_allowed(p: &Parser) {
    p.expect(1);
}

#[cfg(test)]
mod tests {
    #[test]
    fn allows_test_only_unwrap() {
        let _ = Some(1).unwrap();
    }
}
"#,
    )
    .expect("failed to write good file");

    let good_output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&good_file)
        .output()
        .expect("failed to execute contributing audit script for good file");
    assert!(
        good_output.status.success(),
        "good audit input should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good_output.stdout),
        String::from_utf8_lossy(&good_output.stderr)
    );
    let good_stdout = String::from_utf8_lossy(&good_output.stdout);
    assert!(good_stdout.contains("no static findings"));
    assert!(good_stdout.contains("result: PASS (scan-only)"));
}

#[test]
fn contributing_audit_rejects_unstructured_actionable_comments() {
    let root = repo_root();
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("contributing_audit_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "comments");
    fs::create_dir_all(sandbox.join("src")).expect("failed to create sandbox src");

    let bad_file = sandbox.join("src").join("comments.rs");
    let todo_prefix = "TODO";
    fs::write(
        &bad_file,
        format!(
            "\nfn needs_follow_up() {{\n    // {todo_prefix} clean this up before merge\n    let _ = 1;\n}}\n"
        ),
    )
    .expect("failed to write bad comment file");

    let script = root.join("scripts").join("contributing_audit.pl");
    let _audit_guard = contributing_audit_lock().lock().unwrap();
    let output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&bad_file)
        .output()
        .expect("failed to execute contributing audit script for bad comment file");
    assert!(
        !output.status.success(),
        "bad comment audit input should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("error[comment-prefix]"));
}

#[test]
fn contributing_audit_validates_contributing_md_structure() {
    let root = repo_root();
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("contributing_audit_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "contributing");
    fs::create_dir_all(&sandbox).expect("failed to create contributing sandbox");

    let bad_file = sandbox.join("CONTRIBUTING.md");
    fs::write(
        &bad_file,
        r#"
# Contributing

## Scope

## Core Principles

## Rule Levels

## Rules

### 1) Deterministic Output and Traversal

- `MUST` keep behavior deterministic.

## Exception Process

## PR Checklist

- Behavior is deterministic for same input/config.
"#,
    )
    .expect("failed to write bad CONTRIBUTING.md");

    let script = root.join("scripts").join("contributing_audit.pl");
    let _audit_guard = contributing_audit_lock().lock().unwrap();
    let output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&bad_file)
        .output()
        .expect("failed to execute contributing audit script for bad contributing doc");
    assert!(
        !output.status.success(),
        "bad contributing doc should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("error[contributing-rule-topic]"));
    assert!(stdout.contains("error[contributing-pr-checklist]"));
}

#[test]
fn contributing_audit_all_scope_includes_fuzz_and_native_paths() {
    let root = repo_root();
    let script = root.join("scripts").join("contributing_audit.pl");
    let _audit_guard = contributing_audit_lock().lock().unwrap();

    let expected_output = Command::new("bash")
        .arg("-lc")
        .arg("git ls-files -- CONTRIBUTING.md .github/pull_request_template.md src tests docs scripts fuzz native policy | sort -u | wc -l")
        .current_dir(&root)
        .output()
        .expect("failed to count expected audit files");
    assert!(
        expected_output.status.success(),
        "expected scope count command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&expected_output.stdout),
        String::from_utf8_lossy(&expected_output.stderr)
    );
    let expected_files: usize = String::from_utf8_lossy(&expected_output.stdout)
        .trim()
        .parse()
        .expect("expected file count should parse");

    let audit_output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--all")
        .output()
        .expect("failed to execute contributing audit script for full scope");
    assert!(
        audit_output.status.success(),
        "full-scope audit should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&audit_output.stdout),
        String::from_utf8_lossy(&audit_output.stderr)
    );
    let audit_stdout = String::from_utf8_lossy(&audit_output.stdout);
    let scanned_line = audit_stdout
        .lines()
        .find(|line| line.starts_with("files scanned: "))
        .expect("audit output should report scanned file count");
    let scanned_files: usize = scanned_line
        .trim_start_matches("files scanned: ")
        .parse()
        .expect("scanned file count should parse");
    assert_eq!(
        scanned_files, expected_files,
        "contributing audit --all should cover CONTRIBUTING.md plus .github/pull_request_template.md and src/tests/docs/scripts/fuzz/native/policy"
    );
}

#[test]
fn ci_contributing_audit_scope_mentions_fuzz_and_native() {
    let root = repo_root();
    let ci_script = fs::read_to_string(root.join("scripts").join("ci_contributing_audit.sh"))
        .expect("failed to read ci contributing audit script");
    assert!(ci_script.contains("^fuzz\\/"));
    assert!(ci_script.contains("^native\\/"));
    assert!(ci_script.contains("^policy\\/"));
    assert!(ci_script.contains("^\\.github\\/pull_request_template\\.md$"));
}

#[test]
fn contributing_audit_flags_core_nondeterminism_and_mutable_globals() {
    let root = repo_root();
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("contributing_audit_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "nondeterminism");
    fs::create_dir_all(sandbox.join("src").join("compiler"))
        .expect("failed to create compiler sandbox");

    let risky_file = sandbox.join("src").join("compiler").join("risky.rs");
    fs::write(
        &risky_file,
        r#"
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

static mut BAD_COUNTER: usize = 0;
static CACHE: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn risky(all_fns: HashMap<String, usize>) {
    let names: Vec<String> = all_fns.keys().cloned().collect();
    let _ = SystemTime::now();
    let _ = std::env::current_dir();
    let _ = std::env::temp_dir();
    let _ = rand::thread_rng();
    let _ = std::thread::spawn(|| 1usize);
    let _ = Command::new("echo");
}
"#,
    )
    .expect("failed to write risky file");

    let script = root.join("scripts").join("contributing_audit.pl");
    let _audit_guard = contributing_audit_lock().lock().unwrap();
    let output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&risky_file)
        .output()
        .expect("failed to execute contributing audit script for risky file");
    assert!(
        !output.status.success(),
        "risky audit input should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("error[static-mut]"));
    assert!(stdout.contains("error[nondeterministic-rng]"));
    assert!(stdout.contains("warn[mutable-global-review]"));
    assert!(stdout.contains("warn[wall-clock-review]"));
    assert!(stdout.contains("warn[current-dir-review]"));
    assert!(stdout.contains("warn[temp-dir-review]"));
    assert!(stdout.contains("warn[hash-order-review]"));
    assert!(stdout.contains("warn[thread-spawn-review]"));
    assert!(stdout.contains("warn[process-command-review]"));
}

#[test]
fn contributing_docs_match_enforced_audit_commands() {
    let audit_doc = read_repo_file("docs/compiler/contributing-audit.md");
    let testing_doc = read_repo_file("docs/compiler/testing.md");

    let fast_audit = fenced_code_block_after_marker(&audit_doc, "## Fast Audit");
    assert_eq!(
        fast_audit,
        vec![
            "perl scripts/contributing_audit.pl",
            "cargo check",
            "cargo clippy --all-targets -- -D warnings",
            "bash scripts/test_tier.sh tier0",
            "bash scripts/test_tier.sh tier1",
            "bash scripts/optimizer_suite.sh legality",
            "FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh",
        ],
        "docs/compiler/contributing-audit.md fast-audit block drifted from the enforced local verification stack"
    );

    let audit_helper = fenced_code_block_after_marker(&testing_doc, "Audit helper:");
    assert_eq!(
        audit_helper,
        vec![
            "perl scripts/contributing_audit.pl",
            "perl scripts/contributing_audit.pl --scan-only",
        ],
        "docs/compiler/testing.md audit-helper block drifted from the supported contributing audit entrypoints"
    );

    let cleanroom_helper =
        fenced_code_block_after_marker(&testing_doc, "Cleanroom strict verification helper:");
    assert_eq!(
        cleanroom_helper,
        vec![
            "scripts/verify_cleanroom.sh",
            "scripts/verify_cleanroom.sh --files src/syntax/parse.rs tests/statement_boundaries.rs",
            "scripts/verify_cleanroom.sh --fast --files scripts/verify_cleanroom.sh",
        ],
        "docs/compiler/testing.md cleanroom helper block drifted from the documented verify_cleanroom usage"
    );
}

#[test]
fn contributing_ci_and_cleanroom_wiring_match_documented_contract() {
    let ci_workflow = read_repo_file(".github/workflows/ci.yml");
    assert!(
        ci_workflow.contains("- name: Contributing Audit"),
        "tier0 CI should expose a dedicated Contributing Audit step"
    );
    assert!(
        ci_workflow.contains("run: bash scripts/ci_contributing_audit.sh"),
        "tier0 CI should run scripts/ci_contributing_audit.sh"
    );
    assert!(
        ci_workflow.contains("Generated Contributing Docs"),
        "tier0 CI should check generated contributing docs"
    );
    assert!(
        ci_workflow.contains("PR Evidence Check"),
        "CI should expose a PR evidence gate"
    );
    assert!(
        ci_workflow.contains("Semantic Contributing Audit"),
        "CI should expose a semantic contributing audit lane"
    );

    let ci_script = read_repo_file("scripts/ci_contributing_audit.sh");
    assert!(
        ci_script.contains("--scan-only"),
        "ci contributing audit wrapper should stay diff-scoped and scan-only"
    );
    assert!(
        ci_script.contains("pull_request") && ci_script.contains("push"),
        "ci contributing audit wrapper should keep handling both PR and push event bases"
    );
    let semantic_ci_script = read_repo_file("scripts/ci_contributing_semantic_audit.sh");
    assert!(semantic_ci_script.contains("--skip-fuzz"));
    assert!(semantic_ci_script.contains("--all"));
    assert!(semantic_ci_script.contains("RR_SEMANTIC_AUDIT_SCOPE"));

    let cleanroom = read_repo_file("scripts/verify_cleanroom.sh");
    for required in [
        "cargo fmt --all --check",
        "cargo check",
        "cargo clippy --all-targets -- -D warnings",
        "python3 scripts/render_contributing_docs.py --check",
        "cargo test -q --no-fail-fast",
        "RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples",
        "perl scripts/contributing_audit.pl --all --scan-only",
        "FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh",
        "pnpm install --frozen-lockfile",
        "run_step \"Format Check\" cargo fmt --all --check",
        "run_step \"Cargo Check\" cargo check",
        "run_step \"Clippy\" cargo clippy --all-targets -- -D warnings",
        "run_step \"Generated Contributing Docs\" python3 scripts/render_contributing_docs.py --check",
        "run_step \"Tests\" cargo test -q --no-fail-fast",
        "run_step \"Pass Verify\" env RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples",
        "run_step \"Contributing Audit\" perl scripts/contributing_audit.pl --all --scan-only",
    ] {
        assert!(
            cleanroom.contains(required),
            "scripts/verify_cleanroom.sh missing documented verification contract fragment: {required}"
        );
    }
}

#[test]
fn contributing_audit_wires_semantic_smoke_lanes() {
    let audit_script = read_repo_file("scripts/contributing_audit.pl");
    for required in [
        "--skip-semantic-smoke",
        "incremental_phase1",
        "incremental_phase2",
        "incremental_phase3",
        "incremental_auto",
        "incremental_strict_verify",
        "cli_incremental_default",
        "sccp_overflow_regression",
        "rr_logic_equivalence_matrix",
        "opt_level_equivalence",
        "hybrid_fallback",
        "parallel_optional_fallback_semantics",
        "native_optional_fallback",
        "poly_vopt_fallback",
        "runtime_semantics_regression",
        "commercial_determinism",
        "compiler_parallel_equivalence",
        "random_differential",
        "RR_RANDOM_DIFFERENTIAL_SECOND_SEED",
    ] {
        assert!(
            audit_script.contains(required),
            "scripts/contributing_audit.pl missing semantic smoke wiring fragment: {required}"
        );
    }

    let audit_doc = read_repo_file("docs/compiler/contributing-audit.md");
    assert!(audit_doc.contains("--skip-semantic-smoke"));
    assert!(audit_doc.contains("random_differential"));

    let testing_doc = read_repo_file("docs/compiler/testing.md");
    assert!(testing_doc.contains("--skip-semantic-smoke"));
}

#[test]
fn contributing_policy_render_is_in_sync() {
    let root = repo_root();
    let output = Command::new("python3")
        .arg(root.join("scripts").join("render_contributing_docs.py"))
        .arg("--check")
        .current_dir(&root)
        .output()
        .expect("failed to run contributing doc renderer");
    assert!(
        output.status.success(),
        "generated contributing docs should be in sync\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let pr_template = read_repo_file(".github/pull_request_template.md");
    assert!(pr_template.contains("## Why"));
    assert!(pr_template.contains("## Risk"));
    assert!(pr_template.contains("## Test Plan"));
    assert!(pr_template.contains("## Performance Impact"));
    assert!(pr_template.contains("## Subsystems"));
    assert!(pr_template.contains("## Verification"));
    assert!(pr_template.contains("## Benchmark Evidence"));
    assert!(pr_template.contains("## Dependency Impact"));
    assert!(pr_template.contains("## Exceptions"));
}

#[test]
fn contributing_audit_keeps_docs_cli_out_of_cache_logic_scope() {
    let root = repo_root();
    let script = root.join("scripts").join("contributing_audit.pl");
    let _audit_guard = contributing_audit_lock().lock().unwrap();
    let output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(root.join("docs").join("cli.md"))
        .current_dir(&root)
        .output()
        .expect("failed to run contributing audit on CLI docs");
    assert!(
        output.status.success(),
        "CLI docs-only audit should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("warn[cache-tests-review]"));
}

#[test]
fn contributing_audit_policy_sync_warning_respects_generated_check() {
    let root = repo_root();
    let script = root.join("scripts").join("contributing_audit.pl");
    let _audit_guard = contributing_audit_lock().lock().unwrap();
    let output = Command::new("perl")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(root.join("policy").join("contributing_rules.toml"))
        .arg(
            root.join("docs")
                .join("compiler")
                .join("contributing-audit.md"),
        )
        .current_dir(&root)
        .output()
        .expect("failed to run contributing audit on synced policy docs");
    assert!(
        output.status.success(),
        "synced contributing policy audit should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("warn[contributing-policy-sync]"));
}

#[test]
fn pr_evidence_script_enforces_policy_sections() {
    let root = repo_root();
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("contributing_audit_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "pr_evidence");
    fs::create_dir_all(&sandbox).expect("failed to create PR evidence sandbox");

    let bad_body = sandbox.join("bad_pr.md");
    fs::write(
        &bad_body,
        "## Verification\nRan cargo check only.\n\n## Benchmark Evidence\nN/A\n",
    )
    .expect("failed to write bad PR body");
    let changed_files = sandbox.join("changed_files.txt");
    fs::write(
        &changed_files,
        "src/compiler/pipeline.rs\nCargo.toml\npolicy/contributing_rules.toml\n",
    )
    .expect("failed to write changed files");

    let script = root.join("scripts").join("check_pr_evidence.py");
    let bad = Command::new("python3")
        .arg(&script)
        .arg("--body-file")
        .arg(&bad_body)
        .arg("--changed-files-file")
        .arg(&changed_files)
        .current_dir(&root)
        .output()
        .expect("failed to run PR evidence script on bad input");
    assert!(
        !bad.status.success(),
        "bad PR evidence should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );

    let good_body = sandbox.join("good_pr.md");
    fs::write(
        &good_body,
        r#"## Verification
Ran `cargo check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test -q --test contributing_audit_smoke`.

## Benchmark Evidence
Measured the touched compiler path locally; no regression beyond noise on the focused workload.

## Dependency Impact
`Cargo.toml` changed only to support audit tooling. No runtime dependency was added to the shipping compiler crate.

## Exceptions
No rule exception was introduced. Policy and audit wiring changed together in this PR.
"#,
    )
    .expect("failed to write good PR body");
    let good = Command::new("python3")
        .arg(&script)
        .arg("--body-file")
        .arg(&good_body)
        .arg("--changed-files-file")
        .arg(&changed_files)
        .current_dir(&root)
        .output()
        .expect("failed to run PR evidence script on good input");
    assert!(
        good.status.success(),
        "good PR evidence should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );
}
