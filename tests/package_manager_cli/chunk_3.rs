use super::package_manager_cli_common::*;

#[test]
fn publish_with_push_tag_pushes_git_tag_to_remote() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let proj_dir = unique_dir(&sandbox_root, "publish_tag_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/publish-tag-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let remote_dir = unique_dir(&sandbox_root, "publish_remote");
    let status = Command::new("git")
        .args(["init", "--bare", &remote_dir.to_string_lossy()])
        .status()
        .expect("failed to init bare remote");
    assert!(status.success(), "git init --bare failed");

    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg("--initial-branch=main")
        .arg(&proj_dir)
        .status()
        .expect("failed to init project repo");
    assert!(status.success(), "git init failed");
    for args in [
        ["config", "user.email", "rr-tests@example.com"],
        ["config", "user.name", "RR Tests"],
    ] {
        let status = Command::new("git")
            .current_dir(&proj_dir)
            .args(args)
            .status()
            .expect("failed to configure repo");
        assert!(status.success(), "git config failed");
    }
    let status = Command::new("git")
        .current_dir(&proj_dir)
        .args(["remote", "add", "origin", &remote_dir.to_string_lossy()])
        .status()
        .expect("failed to add remote");
    assert!(status.success(), "git remote add failed");
    let status = Command::new("git")
        .current_dir(&proj_dir)
        .args(["add", "."])
        .status()
        .expect("failed to git add");
    assert!(status.success(), "git add failed");
    let status = Command::new("git")
        .current_dir(&proj_dir)
        .args(["commit", "-q", "-m", "initial"])
        .status()
        .expect("failed to git commit");
    assert!(status.success(), "git commit failed");

    let output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--push-tag")
        .arg("--remote")
        .arg("origin")
        .output()
        .expect("failed to run rr publish --push-tag");
    assert!(
        output.status.success(),
        "rr publish --push-tag failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let show_ref = Command::new("git")
        .current_dir(&remote_dir)
        .args(["show-ref", "--tags", "v1.0.0"])
        .output()
        .expect("failed to show remote tag");
    assert!(
        show_ref.status.success(),
        "expected pushed tag in bare remote:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&show_ref.stdout),
        String::from_utf8_lossy(&show_ref.stderr)
    );
}

#[test]
fn publish_to_local_registry_allows_registry_backed_install() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let pkg_dir = unique_dir(&sandbox_root, "registry_pkg");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/mathlib")
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

    let registry_dir = unique_dir(&sandbox_root, "registry_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr publish --registry");
    assert!(
        publish.status.success(),
        "rr publish --registry failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let index_path = registry_dir
        .join("index")
        .join("rr.local")
        .join("mathlib.toml");
    assert!(index_path.is_file(), "expected registry index file");

    let app_dir = unique_dir(&sandbox_root, "registry_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    fs::write(
        app_dir.join("src").join("main.rr"),
        r#"
import "rr.local/mathlib"

fn main() {
  return add_one(40L)
}
"#,
    )
    .expect("failed to write app main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");

    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/mathlib@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to run rr install from registry");
    assert!(
        install.status.success(),
        "rr install from registry failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );

    let build = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("build")
        .arg(".")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to run rr build");
    assert!(
        build.status.success(),
        "rr build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let built_r = fs::read_to_string(
        app_dir
            .join("Build")
            .join("debug")
            .join("src")
            .join("main.R"),
    )
    .expect("failed to read built main.R");
    assert!(
        built_r.contains("40L + 1L") || built_r.contains("x + 1L"),
        "expected registry dependency logic in built artifact, got:\n{}",
        built_r
    );
}

#[test]
fn publish_to_remote_registry_git_repo_allows_registry_install() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let pkg_dir = unique_dir(&sandbox_root, "remote_registry_pkg");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.remote/mathlib")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 3L
}
"#,
    )
    .expect("failed to write library source");

    let registry_remote = unique_dir(&sandbox_root, "registry_remote");
    let status = Command::new("git")
        .args(["init", "--bare", &registry_remote.to_string_lossy()])
        .status()
        .expect("failed to init bare registry");
    assert!(status.success(), "git init --bare failed");

    let registry_url = format!("file://{}", registry_remote.to_string_lossy());
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_url)
        .output()
        .expect("failed to run rr publish --registry remote");
    assert!(
        publish.status.success(),
        "rr publish remote registry failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let app_dir = unique_dir(&sandbox_root, "remote_registry_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/remote-registry-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    fs::write(
        app_dir.join("src").join("main.rr"),
        r#"
import "rr.remote/mathlib"

fn main() {
  return add_one(40L)
}
"#,
    )
    .expect("failed to write app main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_remote_registry");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.remote/mathlib@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_url)
        .output()
        .expect("failed to run rr install from remote registry");
    assert!(
        install.status.success(),
        "rr install from remote registry failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );
}

