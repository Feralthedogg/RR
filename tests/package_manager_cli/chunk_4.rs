use super::package_manager_cli_common::*;

#[test]
pub(crate) fn registry_verify_detects_missing_archive_and_admin_commands_restore_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_verify_dirty_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/adminlib")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    fs::write(
        pkg_dir.join("rr.mod"),
        r#"module rr.local/adminlib

rr 8.0
description = "Administrative registry package"
"#,
    )
    .expect("failed to write rr.mod");

    let registry_dir = unique_dir(&sandbox_root, "registry_verify_dirty_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    for (version, body) in [
        (
            "v1.0.0",
            r#"
fn add_one(x) {
  return x + 1L
}
"#,
        ),
        (
            "v1.1.0",
            r#"
fn add_one(x) {
  return x + 2L
}
"#,
        ),
    ] {
        fs::write(pkg_dir.join("src").join("lib.rr"), body).expect("failed to write lib source");
        let publish = Command::new(&rr_bin)
            .current_dir(&pkg_dir)
            .arg("publish")
            .arg(version)
            .arg("--registry")
            .arg(&registry_dir)
            .output()
            .expect("failed to publish registry package");
        assert!(
            publish.status.success(),
            "publish {} failed:\nstdout:\n{}\nstderr:\n{}",
            version,
            String::from_utf8_lossy(&publish.stdout),
            String::from_utf8_lossy(&publish.stderr)
        );
    }

    let yank = Command::new(&rr_bin)
        .arg("registry")
        .arg("yank")
        .arg("rr.local/adminlib")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry yank");
    assert!(yank.status.success(), "rr registry yank failed");

    let deprecate = Command::new(&rr_bin)
        .arg("registry")
        .arg("deprecate")
        .arg("rr.local/adminlib")
        .arg("legacy package")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry deprecate");
    assert!(deprecate.status.success(), "rr registry deprecate failed");

    let archive = registry_dir
        .join("pkg")
        .join("rr.local")
        .join("adminlib")
        .join("adminlib@v1.1.0.tar.gz");
    fs::remove_file(&archive).expect("failed to remove archive");

    let verify = Command::new(&rr_bin)
        .arg("registry")
        .arg("verify")
        .arg("rr.local/adminlib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry verify");
    assert!(
        !verify.status.success(),
        "rr registry verify should fail when archive is missing"
    );
    let verify_stderr = String::from_utf8_lossy(&verify.stderr);
    assert!(
        verify_stderr.contains("rr.local/adminlib")
            && verify_stderr.contains("v1.1.0")
            && verify_stderr.contains("archive is missing"),
        "expected missing archive verify output, got:\n{}",
        verify_stderr
    );

    let unyank = Command::new(&rr_bin)
        .arg("registry")
        .arg("unyank")
        .arg("rr.local/adminlib")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry unyank");
    assert!(unyank.status.success(), "rr registry unyank failed");

    let undeprecate = Command::new(&rr_bin)
        .arg("registry")
        .arg("undeprecate")
        .arg("rr.local/adminlib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry undeprecate");
    assert!(
        undeprecate.status.success(),
        "rr registry undeprecate failed"
    );

    let info = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/adminlib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info.status.success(), "rr registry info failed");
    let info_stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        !info_stdout.contains("deprecated legacy package")
            && info_stdout.contains("release v1.1.0")
            && info_stdout.contains("yanked=false"),
        "expected undeprecated and unyanked registry info output, got:\n{}",
        info_stdout
    );
}

#[test]
pub(crate) fn signed_registry_release_installs_and_verifies_with_trust_key() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_signed_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/signedlib")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write library source");

    let registry_dir = unique_dir(&sandbox_root, "registry_signed_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let signing_key = "rr-test-signing-key";
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_KEY", signing_key)
        .output()
        .expect("failed to publish signed registry package");
    assert!(
        publish.status.success(),
        "signed publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let index = fs::read_to_string(
        registry_dir
            .join("index")
            .join("rr.local")
            .join("signedlib.toml"),
    )
    .expect("failed to read registry index");
    assert!(
        index.contains("sig = \"hmac-sha256:"),
        "expected registry signature in index, got:\n{}",
        index
    );

    let verify = Command::new(&rr_bin)
        .arg("registry")
        .arg("verify")
        .arg("rr.local/signedlib")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_TRUST_KEY", signing_key)
        .output()
        .expect("failed to verify signed registry");
    assert!(
        verify.status.success(),
        "signed registry verify failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );

    let info = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/signedlib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info.status.success(), "rr registry info failed");
    let info_stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        info_stdout.contains("signed=true"),
        "expected signed release in registry info, got:\n{}",
        info_stdout
    );

    let app_dir = unique_dir(&sandbox_root, "registry_signed_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-signed-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_signed");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/signedlib@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .env("RR_REGISTRY_TRUST_KEY", signing_key)
        .output()
        .expect("failed to install signed registry package");
    assert!(
        install.status.success(),
        "signed install failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );
}

