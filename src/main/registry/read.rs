use rr::compiler::CliLog;
use std::path::Path;

pub(super) fn cmd_registry_list(ui: &CliLog, registry: Option<&Path>) -> i32 {
    match rr::pkg::list_registry_modules(registry) {
        Ok(results) => {
            if results.is_empty() {
                ui.success("Registry is empty");
                return 0;
            }
            for result in results {
                print_registry_search_result(result);
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(super) fn cmd_registry_report(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: Option<&str>,
) -> i32 {
    match rr::pkg::registry_report(registry, module_path) {
        Ok(report) => {
            println!(
                "modules={} channels={} releases={} approved={} pending={} yanked={} signed={} deprecated={}",
                report.module_count,
                report.channel_count,
                report.release_count,
                report.approved_count,
                report.pending_count,
                report.yanked_count,
                report.signed_count,
                report.deprecated_module_count
            );
            for module in report.modules {
                println!(
                    "{} latest={} channels={} releases={} approved={} pending={} yanked={} signed={} deprecated={}",
                    module.path,
                    module.latest_version.unwrap_or_else(|| "-".to_string()),
                    module.channel_count,
                    module.release_count,
                    module.approved_count,
                    module.pending_count,
                    module.yanked_count,
                    module.signed_count,
                    if module.deprecated { "true" } else { "false" }
                );
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(super) fn cmd_registry_diff(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    from_version: &str,
    to_version: &str,
) -> i32 {
    match rr::pkg::registry_diff(registry, module_path, from_version, to_version) {
        Ok(diff) => {
            println!(
                "module={} from={} to={}",
                diff.module_path, diff.from_version, diff.to_version
            );
            println!(
                "meta approved={}=>{} yanked={}=>{} signed={}=>{} signer={}=>{}",
                if diff.from_approved { "true" } else { "false" },
                if diff.to_approved { "true" } else { "false" },
                if diff.from_yanked { "true" } else { "false" },
                if diff.to_yanked { "true" } else { "false" },
                if diff.from_signed { "true" } else { "false" },
                if diff.to_signed { "true" } else { "false" },
                diff.from_signer.as_deref().unwrap_or("-"),
                diff.to_signer.as_deref().unwrap_or("-")
            );
            println!(
                "files added={} removed={} changed={}",
                diff.added_files.len(),
                diff.removed_files.len(),
                diff.changed_files.len()
            );
            for path in diff.added_files {
                println!("+ {}", path);
            }
            for path in diff.removed_files {
                println!("- {}", path);
            }
            for path in diff.changed_files {
                println!("~ {}", path);
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(super) fn cmd_registry_queue(ui: &CliLog, registry: Option<&Path>) -> i32 {
    match rr::pkg::list_registry_queue(registry) {
        Ok(items) => {
            if items.is_empty() {
                ui.success("Registry approval queue is empty");
                return 0;
            }
            for item in items {
                println!(
                    "{} {} yanked={} signed={} signer={}",
                    item.path,
                    item.version,
                    if item.yanked { "true" } else { "false" },
                    if item.signed { "true" } else { "false" },
                    item.signer.as_deref().unwrap_or("-")
                );
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(super) fn cmd_registry_info(ui: &CliLog, registry: Option<&Path>, module_path: &str) -> i32 {
    match rr::pkg::registry_module_info(module_path, registry) {
        Ok(info) => {
            println!("module {}", info.path);
            if let Some(description) = info.description.filter(|value| !value.is_empty()) {
                println!("description {}", description);
            }
            if let Some(license) = info.license.filter(|value| !value.is_empty()) {
                println!("license {}", license);
            }
            if let Some(homepage) = info.homepage.filter(|value| !value.is_empty()) {
                println!("homepage {}", homepage);
            }
            if let Some(deprecated) = info.deprecated.filter(|value| !value.is_empty()) {
                println!("deprecated {}", deprecated);
            }
            for (channel, version) in info.channels {
                println!("channel {} {}", channel, version);
            }
            for release in info.releases {
                let signer = release
                    .signer
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("-");
                let scheme = release
                    .signature_scheme
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("-");
                println!(
                    "release {} files={} yanked={} approved={} signed={} scheme={} signer={} archive={} sum={}",
                    release.version,
                    release.file_count,
                    if release.yanked { "true" } else { "false" },
                    if release.approved { "true" } else { "false" },
                    if release.signed { "true" } else { "false" },
                    scheme,
                    signer,
                    release.archive_rel,
                    release.archive_sum
                );
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(super) fn cmd_registry_verify(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: Option<&str>,
) -> i32 {
    match rr::pkg::verify_registry(registry, module_path) {
        Ok(report) => {
            if report.issues.is_empty() {
                if let Some(module_path) = module_path {
                    ui.success(&format!(
                        "Verified registry module {}: {} release(s)",
                        module_path, report.checked_releases
                    ));
                } else {
                    ui.success(&format!(
                        "Verified registry: {} module(s), {} release(s)",
                        report.checked_modules, report.checked_releases
                    ));
                }
                return 0;
            }
            for issue in report.issues {
                ui.error(&format!(
                    "{} {} archive={} {}",
                    issue.path,
                    issue.version,
                    issue.archive_path.display(),
                    issue.message
                ));
            }
            1
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

fn print_registry_search_result(result: rr::pkg::RegistrySearchResult) {
    let latest = result.latest_version.unwrap_or_else(|| "-".to_string());
    let description = result
        .description
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or("-");
    let license = result
        .license
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or("-");
    let deprecated = result
        .deprecated
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or("-");
    println!(
        "{} latest={} releases={} pending={} yanked={} license={} deprecated={} desc={}",
        result.path,
        latest,
        result.release_count,
        result.pending_count,
        result.yanked_count,
        license,
        deprecated,
        description
    );
}
