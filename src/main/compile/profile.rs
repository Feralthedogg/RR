use rr::compiler::{CliLog, CompileProfile, json_escape};
use std::fs;
use std::path::PathBuf;

use super::{report_dir_create_failure, report_file_write_failure};

pub(crate) fn write_compile_profile_artifact(
    ui: &CliLog,
    profile: &CompileProfile,
    out_path: Option<&str>,
) -> Result<(), i32> {
    let json = profile.to_json_string();
    if let Some(out_path) = out_path {
        let path = PathBuf::from(out_path);
        if let Some(parent) = path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            report_dir_create_failure(ui, parent, &e, "compile profile output directory");
            return Err(1);
        }
        if let Err(e) = fs::write(&path, json) {
            report_file_write_failure(ui, &path, &e, "compile profile output path");
            return Err(1);
        }
        ui.success(&format!("Compile profile -> {}", path.display()));
    } else {
        eprintln!("{json}");
    }
    Ok(())
}

fn compile_profile_collection_to_json(entries: &[(String, CompileProfile)]) -> String {
    let mut out = String::from(
        "{\n  \"schema\": \"rr-compile-profile-collection\",\n  \"version\": 2,\n  \"profiles\": [\n",
    );
    for (idx, (input, profile)) in entries.iter().enumerate() {
        if idx > 0 {
            out.push_str(",\n");
        }
        out.push_str("    {\"input\": \"");
        out.push_str(&json_escape(input));
        out.push_str("\", \"profile\": ");
        out.push_str(&profile.to_json_string());
        out.push('}');
    }
    out.push_str("\n  ]\n}");
    out
}

pub(crate) fn write_compile_profile_collection(
    ui: &CliLog,
    entries: &[(String, CompileProfile)],
    out_path: Option<&str>,
) -> Result<(), i32> {
    let json = compile_profile_collection_to_json(entries);
    if let Some(out_path) = out_path {
        let path = PathBuf::from(out_path);
        if let Some(parent) = path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            report_dir_create_failure(ui, parent, &e, "compile profile collection directory");
            return Err(1);
        }
        if let Err(e) = fs::write(&path, json) {
            report_file_write_failure(ui, &path, &e, "compile profile collection path");
            return Err(1);
        }
        ui.success(&format!("Compile profile -> {}", path.display()));
    } else {
        eprintln!("{json}");
    }
    Ok(())
}
