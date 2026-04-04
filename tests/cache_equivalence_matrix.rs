use RR::compiler::{
    CompileOutputOptions, IncrementalOptions, IncrementalSession, OptLevel,
    compile_with_configs_incremental_with_output_options, compile_with_configs_with_options,
    default_parallel_config, default_type_config,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn unique_dir(root: &std::path::Path, name: &str) -> PathBuf {
    static UNIQUE_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let seq = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    root.join(format!("{}_{}_{}", name, std::process::id(), seq))
}

#[test]
fn cache_modes_preserve_emitted_output_per_output_mode() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("cache_equivalence_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "matrix");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let rr_path = proj.join("main.rr");
    let src = r#"
fn helper(x) {
  return x + 1L
}

fn main() {
  let xs = seq_len(8L)
  let ys = xs
  for (i in 1..length(xs)) {
    ys[i] = helper(xs[i]) * 2L
  }
  print(sum(ys))
}

main()
"#;
    fs::write(&rr_path, src).expect("failed to write RR source");
    let entry = rr_path.to_str().expect("non-utf8 path");

    let output_modes = [
        (
            "runtime",
            CompileOutputOptions {
                inject_runtime: true,
                preserve_all_defs: false,
                ..CompileOutputOptions::default()
            },
        ),
        (
            "helper_only",
            CompileOutputOptions {
                inject_runtime: false,
                preserve_all_defs: false,
                ..CompileOutputOptions::default()
            },
        ),
        (
            "preserve_all_defs",
            CompileOutputOptions {
                inject_runtime: true,
                preserve_all_defs: true,
                ..CompileOutputOptions::default()
            },
        ),
    ];

    for (label, output_opts) in output_modes {
        let baseline = compile_with_configs_with_options(
            entry,
            src,
            OptLevel::O2,
            default_type_config(),
            default_parallel_config(),
            output_opts,
        )
        .expect("baseline compile should succeed");

        let mut session = IncrementalSession::default();
        let first = compile_with_configs_incremental_with_output_options(
            entry,
            src,
            OptLevel::O2,
            default_type_config(),
            default_parallel_config(),
            IncrementalOptions::all_phases(),
            output_opts,
            Some(&mut session),
        )
        .expect("first incremental compile should succeed");
        let second = compile_with_configs_incremental_with_output_options(
            entry,
            src,
            OptLevel::O2,
            default_type_config(),
            default_parallel_config(),
            IncrementalOptions::all_phases(),
            output_opts,
            Some(&mut session),
        )
        .expect("second incremental compile should succeed");
        let strict = compile_with_configs_incremental_with_output_options(
            entry,
            src,
            OptLevel::O2,
            default_type_config(),
            default_parallel_config(),
            IncrementalOptions {
                strict_verify: true,
                ..IncrementalOptions::all_phases()
            },
            output_opts,
            Some(&mut session),
        )
        .expect("strict incremental seed compile should succeed");
        let strict_hit = compile_with_configs_incremental_with_output_options(
            entry,
            src,
            OptLevel::O2,
            default_type_config(),
            default_parallel_config(),
            IncrementalOptions {
                strict_verify: true,
                ..IncrementalOptions::all_phases()
            },
            output_opts,
            Some(&mut session),
        )
        .expect("strict incremental hit compile should succeed");

        assert_eq!(
            baseline.0, first.r_code,
            "{label}: incremental first compile changed emitted code"
        );
        assert_eq!(
            baseline.0, second.r_code,
            "{label}: incremental cache hit changed emitted code"
        );
        assert_eq!(
            baseline.0, strict.r_code,
            "{label}: strict incremental verify changed emitted code"
        );
        assert_eq!(
            baseline.0, strict_hit.r_code,
            "{label}: strict incremental verify hit changed emitted code"
        );
        assert_eq!(
            baseline.1, first.source_map,
            "{label}: incremental first compile changed source map"
        );
        assert_eq!(
            baseline.1, second.source_map,
            "{label}: incremental cache hit changed source map"
        );
        assert_eq!(
            baseline.1, strict.source_map,
            "{label}: strict incremental verify changed source map"
        );
        assert_eq!(
            baseline.1, strict_hit.source_map,
            "{label}: strict incremental verify hit changed source map"
        );
        assert!(
            second.stats.phase1_artifact_hit || second.stats.phase3_memory_hit,
            "{label}: expected cache reuse on second incremental compile"
        );
        assert!(
            strict_hit.stats.strict_verification_checked
                && strict_hit.stats.strict_verification_passed,
            "{label}: strict verify hit should have executed and passed"
        );
    }
}
