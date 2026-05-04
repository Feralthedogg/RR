use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

pub(super) fn summarize_watch_changes(
    prev: &[(PathBuf, u64)],
    next: &[(PathBuf, u64)],
) -> Option<String> {
    let prev_map: FxHashMap<&PathBuf, u64> =
        prev.iter().map(|(path, hash)| (path, *hash)).collect();
    let next_map: FxHashMap<&PathBuf, u64> =
        next.iter().map(|(path, hash)| (path, *hash)).collect();

    let mut changed = Vec::new();
    for (path, hash) in &next_map {
        match prev_map.get(path) {
            Some(prev_hash) if prev_hash == hash => {}
            Some(_) | None => changed.push(display_watch_path(path)),
        }
    }
    for path in prev_map.keys() {
        if !next_map.contains_key(path) {
            changed.push(display_watch_path(path));
        }
    }

    changed.sort();
    changed.dedup();
    if changed.is_empty() {
        return None;
    }
    let first = changed[0].clone();
    if changed.len() == 1 {
        Some(first)
    } else {
        Some(format!("{first} (+{} more)", changed.len() - 1))
    }
}

fn display_watch_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{display_watch_path, summarize_watch_changes};
    use std::path::PathBuf;

    #[test]
    fn summarize_watch_changes_reports_single_changed_module() {
        let prev = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/module.rr"), 2_u64),
        ];
        let next = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/module.rr"), 3_u64),
        ];
        assert_eq!(
            summarize_watch_changes(&prev, &next),
            Some("module.rr".to_string())
        );
    }

    #[test]
    fn summarize_watch_changes_reports_added_and_removed_modules_compactly() {
        let prev = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/old.rr"), 2_u64),
        ];
        let next = vec![
            (PathBuf::from("/tmp/main.rr"), 1_u64),
            (PathBuf::from("/tmp/new.rr"), 9_u64),
        ];
        assert_eq!(
            summarize_watch_changes(&prev, &next),
            Some("new.rr (+1 more)".to_string())
        );
    }

    #[test]
    fn display_watch_path_prefers_file_name() {
        assert_eq!(
            display_watch_path(&PathBuf::from("/tmp/sub/module.rr")),
            "module.rr".to_string()
        );
    }
}
