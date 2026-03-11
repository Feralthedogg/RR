#![no_main]

#[path = "../../tests/common/random_error_cases.rs"]
mod random_error_cases;

use RR::compiler::{OptLevel, ParallelConfig, compile_with_configs};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use libfuzzer_sys::{Corpus, fuzz_target};
use std::sync::Once;

fn seed_from_bytes(data: &[u8]) -> u64 {
    let mut seed = 0xA11C_EBAD_F00D_5EEDu64;
    for &byte in data.iter().take(32) {
        seed = seed.rotate_left(7) ^ u64::from(byte);
        seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    seed
}

fn ensure_quiet_log() {
    static QUIET: Once = Once::new();
    QUIET.call_once(|| {
        // SAFETY: libFuzzer drives this target on a single thread, and this
        // process only uses RR_QUIET_LOG to suppress noisy progress output.
        unsafe {
            std::env::set_var("RR_QUIET_LOG", "1");
        }
    });
}

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: this target mutates process env only inside libFuzzer's
        // single-threaded execution loop so Lowerer can read RR_STRICT_LET.
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: restores the same single-threaded env mutation described above.
        unsafe {
            match &self.prev {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

fn assert_expected_error(
    case: &random_error_cases::GeneratedErrorCase,
    opt_level: OptLevel,
    result: RR::error::RR<(String, Vec<RR::codegen::mir_emit::MapEntry>)>,
) {
    match result {
        Ok((code, _map)) => {
            panic!(
                "invalid case unexpectedly compiled: {} ({})\nsource:\n{}\nemitted:\n{}",
                case.name,
                opt_level.label(),
                case.rr_src,
                code
            );
        }
        Err(err) => {
            assert_eq!(
                err.module,
                case.expected_module,
                "wrong diagnostic module for {} ({})",
                case.name,
                opt_level.label()
            );
            assert!(
                err.message
                    .as_ref()
                    .contains(case.expected_message_fragment.as_str()),
                "missing diagnostic fragment for {} ({})",
                case.name,
                opt_level.label()
            );
            if let Some(help_fragment) = &case.expected_help_fragment {
                assert!(
                    err.helps.iter().any(|help| help.contains(help_fragment)),
                    "missing suggestion for {} ({})",
                    case.name,
                    opt_level.label()
                );
            }
            assert_ne!(
                err.module,
                "RR.InternalError",
                "case triggered internal compiler error: {} ({})",
                case.name,
                opt_level.label()
            );
        }
    }
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.is_empty() {
        return Corpus::Reject;
    }

    ensure_quiet_log();

    let count = (usize::from(data[0]) % 8) + 1;
    let seed = seed_from_bytes(data);
    let cases = random_error_cases::generate_cases(seed, count);
    let type_cfg = TypeConfig {
        mode: TypeMode::Strict,
        native_backend: NativeBackend::Off,
    };
    let parallel_cfg = ParallelConfig::default();

    for case in &cases {
        let _strict_let = EnvGuard::set("RR_STRICT_LET", case.strict_let.then_some("1"));
        for opt_level in [OptLevel::O0, OptLevel::O2] {
            let entry_path = format!("fuzz/{}.rr", case.name);
            let result =
                compile_with_configs(&entry_path, &case.rr_src, opt_level, type_cfg, parallel_cfg);
            assert_expected_error(case, opt_level, result);
        }
    }

    Corpus::Keep
});