#[test]
pub(crate) fn signed_registry_release_rejects_wrong_trust_key() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_signed_fail_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/signedfail")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let registry_dir = unique_dir(&sandbox_root, "registry_signed_fail_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_KEY", "correct-key")
        .output()
        .expect("failed to publish signed registry package");
    assert!(
        publish.status.success(),
        "signed publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let app_dir = unique_dir(&sandbox_root, "registry_signed_fail_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-signed-fail-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_signed_fail");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/signedfail@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .env("RR_REGISTRY_TRUST_KEY", "wrong-key")
        .output()
        .expect("failed to install signed registry package");
    assert!(
        !install.status.success(),
        "signed install should fail with wrong trust key"
    );
    let stderr = String::from_utf8_lossy(&install.stderr);
    assert!(
        stderr.contains("signature mismatch"),
        "expected signature mismatch, got:\n{}",
        stderr
    );
}

#[test]
pub(crate) fn ed25519_signed_registry_release_supports_signer_identity_and_key_rotation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_ed25519_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/edlib")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let secret_hex = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";
    let public_hex = ed25519_public_hex(secret_hex);
    let trust_keys = format!(
        "{},{}",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", public_hex
    );

    let registry_dir = unique_dir(&sandbox_root, "registry_ed25519_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_ED25519_SECRET", secret_hex)
        .env("RR_REGISTRY_SIGNING_IDENTITY", "release-bot")
        .output()
        .expect("failed to publish ed25519 registry package");
    assert!(
        publish.status.success(),
        "ed25519 publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let index = fs::read_to_string(
        registry_dir
            .join("index")
            .join("rr.local")
            .join("edlib.toml"),
    )
    .expect("failed to read registry index");
    assert!(
        index.contains("sig = \"ed25519:")
            && index.contains(&public_hex)
            && index.contains("signer = \"release-bot\""),
        "expected ed25519 signature and signer in index, got:\n{}",
        index
    );

    let verify = Command::new(&rr_bin)
        .arg("registry")
        .arg("verify")
        .arg("rr.local/edlib")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_TRUST_ED25519_KEYS", &trust_keys)
        .output()
        .expect("failed to verify ed25519 registry");
    assert!(
        verify.status.success(),
        "ed25519 registry verify failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );

    let info = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/edlib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info.status.success(), "rr registry info failed");
    let info_stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        info_stdout.contains("signed=true")
            && info_stdout.contains("scheme=ed25519")
            && info_stdout.contains("signer=release-bot"),
        "expected signer identity and ed25519 scheme in info output, got:\n{}",
        info_stdout
    );

    let app_dir = unique_dir(&sandbox_root, "registry_ed25519_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-ed25519-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_ed25519");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/edlib@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .env("RR_REGISTRY_TRUST_ED25519_KEYS", &trust_keys)
        .output()
        .expect("failed to install ed25519 registry package");
    assert!(
        install.status.success(),
        "ed25519 install failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );
}

#[test]
pub(crate) fn ed25519_signed_registry_release_rejects_untrusted_public_key() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_ed25519_fail_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/edfail")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let secret_hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    let registry_dir = unique_dir(&sandbox_root, "registry_ed25519_fail_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_ED25519_SECRET", secret_hex)
        .env("RR_REGISTRY_SIGNING_IDENTITY", "release-bot")
        .output()
        .expect("failed to publish ed25519 registry package");
    assert!(
        publish.status.success(),
        "ed25519 publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let app_dir = unique_dir(&sandbox_root, "registry_ed25519_fail_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-ed25519-fail-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_ed25519_fail");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/edfail@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .env(
            "RR_REGISTRY_TRUST_ED25519_KEYS",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .output()
        .expect("failed to install ed25519 registry package");
    assert!(
        !install.status.success(),
        "ed25519 install should fail with untrusted public key"
    );
    let stderr = String::from_utf8_lossy(&install.stderr);
    assert!(
        stderr.contains("not trusted"),
        "expected untrusted signer error, got:\n{}",
        stderr
    );
}

