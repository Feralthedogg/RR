mod common;

use common::unique_dir;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn clear_whole_output_emit_caches(cache_dir: &PathBuf) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "Rraw" || ext == "Rpee" || ext == "linemap")
        {
            fs::remove_file(&path).expect("failed to remove whole-output emit cache");
        }
    }
}

#[test]
fn legacy_cli_emits_compile_profile_json() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "profile");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = proj_dir.join("main.rr");
    let out_file = proj_dir.join("out.R");
    let profile_file = proj_dir.join("compile-profile.json");
    fs::write(
        &main_path,
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
    let status = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_file)
        .status()
        .expect("failed to run RR CLI");
    assert!(status.success(), "RR CLI failed");

    let profile = fs::read_to_string(&profile_file).expect("failed to read compile profile json");
    assert!(profile.contains("\"schema\": \"rr-compile-profile\""));
    assert!(profile.contains("\"source_analysis\""));
    assert!(profile.contains("\"canonicalization\""));
    assert!(profile.contains("\"mir_synthesis\""));
    assert!(profile.contains("\"tachyon\""));
    assert!(profile.contains("\"optimized_mir_cache_hit\""));
    assert!(profile.contains("\"emit\""));
    assert!(profile.contains("\"breakdown\""));
    assert!(profile.contains("\"fragment_build_elapsed_ns\""));
    assert!(profile.contains("\"optimized_fragment_cache_hits\""));
    assert!(profile.contains("\"optimized_fragment_cache_misses\""));
    assert!(profile.contains("\"optimized_fragment_final_artifact_hits\""));
    assert!(profile.contains("\"optimized_fragment_fast_path_direct_hits\""));
    assert!(profile.contains("\"optimized_fragment_fast_path_raw_hits\""));
    assert!(profile.contains("\"optimized_fragment_fast_path_peephole_hits\""));
    assert!(profile.contains("\"raw_rewrite_elapsed_ns\""));
    assert!(profile.contains("\"peephole_elapsed_ns\""));
    assert!(profile.contains("\"peephole_linear_scan_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_rewrite_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_flow_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_inline_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_reuse_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_dead_zero_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_normalize_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_hoist_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_repeat_to_for_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_tail_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_guard_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_helper_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_pre_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_reuse_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_reuse_prepare_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_reuse_forward_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_reuse_pure_call_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_reuse_expr_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_reuse_vector_alias_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_reuse_rebind_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_fixpoint_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_fixpoint_prepare_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_fixpoint_forward_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_fixpoint_pure_call_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_fixpoint_expr_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_fixpoint_rebind_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_fixpoint_rounds\""));
    assert!(profile.contains("\"peephole_primary_loop_exact_finalize_elapsed_ns\""));
    assert!(profile.contains("\"peephole_primary_loop_dead_temp_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_rewrite_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_branch_hoist_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_immediate_scalar_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_named_index_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_named_expr_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_scalar_region_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_immediate_index_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_inline_adjacent_dedup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_exact_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_helper_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_helper_wrapper_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_helper_metric_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_helper_alias_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_helper_simple_expr_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_helper_full_range_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_helper_named_copy_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_finalize_cleanup_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_finalize_bundle_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_finalize_dead_temp_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_finalize_dead_temp_facts_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_finalize_dead_temp_mark_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_finalize_dead_temp_reverse_elapsed_ns\""));
    assert!(profile.contains("\"peephole_secondary_finalize_dead_temp_compact_elapsed_ns\""));
    assert!(profile.contains("\"peephole_finalize_elapsed_ns\""));
    assert!(profile.contains("\"source_map_remap_elapsed_ns\""));
    assert!(profile.contains("\"runtime_injection\""));
    assert!(profile.contains("\"parsed_modules\": 1"));
    assert!(profile.contains("\"lowered_functions\": 2"));
    assert!(profile.contains("\"passes\":"));
    assert!(profile.contains("\"elapsed_ns\":"));
    assert!(profile.contains("\"inject_runtime\": false"));
}

