use crate::codegen::mir_emit::MapEntry;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::io::IsTerminal;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use regex::Regex;

pub struct Runner;

impl Runner {
    pub fn run(
        source_path: &str,
        _source_src: &str,
        r_code: &str,
        source_map: &[MapEntry],
        rscript_path: Option<&str>,
        keep_r: bool,
    ) -> i32 {
        let color = color_enabled_stderr();
        let runner_color = palette_for_module("RR.RunnerError");
        let gen_path = PathBuf::from(source_path).with_extension("gen.R");

        if let Err(e) = fs::write(&gen_path, r_code) {
            eprintln!(
                "{}",
                style(
                    color,
                    runner_color,
                    &format!(
                        "** (RR.RunnerError) {}: failed to write generated R file: {}",
                        source_path, e
                    ),
                )
            );
            return 1;
        }

        let rscript = rscript_path.unwrap_or("Rscript");
        let output = match Command::new(rscript)
            .arg("--vanilla")
            .arg(&gen_path)
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                eprintln!(
                    "{}",
                    style(
                        color,
                        runner_color,
                        &format!(
                            "** (RR.RunnerError) {}: failed to execute '{}': {}",
                            source_path, rscript, e
                        ),
                    )
                );
                if !keep_r {
                    fs::remove_file(&gen_path).ok();
                }
                return 1;
            }
        };

        // Stdout
        if let Err(e) = std::io::stdout().write_all(&output.stdout) {
            if e.kind() == ErrorKind::BrokenPipe {
                if !keep_r {
                    fs::remove_file(&gen_path).ok();
                }
                return 0;
            }
            eprintln!(
                "{}",
                style(
                    color,
                    runner_color,
                    &format!(
                        "** (RR.RunnerError) {}: failed to write stdout: {}",
                        source_path, e
                    ),
                )
            );
            if !keep_r {
                fs::remove_file(&gen_path).ok();
            }
            return 1;
        }

        // Stderr Mapping
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            Self::emit_mapped_stderr(source_path, &stderr, source_map, color);
        }

        if !keep_r {
            fs::remove_file(&gen_path).ok();
        }

        output.status.code().unwrap_or(1)
    }

    fn find_mapping(r_line: u32, map: &[MapEntry]) -> Option<&MapEntry> {
        map.iter()
            .filter(|e| e.r_line <= r_line)
            .max_by_key(|e| e.r_line)
    }

    fn emit_mapped_stderr(source_path: &str, stderr: &str, source_map: &[MapEntry], color: bool) {
        let mut active_module_color = palette_for_module("RR.RuntimeError");
        let mut active_code_color = active_module_color;

        for line in stderr.lines() {
            if let Some(module) = extract_rr_module(line) {
                active_module_color = palette_for_module(module);
                active_code_color = active_module_color;
            }
            if let Some(cap) = rr_code_regex().captures(line) {
                active_code_color = palette_for_rr_code(&cap[1], active_module_color);
            }
            if rrdiag_regex().captures(line).is_some() {
                // Runtime already emits formatted multi-line user diagnostics.
                continue;
            }

            if let Some(cap) = rr_runtime_loc_regex().captures(line) {
                let rr_line: u32 = cap[1].parse().unwrap_or(0);
                let rr_col: u32 = cap[2].parse().unwrap_or(0);
                eprintln!(
                    "{}",
                    style(
                        color,
                        active_module_color,
                        &format!("{}:{}:{}: {}", source_path, rr_line, rr_col, line),
                    )
                );
                continue;
            }

            if let Some(cap) = r_loc_regex().captures(line) {
                let file = &cap[1];
                let r_line: u32 = cap[2].parse().unwrap_or(0);
                if (file.ends_with(".gen.R") || file.ends_with(".R"))
                    && let Some(entry) = Self::find_mapping(r_line, source_map)
                {
                    eprintln!(
                        "{}",
                        style(
                            color,
                            active_module_color,
                            &format!(
                                "{}:{}:{}: (R {}): {}",
                                source_path,
                                entry.rr_span.start_line,
                                entry.rr_span.start_col,
                                r_line,
                                line
                            ),
                        )
                    );
                    continue;
                }
            }

            let code = fallback_stderr_style(line, active_module_color, active_code_color);
            eprintln!("{}", style(color, code, line));
        }
    }
}

