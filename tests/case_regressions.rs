mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Expectation {
    CompileOk,
    ParseError,
    SemanticError,
    TypeError,
    RunEqualO0O2,
}

impl Expectation {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "compile-ok" => Some(Self::CompileOk),
            "parse-error" => Some(Self::ParseError),
            "semantic-error" => Some(Self::SemanticError),
            "type-error" => Some(Self::TypeError),
            "run-equal-o0-o2" => Some(Self::RunEqualO0O2),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct CaseSpec {
    category: String,
    name: String,
    src_path: PathBuf,
    expect: Option<Expectation>,
    flags: Vec<String>,
    env: Vec<(String, String)>,
    stdout_contains: Vec<String>,
    stdout_not_contains: Vec<String>,
    stderr_contains: Vec<String>,
    stderr_not_contains: Vec<String>,
    emit_contains: Vec<String>,
    emit_not_contains: Vec<String>,
}

#[derive(Debug)]
struct CompileResult {
    ok: bool,
    stdout: String,
    stderr: String,
    out_path: PathBuf,
}

#[test]
fn parser_file_regressions() {
    run_category("parser", 10);
}

#[test]
fn semantic_file_regressions() {
    run_category("semantic", 6);
}

#[test]
fn typeck_file_regressions() {
    run_category("typeck", 5);
}

#[test]
fn optimizer_file_regressions() {
    run_category("optimizer", 8);
}

#[test]
fn docs_file_regressions() {
    run_category("docs", 4);
}

#[test]
fn file_regression_suite_is_not_too_small() {
    let cases = collect_cases();
    assert!(
        cases.len() >= 33,
        "file regression suite unexpectedly small: found {} cases",
        cases.len()
    );
}

fn run_category(category: &str, min_cases: usize) {
    let mut cases: Vec<CaseSpec> = collect_cases()
        .into_iter()
        .filter(|case| case.category == category)
        .collect();
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    assert!(
        cases.len() >= min_cases,
        "category '{}' has too few file cases: found {}, expected at least {}",
        category,
        cases.len(),
        min_cases
    );
    for case in cases {
        run_case(&case);
    }
}

fn collect_cases() -> Vec<CaseSpec> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cases_root = root.join("tests").join("cases");
    assert!(
        cases_root.exists(),
        "missing file regression directory: {}",
        cases_root.display()
    );

    let mut out = Vec::new();
    let categories = fs::read_dir(&cases_root).expect("failed to read tests/cases");
    for category_entry in categories.flatten() {
        let category_path = category_entry.path();
        if !category_path.is_dir() {
            continue;
        }
        let category_name = category_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let case_entries = fs::read_dir(&category_path)
            .unwrap_or_else(|_| panic!("failed to read {}", category_path.display()));
        for case_entry in case_entries.flatten() {
            let case_dir = case_entry.path();
            if !case_dir.is_dir() {
                continue;
            }
            out.push(load_case(&category_name, &case_dir));
        }
    }
    out
}

fn load_case(category: &str, case_dir: &Path) -> CaseSpec {
    let mut spec = CaseSpec {
        category: category.to_string(),
        name: case_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("case")
            .to_string(),
        src_path: case_dir.join("main.rr"),
        ..Default::default()
    };
    assert!(
        spec.src_path.exists(),
        "missing main.rr for case {}",
        case_dir.display()
    );

    let meta_path = case_dir.join("case.meta");
    let meta = fs::read_to_string(&meta_path)
        .unwrap_or_else(|_| panic!("missing case.meta for {}", case_dir.display()));

    for (line_no, raw_line) in meta.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            panic!(
                "invalid case meta line {} in {}: {}",
                line_no + 1,
                meta_path.display(),
                raw_line
            );
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "expect" => {
                spec.expect = Expectation::parse(value);
                assert!(
                    spec.expect.is_some(),
                    "unknown expectation '{}' in {}",
                    value,
                    meta_path.display()
                );
            }
            "flag" => spec.flags.push(value.to_string()),
            "env" => {
                let Some((env_key, env_value)) = value.split_once('=') else {
                    panic!(
                        "invalid env directive '{}' in {}",
                        value,
                        meta_path.display()
                    );
                };
                spec.env
                    .push((env_key.trim().to_string(), env_value.trim().to_string()));
            }
            "stdout_contains" => spec.stdout_contains.push(value.to_string()),
            "stdout_not_contains" => spec.stdout_not_contains.push(value.to_string()),
            "stderr_contains" => spec.stderr_contains.push(value.to_string()),
            "stderr_not_contains" => spec.stderr_not_contains.push(value.to_string()),
            "emit_contains" => spec.emit_contains.push(value.to_string()),
            "emit_not_contains" => spec.emit_not_contains.push(value.to_string()),
            _ => panic!("unknown case meta key '{}' in {}", key, meta_path.display()),
        }
    }

    assert!(
        spec.expect.is_some(),
        "missing expect=... in {}",
        meta_path.display()
    );
    spec
}

