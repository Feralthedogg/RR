use regex::Regex;
use std::collections::BTreeSet;
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

fn regex_safe_namespaced_calls_for_package(body: &str, package: &str) -> BTreeSet<String> {
    let re = Regex::new(r#""([A-Za-z0-9_.]+::[A-Za-z0-9_.]+)""#).expect("regex");
    re.captures_iter(body)
        .map(|caps| caps[1].to_string())
        .filter(|name| name.starts_with(&format!("{package}::")))
        .collect()
}

fn regex_safe_exports_for_package(package: &str) -> Option<BTreeSet<String>> {
    let script = format!(
        "if (!requireNamespace('{package}', quietly=TRUE)) quit(status=2); cat(getNamespaceExports('{package}'), sep='\\n')"
    );
    let out = Command::new("R")
        .args(["--slave", "-e", &script])
        .output()
        .expect("failed to execute R");

    if out.status.code() == Some(2) {
        eprintln!("Skipping package surface closure for `{package}`: package unavailable.");
        return None;
    }
    assert!(
        out.status.success(),
        "R failed for package `{package}`: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let regex_safe = Regex::new(&format!(
        r"^{pkg}::[A-Za-z0-9_.]+$",
        pkg = regex::escape(package)
    ))
    .expect("regex");

    let exports = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| format!("{package}::{}", line.trim()))
        .filter(|name| regex_safe.is_match(name))
        .collect();
    Some(exports)
}

#[test]
fn regex_safe_core_package_surfaces_are_closed() {
    if !r_available() {
        eprintln!("Skipping package surface closure test: R not available.");
        return;
    }

    let code = read("src/mir/semantics/call_model.rs");
    let body = extract_function_body(&code, "pub(crate) fn is_supported_package_call");

    // `base` is covered separately in `base_surface_closure.rs`.
    // It also has a package-wide direct fallback in `call_model`, so the
    // generic quoted-name closure check here is mainly for the remaining
    // packages that do not have the same special handling.
    let packages = [
        "compiler",
        "graphics",
        "grDevices",
        "grid",
        "methods",
        "parallel",
        "splines",
        "stats",
        "stats4",
        "tools",
        "utils",
        "readr",
        "tidyr",
        "dplyr",
        "ggplot2",
    ];

    for package in packages {
        let Some(exports) = regex_safe_exports_for_package(package) else {
            continue;
        };

        let code_calls = regex_safe_namespaced_calls_for_package(body, package);
        let missing: Vec<String> = exports.difference(&code_calls).cloned().collect();

        assert!(
            missing.is_empty(),
            "regex-safe `{package}` exports missing from direct surface: {}",
            missing.join(", ")
        );
    }
}