#[test]
pub(crate) fn registry_policy_file_supplies_trust_and_signer_allowlist() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_policy_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/policylib")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let secret_hex = "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
    let public_hex = ed25519_public_hex(secret_hex);
    let registry_dir = unique_dir(&sandbox_root, "registry_policy_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_ED25519_SECRET", secret_hex)
        .env("RR_REGISTRY_SIGNING_IDENTITY", "release-bot")
        .output()
        .expect("failed to publish policy-signed package");
    assert!(
        publish.status.success(),
        "policy publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    fs::write(
        registry_dir.join("policy.toml"),
        format!(
            "version = 1\nrequire_signed = true\ntrusted_ed25519 = \"{}\"\nallowed_signer = \"release-bot\"\n",
            public_hex
        ),
    )
    .expect("failed to write policy.toml");

    let verify = Command::new(&rr_bin)
        .arg("registry")
        .arg("verify")
        .arg("rr.local/policylib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to verify policy-backed registry");
    assert!(
        verify.status.success(),
        "policy-backed verify failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );

    let app_dir = unique_dir(&sandbox_root, "registry_policy_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-policy-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_policy");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/policylib@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to install policy-backed registry package");
    assert!(
        install.status.success(),
        "policy-backed install failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );
}

#[test]
pub(crate) fn registry_policy_file_rejects_revoked_key() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_policy_revoke_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/revokedlib")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let secret_hex = "404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f";
    let public_hex = ed25519_public_hex(secret_hex);
    let registry_dir = unique_dir(&sandbox_root, "registry_policy_revoke_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_ED25519_SECRET", secret_hex)
        .env("RR_REGISTRY_SIGNING_IDENTITY", "release-bot")
        .output()
        .expect("failed to publish revoked-key package");
    assert!(
        publish.status.success(),
        "revoked-key publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    fs::write(
        registry_dir.join("policy.toml"),
        format!(
            "version = 1\ntrusted_ed25519 = \"{}\"\nrevoked_ed25519 = \"{}\"\nallowed_signer = \"release-bot\"\n",
            public_hex, public_hex
        ),
    )
    .expect("failed to write policy.toml");

    let verify = Command::new(&rr_bin)
        .arg("registry")
        .arg("verify")
        .arg("rr.local/revokedlib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to verify revoked-key registry");
    assert!(
        !verify.status.success(),
        "verify should fail for revoked signer key"
    );
    let verify_stderr = String::from_utf8_lossy(&verify.stderr);
    assert!(
        verify_stderr.contains("revoked"),
        "expected revoked key error, got:\n{}",
        verify_stderr
    );
}

#[test]
pub(crate) fn registry_keygen_writes_files_and_matching_public_key() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let out_dir = unique_dir(&sandbox_root, "registry_keygen");
    let output = Command::new(&rr_bin)
        .arg("registry")
        .arg("keygen")
        .arg("release-bot")
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run rr registry keygen");
    assert!(
        output.status.success(),
        "rr registry keygen failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let public = fs::read_to_string(out_dir.join("registry-ed25519-public.key"))
        .expect("failed to read public key")
        .trim()
        .to_string();
    let secret = fs::read_to_string(out_dir.join("registry-ed25519-secret.key"))
        .expect("failed to read secret key")
        .trim()
        .to_string();
    let env_file = fs::read_to_string(out_dir.join("registry-signing.env"))
        .expect("failed to read env template");
    assert_eq!(
        public,
        ed25519_public_hex(&secret),
        "public key should match generated secret"
    );
    assert!(
        env_file.contains(&format!("RR_REGISTRY_SIGNING_ED25519_SECRET={secret}"))
            && env_file.contains(&format!("RR_REGISTRY_TRUST_ED25519_KEYS={public}"))
            && env_file.contains("RR_REGISTRY_SIGNING_IDENTITY=release-bot"),
        "expected env template, got:\n{}",
        env_file
    );
}