#[test]
fn workspace_member_import_resolves_without_install() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let ws_root = unique_dir(&sandbox_root, "workspace");
    fs::create_dir_all(&ws_root).expect("failed to create workspace root");
    fs::write(
        ws_root.join("rr.work"),
        "use (\n    ./app\n    ./mathlib\n)\n",
    )
    .expect("failed to write rr.work");

    let app_dir = ws_root.join("app");
    let math_dir = ws_root.join("mathlib");
    fs::create_dir_all(app_dir.join("src")).expect("failed to create app src");
    fs::create_dir_all(math_dir.join("src")).expect("failed to create math src");

    fs::write(
        app_dir.join("rr.mod"),
        "module github.com/example/workspace-app\n\nrr 8.0\n",
    )
    .expect("failed to write app rr.mod");
    fs::write(
        app_dir.join("src").join("main.rr"),
        r#"
import "github.com/example/mathlib"

fn main() {
  return add_one(40L)
}
"#,
    )
    .expect("failed to write app main.rr");

    fs::write(
        math_dir.join("rr.mod"),
        "module github.com/example/mathlib\n\nrr 8.0\n",
    )
    .expect("failed to write math rr.mod");
    fs::write(
        math_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write math lib.rr");

    let rr_bin = rr_bin();
    let build_output = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("build")
        .arg(".")
        .output()
        .expect("failed to run rr build in workspace");
    assert!(
        build_output.status.success(),
        "rr build failed in workspace:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );
    let built_r = fs::read_to_string(
        app_dir
            .join("Build")
            .join("debug")
            .join("src")
            .join("main.R"),
    )
    .expect("failed to read built main.R");
    assert!(
        built_r.contains("40L + 1L") || built_r.contains("x + 1L"),
        "expected workspace member logic in built artifact, got:\n{}",
        built_r
    );
}

#[test]
fn mod_tidy_ignores_workspace_member_imports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let ws_root = unique_dir(&sandbox_root, "workspace_tidy");
    fs::create_dir_all(&ws_root).expect("failed to create workspace root");
    fs::write(
        ws_root.join("rr.work"),
        "use (\n    ./app\n    ./mathlib\n)\n",
    )
    .expect("failed to write rr.work");

    let app_dir = ws_root.join("app");
    let math_dir = ws_root.join("mathlib");
    fs::create_dir_all(app_dir.join("src")).expect("failed to create app src");
    fs::create_dir_all(math_dir.join("src")).expect("failed to create math src");

    fs::write(
        app_dir.join("rr.mod"),
        "module github.com/example/workspace-app\n\nrr 8.0\n",
    )
    .expect("failed to write app rr.mod");
    fs::write(
        app_dir.join("src").join("main.rr"),
        r#"
import "github.com/example/mathlib"

fn main() {
  return add_one(40L)
}
"#,
    )
    .expect("failed to write app main.rr");

    fs::write(
        math_dir.join("rr.mod"),
        "module github.com/example/mathlib\n\nrr 8.0\n",
    )
    .expect("failed to write math rr.mod");
    fs::write(
        math_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write math lib.rr");

    let rr_bin = rr_bin();
    let tidy_output = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("mod")
        .arg("tidy")
        .output()
        .expect("failed to run rr mod tidy in workspace");
    assert!(
        tidy_output.status.success(),
        "rr mod tidy failed in workspace:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&tidy_output.stdout),
        String::from_utf8_lossy(&tidy_output.stderr)
    );

    let manifest = fs::read_to_string(app_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        !manifest.contains("github.com/example/mathlib"),
        "workspace member import should not become an external require, got:\n{}",
        manifest
    );
}

#[test]
fn registry_search_and_info_surface_metadata() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_metadata_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/catalog")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    fs::write(
        pkg_dir.join("rr.mod"),
        r#"module rr.local/catalog

rr 8.0
description = "Catalog utilities"
license = "MIT"
homepage = "https://example.com/catalog"
"#,
    )
    .expect("failed to write rr.mod");
    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write library source");

    let registry_dir = unique_dir(&sandbox_root, "registry_metadata_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");

    let publish_v1 = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish v1.0.0");
    assert!(
        publish_v1.status.success(),
        "publish v1.0.0 failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish_v1.stdout),
        String::from_utf8_lossy(&publish_v1.stderr)
    );

    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 2L
}
"#,
    )
    .expect("failed to update library source");

    let publish_v2 = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish v1.1.0");
    assert!(
        publish_v2.status.success(),
        "publish v1.1.0 failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish_v2.stdout),
        String::from_utf8_lossy(&publish_v2.stderr)
    );

    let search = Command::new(&rr_bin)
        .arg("search")
        .arg("catalog")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr search");
    assert!(search.status.success(), "rr search failed");
    let search_stdout = String::from_utf8_lossy(&search.stdout);
    assert!(
        search_stdout.contains("rr.local/catalog")
            && search_stdout.contains("latest=v1.1.0")
            && search_stdout.contains("license=MIT")
            && search_stdout.contains("desc=Catalog utilities"),
        "expected registry metadata in search output, got:\n{}",
        search_stdout
    );

    let info = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/catalog")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info.status.success(), "rr registry info failed");
    let info_stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        info_stdout.contains("module rr.local/catalog")
            && info_stdout.contains("description Catalog utilities")
            && info_stdout.contains("license MIT")
            && info_stdout.contains("homepage https://example.com/catalog")
            && info_stdout.contains("release v1.0.0")
            && info_stdout.contains("release v1.1.0"),
        "expected registry info output, got:\n{}",
        info_stdout
    );
}