fn run_case(case: &CaseSpec) {
    match case.expect.clone().expect("expectation must exist") {
        Expectation::CompileOk => {
            let compile = compile_case(case, None);
            assert_compile_success(case, &compile);
            assert_compile_messages(case, &compile);
            assert_emit_expectations(case, &compile.out_path);
        }
        Expectation::ParseError => {
            let compile = compile_case(case, None);
            assert!(
                !compile.ok,
                "case '{}': compile unexpectedly succeeded",
                case.name
            );
            assert!(
                compile.stdout.contains("** (RR.ParseError)"),
                "case '{}': missing parse error header\nstdout:\n{}",
                case.name,
                compile.stdout
            );
            assert_compile_messages(case, &compile);
        }
        Expectation::SemanticError => {
            let compile = compile_case(case, None);
            assert!(
                !compile.ok,
                "case '{}': compile unexpectedly succeeded",
                case.name
            );
            assert!(
                compile.stdout.contains("** (RR.SemanticError)"),
                "case '{}': missing semantic error header\nstdout:\n{}",
                case.name,
                compile.stdout
            );
            assert_compile_messages(case, &compile);
        }
        Expectation::TypeError => {
            let compile = compile_case(case, None);
            assert!(
                !compile.ok,
                "case '{}': compile unexpectedly succeeded",
                case.name
            );
            assert!(
                compile.stdout.contains("** (RR.TypeError)"),
                "case '{}': missing type error header\nstdout:\n{}",
                case.name,
                compile.stdout
            );
            assert_compile_messages(case, &compile);
        }
        Expectation::RunEqualO0O2 => {
            let rscript = match rscript_path() {
                Some(path) if rscript_available(&path) => path,
                _ => {
                    eprintln!(
                        "Skipping run-equal-o0-o2 case '{}': Rscript not available",
                        case.name
                    );
                    return;
                }
            };
            assert!(
                !case.flags.iter().any(|flag| flag == "--no-runtime"),
                "case '{}': run-equal-o0-o2 cases must not use --no-runtime",
                case.name
            );
            assert!(
                !has_opt_flag(&case.flags),
                "case '{}': run-equal-o0-o2 cases must not hardcode an optimization flag",
                case.name
            );

            let compile_o0 = compile_case(case, Some("-O0"));
            let compile_o2 = compile_case(case, Some("-O2"));
            assert_compile_success(case, &compile_o0);
            assert_compile_success(case, &compile_o2);
            assert_compile_messages(case, &compile_o2);
            assert_emit_expectations(case, &compile_o2.out_path);

            let run_o0 = run_rscript(&rscript, &compile_o0.out_path);
            let run_o2 = run_rscript(&rscript, &compile_o2.out_path);
            assert_eq!(
                run_o0.status, run_o2.status,
                "case '{}': exit status mismatch between -O0 and -O2",
                case.name
            );
            assert_eq!(
                normalize(&run_o0.stdout),
                normalize(&run_o2.stdout),
                "case '{}': stdout mismatch between -O0 and -O2\n-O0:\n{}\n-O2:\n{}",
                case.name,
                run_o0.stdout,
                run_o2.stdout
            );
            assert_eq!(
                normalize(&run_o0.stderr),
                normalize(&run_o2.stderr),
                "case '{}': stderr mismatch between -O0 and -O2\n-O0:\n{}\n-O2:\n{}",
                case.name,
                run_o0.stderr,
                run_o2.stderr
            );
        }
    }
}

