mod common;

use common::unique_dir;
use rr::compiler::{
    CompileOutputOptions, IncrementalOptions, OptLevel, compile_with_configs_incremental,
    default_parallel_config, default_type_config,
};
use std::fs;
use std::path::PathBuf;

fn opts() -> IncrementalOptions {
    IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: true,
        phase2: true,
        phase3: false,
        strict_verify: false,
    }
}

#[test]
fn incremental_reports_entry_and_import_miss_reasons() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_miss_reason");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "entry_import");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let module_path = proj_dir.join("dep.rr");
    fs::write(
        &module_path,
        r#"
fn helper() {
  return 1L
}
"#,
    )
    .expect("failed to write dep.rr");

    let main_path = proj_dir.join("main.rr");
    let source = r#"
import "./dep.rr"

fn main() {
  print(helper())
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");
    let path_str = main_path.to_string_lossy().to_string();

    let first = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("seed compile failed");
    assert!(first.stats.miss_reasons.contains(&"cold_start".to_string()));

    fs::write(
        &main_path,
        r#"
import "./dep.rr"

fn main() {
  print(helper() + 1L)
}
main()
"#,
    )
    .expect("failed to update main.rr");
    let source_changed = fs::read_to_string(&main_path).expect("failed to read updated main.rr");
    let second = compile_with_configs_incremental(
        &path_str,
        &source_changed,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("entry change compile failed");
    assert!(
        second
            .stats
            .miss_reasons
            .contains(&"entry_changed".to_string()),
        "expected entry_changed miss reason, got {:?}",
        second.stats.miss_reasons
    );

    fs::write(
        &module_path,
        r#"
fn helper() {
  return 2L
}
"#,
    )
    .expect("failed to update dep.rr");
    let third = compile_with_configs_incremental(
        &path_str,
        &source_changed,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("import change compile failed");
    assert!(
        third
            .stats
            .miss_reasons
            .contains(&"import_fingerprint_changed".to_string()),
        "expected import_fingerprint_changed miss reason, got {:?}",
        third.stats.miss_reasons
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_reports_option_change_miss_reasons() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_miss_reason");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "option_change");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(9L)
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");
    let path_str = main_path.to_string_lossy().to_string();

    compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("seed option compile failed");

    let changed_opt = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("opt-level change compile failed");
    assert!(
        changed_opt
            .stats
            .miss_reasons
            .contains(&"opt_level_changed".to_string()),
        "expected opt_level_changed miss reason, got {:?}",
        changed_opt.stats.miss_reasons
    );

    let helper_only = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("reset baseline compile failed");
    assert!(helper_only.stats.phase1_artifact_hit);

    let output_changed =
        rr::compiler::compile_incremental_request(rr::compiler::IncrementalCompileRequest {
            entry_path: &path_str,
            entry_input: source,
            opt_level: OptLevel::O1,
            type_cfg: default_type_config(),
            parallel_cfg: default_parallel_config(),
            compiler_parallel_cfg: rr::compiler::default_compiler_parallel_config(),
            options: opts(),
            output_options: CompileOutputOptions {
                inject_runtime: false,
                ..Default::default()
            },
            session: None,
            profile: None,
        })
        .expect("output-option change compile failed");
    assert_eq!(
        output_changed.stats.miss_reasons,
        vec!["output_options_changed".to_string()],
        "phase1 hits should refresh latest-build metadata before later miss diagnostics"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_separates_o3_and_oz_artifacts() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_miss_reason");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "o3_oz");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn helper(x) {
  let y = x + 1L
  return y * y
}

fn main() {
  print(helper(4L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");
    let path_str = main_path.to_string_lossy().to_string();

    let o3_seed = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O3,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("O3 seed compile failed");
    assert!(
        o3_seed
            .stats
            .miss_reasons
            .contains(&"cold_start".to_string())
    );

    let oz = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::Oz,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("Oz compile failed");
    assert!(
        oz.stats
            .miss_reasons
            .contains(&"opt_level_changed".to_string()),
        "expected O3 -> Oz to miss by opt level, got {:?}",
        oz.stats.miss_reasons
    );
    assert!(
        !oz.stats.phase1_artifact_hit,
        "Oz must not reuse the O3 phase1 artifact"
    );

    let o3_hit = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O3,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("O3 cache compile failed");
    assert!(
        o3_hit.stats.phase1_artifact_hit,
        "returning to O3 should reuse only the O3 artifact"
    );

    let oz_hit = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::Oz,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("Oz cache compile failed");
    assert!(
        oz_hit.stats.phase1_artifact_hit,
        "returning to Oz should reuse only the Oz artifact"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_reports_phase_ordering_change_miss_reason() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_miss_reason");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "phase_ordering");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(7L)
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");
    let path_str = main_path.to_string_lossy().to_string();

    common::set_env_var_for_test(&env_guard, "RR_PHASE_ORDERING", "balanced");
    compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("balanced phase ordering seed compile failed");

    common::set_env_var_for_test(&env_guard, "RR_PHASE_ORDERING", "off");
    let changed_phase_ordering = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        opts(),
        None,
    )
    .expect("phase-ordering change compile failed");
    assert!(
        changed_phase_ordering
            .stats
            .miss_reasons
            .contains(&"phase_ordering_changed".to_string()),
        "expected phase_ordering_changed miss reason, got {:?}",
        changed_phase_ordering.stats.miss_reasons
    );
    assert!(
        !changed_phase_ordering
            .stats
            .miss_reasons
            .contains(&"opt_level_changed".to_string()),
        "same opt level should not be reported as an opt-level change: {:?}",
        changed_phase_ordering.stats.miss_reasons
    );

    common::remove_env_var_for_test(&env_guard, "RR_PHASE_ORDERING");
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
