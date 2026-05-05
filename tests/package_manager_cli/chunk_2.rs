use super::package_manager_cli_common::*;

#[test]
pub(crate) fn install_latest_respects_v2_module_path_suffix() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_v2");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    let repo_dir = acme_root.join("mathlib");
    fs::create_dir_all(&repo_dir).expect("failed to create repo dir");
    fs::create_dir_all(repo_dir.join("v2").join("src")).expect("failed to create v2 src");
    fs::write(
        repo_dir.join("v2").join("rr.mod"),
        "module github.com/acme/mathlib/v2\n\nrr 8.0\n",
    )
    .expect("failed to write v2 rr.mod");
    fs::write(
        repo_dir.join("v2").join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 2L
}
"#,
    )
    .expect("failed to write v2 lib.rr");

    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg("--initial-branch=main")
        .arg(&repo_dir)
        .status()
        .expect("failed to init git repo");
    assert!(status.success(), "git init failed");
    let git_commands: &[&[&str]] = &[
        &["config", "user.email", "rr-tests@example.com"],
        &["config", "user.name", "RR Tests"],
        &["add", "."],
        &["commit", "-q", "-m", "initial"],
    ];
    for args in git_commands {
        let status = Command::new("git")
            .current_dir(&repo_dir)
            .args(*args)
            .status()
            .expect("failed to run git command");
        assert!(status.success(), "git command failed");
    }
    for tag in ["v1.9.0", "v2.1.0"] {
        let status = Command::new("git")
            .current_dir(&repo_dir)
            .args(["tag", tag])
            .status()
            .expect("failed to git tag");
        assert!(status.success(), "git tag failed");
    }

    let proj_dir = unique_dir(&sandbox_root, "v2_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/v2-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_v2");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");

    let mut install = Command::new(&rr_bin);
    install
        .current_dir(&proj_dir)
        .arg("install")
        .arg("https://github.com/acme/mathlib/tree/main/v2@latest");
    configure_github_mapping(&mut install, &github_root, &pkg_home);
    let install_output = install.output().expect("failed to run rr install");
    assert!(
        install_output.status.success(),
        "rr install failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install_output.stdout),
        String::from_utf8_lossy(&install_output.stderr)
    );

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("require github.com/acme/mathlib/v2 v2.1.0"),
        "expected latest install to select v2 tag, got:\n{}",
        manifest
    );
}

#[test]
pub(crate) fn build_fails_when_cached_module_checksum_does_not_match_lock() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_checksum");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    init_git_repo(
        &acme_root.join("mathlib"),
        &[
            ("rr.mod", "module github.com/acme/mathlib\n\nrr 8.0\n"),
            (
                "src/lib.rr",
                r#"
fn add_one(x) {
  return x + 1L
}
"#,
            ),
        ],
        "v1.2.3",
    );

    let proj_dir = unique_dir(&sandbox_root, "checksum_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/checksum-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
import "github.com/acme/mathlib"

fn main() {
  return add_one(40L)
}
main()
"#,
    )
    .expect("failed to write src/main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_checksum");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");

    let mut install = Command::new(&rr_bin);
    install
        .current_dir(&proj_dir)
        .arg("install")
        .arg("https://github.com/acme/mathlib@latest");
    configure_github_mapping(&mut install, &github_root, &pkg_home);
    let install_output = install.output().expect("failed to run rr install");
    assert!(
        install_output.status.success(),
        "rr install failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install_output.stdout),
        String::from_utf8_lossy(&install_output.stderr)
    );

    let cached_file = pkg_home
        .join("pkg")
        .join("mod")
        .join("github.com")
        .join("acme")
        .join("mathlib@v1.2.3")
        .join("src")
        .join("lib.rr");
    fs::write(
        &cached_file,
        r#"
fn add_one(x) {
  return x + 99L
}
"#,
    )
    .expect("failed to tamper cached module");

    let build_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("build")
        .arg(".")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr build");
    assert!(
        !build_output.status.success(),
        "rr build should fail on checksum mismatch"
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );
    assert!(
        combined.contains("checksum mismatch for module 'github.com/acme/mathlib'"),
        "expected checksum mismatch diagnostic, got:\n{}",
        combined
    );
}