fn compile_case(case: &CaseSpec, forced_opt: Option<&str>) -> CompileResult {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root
        .join("target")
        .join("tests")
        .join("case_regressions")
        .join(&case.category)
        .join(&case.name);
    fs::create_dir_all(&sandbox).expect("failed to create case sandbox");

    let out_name = match forced_opt {
        Some("-O0") => "out_o0.R",
        Some("-O2") => "out_o2.R",
        _ => "out.R",
    };
    let out_path = sandbox.join(out_name);
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let mut cmd = Command::new(rr_bin);
    cmd.arg(&case.src_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-incremental");
    for flag in &case.flags {
        cmd.arg(flag);
    }
    if let Some(opt_flag) = forced_opt {
        cmd.arg(opt_flag);
    } else if !has_opt_flag(&case.flags) {
        cmd.arg("-O1");
    }
    let strict_let_overridden = case
        .env
        .iter()
        .any(|(key, _)| key == "RR_STRICT_LET" || key == "RR_STRICT_ASSIGN");
    if !strict_let_overridden {
        // Most file-regression fixtures predate the strict-let default and are
        // intended to exercise parser/typeck/optimizer behavior instead.
        cmd.arg("--strict-let").arg("0");
    }
    for arg in compile_env_args(&case.env) {
        cmd.arg(arg);
    }
    for (key, value) in &case.env {
        if !is_compile_policy_env(key) {
            cmd.env(key, value);
        }
    }
    let output = cmd
        .output()
        .unwrap_or_else(|_| panic!("failed to run RR for case '{}'", case.name));

    CompileResult {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        out_path,
    }
}

fn has_opt_flag(flags: &[String]) -> bool {
    flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "-O0" | "-O1" | "-O2" | "-o0" | "-o1" | "-o2"))
}

fn is_compile_policy_env(key: &str) -> bool {
    matches!(
        key,
        "RR_STRICT_LET" | "RR_STRICT_ASSIGN" | "RR_WARN_IMPLICIT_DECL"
    )
}

fn compile_env_args(env_kv: &[(String, String)]) -> Vec<&str> {
    let mut args = Vec::new();
    for (key, value) in env_kv {
        match key.as_str() {
            "RR_STRICT_LET" | "RR_STRICT_ASSIGN" => {
                args.push("--strict-let");
                args.push(value.as_str());
            }
            "RR_WARN_IMPLICIT_DECL" => {
                args.push("--warn-implicit-decl");
                args.push(value.as_str());
            }
            _ => {}
        }
    }
    args
}

fn assert_compile_success(case: &CaseSpec, compile: &CompileResult) {
    assert!(
        compile.ok,
        "case '{}': compile failed unexpectedly\nstdout:\n{}\nstderr:\n{}",
        case.name, compile.stdout, compile.stderr
    );
}

fn assert_compile_messages(case: &CaseSpec, compile: &CompileResult) {
    for needle in &case.stdout_contains {
        assert!(
            compile.stdout.contains(needle),
            "case '{}': stdout missing '{}'\nstdout:\n{}",
            case.name,
            needle,
            compile.stdout
        );
    }
    for needle in &case.stdout_not_contains {
        assert!(
            !compile.stdout.contains(needle),
            "case '{}': stdout unexpectedly contained '{}'\nstdout:\n{}",
            case.name,
            needle,
            compile.stdout
        );
    }
    for needle in &case.stderr_contains {
        assert!(
            compile.stderr.contains(needle),
            "case '{}': stderr missing '{}'\nstderr:\n{}",
            case.name,
            needle,
            compile.stderr
        );
    }
    for needle in &case.stderr_not_contains {
        assert!(
            !compile.stderr.contains(needle),
            "case '{}': stderr unexpectedly contained '{}'\nstderr:\n{}",
            case.name,
            needle,
            compile.stderr
        );
    }
}

fn assert_emit_expectations(case: &CaseSpec, out_path: &Path) {
    if case.emit_contains.is_empty() && case.emit_not_contains.is_empty() {
        return;
    }
    let emitted = fs::read_to_string(out_path).unwrap_or_else(|_| {
        panic!(
            "case '{}': failed to read emitted output {}",
            case.name,
            out_path.display()
        )
    });
    for needle in &case.emit_contains {
        assert!(
            emitted.contains(needle),
            "case '{}': emitted R missing '{}'\noutput: {}",
            case.name,
            needle,
            out_path.display()
        );
    }
    for needle in &case.emit_not_contains {
        assert!(
            !emitted.contains(needle),
            "case '{}': emitted R unexpectedly contained '{}'\noutput: {}",
            case.name,
            needle,
            out_path.display()
        );
    }
}
