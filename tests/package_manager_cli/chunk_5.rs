use super::package_manager_cli_common::*;

#[test]
fn registry_policy_lint_and_rotate_key_update_policy_file() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let registry_dir = unique_dir(&sandbox_root, "registry_policy_manage");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    let old_key = "1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let new_key = "2222222222222222222222222222222222222222222222222222222222222222".to_string();
    fs::write(
        registry_dir.join("policy.toml"),
        format!(
            "version = 1\nrequire_signed = true\ntrusted_ed25519 = \"{old_key}\"\ntrusted_ed25519 = \"{old_key}\"\nallowed_signer = \"release-bot\"\nallowed_signer = \"release-bot\"\n"
        ),
    )
    .expect("failed to write policy.toml");

    let lint = Command::new(&rr_bin)
        .arg("registry")
        .arg("policy")
        .arg("lint")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry policy lint");
    assert!(lint.status.success(), "rr registry policy lint failed");
    let lint_stderr = String::from_utf8_lossy(&lint.stderr);
    assert!(
        lint_stderr.contains("duplicate trusted_ed25519 entry")
            && lint_stderr.contains("duplicate allowed_signer entry"),
        "expected duplicate warnings, got:\n{}",
        lint_stderr
    );

    let rotate = Command::new(&rr_bin)
        .arg("registry")
        .arg("policy")
        .arg("rotate-key")
        .arg(&old_key)
        .arg(&new_key)
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry policy rotate-key");
    assert!(
        rotate.status.success(),
        "rr registry policy rotate-key failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rotate.stdout),
        String::from_utf8_lossy(&rotate.stderr)
    );

    let policy = fs::read_to_string(registry_dir.join("policy.toml"))
        .expect("failed to read rotated policy.toml");
    assert!(
        policy.contains(&format!("trusted_ed25519 = \"{new_key}\""))
            && policy.contains(&format!("revoked_ed25519 = \"{old_key}\""))
            && !policy.contains(&format!("trusted_ed25519 = \"{old_key}\"")),
        "expected rotated policy contents, got:\n{}",
        policy
    );
}

