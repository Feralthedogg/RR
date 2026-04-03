use std::env;
use std::path::Path;
use std::process::Command;

pub(super) fn run_git(cwd: Option<&Path>, args: &[&str]) -> Result<String, String> {
    run_git_with_env(cwd, &[], args)
}

fn github_token() -> Option<String> {
    env::var("RRGITHUB_TOKEN")
        .ok()
        .filter(|token| !token.trim().is_empty())
        .or_else(|| {
            env::var("GITHUB_TOKEN")
                .ok()
                .filter(|token| !token.trim().is_empty())
        })
}

fn apply_github_auth_env(command: &mut Command, args: &[&str]) {
    let Some(repo_arg) = args
        .iter()
        .find(|arg| arg.starts_with("https://github.com/"))
    else {
        return;
    };
    let Some(token) = github_token() else {
        return;
    };

    let existing_count = env::var("GIT_CONFIG_COUNT")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(0);
    command.env("GIT_CONFIG_COUNT", (existing_count + 1).to_string());
    command.env(
        format!("GIT_CONFIG_KEY_{}", existing_count),
        "http.https://github.com/.extraheader",
    );
    command.env(
        format!("GIT_CONFIG_VALUE_{}", existing_count),
        format!(
            "AUTHORIZATION: basic {}",
            base64_encode(format!("x-access-token:{token}").as_bytes())
        ),
    );
    command.env("GIT_TERMINAL_PROMPT", "0");
    let _ = repo_arg;
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut idx = 0;
    while idx < bytes.len() {
        let b0 = bytes[idx];
        let b1 = *bytes.get(idx + 1).unwrap_or(&0);
        let b2 = *bytes.get(idx + 2).unwrap_or(&0);

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if idx + 1 < bytes.len() {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if idx + 2 < bytes.len() {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        idx += 3;
    }
    out
}

pub(super) fn run_git_with_env(
    cwd: Option<&Path>,
    envs: &[(&str, &str)],
    args: &[&str],
) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    for (key, value) in envs {
        command.env(key, value);
    }
    apply_github_auth_env(&mut command, args);
    let output = command
        .output()
        .map_err(|e| format!("failed to run git {:?}: {}", args, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git {:?} failed with status {}", args, output.status)
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(super) fn clean_git_error(message: String) -> String {
    message.trim().to_string()
}

pub(super) fn ensure_git_identity(repo: &Path) -> Result<(), String> {
    let email = Command::new("git")
        .current_dir(repo)
        .args(["config", "user.email", "rr@local"])
        .status()
        .map_err(|e| format!("failed to configure git user.email: {}", e))?;
    if !email.success() {
        return Err("failed to configure git user.email".to_string());
    }
    let name = Command::new("git")
        .current_dir(repo)
        .args(["config", "user.name", "RR"])
        .status()
        .map_err(|e| format!("failed to configure git user.name: {}", e))?;
    if !name.success() {
        return Err("failed to configure git user.name".to_string());
    }
    Ok(())
}

pub(super) fn git_repo_is_dirty(project_root: &Path) -> Result<bool, String> {
    if !project_root.join(".git").exists() {
        return Ok(false);
    }
    let output = run_git(Some(project_root), &["status", "--porcelain"])?;
    Ok(!output.trim().is_empty())
}

pub(super) fn git_tag_exists(project_root: &Path, tag: &str) -> Result<bool, String> {
    if !project_root.join(".git").exists() {
        return Ok(false);
    }
    let output = Command::new("git")
        .current_dir(project_root)
        .args(["rev-parse", "-q", "--verify", &format!("refs/tags/{tag}")])
        .output()
        .map_err(|e| format!("failed to check git tag '{}': {}", tag, e))?;
    Ok(output.status.success())
}

pub(super) fn create_git_tag(project_root: &Path, tag: &str) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(project_root)
        .args(["tag", tag])
        .output()
        .map_err(|e| format!("failed to create git tag '{}': {}", tag, e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

pub(super) fn push_git_tag(project_root: &Path, remote: &str, tag: &str) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(project_root)
        .args(["push", remote, tag])
        .output()
        .map_err(|e| format!("failed to push git tag '{}': {}", tag, e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}