#[test]
fn no_incremental_cli_reuses_optimized_mir_cache_across_output_modes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "optimized_mir_cli");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = proj_dir.join("main.rr");
    let out_runtime = proj_dir.join("runtime.R");
    let out_helper_only = proj_dir.join("helper-only.R");
    let profile_first = proj_dir.join("first-profile.json");
    let profile_second = proj_dir.join("second-profile.json");
    let cache_dir = proj_dir.join(".rr-cache");
    fs::write(
        &main_path,
        r#"
fn square_cli_optmir(x) {
  return x * x
}

fn main() {
  print(square_cli_optmir(4L))
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let first = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_runtime)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_first)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run first RR CLI compile");
    assert!(first.success(), "first RR CLI compile failed");

    let second = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_helper_only)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_second)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run second RR CLI compile");
    assert!(second.success(), "second RR CLI compile failed");

    let first_profile =
        fs::read_to_string(&profile_first).expect("failed to read first compile profile json");
    let second_profile =
        fs::read_to_string(&profile_second).expect("failed to read second compile profile json");
    assert!(
        first_profile.contains("\"optimized_mir_cache_hit\": false"),
        "first compile should seed optimized MIR cache"
    );
    assert!(
        second_profile.contains("\"optimized_mir_cache_hit\": true"),
        "second compile should reuse optimized MIR cache even when output mode changes"
    );
}

#[test]
fn no_incremental_cli_reuses_emit_cache_and_preserves_raw_dump() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "direct_emit_cache");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = proj_dir.join("main.rr");
    let out_file = proj_dir.join("out.R");
    let raw_first = proj_dir.join("raw-first.R");
    let raw_second = proj_dir.join("raw-second.R");
    let profile_first = proj_dir.join("first-profile.json");
    let profile_second = proj_dir.join("second-profile.json");
    let cache_dir = proj_dir.join(".rr-cache");
    fs::write(
        &main_path,
        r#"
fn square_emit_cache(x) {
  return x * x
}

fn bump_emit_cache(x) {
  return x + 1L
}

fn main() {
  let a = square_emit_cache(3L)
  print(bump_emit_cache(a))
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let first = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_first)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .env("RR_DEBUG_RAW_R_PATH", &raw_first)
        .status()
        .expect("failed to run first direct emit-cache compile");
    assert!(first.success(), "first direct emit-cache compile failed");

    let second = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_second)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .env("RR_DEBUG_RAW_R_PATH", &raw_second)
        .status()
        .expect("failed to run second direct emit-cache compile");
    assert!(second.success(), "second direct emit-cache compile failed");

    let raw_first_text = fs::read_to_string(&raw_first).expect("failed to read first raw dump");
    let raw_second_text = fs::read_to_string(&raw_second).expect("failed to read second raw dump");
    assert_eq!(
        raw_first_text, raw_second_text,
        "direct emit cache reuse must preserve raw debug output"
    );

    let first_profile: Value =
        serde_json::from_str(&fs::read_to_string(&profile_first).expect("read first profile"))
            .expect("parse first compile profile json");
    let second_profile: Value =
        serde_json::from_str(&fs::read_to_string(&profile_second).expect("read second profile"))
            .expect("parse second compile profile json");
    assert_eq!(
        first_profile["emit"]["cache_hits"].as_u64().unwrap_or(0),
        0,
        "first compile should seed direct emit cache"
    );
    assert!(
        second_profile["emit"]["cache_hits"].as_u64().unwrap_or(0) > 0,
        "second compile should reuse direct emit cache"
    );
}