#[test]
pub(crate) fn mvs_prefers_highest_transitive_version() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_mvs");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    let base_repo = acme_root.join("baseutil");
    fs::create_dir_all(base_repo.join("src")).expect("failed to create baseutil src");
    fs::write(
        base_repo.join("rr.mod"),
        "module github.com/acme/baseutil\n\nrr 8.0\n",
    )
    .expect("failed to write baseutil rr.mod");
    fs::write(
        base_repo.join("src").join("lib.rr"),
        r#"
fn add_ver(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write baseutil v0.1");
    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg("--initial-branch=main")
        .arg(&base_repo)
        .status()
        .expect("failed to init baseutil repo");
    assert!(status.success(), "git init failed");
    let git_commands: &[&[&str]] = &[
        &["config", "user.email", "rr-tests@example.com"],
        &["config", "user.name", "RR Tests"],
        &["add", "."],
        &["commit", "-q", "-m", "v0.1.0"],
    ];
    for args in git_commands {
        let status = Command::new("git")
            .current_dir(&base_repo)
            .args(*args)
            .status()
            .expect("failed to run git command");
        assert!(status.success(), "git command failed");
    }
    let status = Command::new("git")
        .current_dir(&base_repo)
        .args(["tag", "v0.1.0"])
        .status()
        .expect("failed to tag v0.1.0");
    assert!(status.success(), "git tag failed");
    fs::write(
        base_repo.join("src").join("lib.rr"),
        r#"
fn add_ver(x) {
  return x + 2L
}
"#,
    )
    .expect("failed to write baseutil v0.2");
    let status = Command::new("git")
        .current_dir(&base_repo)
        .args(["add", "."])
        .status()
        .expect("failed to git add update");
    assert!(status.success(), "git add failed");
    let status = Command::new("git")
        .current_dir(&base_repo)
        .args(["commit", "-q", "-m", "v0.2.0"])
        .status()
        .expect("failed to git commit update");
    assert!(status.success(), "git commit failed");
    let status = Command::new("git")
        .current_dir(&base_repo)
        .args(["tag", "v0.2.0"])
        .status()
        .expect("failed to tag v0.2.0");
    assert!(status.success(), "git tag failed");

    init_git_repo(
        &acme_root.join("liba"),
        &[
            (
                "rr.mod",
                "module github.com/acme/liba\n\nrr 8.0\n\nrequire github.com/acme/baseutil v0.1.0\n",
            ),
            (
                "src/lib.rr",
                r#"
import "github.com/acme/baseutil"

fn a(x) {
  return add_ver(x)
}
"#,
            ),
        ],
        "v1.0.0",
    );

    init_git_repo(
        &acme_root.join("libb"),
        &[
            (
                "rr.mod",
                "module github.com/acme/libb\n\nrr 8.0\n\nrequire github.com/acme/baseutil v0.2.0\n",
            ),
            (
                "src/lib.rr",
                r#"
import "github.com/acme/baseutil"

fn b(x) {
  return add_ver(x)
}
"#,
            ),
        ],
        "v1.0.0",
    );

    let proj_dir = unique_dir(&sandbox_root, "mvs_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/mvs-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
import "github.com/acme/liba"
import "github.com/acme/libb"

fn main() {
  return a(40L) + b(40L)
}
main()
"#,
    )
    .expect("failed to write src/main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_mvs");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");

    for dep in [
        "https://github.com/acme/liba@latest",
        "https://github.com/acme/libb@latest",
    ] {
        let mut install = Command::new(&rr_bin);
        install.current_dir(&proj_dir).arg("install").arg(dep);
        configure_github_mapping(&mut install, &github_root, &pkg_home);
        let output = install.output().expect("failed to run rr install");
        assert!(
            output.status.success(),
            "rr install failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let lock = fs::read_to_string(proj_dir.join("rr.lock")).expect("failed to read rr.lock");
    assert!(
        lock.contains("path = \"github.com/acme/baseutil\"")
            && lock.contains("version = \"v0.2.0\""),
        "expected lock to prefer highest transitive version, got:\n{}",
        lock
    );

    let build_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("build")
        .arg(".")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr build");
    assert!(
        build_output.status.success(),
        "rr build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );
    let built_r = fs::read_to_string(
        proj_dir
            .join("Build")
            .join("debug")
            .join("src")
            .join("main.R"),
    )
    .expect("failed to read built main.R");
    assert!(
        (built_r.contains("40L + 2L")
            || built_r.contains("x + 2L")
            || built_r.contains("return(84L)"))
            && !built_r.contains("40L + 1L")
            && !built_r.contains("x + 1L"),
        "expected highest transitive version logic in built artifact, got:\n{}",
        built_r
    );
}

#[test]
pub(crate) fn mod_graph_and_mod_why_report_dependency_chain() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_graph");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    init_git_repo(
        &acme_root.join("baseutil"),
        &[
            ("rr.mod", "module github.com/acme/baseutil\n\nrr 8.0\n"),
            (
                "src/lib.rr",
                r#"
fn plus_one(x) {
  return x + 1L
}
"#,
            ),
        ],
        "v0.1.0",
    );

    init_git_repo(
        &acme_root.join("mathlib"),
        &[
            (
                "rr.mod",
                "module github.com/acme/mathlib\n\nrr 8.0\n\nrequire github.com/acme/baseutil v0.1.0\n",
            ),
            (
                "src/lib.rr",
                r#"
import "github.com/acme/baseutil"

fn add_one(x) {
  return plus_one(x)
}
"#,
            ),
        ],
        "v1.2.3",
    );

    let proj_dir = unique_dir(&sandbox_root, "graph_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/graph-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_graph");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let mut install = Command::new(&rr_bin);
    install
        .current_dir(&proj_dir)
        .arg("install")
        .arg("https://github.com/acme/mathlib@latest");
    configure_github_mapping(&mut install, &github_root, &pkg_home);
    let output = install.output().expect("failed to run rr install");
    assert!(output.status.success(), "rr install failed");

    let graph_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("mod")
        .arg("graph")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr mod graph");
    assert!(graph_output.status.success(), "rr mod graph failed");
    let graph_stdout = String::from_utf8_lossy(&graph_output.stdout);
    assert!(
        graph_stdout.contains("github.com/example/graph-app github.com/acme/mathlib@v1.2.3")
            && graph_stdout
                .contains("github.com/acme/mathlib@v1.2.3 github.com/acme/baseutil@v0.1.0"),
        "expected graph edges, got:\n{}",
        graph_stdout
    );

    let why_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("mod")
        .arg("why")
        .arg("github.com/acme/baseutil")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr mod why");
    assert!(why_output.status.success(), "rr mod why failed");
    let why_stdout = String::from_utf8_lossy(&why_output.stdout);
    assert!(
        why_stdout.contains("github.com/example/graph-app")
            && why_stdout.contains("-> github.com/acme/mathlib@v1.2.3")
            && why_stdout.contains("-> github.com/acme/baseutil@v0.1.0"),
        "expected why chain, got:\n{}",
        why_stdout
    );
}

#[test]
pub(crate) fn mod_verify_reports_checksum_mismatch() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_verify");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    init_git_repo(
        &acme_root.join("mathlib"),
        &[
            ("rr.mod", "module github.com/acme/mathlib\n\nrr 8.0\n"),
            (
                "src/lib.rr",
                r#"
fn add_one(x) {
  return x + 1L
}
"#,
            ),
        ],
        "v1.2.3",
    );

    let proj_dir = unique_dir(&sandbox_root, "verify_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/verify-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_verify");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let mut install = Command::new(&rr_bin);
    install
        .current_dir(&proj_dir)
        .arg("install")
        .arg("https://github.com/acme/mathlib@latest");
    configure_github_mapping(&mut install, &github_root, &pkg_home);
    let output = install.output().expect("failed to run rr install");
    assert!(output.status.success(), "rr install failed");

    fs::write(
        pkg_home
            .join("pkg")
            .join("mod")
            .join("github.com")
            .join("acme")
            .join("mathlib@v1.2.3")
            .join("src")
            .join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 9L
}
"#,
    )
    .expect("failed to tamper cached module");

    let verify_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("mod")
        .arg("verify")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr mod verify");
    assert!(
        !verify_output.status.success(),
        "rr mod verify should fail on checksum mismatch"
    );
    let stderr = String::from_utf8_lossy(&verify_output.stderr);
    assert!(
        stderr.contains("github.com/acme/mathlib"),
        "expected verify mismatch output, got:\n{}",
        stderr
    );
}

