use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn collect_rr_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "Build"
                || name == "build"
                || name == "target"
                || name == ".git"
                || name == "vendor"
            {
                continue;
            }
            collect_rr_files(&path, files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn build_output_file(
    dir_mode: bool,
    rr: &Path,
    rr_abs: &Path,
    target_path: &Path,
    root_abs: Option<&PathBuf>,
    out_root: &Path,
) -> PathBuf {
    if dir_mode {
        let rel = rr
            .strip_prefix(target_path)
            .ok()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .or_else(|| {
                root_abs.and_then(|root| {
                    rr_abs
                        .strip_prefix(root)
                        .ok()
                        .filter(|p| !p.as_os_str().is_empty())
                        .map(Path::to_path_buf)
                })
            })
            .or_else(|| rr.file_name().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("out.rr"));
        out_root.join(rel).with_extension("R")
    } else {
        let stem = rr.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
        out_root.join(format!("{}.R", stem))
    }
}