#[test]
fn registry_yank_and_deprecate_update_latest_resolution() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_yank_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/yanklib")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    fs::write(
        pkg_dir.join("rr.mod"),
        r#"module rr.local/yanklib

rr 8.0
description = "Yankable registry package"
license = "Apache-2.0"
"#,
    )
    .expect("failed to write rr.mod");
    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write library source");

    let registry_dir = unique_dir(&sandbox_root, "registry_yank_root");
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
        fs::write(pkg_dir.join("src").join("lib.rr"), body).expect("failed to write lib source");
        let output = Command::new(&rr_bin)
            .current_dir(&pkg_dir)
            .arg("publish")
            .arg(version)
            .arg("--registry")
            .arg(&registry_dir)
            .output()
            .expect("failed to publish release");
        assert!(
            output.status.success(),
            "publish {} failed:\nstdout:\n{}\nstderr:\n{}",
            version,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let yank = Command::new(&rr_bin)
        .arg("registry")
        .arg("yank")
        .arg("rr.local/yanklib")
        .arg("v1.1.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry yank");
    assert!(yank.status.success(), "rr registry yank failed");

    let deprecate = Command::new(&rr_bin)
        .arg("registry")
        .arg("deprecate")
        .arg("rr.local/yanklib")
        .arg("use rr.local/newlib instead")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry deprecate");
    assert!(deprecate.status.success(), "rr registry deprecate failed");

    let search = Command::new(&rr_bin)
        .arg("search")
        .arg("yanklib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr search");
    assert!(search.status.success(), "rr search failed");
    let search_stdout = String::from_utf8_lossy(&search.stdout);
    assert!(
        search_stdout.contains("latest=v1.0.0")
            && search_stdout.contains("yanked=1")
            && search_stdout.contains("deprecated=use rr.local/newlib instead"),
        "expected yanked/deprecated state in search output, got:\n{}",
        search_stdout
    );

    let info = Command::new(&rr_bin)
        .arg("registry")
        .arg("info")
        .arg("rr.local/yanklib")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry info");
    assert!(info.status.success(), "rr registry info failed");
    let info_stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        info_stdout.contains("deprecated use rr.local/newlib instead")
            && info_stdout.contains("release v1.1.0")
            && info_stdout.contains("yanked=true"),
        "expected yanked release in registry info output, got:\n{}",
        info_stdout
    );

    let app_dir = unique_dir(&sandbox_root, "registry_yank_app");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/registry-yank-app")
        .arg(&app_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");
    fs::write(
        app_dir.join("src").join("main.rr"),
        r#"
import "rr.local/yanklib"

fn main() {
  return add_one(40L)
}
"#,
    )
    .expect("failed to write app main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_registry_yank");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");
    let install = Command::new(&rr_bin)
        .current_dir(&app_dir)
        .arg("install")
        .arg("rr.local/yanklib@latest")
        .env("RRPKGHOME", &pkg_home)
        .env("RR_REGISTRY_DIR", &registry_dir)
        .output()
        .expect("failed to install yanked registry package");
    assert!(
        install.status.success(),
        "rr install after yank failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );

    let manifest = fs::read_to_string(app_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("require rr.local/yanklib v1.0.0"),
        "expected latest install to skip yanked release, got:\n{}",
        manifest
    );
}

#[test]
fn registry_list_and_verify_report_clean_registry() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let rr_bin = rr_bin();
    let pkg_dir = unique_dir(&sandbox_root, "registry_verify_clean_pkg");
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("--lib")
        .arg("rr.local/verifyclean")
        .arg(&pkg_dir)
        .status()
        .expect("failed to run rr new --lib");
    assert!(status.success(), "rr new --lib failed");

    fs::write(
        pkg_dir.join("rr.mod"),
        r#"module rr.local/verifyclean

rr 8.0
description = "Registry verification package"
license = "MIT"
"#,
    )
    .expect("failed to write rr.mod");
    fs::write(
        pkg_dir.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write library source");

    let registry_dir = unique_dir(&sandbox_root, "registry_verify_clean_root");
    fs::create_dir_all(&registry_dir).expect("failed to create registry dir");
    let publish = Command::new(&rr_bin)
        .current_dir(&pkg_dir)
        .arg("publish")
        .arg("v1.0.0")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to publish registry package");
    assert!(
        publish.status.success(),
        "publish failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );

    let list = Command::new(&rr_bin)
        .arg("registry")
        .arg("list")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry list");
    assert!(list.status.success(), "rr registry list failed");
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        list_stdout.contains("rr.local/verifyclean")
            && list_stdout.contains("latest=v1.0.0")
            && list_stdout.contains("license=MIT"),
        "expected registry list output, got:\n{}",
        list_stdout
    );

    let verify = Command::new(&rr_bin)
        .arg("registry")
        .arg("verify")
        .arg("--registry")
        .arg(&registry_dir)
        .output()
        .expect("failed to run rr registry verify");
    assert!(
        verify.status.success(),
        "rr registry verify failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );
    let verify_stdout = String::from_utf8_lossy(&verify.stdout);
    assert!(
        verify_stdout.contains("Verified registry: 1 module(s), 1 release(s)"),
        "expected clean registry verify summary, got:\n{}",
        verify_stdout
    );
}
