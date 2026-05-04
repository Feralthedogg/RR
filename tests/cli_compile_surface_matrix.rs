mod common;

use common::unique_dir;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn rr_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_RR"))
}

fn output_text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_success(output: Output, ctx: &str) {
    assert!(
        output.status.success(),
        "{ctx} failed\n{}",
        output_text(&output)
    );
}

fn write_basic_entry(path: &Path) {
    fs::write(
        path,
        r#"
fn square(x) {
  return x * x
}

fn main() {
  let xs = c(1.0, 2.0, 3.0, 4.0)
  let total = 0.0
  let i = 1L
  while (i <= length(xs)) {
    total = total + square(xs[i])
    i = i + 1L
  }
  print(total)
}

main()
"#,
    )
    .expect("failed to write RR entry");
}

#[test]
fn legacy_compile_accepts_all_opt_level_spellings() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root
        .join("target")
        .join("tests")
        .join("cli_compile_surface_matrix");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox, "legacy_opts");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let rr_path = proj.join("main.rr");
    write_basic_entry(&rr_path);

    let cases = [
        ("-O0", "upper_o0"),
        ("-O1", "upper_o1"),
        ("-O2", "upper_o2"),
        ("-O3", "upper_o3"),
        ("-Oz", "upper_oz"),
        ("-O", "alias_o2"),
        ("-o0", "lower_o0"),
        ("-o1", "lower_o1"),
        ("-o2", "lower_o2"),
        ("-o3", "lower_o3"),
        ("-oz", "lower_oz"),
    ];

    let rr_bin = rr_bin();
    for (opt, stem) in cases {
        let out = proj.join(format!("{stem}.R"));
        let profile = proj.join(format!("{stem}.profile.json"));
        let output = Command::new(&rr_bin)
            .arg(&rr_path)
            .arg("-o")
            .arg(&out)
            .arg(opt)
            .arg("--no-incremental")
            .arg("--profile-compile-out")
            .arg(&profile)
            .output()
            .expect("failed to run RR legacy compile");
        assert_success(output, &format!("legacy compile {opt}"));
        assert!(out.is_file(), "expected output for {opt}");
        let profile_text = fs::read_to_string(&profile).expect("failed to read compile profile");
        assert!(
            profile_text.contains("\"tachyon\"") && profile_text.contains("\"incremental\""),
            "compile profile for {opt} is missing expected sections:\n{}",
            profile_text
        );
    }
}

#[test]
fn legacy_compile_accepts_common_flags_artifact_modes_and_profile_use() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root
        .join("target")
        .join("tests")
        .join("cli_compile_surface_matrix");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox, "legacy_common_flags");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let rr_path = proj.join("r_style.rr");
    fs::write(
        &rr_path,
        r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y
}

value <- 1.0
value <- value + 2.0
print(addv(c(value, 2.0), c(3.0, 4.0)))
"#,
    )
    .expect("failed to write R-style RR source");
    let hot_counts = proj.join("hot-counts.txt");
    fs::write(&hot_counts, "addv=100\n").expect("failed to write profile-use counts");

    let out = proj.join("common_flags.R");
    let profile = proj.join("common_flags.profile.json");
    let output = Command::new(rr_bin())
        .env("RR_ALLOW_LEGACY_IMPLICIT_DECL", "1")
        .env("RR_ALLOW_GRADUAL_TYPE_MODE", "1")
        .arg(&rr_path)
        .arg("-o")
        .arg(&out)
        .arg("-O2")
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("--strict-let")
        .arg("off")
        .arg("--warn-implicit-decl")
        .arg("off")
        .arg("--type-mode")
        .arg("gradual")
        .arg("--native-backend")
        .arg("optional")
        .arg("--parallel-mode")
        .arg("optional")
        .arg("--parallel-backend")
        .arg("r")
        .arg("--parallel-threads")
        .arg("2")
        .arg("--parallel-min-trip")
        .arg("1")
        .arg("--compiler-parallel-mode")
        .arg("on")
        .arg("--compiler-parallel-threads")
        .arg("2")
        .arg("--compiler-parallel-min-functions")
        .arg("1")
        .arg("--compiler-parallel-min-fn-ir")
        .arg("1")
        .arg("--compiler-parallel-max-jobs")
        .arg("2")
        .arg("--compile-mode")
        .arg("standard")
        .arg("--profile-use")
        .arg(&hot_counts)
        .arg("--profile-compile-out")
        .arg(&profile)
        .output()
        .expect("failed to run RR common flag compile");
    assert_success(output, "legacy common flag compile");

    let generated = fs::read_to_string(&out).expect("failed to read generated R");
    assert!(
        generated.contains("# --- RR runtime (auto-generated) ---")
            && generated.contains("# --- RR generated code (from user RR source) ---")
            && !generated.contains("rr_set_source <- function"),
        "--no-runtime helper-only output shape was not preserved:\n{}",
        generated
    );
    let profile_text = fs::read_to_string(&profile).expect("failed to read profile json");
    assert!(
        profile_text.contains("\"pass_decisions\"")
            && profile_text.contains("\"compile_mode\": \"standard\""),
        "profile-use/compile-mode path did not populate expected profile fields:\n{}",
        profile_text
    );
}

