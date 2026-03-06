mod common;

use RR::compiler::{
    IncrementalOptions, OptLevel, compile_with_configs_incremental, parallel_config_from_env,
    type_config_from_env,
};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn incremental_phase2_reuses_function_emit_cache() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase2");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    // SAFETY: Scoped test setup; value is removed at the end of this test.
    unsafe {
        std::env::set_var("RR_INCREMENTAL_CACHE_DIR", &cache_dir);
    }

    let main_path = proj_dir.join("main.rr");
    let uniq = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let square_name = format!("square_{}", uniq);
    let bump_name = format!("bump_{}", uniq);
    let source = format!(
        r#"
fn {square_name}(x) {{
  return x * x;
}}

fn {bump_name}(x) {{
  return x + 1L;
}}

fn main() {{
  let a = {square_name}(3L);
  print({bump_name}(a));
}}
main();
"#
    );
    fs::write(&main_path, &source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();

    let opts = IncrementalOptions {
        enabled: true,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_with_configs_incremental(
        &path_str,
        &source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase2 first compile failed");
    assert!(
        first.stats.phase2_emit_misses > 0,
        "first compile should populate function emit cache"
    );

    let second = compile_with_configs_incremental(
        &path_str,
        &source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase2 second compile failed");
    assert!(
        second.stats.phase2_emit_hits > 0,
        "second compile should reuse function emit cache"
    );
    assert!(
        second.stats.phase2_emit_hits >= first.stats.phase2_emit_hits,
        "second compile should not reduce emit cache hits"
    );
    assert_eq!(first.r_code, second.r_code);
    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}
