#![no_main]

mod common;

use RR::compiler::{
    IncrementalOptions, IncrementalSession, OptLevel, ParallelConfig,
    compile_with_configs_incremental,
};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use libfuzzer_sys::{Corpus, fuzz_target};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn case_root(data: &[u8]) -> PathBuf {
    std::env::temp_dir()
        .join("rr-fuzz-incremental")
        .join(format!("{:016x}", stable_hash(&data)))
}

fn write_case(root: &Path, entry_src: &str, helper_src: Option<&str>) -> Option<PathBuf> {
    fs::create_dir_all(root).ok()?;
    let entry_path = root.join("entry.rr");
    fs::write(&entry_path, entry_src).ok()?;
    if let Some(helper) = helper_src {
        fs::write(root.join("helper.rr"), helper).ok()?;
    }
    Some(entry_path)
}

fn run_incremental(entry_path: &Path, entry_src: &str) {
    let parallel_cfg = ParallelConfig::default();
    let mut strict_all = IncrementalOptions::all_phases();
    strict_all.strict_verify = true;

    for cfg in [
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
        TypeConfig {
            mode: TypeMode::Gradual,
            native_backend: NativeBackend::Off,
        },
    ] {
        let _ = compile_with_configs_incremental(
            &entry_path.to_string_lossy(),
            entry_src,
            OptLevel::O1,
            cfg,
            parallel_cfg,
            IncrementalOptions::phase1_only(),
            None,
        );

        let mut session = IncrementalSession::default();
        let _ = compile_with_configs_incremental(
            &entry_path.to_string_lossy(),
            entry_src,
            OptLevel::O2,
            cfg,
            parallel_cfg,
            strict_all,
            Some(&mut session),
        );
        let _ = compile_with_configs_incremental(
            &entry_path.to_string_lossy(),
            entry_src,
            OptLevel::O2,
            cfg,
            parallel_cfg,
            strict_all,
            Some(&mut session),
        );
    }
}

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some(src) = common::decode_source(data) else {
        return Corpus::Reject;
    };

    let helper_src = r#"
fn helper_bias(x) {
  return x + 1;
}

fn helper_pick(flag) {
  if (flag) {
    return 2;
  }
  return 1;
}
"#;

    let root = case_root(data);
    let mut kept = false;

    for (idx, variant) in common::source_variants(src).into_iter().enumerate() {
        let base_root = root.join(format!("base_{idx}"));
        if let Some(entry_path) = write_case(&base_root, &variant, None) {
            run_incremental(&entry_path, &variant);
            kept = true;
        }

        let imported = format!("import \"helper.rr\";\n{variant}\n");
        let import_root = root.join(format!("import_{idx}"));
        if let Some(entry_path) = write_case(&import_root, &imported, Some(helper_src)) {
            run_incremental(&entry_path, &imported);
            kept = true;
        }
    }

    if kept { Corpus::Keep } else { Corpus::Reject }
});
