use rr::compiler::CliLog;

use super::parse_registry_override_args;

pub(crate) fn cmd_search(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let Ok((positional, registry)) =
        parse_registry_override_args(args, "use RR search <query> [--registry <dir>]", &ui)
    else {
        return 1;
    };
    if positional.len() != 1 {
        ui.error("RR search expects exactly one query");
        ui.warn("use RR search <query> [--registry <dir>]");
        return 1;
    }
    match rr::pkg::search_registry_modules(&positional[0], registry.as_deref()) {
        Ok(results) => {
            if results.is_empty() {
                ui.success("No registry modules matched the query");
                return 0;
            }
            for result in results {
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
                ui.success(&format!(
                    "{} latest={} releases={} yanked={} license={} deprecated={} desc={}",
                    result.path,
                    latest,
                    result.release_count,
                    result.yanked_count,
                    license,
                    deprecated,
                    description
                ));
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}
