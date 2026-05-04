mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_profile(
    rr_bin: &PathBuf,
    src_path: &PathBuf,
    out_path: &PathBuf,
    profile_path: &PathBuf,
    extra_args: &[&str],
) -> String {
    let mut cmd = Command::new(rr_bin);
    cmd.arg("build")
        .arg(src_path)
        .arg("--out-dir")
        .arg(out_path)
        .arg("--profile-compile-out")
        .arg(profile_path)
        .arg("--no-incremental");
    for arg in extra_args {
        cmd.arg(arg);
    }
    let status = cmd.status().expect("failed to run RR build for pass plan");
    assert!(status.success(), "RR build failed for pass plan");
    fs::read_to_string(profile_path).expect("failed to read compile profile json")
}

#[test]
fn fast_dev_profile_keeps_only_required_and_dev_cheap_groups() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("tachyon_pass_plan");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let src_path = proj_dir.join("main.rr");
    fs::write(
        &src_path,
        r#"
fn main() {
  let xs = c(1.0, 2.0, 3.0)
  print(sum(xs))
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("profile.json");
    let profile = compile_profile(&rr_bin, &src_path, &out_dir, &profile_path, &["-O1"]);
    assert!(profile.contains("\"active_pass_groups\": [\"required\", \"dev-cheap\"]"));
    assert!(profile.contains("\"disabled_pass_groups\": [\"poly\"]"));
}

#[test]
fn standard_profile_includes_release_expensive_group() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("tachyon_pass_plan");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "standard");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let src_path = proj_dir.join("main.rr");
    fs::write(
        &src_path,
        r#"
fn main() {
  let xs = c(1.0, 2.0, 3.0)
  let ys = xs + 1.0
  print(sum(ys))
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("profile.json");
    let profile = compile_profile(
        &rr_bin,
        &src_path,
        &out_dir,
        &profile_path,
        &["-O1", "--compile-mode", "standard"],
    );
    assert!(profile.contains("\"active_pass_groups\": [\"required\", \"dev-cheap\", \"release-expensive\", \"experimental\"]"));
    assert!(profile.contains("\"disabled_pass_groups\": []"));
    assert!(profile.contains("groups=required,dev-cheap,release-expensive"));
    assert!(profile.contains("\"plan_summary\": ["));
}

#[test]
fn compute_heavy_o2_profile_can_expose_experimental_group() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("tachyon_pass_plan");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compute_heavy");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let src_path = proj_dir.join("main.rr");
    let mut source =
        String::from("fn heavy(n) {\n  let acc = 0.0\n  let i = 1.0\n  while (i <= n) {\n");
    for idx in 0..32 {
        source.push_str(&format!("  let t{} = (i + {}.0) * (i + 2.0)\n", idx, idx));
    }
    source.push_str("  acc = acc + t31\n  i = i + 1.0\n  }\n  return acc\n}\n\nfn main() {\n  print(heavy(32.0))\n}\nmain()\n");
    fs::write(&src_path, source).expect("failed to write compute-heavy main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("profile.json");
    let profile = compile_profile(
        &rr_bin,
        &src_path,
        &out_dir,
        &profile_path,
        &["-O2", "--compile-mode", "standard"],
    );
    assert!(profile.contains("\"active_pass_groups\": [\"required\", \"dev-cheap\", \"release-expensive\", \"experimental\"]"));
    assert!(profile.contains("groups=required,dev-cheap,release-expensive,experimental"));
}

#[test]
fn o3_profile_enables_aggressive_plan_group_for_balanced_functions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("tachyon_pass_plan");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "o3_aggressive");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let src_path = proj_dir.join("main.rr");
    fs::write(
        &src_path,
        r#"
fn helper(x) {
  let y = x + 1.0
  return y
}

fn main() {
  print(helper(41.0))
}
main()
"#,
    )
    .expect("failed to write O3 main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("profile.json");
    let profile = compile_profile(&rr_bin, &src_path, &out_dir, &profile_path, &["-O3"]);
    assert!(profile.contains("\"active_pass_groups\": [\"required\", \"dev-cheap\", \"release-expensive\", \"experimental\"]"));
    assert!(profile.contains("groups=required,dev-cheap,release-expensive,experimental"));
}

#[test]
fn oz_profile_is_accepted_and_avoids_experimental_group() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("tachyon_pass_plan");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "oz_size");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let src_path = proj_dir.join("main.rr");
    fs::write(
        &src_path,
        r#"
fn main() {
  let xs = c(1.0, 2.0, 3.0)
  print(sum(xs))
}
main()
"#,
    )
    .expect("failed to write Oz main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("profile.json");
    let profile = compile_profile(&rr_bin, &src_path, &out_dir, &profile_path, &["-Oz"]);
    assert!(
        profile.contains(
            "\"active_pass_groups\": [\"required\", \"dev-cheap\", \"release-expensive\"]"
        )
    );
    assert!(!profile.contains("experimental"));
    assert!(profile.contains("groups=required,dev-cheap,release-expensive"));
}

#[test]
fn profile_use_and_mir_dump_are_wired_into_tachyon() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("tachyon_pass_plan");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "profile_use_dump");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let src_path = proj_dir.join("main.rr");
    fs::write(
        &src_path,
        r#"
fn hot(n) {
  let acc = 0.0
  let i = 1.0
  while (i <= n) {
    acc = acc + i
    i = i + 1.0
  }
  return acc
}

fn main() {
  print(hot(8.0))
}
main()
"#,
    )
    .expect("failed to write profile-use main.rr");
    let profile_use_path = proj_dir.join("hot-counts.txt");
    fs::write(&profile_use_path, "hot=1000\n").expect("failed to write profile counts");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("profile.json");
    let dump_dir = proj_dir.join("mir-dumps");
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&src_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .arg("--profile-use")
        .arg(&profile_use_path)
        .arg("--no-incremental")
        .arg("-O3")
        .env("RR_MIR_DUMP_DIR", &dump_dir)
        .env("RR_MIR_DUMP_FILTER", "hot")
        .env("RR_MIR_DUMP_WHEN", "before,after")
        .status()
        .expect("failed to run RR build with profile-use");
    assert!(status.success(), "RR build with profile-use failed");

    let profile = fs::read_to_string(&profile_path).expect("failed to read profile json");
    assert!(profile.contains("\"pass_decisions\":"));
    assert!(profile.contains("\"optimization_opportunities\":"));
    assert!(profile.contains("\"function\": \"hot\""));
    assert!(profile.contains("\"budget_class\":"));

    let dump_count = fs::read_dir(&dump_dir)
        .expect("failed to read MIR dump dir")
        .filter_map(Result::ok)
        .count();
    assert!(dump_count > 0, "expected MIR dump files in {:?}", dump_dir);
}