#[test]
fn no_incremental_cli_raw_debug_keeps_direct_fragment_fast_path() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "direct_fast_raw_debug");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = proj_dir.join("main.rr");
    let out_file = proj_dir.join("out.R");
    let raw_first = proj_dir.join("raw-first.R");
    let raw_second = proj_dir.join("raw-second.R");
    let profile_first = proj_dir.join("first-profile.json");
    let profile_second = proj_dir.join("second-profile.json");
    let cache_dir = proj_dir.join(".rr-cache");
    fs::write(
        &main_path,
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let first = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_first)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .env("RR_DEBUG_RAW_R_PATH", &raw_first)
        .status()
        .expect("failed to run first raw-debug direct compile");
    assert!(first.success(), "first raw-debug direct compile failed");

    clear_whole_output_emit_caches(&cache_dir);

    let second = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_second)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .env("RR_DEBUG_RAW_R_PATH", &raw_second)
        .status()
        .expect("failed to run second raw-debug direct compile");
    assert!(second.success(), "second raw-debug direct compile failed");

    let raw_first_text = fs::read_to_string(&raw_first).expect("failed to read first raw dump");
    let raw_second_text = fs::read_to_string(&raw_second).expect("failed to read second raw dump");
    assert_eq!(
        raw_first_text, raw_second_text,
        "raw debug output should stay stable while direct fragment fast path is used"
    );

    let second_profile: Value =
        serde_json::from_str(&fs::read_to_string(&profile_second).expect("read second profile"))
            .expect("parse second compile profile json");
    let direct_hits = second_profile["emit"]["breakdown"]["optimized_fragment_fast_path_direct_hits"]
        .as_u64()
        .unwrap_or(0);
    let final_artifact_hits = second_profile["emit"]["breakdown"]
        ["optimized_fragment_final_artifact_hits"]
        .as_u64()
        .unwrap_or(0);
    assert!(
        direct_hits + final_artifact_hits > 0,
        "expected direct fragment or optimized final artifact hit with raw debug output enabled"
    );
    assert_eq!(
        second_profile["emit"]["breakdown"]["peephole_elapsed_ns"]
            .as_u64()
            .unwrap_or(0),
        0,
        "direct fast path should still skip peephole"
    );
    assert!(
        second_profile["emit"]["breakdown"]["raw_rewrite_elapsed_ns"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "raw debug output should still materialize the raw rewrite stage"
    );
}

#[test]
fn no_incremental_cli_nontrivial_program_uses_optimized_final_artifact_after_cache_clear() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "opt_final_artifact");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let input = root
        .join("example")
        .join("data_science")
        .join("logistic_ensemble.rr");
    let out_file = proj_dir.join("out.R");
    let profile_first = proj_dir.join("first-profile.json");
    let profile_second = proj_dir.join("second-profile.json");
    let profile_third = proj_dir.join("third-profile.json");
    let cache_dir = proj_dir.join(".rr-cache");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    for profile in [&profile_first, &profile_second] {
        let status = Command::new(&rr_bin)
            .arg(&input)
            .arg("-o")
            .arg(&out_file)
            .arg("-O2")
            .arg("--no-runtime")
            .arg("--no-incremental")
            .arg("--profile-compile")
            .arg("--profile-compile-out")
            .arg(profile)
            .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
            .status()
            .expect("failed to seed optimized final artifact cache");
        assert!(status.success(), "optimized final artifact seed compile failed");
    }

    clear_whole_output_emit_caches(&cache_dir);

    let third = Command::new(&rr_bin)
        .arg(&input)
        .arg("-o")
        .arg(&out_file)
        .arg("-O2")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_third)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run optimized final artifact compile");
    assert!(third.success(), "optimized final artifact compile failed");

    let third_profile: Value =
        serde_json::from_str(&fs::read_to_string(&profile_third).expect("read third profile"))
            .expect("parse third compile profile json");
    assert_eq!(
        third_profile["emit"]["breakdown"]["optimized_fragment_final_artifact_hits"]
            .as_u64()
            .unwrap_or(0),
        1,
        "expected optimized final artifact hit for nontrivial program after raw/pee cache clear"
    );
    assert_eq!(
        third_profile["emit"]["breakdown"]["peephole_elapsed_ns"]
            .as_u64()
            .unwrap_or(0),
        0,
        "optimized final artifact hit should skip peephole"
    );
}