#[test]
pub(crate) fn outdated_and_update_refresh_direct_dependency_version() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_update");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    let repo_dir = acme_root.join("mathlib");
    fs::create_dir_all(repo_dir.join("src")).expect("failed to create repo src");
    fs::write(
        repo_dir.join("rr.mod"),
        "module github.com/acme/mathlib\n\nrr 8.0\n",
    )
    .expect("failed to write rr.mod");
    fs::write(
        repo_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write lib.rr");
    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg("--initial-branch=main")
        .arg(&repo_dir)
        .status()
        .expect("failed to init git repo");
    assert!(status.success(), "git init failed");
    let git_commands: &[&[&str]] = &[
        &["config", "user.email", "rr-tests@example.com"],
        &["config", "user.name", "RR Tests"],
        &["add", "."],
        &["commit", "-q", "-m", "v1.0.0"],
    ];
    for args in git_commands {
        let status = Command::new("git")
            .current_dir(&repo_dir)
            .args(*args)
            .status()
            .expect("failed to run git command");
        assert!(status.success(), "git command failed");
    }
    let status = Command::new("git")
        .current_dir(&repo_dir)
        .args(["tag", "v1.0.0"])
        .status()
        .expect("failed to tag v1.0.0");
    assert!(status.success(), "git tag failed");
    fs::write(
        repo_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 2L
}
"#,
    )
    .expect("failed to update lib.rr");
    let status = Command::new("git")
        .current_dir(&repo_dir)
        .args(["add", "."])
        .status()
        .expect("failed to git add");
    assert!(status.success(), "git add failed");
    let status = Command::new("git")
        .current_dir(&repo_dir)
        .args(["commit", "-q", "-m", "v1.1.0"])
        .status()
        .expect("failed to git commit");
    assert!(status.success(), "git commit failed");
    let status = Command::new("git")
        .current_dir(&repo_dir)
        .args(["tag", "v1.1.0"])
        .status()
        .expect("failed to tag v1.1.0");
    assert!(status.success(), "git tag failed");

    let proj_dir = unique_dir(&sandbox_root, "update_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/update-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_update");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let mut install = Command::new(&rr_bin);
    install
        .current_dir(&proj_dir)
        .arg("install")
        .arg("https://github.com/acme/mathlib@v1.0.0");
    configure_github_mapping(&mut install, &github_root, &pkg_home);
    let output = install.output().expect("failed to run rr install");
    assert!(output.status.success(), "rr install failed");

    let mut outdated = Command::new(&rr_bin);
    outdated.current_dir(&proj_dir).arg("outdated");
    configure_github_mapping(&mut outdated, &github_root, &pkg_home);
    let outdated_output = outdated.output().expect("failed to run rr outdated");
    assert!(outdated_output.status.success(), "rr outdated failed");
    let outdated_stdout = String::from_utf8_lossy(&outdated_output.stdout);
    assert!(
        outdated_stdout.contains("github.com/acme/mathlib")
            && outdated_stdout.contains("current=v1.0.0")
            && outdated_stdout.contains("latest=v1.1.0")
            && outdated_stdout.contains("status=outdated"),
        "expected outdated report, got:\n{}",
        outdated_stdout
    );

    let mut update = Command::new(&rr_bin);
    update.current_dir(&proj_dir).arg("update");
    configure_github_mapping(&mut update, &github_root, &pkg_home);
    let update_output = update.output().expect("failed to run rr update");
    assert!(update_output.status.success(), "rr update failed");

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("require github.com/acme/mathlib v1.1.0"),
        "expected updated version in rr.mod, got:\n{}",
        manifest
    );
}

