use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn r_available() -> bool {
    Command::new("R")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn regex_safe_namespaced_calls_in_rs_tree(dir_rel: &str) -> BTreeSet<String> {
    let dir = repo_root().join(dir_rel);
    let mut items = BTreeSet::new();
    let re = Regex::new(r#""([A-Za-z0-9_.]+::[A-Za-z0-9_.]+)""#).expect("regex");

    let mut stack = vec![dir];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).expect("failed to read rust dir") {
            let entry = entry.expect("failed to read entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let text = fs::read_to_string(&path).expect("failed to read rust file");
            items.extend(re.captures_iter(&text).map(|caps| caps[1].to_string()));
        }
    }

    items
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

    let code_calls =
        regex_safe_namespaced_calls_in_rs_tree("src/mir/semantics/call_model_package_surface");

    // `base` is covered separately in `base_surface_closure.rs`.
    // It also has a package-wide direct fallback in `call_model_package_surface`, so the
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

        let package_calls = code_calls
            .iter()
            .filter(|name| name.starts_with(&format!("{package}::")))
            .cloned()
            .collect::<BTreeSet<_>>();
        let missing: Vec<String> = exports.difference(&package_calls).cloned().collect();

        assert!(
            missing.is_empty(),
            "regex-safe `{package}` exports missing from direct surface: {}",
            missing.join(", ")
        );
    }
}