#[cfg(unix)]
#[test]
fn build_run_watch_and_incremental_flags_work_together() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root
        .join("target")
        .join("tests")
        .join("cli_compile_surface_matrix");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox, "build_run_watch");
    fs::create_dir_all(proj.join("src")).expect("failed to create project dirs");

    let main_path = proj.join("src").join("main.rr");
    write_basic_entry(&main_path);
    let cache_dir = proj.join(".rr-cache");

    let build_out = proj.join("build-out");
    let build_profile = proj.join("build-profile.json");
    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&proj)
        .arg("--out-dir")
        .arg(&build_out)
        .arg("-O3")
        .arg("--incremental=all")
        .arg("--strict-incremental-verify")
        .arg("--compile-mode")
        .arg("standard")
        .arg("--compiler-parallel-mode")
        .arg("on")
        .arg("--compiler-parallel-threads")
        .arg("2")
        .arg("--compiler-parallel-min-functions")
        .arg("1")
        .arg("--compiler-parallel-min-fn-ir")
        .arg("1")
        .arg("--compiler-parallel-max-jobs")
        .arg("2")
        .arg("--profile-compile-out")
        .arg(&build_profile)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .output()
        .expect("failed to run RR build");
    assert_success(output, "build with O3 incremental all");
    assert!(
        build_out.join("src").join("main.R").is_file(),
        "build output missing"
    );
    let profile_text = fs::read_to_string(&build_profile).expect("failed to read build profile");
    assert!(
        profile_text.contains("\"enabled\": true")
            && profile_text.contains("\"strict_verification_checked\"")
            && profile_text.contains("\"strict_verification_passed\""),
        "build profile did not record incremental strict verify:\n{}",
        profile_text
    );

    let warm_profile = proj.join("build-warm-profile.json");
    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&proj)
        .arg("--out-dir")
        .arg(&build_out)
        .arg("-O3")
        .arg("--incremental-phases")
        .arg("1,2,3")
        .arg("--profile-compile-out")
        .arg(&warm_profile)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .output()
        .expect("failed to rerun RR build warm");
    assert_success(output, "warm build with explicit incremental phases");
    let warm_profile_text = fs::read_to_string(&warm_profile).expect("failed to read warm profile");
    assert!(
        warm_profile_text.contains("\"phase1_artifact_hit\": true")
            || warm_profile_text.contains("\"optimized_mir_cache_hit\": true")
            || warm_profile_text.contains("\"phase3_memory_hit\": true")
            || !warm_profile_text.contains("\"phase2_emit_hits\": 0"),
        "warm build did not report any incremental hit:\n{}",
        warm_profile_text
    );

    let cold_profile = proj.join("build-cold-profile.json");
    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&proj)
        .arg("--out-dir")
        .arg(&build_out)
        .arg("-O3")
        .arg("--cold")
        .arg("--profile-compile-out")
        .arg(&cold_profile)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .output()
        .expect("failed to run RR cold build");
    assert_success(output, "cold build");
    let cold_profile_text = fs::read_to_string(&cold_profile).expect("failed to read cold profile");
    assert!(
        cold_profile_text.contains("\"phase1_artifact_hit\": false")
            && cold_profile_text.contains("\"optimized_mir_cache_hit\": false"),
        "cold build unexpectedly used warm caches:\n{}",
        cold_profile_text
    );

    let fake_rscript = proj.join("fake_rscript.sh");
    fs::write(&fake_rscript, "#!/bin/sh\nprintf '[1] 30\\n'\nexit 0\n")
        .expect("failed to write fake Rscript");
    let mut perms = fs::metadata(&fake_rscript)
        .expect("failed to stat fake Rscript")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_rscript, perms).expect("failed to chmod fake Rscript");

    let run_profile = proj.join("run-profile.json");
    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("run")
        .arg(".")
        .arg("-O1")
        .arg("--keep-r")
        .arg("--incremental=2")
        .arg("--compile-mode")
        .arg("fast-dev")
        .arg("--profile-compile-out")
        .arg(&run_profile)
        .env("RRSCRIPT", &fake_rscript)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .output()
        .expect("failed to run RR run");
    assert_success(output, "run with keep-r and phase2 incremental");
    assert!(
        proj.join("src").join("main.gen.R").is_file(),
        "run --keep-r did not preserve generated R"
    );

    let watch_out = proj.join("watched.R");
    let watch_profile = proj.join("watch-profile.json");
    let output = Command::new(rr_bin())
        .arg("watch")
        .arg(&proj)
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .arg("-o")
        .arg(&watch_out)
        .arg("-O1")
        .arg("--incremental-phases")
        .arg("phase1,phase2")
        .arg("--profile-compile-out")
        .arg(&watch_profile)
        .arg("--compiler-parallel-mode")
        .arg("off")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .output()
        .expect("failed to run RR watch --once");
    assert_success(output, "watch once with incremental phases");
    assert!(watch_out.is_file(), "watch output missing");
    let watch_profile_text =
        fs::read_to_string(&watch_profile).expect("failed to read watch profile");
    assert!(
        watch_profile_text.contains("\"enabled\": true"),
        "watch profile did not record incremental mode:\n{}",
        watch_profile_text
    );
}
