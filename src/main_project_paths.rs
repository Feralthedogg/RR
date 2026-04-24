use std::path::{Path, PathBuf};

pub(super) fn project_dir_name_from_module_path(module_path: &str) -> String {
    module_path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("rr-app")
        .to_string()
}

pub(super) fn default_module_path_for_dir(dir: &Path) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("rr-app")
        .to_string()
}

fn find_project_root_from_path(path: &Path) -> Option<PathBuf> {
    let start = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let mut cur = start;
    loop {
        let managed_root = cur.join("rr.mod").is_file()
            || cur.join("src").join("main.rr").is_file()
            || cur.join("src").join("lib.rr").is_file();
        let legacy_root = cur.file_name().and_then(|name| name.to_str()) != Some("src")
            && cur.join("main.rr").is_file();
        if managed_root || legacy_root {
            return Some(cur);
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }
    None
}

pub(crate) fn default_build_output_dir(target_path: &Path) -> PathBuf {
    let root = find_project_root_from_path(target_path).unwrap_or_else(|| {
        if target_path.is_dir() {
            target_path.to_path_buf()
        } else {
            target_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        }
    });
    root.join("Build").join("debug")
}

pub(crate) fn default_watch_output_file(entry_path: &Path) -> PathBuf {
    let root = find_project_root_from_path(entry_path).unwrap_or_else(|| {
        entry_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let stem = entry_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    root.join("Build").join("watch").join(format!("{stem}.R"))
}