#[test]
fn registry_policy_bootstrap_and_approval_flow_gate_latest_install() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_approval_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/approval")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let secret_hex = "606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f";
    let public_hex = ed25519_public_hex(secret_hex);
    let registry_dir = unique_dir(&sandbox_root, "registry_approval_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    let bootstrap = Command::new(&rr_bin)
        .arg("registry")
        .arg("policy")
        .arg("bootstrap")
        .arg(&public_hex)
        .arg("--signer")
        .arg("release-bot")
        .arg("--require-signed")
        .arg("--require-approval")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry policy bootstrap");
    assert!(
        bootstrap.status.success(),
        "registry policy bootstrap failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bootstrap.stdout),
        String::from_utf8_lossy(&bootstrap.stderr)
    );

    let policy = fs::read_to_string(registry_dir.join("policy.toml"))
        .expect("failed to read bootstrapped policy");
    assert!(
        policy.contains("require_signed = true")
            && policy.contains("require_approval = true")
            && policy.contains(&format!("trusted_ed25519 = \"{public_hex}\""))
            && policy.contains("allowed_signer = \"release-bot\""),
        "expected bootstrapped policy contents, got:\n{}",
        policy
    );

    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_ED25519_SECRET", secret_hex)
        .env("RR_REGISTRY_SIGNING_IDENTITY", "release-bot")
        .output()
        .expect("failed to publish approval-gated package");
    assert!(
        publish.status.success(),
        "approval-gated publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let info_before = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/approval")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info_before.status.success(), "rr registry info failed");
    let info_before_stdout = String::from_utf8_lossy(&info_before.stdout);
    assert!(
        info_before_stdout.contains("approved=false"),
        "expected pending approval in registry info, got:\n{}",
        info_before_stdout
    );

    let queue_before = Command::new(&rr_bin)
        .arg("registry")
        .arg("queue")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry queue");
    assert!(queue_before.status.success(), "rr registry queue failed");
    let queue_before_stdout = String::from_utf8_lossy(&queue_before.stdout);
    assert!(
        queue_before_stdout.contains("rr.local/approval") && queue_before_stdout.contains("v1.0.0"),
        "expected pending release in queue, got:\n{}",
        queue_before_stdout
    );

    let app_dir = unique_dir(&sandbox_root, "registry_approval_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-approval-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_approval");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install_before = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/approval@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to install approval-gated package");
    assert!(
        !install_before.status.success(),
        "install should fail before approval"
    );

    let approve = Command::new(&rr_bin)
        .arg("registry")
        .arg("approve")
        .arg("rr.local/approval")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry approve");
    assert!(approve.status.success(), "rr registry approve failed");

    let queue_after = Command::new(&rr_bin)
        .arg("registry")
        .arg("queue")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry queue");
    assert!(queue_after.status.success(), "rr registry queue failed");
    let queue_after_stdout = String::from_utf8_lossy(&queue_after.stdout);
    assert!(
        queue_after_stdout.contains("Registry approval queue is empty"),
        "expected empty queue after approval, got:\n{}",
        queue_after_stdout
    );

    let install_after = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/approval@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to install approved package");
    assert!(
        install_after.status.success(),
        "install should succeed after approval:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install_after.stdout),
        String::from_utf8_lossy(&install_after.stderr)
    );

    let unapprove = Command::new(&rr_bin)
        .arg("registry")
        .arg("unapprove")
        .arg("rr.local/approval")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry unapprove");
    assert!(unapprove.status.success(), "rr registry unapprove failed");

    let info_after = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/approval")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info_after.status.success(), "rr registry info failed");
    let info_after_stdout = String::from_utf8_lossy(&info_after.stdout);
    assert!(
        info_after_stdout.contains("approved=false"),
        "expected unapproved release in registry info, got:\n{}",
        info_after_stdout
    );

    let audit = Command::new(&rr_bin)
        .arg("registry")
        .arg("audit")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry audit");
    assert!(audit.status.success(), "rr registry audit failed");
    let audit_stdout = String::from_utf8_lossy(&audit.stdout);
    assert!(
        audit_stdout.contains("\tregistry-policy\tbootstrap registry policy")
            && audit_stdout
                .contains("\tpublish\tmodule=rr.local/approval version=v1.0.0 approved=false")
            && audit_stdout.contains("\tregistry-index\tapprove rr.local/approval v1.0.0")
            && audit_stdout.contains("\tregistry-index\tunapprove rr.local/approval v1.0.0"),
        "expected audit entries, got:\n{}",
        audit_stdout
    );
}

#[test]
fn registry_auto_approve_signer_skips_manual_queue() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_autoapprove_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/autoapprove")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let secret_hex = "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f";
    let public_hex = ed25519_public_hex(secret_hex);
    let registry_dir = unique_dir(&sandbox_root, "registry_autoapprove_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    let bootstrap = Command::new(&rr_bin)
        .arg("registry")
        .arg("policy")
        .arg("bootstrap")
        .arg(&public_hex)
        .arg("--signer")
        .arg("release-bot")
        .arg("--auto-approve-signer")
        .arg("release-bot")
        .arg("--require-signed")
        .arg("--require-approval")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry policy bootstrap");
    assert!(
        bootstrap.status.success(),
        "registry policy bootstrap failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bootstrap.stdout),
        String::from_utf8_lossy(&bootstrap.stderr)
    );

    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_ED25519_SECRET", secret_hex)
        .env("RR_REGISTRY_SIGNING_IDENTITY", "release-bot")
        .output()
        .expect("failed to publish auto-approved package");
    assert!(
        publish.status.success(),
        "auto-approved publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let info = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/autoapprove")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info.status.success(), "rr registry info failed");
    let info_stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        info_stdout.contains("approved=true"),
        "expected auto-approved release, got:\n{}",
        info_stdout
    );

    let queue = Command::new(&rr_bin)
        .arg("registry")
        .arg("queue")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry queue");
    assert!(queue.status.success(), "rr registry queue failed");
    let queue_stdout = String::from_utf8_lossy(&queue.stdout);
    assert!(
        queue_stdout.contains("Registry approval queue is empty"),
        "expected empty queue for auto-approved release, got:\n{}",
        queue_stdout
    );
}

