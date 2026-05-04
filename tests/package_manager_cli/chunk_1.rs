use super::package_manager_cli_common::*;

#[test]
pub(crate) fn install_command_fetches_github_dependency_and_build_uses_transitive_package_imports()
{
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github");
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

    let proj_dir = unique_dir(&sandbox_root, "app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
import "github.com/acme/mathlib"

fn main() {
  print(add_one(41L))
}

main()
"#,
    )
    .expect("failed to write app src/main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home");
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

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("require github.com/acme/mathlib v1.2.3"),
        "expected installed dependency in rr.mod, got:\n{}",
        manifest
    );

    let lock = fs::read_to_string(proj_dir.join("rr.lock")).expect("failed to read rr.lock");
    assert!(
        lock.contains("path = \"github.com/acme/mathlib\"")
            && lock.contains("path = \"github.com/acme/baseutil\""),
        "expected direct and transitive modules in rr.lock, got:\n{}",
        lock
    );

    let mut build = Command::new(&rr_bin);
    build.current_dir(&proj_dir).arg("build").arg(".");
    build.env("RRPKGHOME", &pkg_home);
    let build_output = build.output().expect("failed to run rr build");
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
        built_r.contains("41L + 1L")
            || built_r.contains("x + 1L")
            || built_r.contains("print(42L)"),
        "expected imported package logic in built artifact, got:\n{}",
        built_r
    );
}

#[test]
pub(crate) fn installed_module_subpackage_directory_import_resolves_from_cache() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_subpkg");
    let acme_root = github_root.join("acme");
    fs::create_dir_all(&acme_root).expect("failed to create fake github org root");

    init_git_repo(
        &acme_root.join("mathlib"),
        &[
            ("rr.mod", "module github.com/acme/mathlib\n\nrr 8.0\n"),
            (
                "src/lib.rr",
                r#"
fn root_fn() {
  return 0L
}
"#,
            ),
            (
                "src/vector/inc.rr",
                r#"
fn inc(x) {
  return x + 1L
}
"#,
            ),
            (
                "src/vector/plus_two.rr",
                r#"
fn plus_two(x) {
  return inc(x) + 1L
}
"#,
            ),
        ],
        "v1.2.3",
    );

    let proj_dir = unique_dir(&sandbox_root, "subpkg_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/subpkg-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
import "github.com/acme/mathlib/vector"

fn main() {
  return plus_two(40L)
}
main()
"#,
    )
    .expect("failed to write app src/main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_subpkg");
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

    let mut build = Command::new(&rr_bin);
    build.current_dir(&proj_dir).arg("build").arg(".");
    build.env("RRPKGHOME", &pkg_home);
    let build_output = build.output().expect("failed to run rr build");
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
        built_r.contains("40L + 1L")
            || built_r.contains("41L + 1L")
            || built_r.contains("x + 1L")
            || built_r.contains("return(42L)"),
        "expected imported subpackage logic in built artifact, got:\n{}",
        built_r
    );
}

#[test]
pub(crate) fn replace_directive_prefers_local_module_path() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let local_dep = unique_dir(&sandbox_root, "local_replace_dep");
    fs::create_dir_all(local_dep.join("src")).expect("failed to create local dep src");
    fs::write(
        local_dep.join("rr.mod"),
        "module github.com/acme/mathlib\n\nrr 8.0\n",
    )
    .expect("failed to write local dep rr.mod");
    fs::write(
        local_dep.join("src").join("lib.rr"),
        r#"
fn add_one(x) {
  return x + 2L
}
"#,
    )
    .expect("failed to write local dep lib.rr");

    let proj_dir = unique_dir(&sandbox_root, "replace_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/replace-app")
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
    .expect("failed to write app src/main.rr");

    fs::write(
        proj_dir.join("rr.mod"),
        format!(
            "module github.com/example/replace-app\n\nrr 8.0\n\nrequire github.com/acme/mathlib v0.0.0\nreplace github.com/acme/mathlib => {}\n",
            local_dep.to_string_lossy()
        ),
    )
    .expect("failed to write rr.mod with replace");

    let build_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("build")
        .arg(".")
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
        built_r.contains("40L + 2L")
            || built_r.contains("x + 2L")
            || built_r.contains("return(42L)"),
        "expected replace target logic in built artifact, got:\n{}",
        built_r
    );
}