fn rr_runtime_loc_regex() -> &'static Regex {
    static RR_RE: OnceLock<Regex> = OnceLock::new();
    RR_RE.get_or_init(|| Regex::new(r"RR:(\d+):(\d+):").expect("valid RR runtime location regex"))
}

fn r_loc_regex() -> &'static Regex {
    static R_LOC_RE: OnceLock<Regex> = OnceLock::new();
    R_LOC_RE.get_or_init(|| Regex::new(r"([^:\s]+):(\d+):(\d+):").expect("valid R location regex"))
}

fn rrdiag_regex() -> &'static Regex {
    static RRDIAG_RE: OnceLock<Regex> = OnceLock::new();
    RRDIAG_RE.get_or_init(|| Regex::new(r"^RRDIAG\|(.+)$").expect("valid RRDIAG regex"))
}

fn rr_code_regex() -> &'static Regex {
    static RR_CODE_RE: OnceLock<Regex> = OnceLock::new();
    RR_CODE_RE.get_or_init(|| Regex::new(r"error\[(ICE\d+|E\d+)\]").expect("valid RR code regex"))
}

fn fallback_stderr_style<'a>(
    line: &str,
    active_module_color: &'a str,
    active_code_color: &'a str,
) -> &'a str {
    if line.starts_with("warning[")
        || line.starts_with("Warning:")
        || line.starts_with("Warning message:")
        || line.contains(" warning[")
    {
        "1;38;5;208"
    } else if line.starts_with("In:") || line.starts_with("Hint:") || line.starts_with("note (R):")
    {
        "1;93"
    } else if line.starts_with("stacktrace:") || line.contains("(rr)") {
        "2"
    } else if line.contains("error[") {
        active_code_color
    } else if line.starts_with("** (") {
        active_module_color
    } else {
        "0"
    }
}

fn color_enabled_stderr() -> bool {
    let no_color = env::var_os("NO_COLOR").is_some();
    let force_color = env::var_os("RR_FORCE_COLOR").is_some();
    let is_tty = std::io::stderr().is_terminal();
    (force_color || is_tty) && !no_color
}

fn style(color: bool, code: &str, text: &str) -> String {
    if !color || code == "0" {
        return text.to_string();
    }
    format!("\x1b[{}m{}\x1b[0m", code, text)
}

fn extract_rr_module(line: &str) -> Option<&str> {
    let start = line.find("** (")?;
    let rest = &line[start + 4..];
    let end = rest.find(')')?;
    Some(&rest[..end])
}

fn palette_for_module(module: &str) -> &'static str {
    if module.contains("Warning") {
        "1;38;5;208"
    } else if module.contains("ParseError") || module.contains("LexError") {
        "1;95"
    } else if module.contains("TypeError") || module.contains("SemanticError") {
        "1;93"
    } else if module.contains("OptError") {
        "1;96"
    } else if module.contains("CodegenError") {
        "1;94"
    } else if module.contains("RunnerError") {
        "1;35"
    } else if module.contains("RuntimeError")
        || module.contains("BoundsError")
        || module.contains("ValueError")
    {
        "1;91"
    } else if module.contains("InternalError") || module.contains("ICE") {
        "1;97;41"
    } else {
        "1;91"
    }
}

fn palette_for_rr_code(code: &str, fallback: &'static str) -> &'static str {
    if code.starts_with("ICE") || code == "E9999" {
        "1;97;41"
    } else if code.starts_with("W") {
        "1;38;5;208"
    } else if code.starts_with("E0") {
        "1;35"
    } else if code.starts_with("E1") {
        "1;33"
    } else if code.starts_with("E2") {
        "1;31"
    } else if code.starts_with("E3") {
        "1;38;5;208"
    } else {
        fallback
    }
}
