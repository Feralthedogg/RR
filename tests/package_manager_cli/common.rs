pub(crate) use crate::common::unique_dir;
pub(crate) use ed25519_dalek::{SigningKey, VerifyingKey};
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::Command;

pub(crate) fn rr_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_RR"))
}

pub(crate) fn ed25519_public_hex(secret_hex: &str) -> String {
    let bytes = hex_to_vec(secret_hex);
    let secret: [u8; 32] = bytes
        .try_into()
        .expect("ed25519 secret must be exactly 32 bytes");
    let signing = SigningKey::from_bytes(&secret);
    let public: VerifyingKey = signing.verifying_key();
    hex_from_bytes(public.as_bytes())
}

pub(crate) fn hex_to_vec(raw: &str) -> Vec<u8> {
    assert_eq!(raw.len() % 2, 0, "hex string length must be even");
    let mut out = Vec::with_capacity(raw.len() / 2);
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let hi = char::from(bytes[i])
            .to_digit(16)
            .expect("invalid hex digit");
        let lo = char::from(bytes[i + 1])
            .to_digit(16)
            .expect("invalid hex digit");
        out.push(((hi << 4) | lo) as u8);
        i += 2;
    }
    out
}

pub(crate) fn hex_from_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub(crate) fn init_git_repo(dir: &Path, files: &[(&str, &str)], tag: &str) {
    fs::create_dir_all(dir).expect("failed to create repo dir");
    for (rel, content) in files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create file parent");
        }
        fs::write(&path, content).expect("failed to write repo file");
    }

    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg("--initial-branch=main")
        .arg(dir)
        .status()
        .expect("failed to init git repo");
    assert!(status.success(), "git init failed");

    let status = Command::new("git")
        .current_dir(dir)
        .args(["config", "user.email", "rr-tests@example.com"])
        .status()
        .expect("failed to configure git email");
    assert!(status.success(), "git config user.email failed");

    let status = Command::new("git")
        .current_dir(dir)
        .args(["config", "user.name", "RR Tests"])
        .status()
        .expect("failed to configure git name");
    assert!(status.success(), "git config user.name failed");

    let status = Command::new("git")
        .current_dir(dir)
        .args(["add", "."])
        .status()
        .expect("failed to git add");
    assert!(status.success(), "git add failed");

    let status = Command::new("git")
        .current_dir(dir)
        .args(["commit", "-q", "-m", "initial"])
        .status()
        .expect("failed to git commit");
    assert!(status.success(), "git commit failed");

    let status = Command::new("git")
        .current_dir(dir)
        .args(["tag", tag])
        .status()
        .expect("failed to git tag");
    assert!(status.success(), "git tag failed");
}

pub(crate) fn configure_github_mapping(cmd: &mut Command, github_root: &Path, pkg_home: &Path) {
    let github_root = fs::canonicalize(github_root).expect("failed to canonicalize github root");
    let mut base = github_root.to_string_lossy().to_string();
    if !base.ends_with('/') {
        base.push('/');
    }
    let file_base = format!("file://{}", base);
    cmd.env("RRPKGHOME", pkg_home)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", format!("url.{}.insteadOf", file_base))
        .env("GIT_CONFIG_VALUE_0", "https://github.com/")
        .env("GIT_ALLOW_PROTOCOL", "file:https");
}
