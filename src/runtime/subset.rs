use crate::runtime::R_RUNTIME;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::OnceLock;

#[derive(Clone)]
struct RuntimeDef {
    source: String,
    deps: Vec<String>,
}

struct RuntimeIndex {
    bootstrap: String,
    order: Vec<String>,
    defs: FxHashMap<String, RuntimeDef>,
}

fn runtime_index() -> &'static RuntimeIndex {
    static INDEX: OnceLock<RuntimeIndex> = OnceLock::new();
    INDEX.get_or_init(parse_runtime_index)
}

fn parse_runtime_index() -> RuntimeIndex {
    let mut bootstrap = String::new();
    let mut order = Vec::new();
    let mut defs = FxHashMap::default();
    let mut lines = R_RUNTIME.split_inclusive('\n');
    let mut outer_depth = 0isize;

    while let Some(line) = lines.next() {
        if outer_depth == 0
            && let Some(name) = runtime_def_name(line)
        {
            let mut source = String::from(line);
            let mut depth = brace_balance_delta(line);
            let mut saw_open = line.contains('{');
            while !saw_open || depth > 0 {
                let Some(next) = lines.next() else {
                    break;
                };
                source.push_str(next);
                saw_open |= next.contains('{');
                depth += brace_balance_delta(next);
            }
            let deps = collect_runtime_symbols(&source)
                .into_iter()
                .filter(|dep| dep != &name)
                .collect::<Vec<_>>();
            order.push(name.clone());
            defs.insert(name, RuntimeDef { source, deps });
        } else {
            bootstrap.push_str(line);
            outer_depth += brace_balance_delta(line);
        }
    }

    RuntimeIndex {
        bootstrap: sanitize_runtime_bootstrap(bootstrap),
        order,
        defs,
    }
}

fn sanitize_runtime_bootstrap(mut bootstrap: String) -> String {
    bootstrap = bootstrap.replace(
        ".rr_env$native_lib <- rr_native_resolve_lib()\n.rr_env$native_loaded <- FALSE\n",
        ".rr_env$native_lib <- \"\"\n.rr_env$native_loaded <- FALSE\n",
    );

    const FAST_START: &str = "# Fast-path rebinding for release mode:";
    const FAST_END: &str = "# -----------------------------------";
    if let Some(start) = bootstrap.find(FAST_START)
        && let Some(rel_end) = bootstrap[start..].find(FAST_END)
    {
        let mut end = start + rel_end + FAST_END.len();
        if bootstrap[end..].starts_with('\n') {
            end += 1;
        }
        bootstrap.replace_range(start..end, "");
    }

    compact_runtime_source(&bootstrap, true)
}

fn compact_runtime_source(source: &str, keep_banner: bool) -> String {
    let mut out = String::new();
    let mut kept_banner = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if keep_banner && !kept_banner && trimmed == "# --- RR runtime (auto-generated) ---" {
            out.push_str(line);
            out.push('\n');
            kept_banner = true;
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn runtime_def_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("rr_") {
        return None;
    }
    let mut end = 3usize;
    for (idx, ch) in trimmed.char_indices().skip(3) {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end = idx + ch.len_utf8();
            continue;
        }
        end = idx;
        break;
    }
    let name = &trimmed[..end];
    let tail = trimmed[end..].trim_start();
    if tail.starts_with("<- function") {
        Some(name.to_string())
    } else {
        None
    }
}

fn brace_balance_delta(line: &str) -> isize {
    let opens = line.bytes().filter(|b| *b == b'{').count() as isize;
    let closes = line.bytes().filter(|b| *b == b'}').count() as isize;
    opens - closes
}

pub fn referenced_runtime_symbols(code: &str) -> FxHashSet<String> {
    let index = runtime_index();
    collect_runtime_symbols(code)
        .into_iter()
        .filter(|name| index.defs.contains_key(name))
        .collect()
}

pub fn render_runtime_subset(roots: &FxHashSet<String>) -> String {
    let index = runtime_index();
    let mut needed = roots.clone();
    let mut stack = roots.iter().cloned().collect::<Vec<_>>();

    while let Some(name) = stack.pop() {
        let Some(def) = index.defs.get(&name) else {
            continue;
        };
        for dep in &def.deps {
            if needed.insert(dep.clone()) {
                stack.push(dep.clone());
            }
        }
    }

    let mut out = String::new();
    out.push_str(&index.bootstrap);
    for name in &index.order {
        if needed.contains(name)
            && let Some(def) = index.defs.get(name)
        {
            out.push_str(&compact_runtime_source(&def.source, false));
        }
    }
    out
}

fn collect_runtime_symbols(text: &str) -> FxHashSet<String> {
    let bytes = text.as_bytes();
    let mut out = FxHashSet::default();
    let mut idx = 0usize;
    while idx + 3 <= bytes.len() {
        if bytes[idx] == b'r'
            && bytes.get(idx + 1) == Some(&b'r')
            && bytes.get(idx + 2) == Some(&b'_')
            && is_symbol_boundary_before(bytes, idx)
        {
            let mut end = idx + 3;
            while end < bytes.len() {
                let ch = bytes[end];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    end += 1;
                } else {
                    break;
                }
            }
            if is_symbol_boundary(bytes, end) {
                out.insert(text[idx..end].to_string());
            }
            idx = end;
            continue;
        }
        idx += 1;
    }
    out
}

fn is_symbol_boundary_before(bytes: &[u8], idx: usize) -> bool {
    if idx == 0 {
        return true;
    }
    is_symbol_boundary(bytes, idx - 1)
}

fn is_symbol_boundary(bytes: &[u8], idx: usize) -> bool {
    if idx >= bytes.len() {
        return true;
    }
    let ch = bytes[idx];
    !(ch.is_ascii_alphanumeric() || ch == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_subset_includes_only_requested_helper_closure() {
        let roots = FxHashSet::from_iter([String::from("rr_assign_slice")]);
        let subset = render_runtime_subset(&roots);
        assert!(subset.contains("rr_assign_slice <- function"));
        assert!(!subset.contains("rr_parallel_typed_vec_call <- function"));
        assert!(!subset.contains("rr_array3_shift_assign <- function"));
    }

    #[test]
    fn runtime_subset_pulls_transitive_dependencies() {
        let roots = FxHashSet::from_iter([String::from("rr_index1_read")]);
        let subset = render_runtime_subset(&roots);
        assert!(subset.contains("rr_index1_read <- function"));
        assert!(subset.contains("rr_index1_read_strict <- function"));
        assert!(subset.contains("rr_bounds_error <- function"));
        assert!(subset.contains("rr_fail <- function"));
    }
}