#[test]
pub(crate) fn install_accepts_ssh_style_github_source() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_ssh");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    init_git_repo(
        &acme_root.join("mathlib"),
        &[
            ("rr.mod", "module github.com/acme/mathlib\n\nrr 8.0\n"),
            (
                "src/lib.rr",
                r#"
fn add_one(x) {
  return x + 1L
}
"#,
            ),
        ],
        "v1.2.3",
    );

    let proj_dir = unique_dir(&sandbox_root, "ssh_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/ssh-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_ssh");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");

    let github_root = fs::canonicalize(&github_root).expect("failed to canonicalize github root");
    let mut base = github_root.to_string_lossy().to_string();
    if !base.ends_with('/') {
        base.push('/');
    }
    let file_base = format!("file://{}", base);
    let output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("install")
        .arg("git@github.com:acme/mathlib.git@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", format!("url.{}.insteadOf", file_base))
        .env("GIT_CONFIG_VALUE_0", "git@github.com:")
        .env("GIT_ALLOW_PROTOCOL", "file:ssh")
        .output()
        .expect("failed to run rr install via ssh source");
    assert!(
        output.status.success(),
        "rr install via ssh source failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("require github.com/acme/mathlib v1.2.3"),
        "expected installed dependency in rr.mod, got:\n{}",
        manifest
    );
}

#[test]
pub(crate) fn publish_creates_tarball_with_project_sources() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let proj_dir = unique_dir(&sandbox_root, "publish_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/publish-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    fs::write(proj_dir.join("README.md"), "# publish-app\n").expect("failed to write README.md");

    let dry_run = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--dry-run")
        .output()
        .expect("failed to run rr publish --dry-run");
    assert!(dry_run.status.success(), "rr publish --dry-run failed");

    let publish = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("publish")
        .arg("v1.0.0")
        .output()
        .expect("failed to run rr publish");
    assert!(publish.status.success(), "rr publish failed");

    let archive = proj_dir
        .join("Build")
        .join("publish")
        .join("publish-app@v1.0.0.tar.gz");
    assert!(archive.is_file(), "expected publish archive");

    let list_output = Command::new("tar")
        .arg("-tzf")
        .arg(&archive)
        .output()
        .expect("failed to list tarball");
    assert!(list_output.status.success(), "tar -tzf failed");
    let listing = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        listing.contains("rr.mod")
            && listing.contains("src/main.rr")
            && listing.contains("README.md"),
        "expected published files in tarball, got:\n{}",
        listing
    );
}
