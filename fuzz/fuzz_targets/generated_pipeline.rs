#![no_main]

mod common;
#[allow(dead_code)]
#[path = "../../tests/common/random_rr.rs"]
mod random_rr;

use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use libfuzzer_sys::{Corpus, fuzz_target};

fn decode_seed(data: &[u8]) -> (u64, usize) {
    let mut seed_bytes = [0u8; 8];
    for (idx, byte) in data.iter().take(8).enumerate() {
        seed_bytes[idx] = *byte;
    }
    let seed = u64::from_le_bytes(seed_bytes);
    let count = if data.len() > 8 {
        1 + (data[8] as usize % 12)
    } else {
        6
    };
    (seed, count)
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.is_empty() {
        return Corpus::Reject;
    }

    let (seed, count) = decode_seed(data);
    let cases = random_rr::generate_cases(seed, count);
    let mut kept_any = false;

    for case in cases {
        let Some(all_fns) = common::build_mir(&case.rr_src) else {
            continue;
        };
        if all_fns.is_empty() {
            continue;
        }
        kept_any = true;

        for cfg in [
            TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Off,
            },
            TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Off,
            },
            TypeConfig {
                mode: TypeMode::Strict,
                native_backend: NativeBackend::Optional,
            },
        ] {
            common::run_full_pipeline(&all_fns, cfg);
        }
    }

    if kept_any {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});
