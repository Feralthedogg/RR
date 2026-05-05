use super::package_manager_cli_common::*;

#[test]
pub(crate) fn registry_diff_reports_changed_and_added_files() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_diff_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/diffing")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let registry_dir = unique_dir(&sandbox_root, "registry_diff_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write first library source");
    let publish_v1 = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish v1.0.0");
    assert!(publish_v1.status.success(), "publish v1.0.0 failed");

    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 9L
}
"#,
    )
    .expect("failed to write second library source");
    fs::write(pkg_dir.join("README.md"), "# diffing\n").expect("failed to write README");
    let publish_v2 = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish v1.1.0");
    assert!(publish_v2.status.success(), "publish v1.1.0 failed");

    let diff = Command::new(&rr_bin)
        .arg("registry")
        .arg("diff")
        .arg("rr.local/diffing")
        .arg("v1.0.0")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry diff");
    assert!(diff.status.success(), "rr registry diff failed");
    let stdout = String::from_utf8_lossy(&diff.stdout);
    assert!(
        stdout.contains("module=rr.local/diffing from=v1.0.0 to=v1.1.0")
            && stdout.contains("files added=1 removed=0 changed=1")
            && stdout.contains("+ README.md")
            && stdout.contains("~ src/lib.rr"),
        "expected registry diff output, got:\n{}",
        stdout
    );
}

#[test]
pub(crate) fn registry_channels_can_be_assigned_and_used_for_install() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_channel_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/channels")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let registry_dir = unique_dir(&sandbox_root, "registry_channel_root");
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
            .expect("failed to publish channel package");
        assert!(publish.status.success(), "publish failed");
    }

    for args in [
        vec!["channel", "set", "rr.local/channels", "stable", "v1.0.0"],
        vec!["channel", "set", "rr.local/channels", "canary", "v1.1.0"],
    ] {
        let output = Command::new(&rr_bin)
            .arg("registry")
            .args(&args)
            .arg("--registry")
            .arg(&registry_dir)
            .output()
            .expect("failed to set channel");
        assert!(output.status.success(), "set channel failed");
    }

    let show = Command::new(&rr_bin)
        .arg("registry")
        .arg("channel")
        .arg("show")
        .arg("rr.local/channels")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to show channels");
    assert!(show.status.success(), "channel show failed");
    let show_stdout = String::from_utf8_lossy(&show.stdout);
    assert!(
        show_stdout.contains("stable v1.0.0") && show_stdout.contains("canary v1.1.0"),
        "expected channel assignments, got:\n{}",
        show_stdout
    );

    let report = Command::new(&rr_bin)
        .arg("registry")
        .arg("report")
        .arg("rr.local/channels")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run registry report");
    assert!(report.status.success(), "registry report failed");
    let report_stdout = String::from_utf8_lossy(&report.stdout);
    assert!(
        report_stdout.contains("modules=1 channels=2")
            && report_stdout.contains("rr.local/channels latest=v1.1.0 channels=2"),
        "expected channel counts in report, got:\n{}",
        report_stdout
    );

    let stable_app = unique_dir(&sandbox_root, "registry_channel_stable_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/channel-stable-app")
        .arg(&stable_app)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_channel");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let stable_install = Command::new(&rr_bin)
        .current_dir(&stable_app)
        .arg("install")
        .arg("rr.local/channels@stable")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to install stable channel");
    assert!(stable_install.status.success(), "stable install failed");
    let stable_manifest =
        fs::read_to_string(stable_app.join("rr.mod")).expect("failed to read stable rr.mod");
    assert!(
        stable_manifest.contains("require rr.local/channels v1.0.0"),
        "expected stable channel version, got:\n{}",
        stable_manifest
    );

    let canary_app = unique_dir(&sandbox_root, "registry_channel_canary_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/channel-canary-app")
        .arg(&canary_app)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    let canary_install = Command::new(&rr_bin)
        .current_dir(&canary_app)
        .arg("install")
        .arg("rr.local/channels@canary")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to install canary channel");
    assert!(canary_install.status.success(), "canary install failed");
    let canary_manifest =
        fs::read_to_string(canary_app.join("rr.mod")).expect("failed to read canary rr.mod");
    assert!(
        canary_manifest.contains("require rr.local/channels v1.1.0"),
        "expected canary channel version, got:\n{}",
        canary_manifest
    );
}

#[test]
pub(crate) fn registry_risk_reports_metadata_and_diff_based_factors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_risk_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/risking")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    let registry_dir = unique_dir(&sandbox_root, "registry_risk_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write first library source");
    let publish_v1 = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish v1.0.0");
    assert!(publish_v1.status.success(), "publish v1.0.0 failed");

    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 9L
}
"#,
    )
    .expect("failed to write second library source");
    fs::write(pkg_dir.join("README.md"), "# risking\n").expect("failed to write README");
    let publish_v2 = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish v1.1.0");
    assert!(publish_v2.status.success(), "publish v1.1.0 failed");

    let deprecate = Command::new(&rr_bin)
        .arg("registry")
        .arg("deprecate")
        .arg("rr.local/risking")
        .arg("use rr.local/newrisk")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry deprecate");
    assert!(deprecate.status.success(), "rr registry deprecate failed");

    let yank = Command::new(&rr_bin)
        .arg("registry")
        .arg("yank")
        .arg("rr.local/risking")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry yank");
    assert!(yank.status.success(), "rr registry yank failed");

    let risk = Command::new(&rr_bin)
        .arg("registry")
        .arg("risk")
        .arg("rr.local/risking")
        .arg("v1.1.0")
        .arg("--against")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry risk");
    assert!(risk.status.success(), "rr registry risk failed");
    let stdout = String::from_utf8_lossy(&risk.stdout);
    assert!(
        stdout.contains("module=rr.local/risking version=v1.1.0 baseline=v1.0.0")
            && stdout.contains("level=")
            && stdout.contains("factor yanked")
            && stdout.contains("factor unsigned")
            && stdout.contains("factor deprecated-module")
            && stdout.contains("factor")
            && stdout.contains("files differ from v1.0.0"),
        "expected risk report output, got:\n{}",
        stdout
    );
}
