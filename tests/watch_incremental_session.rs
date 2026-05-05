mod common;

use common::unique_dir;
use rr::compiler::{
    CompileOutputOptions, IncrementalCompileRequest, IncrementalOptions, IncrementalSession,
    OptLevel, compile_incremental_request, default_parallel_config, default_type_config,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn same_session_second_build_hits_phase3_memory_cache() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("watch_incremental_session");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "session");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn helper(x) {
  return x + 1L
}

fn main() {
  print(helper(5L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let opts = IncrementalOptions::auto();
    let mut session = IncrementalSession::default();

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: rr::compiler::default_compiler_parallel_config(),
        options: opts,
        output_options: CompileOutputOptions::default(),
        session: Some(&mut session),
        profile: None,
    })
    .expect("first session compile failed");
    assert!(
        !first.stats.phase3_memory_hit,
        "first session compile should populate memory cache"
    );
    assert!(
        first.stats.miss_reasons.contains(&"cold_start".to_string()),
        "first session compile should report cold_start miss reason"
    );

    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: rr::compiler::default_compiler_parallel_config(),
        options: opts,
        output_options: CompileOutputOptions::default(),
        session: Some(&mut session),
        profile: None,
    })
    .expect("second session compile failed");
    assert!(
        second.stats.phase3_memory_hit,
        "second session compile should reuse phase3 memory artifact"
    );
    assert!(
        second.stats.miss_reasons.is_empty(),
        "phase3 hit should not report miss reasons: {:?}",
        second.stats.miss_reasons
    );
    assert_eq!(first.r_code, second.r_code);

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
