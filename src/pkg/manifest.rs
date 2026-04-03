use super::Manifest;
use std::fs;
use std::path::Path;

impl Manifest {
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut manifest = Manifest::default();
        let mut in_require_block = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            if in_require_block {
                if trimmed == ")" {
                    in_require_block = false;
                    continue;
                }
                let (path, version) = parse_require_line(trimmed)?;
                manifest.requires.insert(path, version);
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("module ") {
                manifest.module_path = rest.trim().to_string();
            } else if let Some(rest) = trimmed.strip_prefix("rr ") {
                manifest.rr_version = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("description") {
                manifest.description = Some(parse_manifest_string(
                    rest.trim().trim_start_matches('=').trim(),
                )?);
            } else if let Some(rest) = trimmed.strip_prefix("license") {
                manifest.license = Some(parse_manifest_string(
                    rest.trim().trim_start_matches('=').trim(),
                )?);
            } else if let Some(rest) = trimmed.strip_prefix("homepage") {
                manifest.homepage = Some(parse_manifest_string(
                    rest.trim().trim_start_matches('=').trim(),
                )?);
            } else if trimmed == "require (" {
                in_require_block = true;
            } else if let Some(rest) = trimmed.strip_prefix("replace ") {
                let (path, target) = parse_replace_line(rest)?;
                manifest.replaces.insert(path, target);
            } else if let Some(rest) = trimmed.strip_prefix("require ") {
                let (path, version) = parse_require_line(rest)?;
                manifest.requires.insert(path, version);
            }
        }

        if manifest.module_path.trim().is_empty() {
            return Err("rr.mod is missing a `module <path>` line".to_string());
        }

        Ok(manifest)
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let manifest_path = dir.join("rr.mod");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("failed to read '{}': {}", manifest_path.display(), e))?;
        Self::parse(&content)
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("module {}\n\n", self.module_path));
        if let Some(rr_version) = &self.rr_version {
            out.push_str(&format!("rr {}\n", rr_version));
        }
        if let Some(description) = &self.description {
            out.push_str(&format!("description = \"{}\"\n", escape_toml(description)));
        }
        if let Some(license) = &self.license {
            out.push_str(&format!("license = \"{}\"\n", escape_toml(license)));
        }
        if let Some(homepage) = &self.homepage {
            out.push_str(&format!("homepage = \"{}\"\n", escape_toml(homepage)));
        }
        if !self.requires.is_empty() {
            out.push('\n');
            if self.requires.len() == 1 {
                let (path, version) = self.requires.iter().next().unwrap();
                out.push_str(&format!("require {} {}\n", path, version));
            } else {
                out.push_str("require (\n");
                for (path, version) in &self.requires {
                    out.push_str(&format!("    {} {}\n", path, version));
                }
                out.push_str(")\n");
            }
        }
        if !self.replaces.is_empty() {
            out.push('\n');
            for (path, target) in &self.replaces {
                out.push_str(&format!("replace {} => {}\n", path, target));
            }
        }
        out
    }

    pub fn write_to_dir(&self, dir: &Path) -> Result<(), String> {
        let manifest_path = dir.join("rr.mod");
        fs::write(&manifest_path, self.render())
            .map_err(|e| format!("failed to write '{}': {}", manifest_path.display(), e))
    }
}

fn parse_require_line(raw: &str) -> Result<(String, String), String> {
    let mut parts = raw.split_whitespace();
    let Some(path) = parts.next() else {
        return Err("invalid require line: missing module path".to_string());
    };
    let Some(version) = parts.next() else {
        return Err(format!(
            "invalid require line for '{}': missing version",
            path
        ));
    };
    Ok((path.to_string(), version.to_string()))
}

fn parse_manifest_string(raw: &str) -> Result<String, String> {
    if raw.starts_with('"') {
        parse_toml_string(raw)
    } else {
        Ok(raw.to_string())
    }
}

fn parse_replace_line(raw: &str) -> Result<(String, String), String> {
    let Some((path, target)) = raw.split_once("=>") else {
        return Err(format!(
            "invalid replace line '{}': expected `replace <module> => <path>`",
            raw
        ));
    };
    let path = path.trim();
    let target = target.trim();
    if path.is_empty() || target.is_empty() {
        return Err(format!(
            "invalid replace line '{}': module path and target must be non-empty",
            raw
        ));
    }
    Ok((path.to_string(), target.to_string()))
}

pub(super) fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn parse_toml_string(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return Err(format!("expected TOML string, got '{}'", raw));
    }
    Ok(trimmed[1..trimmed.len() - 1]
        .replace("\\\"", "\"")
        .replace("\\\\", "\\"))
}