#[test]
fn no_incremental_cli_corrupted_optimized_final_artifact_falls_back_safely() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "corrupt_opt_final_artifact");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let input = root
        .join("example")
        .join("data_science")
        .join("logistic_ensemble.rr");
    let out_file = proj_dir.join("out.R");
    let profile_first = proj_dir.join("first-profile.json");
    let profile_second = proj_dir.join("second-profile.json");
    let profile_third = proj_dir.join("third-profile.json");
    let cache_dir = proj_dir.join(".rr-cache");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    for profile in [&profile_first, &profile_second] {
        let status = Command::new(&rr_bin)
            .arg(&input)
            .arg("-o")
            .arg(&out_file)
            .arg("-O2")
            .arg("--no-runtime")
            .arg("--no-incremental")
            .arg("--profile-compile")
            .arg("--profile-compile-out")
            .arg(profile)
            .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
            .status()
            .expect("failed to seed optimized final artifact cache");
        assert!(status.success(), "optimized final artifact seed compile failed");
    }

    let fn_cache_dir = cache_dir.join("function-emits");
    let mut corrupted_any = false;
    for entry in fs::read_dir(&fn_cache_dir)
        .expect("function cache dir should exist")
        .flatten()
    {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "Roptasm" || ext == "optasm.map")
        {
            fs::write(&path, "corrupted").expect("failed to corrupt optimized final artifact");
            corrupted_any = true;
        }
    }
    assert!(
        corrupted_any,
        "expected at least one optimized final artifact to corrupt in {}",
        fn_cache_dir.display()
    );

    clear_whole_output_emit_caches(&cache_dir);

    let third = Command::new(&rr_bin)
        .arg(&input)
        .arg("-o")
        .arg(&out_file)
        .arg("-O2")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_third)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run compile with corrupted optimized final artifact");
    assert!(
        third.success(),
        "compile should fall back when optimized final artifact is corrupted"
    );

    let third_profile: Value =
        serde_json::from_str(&fs::read_to_string(&profile_third).expect("read third profile"))
            .expect("parse third compile profile json");
    assert_eq!(
        third_profile["emit"]["breakdown"]["optimized_fragment_final_artifact_hits"]
            .as_u64()
            .unwrap_or(0),
        0,
        "corrupted optimized final artifact should be treated as a cache miss"
    );
    assert!(
        third_profile["emit"]["breakdown"]["peephole_elapsed_ns"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "fallback compile should run peephole after corrupted final artifact miss"
    );
}

#[test]
fn no_incremental_cli_signal_pipeline_warm_compile_skips_raw_and_peephole() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "signal_pipeline_warm");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let input = root
        .join("example")
        .join("benchmarks")
        .join("signal_pipeline_bench.rr");
    let out_file = proj_dir.join("out.R");
    let profile_path = proj_dir.join("warm-profile.json");
    let cache_dir = proj_dir.join(".rr-cache");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let seed = Command::new(&rr_bin)
        .arg(&input)
        .arg("-o")
        .arg(&out_file)
        .arg("-O2")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to seed signal pipeline warm cache");
    assert!(seed.success(), "signal pipeline warm seed compile failed");

    let warm = Command::new(&rr_bin)
        .arg(&input)
        .arg("-o")
        .arg(&out_file)
        .arg("-O2")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run signal pipeline warm compile");
    assert!(warm.success(), "signal pipeline warm compile failed");

    let profile: Value =
        serde_json::from_str(&fs::read_to_string(&profile_path).expect("read warm profile"))
            .expect("parse warm compile profile json");
    let direct_hits = profile["emit"]["breakdown"]["optimized_fragment_fast_path_direct_hits"]
        .as_u64()
        .unwrap_or(0);
    let final_artifact_hits = profile["emit"]["breakdown"]["optimized_fragment_final_artifact_hits"]
        .as_u64()
        .unwrap_or(0);
    assert!(
        direct_hits + final_artifact_hits > 0,
        "expected signal pipeline warm compile to hit an optimized reuse tier"
    );
    assert!(
        profile["tachyon"]["optimized_mir_cache_hit"]
            .as_bool()
            .unwrap_or(false),
        "expected signal pipeline warm compile to reuse optimized MIR"
    );
    assert_eq!(
        profile["emit"]["breakdown"]["raw_rewrite_elapsed_ns"]
            .as_u64()
            .unwrap_or(0),
        0,
        "signal pipeline warm compile should skip raw rewrite"
    );
    assert_eq!(
        profile["emit"]["breakdown"]["peephole_elapsed_ns"]
            .as_u64()
            .unwrap_or(0),
        0,
        "signal pipeline warm compile should skip peephole"
    );
}