#[test]
pub(crate) fn mod_vendor_allows_build_without_package_cache() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_vendor");
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

    let proj_dir = unique_dir(&sandbox_root, "vendor_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/vendor-app")
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
    .expect("failed to write app src/main.rr");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_vendor");
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

    let vendor_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("mod")
        .arg("vendor")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr mod vendor");
    assert!(
        vendor_output.status.success(),
        "rr mod vendor failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&vendor_output.stdout),
        String::from_utf8_lossy(&vendor_output.stderr)
    );

    fs::remove_dir_all(&pkg_home).expect("failed to clear package cache");
    let empty_pkg_home = unique_dir(&sandbox_root, "pkg_home_empty");
    fs::create_dir_all(&empty_pkg_home).expect("failed to create empty pkg home");

    let build_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("build")
        .arg(".")
        .env("RRPKGHOME", &empty_pkg_home)
        .output()
        .expect("failed to run rr build");
    assert!(
        build_output.status.success(),
        "rr build failed without cache:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );

    let modules_txt = fs::read_to_string(proj_dir.join("vendor").join("modules.txt"))
        .expect("failed to read vendor/modules.txt");
    assert!(
        modules_txt.contains("github.com/acme/mathlib v1.2.3")
            && modules_txt.contains("github.com/acme/baseutil v0.1.0"),
        "expected vendored modules list, got:\n{}",
        modules_txt
    );
}

#[test]
pub(crate) fn remove_command_drops_dependency_from_manifest_and_lock() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_remove");
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

    let proj_dir = unique_dir(&sandbox_root, "remove_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/remove-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_remove");
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

    let remove_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("remove")
        .arg("github.com/acme/mathlib")
        .output()
        .expect("failed to run rr remove");
    assert!(
        remove_output.status.success(),
        "rr remove failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&remove_output.stdout),
        String::from_utf8_lossy(&remove_output.stderr)
    );

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        !manifest.contains("github.com/acme/mathlib"),
        "expected dependency to be removed from rr.mod, got:\n{}",
        manifest
    );

    let lock = fs::read_to_string(proj_dir.join("rr.lock")).expect("failed to read rr.lock");
    assert!(
        !lock.contains("github.com/acme/mathlib"),
        "expected dependency to be removed from rr.lock, got:\n{}",
        lock
    );
}

#[test]
pub(crate) fn mod_tidy_adds_missing_direct_dependency_from_imports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_tidy_add");
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

    let proj_dir = unique_dir(&sandbox_root, "tidy_add_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/tidy-add-app")
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

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_tidy_add");
    fs::create_dir_all(&pkg_home).expect("failed to create pkg home");

    let mut tidy = Command::new(&rr_bin);
    tidy.current_dir(&proj_dir).arg("mod").arg("tidy");
    configure_github_mapping(&mut tidy, &github_root, &pkg_home);
    let tidy_output = tidy.output().expect("failed to run rr mod tidy");
    assert!(
        tidy_output.status.success(),
        "rr mod tidy failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&tidy_output.stdout),
        String::from_utf8_lossy(&tidy_output.stderr)
    );

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("require github.com/acme/mathlib v1.2.3"),
        "expected tidy to add direct dependency, got:\n{}",
        manifest
    );
}

#[test]
pub(crate) fn mod_tidy_removes_unused_direct_dependency() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_tidy_remove");
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

    let proj_dir = unique_dir(&sandbox_root, "tidy_remove_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/tidy-remove-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_tidy_remove");
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

    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
fn main() {
  return 1L
}
main()
"#,
    )
    .expect("failed to write src/main.rr");

    let tidy_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("mod")
        .arg("tidy")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr mod tidy");
    assert!(
        tidy_output.status.success(),
        "rr mod tidy failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&tidy_output.stdout),
        String::from_utf8_lossy(&tidy_output.stderr)
    );

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        !manifest.contains("github.com/acme/mathlib"),
        "expected tidy to remove unused dependency, got:\n{}",
        manifest
    );
}

#[test]
pub(crate) fn build_prefers_rr_lock_version_over_manifest_require_version() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("package_manager_cli");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");

    let github_root = unique_dir(&sandbox_root, "github_lock_pref");
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

    let proj_dir = unique_dir(&sandbox_root, "lock_pref_app");
    let rr_bin = rr_bin();
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/example/lock-pref-app")
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

    let pkg_home = unique_dir(&sandbox_root, "pkg_home_lock_pref");
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

    fs::write(
        proj_dir.join("rr.mod"),
        "module github.com/example/lock-pref-app\n\nrr 8.0\n\nrequire github.com/acme/mathlib v0.0.0\n",
    )
    .expect("failed to rewrite rr.mod");

    let build_output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("build")
        .arg(".")
        .env("RRPKGHOME", &pkg_home)
        .output()
        .expect("failed to run rr build");
    assert!(
        build_output.status.success(),
        "rr build should use rr.lock exact version:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );
}