#[test]
fn registry_onboard_and_policy_show_apply_round_trip() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let registry_dir = unique_dir(&sandbox_root, "registry_onboard_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let keys_dir = unique_dir(&sandbox_root, "registry_onboard_keys");

    let onboard = Command::new(&rr_bin)
        .arg("registry")
        .arg("onboard")
        .arg("release-bot")
        .arg("--out-dir")
        .arg(&keys_dir)
        .arg("--require-signed")
        .arg("--require-approval")
        .arg("--auto-approve")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry onboard");
    assert!(
        onboard.status.success(),
        "rr registry onboard failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&onboard.stdout),
        String::from_utf8_lossy(&onboard.stderr)
    );

    let public = fs::read_to_string(keys_dir.join("registry-ed25519-public.key"))
        .expect("failed to read public key")
        .trim()
        .to_string();
    let policy = fs::read_to_string(registry_dir.join("policy.toml"))
        .expect("failed to read onboarded policy");
    assert!(
        policy.contains("require_signed = true")
            && policy.contains("require_approval = true")
            && policy.contains("allowed_signer = \"release-bot\"")
            && policy.contains("auto_approve_signer = \"release-bot\"")
            && policy.contains(&format!("trusted_ed25519 = \"{public}\"")),
        "expected onboarded policy contents, got:\n{}",
        policy
    );

    let show = Command::new(&rr_bin)
        .arg("registry")
        .arg("policy")
        .arg("show")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry policy show");
    assert!(show.status.success(), "rr registry policy show failed");
    let show_stdout = String::from_utf8_lossy(&show.stdout);
    assert!(
        show_stdout.contains("policy ")
            && show_stdout.contains("require_signed = true")
            && show_stdout.contains("auto_approve_signer = \"release-bot\""),
        "expected shown policy output, got:\n{}",
        show_stdout
    );

    let apply_file = unique_dir(&sandbox_root, "registry_policy_apply").join("custom-policy.toml");
    fs::create_dir_all(apply_file.parent().expect("policy parent"))
        .expect("failed to create policy parent");
    fs::write(
        &apply_file,
        format!(
            "version = 1\nrequire_signed = true\nrequire_approval = false\ntrusted_ed25519 = \"{}\"\nallowed_signer = \"ops-bot\"\nauto_approve_signer = \"ops-bot\"\n",
            public
        ),
    )
    .expect("failed to write custom policy");

    let apply = Command::new(&rr_bin)
        .arg("registry")
        .arg("policy")
        .arg("apply")
        .arg(&apply_file)
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry policy apply");
    assert!(
        apply.status.success(),
        "rr registry policy apply failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );

    let applied = fs::read_to_string(registry_dir.join("policy.toml"))
        .expect("failed to read applied policy");
    assert!(
        applied.contains("require_approval = false")
            && applied.contains("allowed_signer = \"ops-bot\"")
            && applied.contains("auto_approve_signer = \"ops-bot\""),
        "expected applied policy contents, got:\n{}",
        applied
    );
}

#[test]
fn registry_audit_filters_by_action_module_and_text() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_audit_filter_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/auditfilter")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let secret_hex = "a0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebf";
    let public_hex = ed25519_public_hex(secret_hex);
    let registry_dir = unique_dir(&sandbox_root, "registry_audit_filter_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    let bootstrap = Command::new(&rr_bin)
        .arg("registry")
        .arg("policy")
        .arg("bootstrap")
        .arg(&public_hex)
        .arg("--signer")
        .arg("release-bot")
        .arg("--require-signed")
        .arg("--require-approval")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry policy bootstrap");
    assert!(
        bootstrap.status.success(),
        "rr registry policy bootstrap failed"
    );

    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .env("RR_REGISTRY_SIGNING_ED25519_SECRET", secret_hex)
        .env("RR_REGISTRY_SIGNING_IDENTITY", "release-bot")
        .output()
        .expect("failed to publish audit-filter package");
    assert!(publish.status.success(), "publish failed");

    let approve = Command::new(&rr_bin)
        .arg("registry")
        .arg("approve")
        .arg("rr.local/auditfilter")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry approve");
    assert!(approve.status.success(), "approve failed");

    let filtered = Command::new(&rr_bin)
        .arg("registry")
        .arg("audit")
        .arg("--action")
        .arg("registry-index")
        .arg("--module")
        .arg("rr.local/auditfilter")
        .arg("--contains")
        .arg("approve")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry audit with filters");
    assert!(
        filtered.status.success(),
        "filtered audit failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&filtered.stdout),
        String::from_utf8_lossy(&filtered.stderr)
    );
    let filtered_stdout = String::from_utf8_lossy(&filtered.stdout);
    assert!(
        filtered_stdout.contains("\tregistry-index\tapprove rr.local/auditfilter v1.0.0")
            && !filtered_stdout.contains("\tpublish\t"),
        "expected filtered audit output, got:\n{}",
        filtered_stdout
    );
}

#[test]
fn registry_promote_switches_latest_release_target() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_promote_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/promote")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let registry_dir = unique_dir(&sandbox_root, "registry_promote_root");
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
  return x + 5L
}
"#,
        ),
    ] {
        fs::write(pkg_dir.join("src").join("lib.rr"), body)
            .expect("failed to write library source");
        let publish = Command::new(&rr_bin)
            .current_dir(&pkg_dir)
            .arg("publish")
            .arg(version)
            .arg("--registry")
            .arg(&registry_dir)
            .output()
            .expect("failed to publish promote package");
        assert!(publish.status.success(), "publish failed");
    }

    let promote = Command::new(&rr_bin)
        .arg("registry")
        .arg("promote")
        .arg("rr.local/promote")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry promote");
    assert!(
        promote.status.success(),
        "rr registry promote failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&promote.stdout),
        String::from_utf8_lossy(&promote.stderr)
    );

    let info = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/promote")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info.status.success(), "rr registry info failed");
    let info_stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        info_stdout.contains("release v1.0.0")
            && info_stdout.contains("release v1.1.0")
            && info_stdout.contains("release v1.0.0")
            && info_stdout.contains("yanked=false approved=true")
            && info_stdout.contains("release v1.1.0")
            && info_stdout.contains("yanked=false approved=false"),
        "expected promoted release state, got:\n{}",
        info_stdout
    );

    let app_dir = unique_dir(&sandbox_root, "registry_promote_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-promote-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_promote");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/promote@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to install promoted package");
    assert!(install.status.success(), "install failed");
    let manifest = fs::read_to_string(app_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("require rr.local/promote v1.0.0"),
        "expected promoted version in manifest, got:\n{}",
        manifest
    );
}

