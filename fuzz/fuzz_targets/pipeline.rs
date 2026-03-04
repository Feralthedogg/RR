#![no_main]

mod common;

use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some(src) = common::decode_source(data) else {
        return Corpus::Reject;
    };

    let mut kept_any = false;
    for variant in common::source_variants(src) {
        let Some(all_fns) = common::build_mir(&variant) else {
            continue;
        };
        if all_fns.is_empty() {
            continue;
        }
        kept_any = true;

        // Exercise the same MIR pipeline under several type/native policies.
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
