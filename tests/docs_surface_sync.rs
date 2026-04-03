use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

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

fn quoted_namespaced_calls(src: &str) -> BTreeSet<String> {
    let re = Regex::new(r#""([A-Za-z0-9_.]+::[A-Za-z0-9_.]+)""#).expect("regex");
    re.captures_iter(src)
        .map(|caps| caps[1].to_string())
        .collect()
}

fn quoted_namespaced_calls_in_rs_tree(dir_rel: &str) -> BTreeSet<String> {
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

fn backtick_items_in_markdown_section(
    doc: &str,
    start_marker: &str,
    end_marker: &str,
) -> BTreeSet<String> {
    let start = doc
        .find(start_marker)
        .unwrap_or_else(|| panic!("missing marker: {start_marker}"));
    let tail = &doc[start + start_marker.len()..];
    let end = tail
        .find(end_marker)
        .unwrap_or_else(|| panic!("missing end marker: {end_marker}"));
    let section = &tail[..end];
    let re = Regex::new(r#"- `([^`]+)`"#).expect("regex");
    re.captures_iter(section)
        .map(|caps| caps[1].to_string())
        .collect()
}

fn backtick_items_in_markdown_files(dir_rel: &str) -> BTreeSet<String> {
    let dir = repo_root().join(dir_rel);
    let mut items = BTreeSet::new();
    let re = Regex::new(r#"- `([^`]+)`"#).expect("regex");

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("failed to read markdown dir")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect();
    entries.sort();

    for path in entries {
        let text = fs::read_to_string(&path).expect("failed to read markdown file");
        items.extend(re.captures_iter(&text).map(|caps| caps[1].to_string()));
    }

    items
}

fn long_options(src: &str) -> BTreeSet<String> {
    let re = Regex::new(r"--[a-z][a-z0-9-]*").expect("regex");
    re.find_iter(src).map(|m| m.as_str().to_string()).collect()
}

fn env_vars(src: &str) -> BTreeSet<String> {
    let re = Regex::new(r"\b(?:RR_[A-Z0-9_]+|NO_COLOR|RRSCRIPT)\b").expect("regex");
    re.find_iter(src).map(|m| m.as_str().to_string()).collect()
}

#[test]
fn docs_direct_interop_surface_matches_code() {
    let code_calls =
        quoted_namespaced_calls_in_rs_tree("src/mir/semantics/call_model_package_surface");

    let doc_calls = backtick_items_in_markdown_files("docs/r-interop");

    assert_eq!(
        doc_calls, code_calls,
        "docs/r-interop/*.md direct interop surface drifted from call_model_package_surface.rs"
    );
}

#[test]
fn docs_tidy_helper_surface_matches_code() {
    let code = read("src/mir/semantics/call_model_builtin_surface.rs");
    let body = extract_function_body(&code, "pub(crate) fn is_tidy_helper_call");
    let re = Regex::new(r#""([A-Za-z_][A-Za-z0-9_]*)""#).expect("regex");
    let code_helpers: BTreeSet<String> = re
        .captures_iter(body)
        .map(|caps| caps[1].to_string())
        .collect();

    let docs = read("docs/r-interop.md");
    let doc_helpers = backtick_items_in_markdown_section(
        &docs,
        "Currently supported tidy helpers:",
        "Special forms:",
    );

    assert_eq!(
        doc_helpers, code_helpers,
        "docs/r-interop.md tidy helper list drifted from call_model_builtin_surface.rs"
    );
}

#[test]
fn docs_tidy_data_mask_surface_matches_code() {
    let code = read("src/mir/semantics/call_model_builtin_surface.rs");
    let body = extract_function_body(&code, "pub(crate) fn is_tidy_data_mask_call");
    let code_calls = quoted_namespaced_calls(body);

    let docs = read("docs/r-interop.md");
    let doc_calls = backtick_items_in_markdown_section(
        &docs,
        "Currently supported tidy-aware calls:",
        "Currently supported tidy helpers:",
    );

    assert_eq!(
        doc_calls, code_calls,
        "docs/r-interop.md tidy-aware call list drifted from call_model_builtin_surface.rs"
    );
}

#[test]
fn docs_cli_long_options_match_driver_usage() {
    let code = read("src/main.rs");
    let body = extract_function_body(&code, "fn print_usage()");
    let code_opts = long_options(body);

    let docs = read("docs/cli.md");
    let doc_opts = long_options(&docs);

    assert_eq!(
        doc_opts, code_opts,
        "docs/cli.md long-option surface drifted from src/main.rs::print_usage"
    );
}

#[test]
fn docs_configuration_envs_match_public_config_surface() {
    let mut code_envs = BTreeSet::new();
    for rel in [
        "build.rs",
        "src/compiler/pipeline.rs",
        "src/compiler/incremental.rs",
        "src/hir/lower.rs",
        "src/mir/opt/config.rs",
        "src/mir/opt/inline.rs",
        "src/mir/opt/bce.rs",
        "src/mir/opt/poly/mod.rs",
        "src/mir/opt/poly/schedule.rs",
        "src/mir/opt/v_opt/debug.rs",
        "src/runtime/runtime_prelude.R",
        "tests/perf_regression_gate.rs",
        "tests/example_perf_smoke.rs",
        "tests/common/mod.rs",
    ] {
        code_envs.extend(env_vars(&read(rel)));
    }
    code_envs.remove("RR_COMPILER_BUILD_HASH");
    code_envs.remove("RR_HAS_ISL");
    code_envs.remove("RR_ISL_LINK_MODE");

    let docs = read("docs/configuration.md");
    let doc_envs = env_vars(&docs);

    assert_eq!(
        doc_envs, code_envs,
        "docs/configuration.md env-var surface drifted from code/test public config sources"
    );
}
