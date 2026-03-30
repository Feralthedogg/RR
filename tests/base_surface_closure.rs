use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).expect("failed to read file")
}

fn extract_function_body<'a>(src: &'a str, fn_name: &str) -> &'a str {
    let start = src
        .find(fn_name)
        .unwrap_or_else(|| panic!("missing function marker: {fn_name}"));
    let tail = &src[start..];
    let next = [
        tail.find("\npub(super) fn "),
        tail.find("\npub(crate) fn "),
        tail.find("\nfn "),
    ]
    .into_iter()
    .flatten()
    .filter(|idx| *idx > 0)
    .min()
    .unwrap_or(tail.len());
    &tail[..next]
}

fn r_available() -> bool {
    Command::new("R")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[test]
fn regex_safe_base_surface_is_closed() {
    if !r_available() {
        eprintln!("Skipping base surface closure test: R not available.");
        return;
    }

    let code = read("src/mir/semantics/call_model.rs");
    let body = extract_function_body(&code, "pub(crate) fn is_supported_package_call");
    let re = Regex::new(r#""(base::[A-Za-z0-9_.]+)""#).expect("regex");
    let code_calls: std::collections::BTreeSet<String> = re
        .captures_iter(body)
        .map(|caps| caps[1].to_string())
        .filter(|name| !name.starts_with("base::."))
        .collect();

    let out = Command::new("R")
        .args([
            "--slave",
            "-e",
            "cat(getNamespaceExports('base'), sep='\\n')",
        ])
        .output()
        .expect("failed to execute R");
    assert!(
        out.status.success(),
        "R failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let export_re = Regex::new(r"^base::[A-Za-z0-9_.]+$").expect("regex");
    let exports: std::collections::BTreeSet<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| format!("base::{}", line.trim()))
        .filter(|name| export_re.is_match(name) && !name.starts_with("base::."))
        .collect();

    let missing: Vec<String> = exports.difference(&code_calls).cloned().collect();
    assert!(
        missing.is_empty(),
        "regex-safe base exports missing from direct surface: {}",
        missing.join(", ")
    );
}