#[test]
fn registry_audit_export_writes_filtered_jsonl_file() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_audit_export_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/auditexport")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let registry_dir = unique_dir(&sandbox_root, "registry_audit_export_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish audit-export package");
    assert!(publish.status.success(), "publish failed");

    let export_path = unique_dir(&sandbox_root, "registry_audit_export_out").join("audit.jsonl");
    fs::create_dir_all(export_path.parent().expect("export parent"))
        .expect("failed to create export parent");
    let export = Command::new(&rr_bin)
        .arg("registry")
        .arg("audit")
        .arg("export")
        .arg(&export_path)
        .arg("--format")
        .arg("jsonl")
        .arg("--action")
        .arg("publish")
        .arg("--module")
        .arg("rr.local/auditexport")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry audit export");
    assert!(
        export.status.success(),
        "audit export failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&export.stdout),
        String::from_utf8_lossy(&export.stderr)
    );
    let exported = fs::read_to_string(&export_path).expect("failed to read exported audit");
    assert!(
        exported.contains("\"action\":\"publish\"")
            && exported.contains("rr.local/auditexport")
            && !exported.contains("\"action\":\"registry-index\""),
        "expected filtered jsonl export, got:\n{}",
        exported
    );
}

#[test]
fn registry_report_summarizes_module_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_report_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/reporting")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let registry_dir = unique_dir(&sandbox_root, "registry_report_root");
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
        fs::write(pkg_dir.join("src").join("lib.rr"), body)
            .expect("failed to write library source");
        let publish = Command::new(&rr_bin)
            .current_dir(&pkg_dir)
            .arg("publish")
            .arg(version)
            .arg("--registry")
            .arg(&registry_dir)
            .output()
            .expect("failed to publish report package");
        assert!(publish.status.success(), "publish failed");
    }

    let report = Command::new(&rr_bin)
        .arg("registry")
        .arg("report")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry report");
    assert!(report.status.success(), "rr registry report failed");
    let stdout = String::from_utf8_lossy(&report.stdout);
    assert!(
        stdout.contains("modules=1 channels=0 releases=2 approved=2 pending=0 yanked=0")
            && stdout.contains(
                "rr.local/reporting latest=v1.1.0 channels=0 releases=2 approved=2 pending=0"
            ),
        "expected registry report output, got:\n{}",
        stdout
    );
}
