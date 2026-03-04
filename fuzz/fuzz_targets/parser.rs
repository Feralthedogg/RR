#![no_main]

mod common;

use RR::syntax::parse::Parser;
use libfuzzer_sys::{Corpus, fuzz_target};

fn parse_one(src: &str) {
    let mut parser = Parser::new(src);
    let _ = parser.parse_program();
}

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some(src) = common::decode_source(data) else {
        return Corpus::Reject;
    };

    for variant in common::source_variants(src) {
        parse_one(&variant);
    }

    Corpus::Keep
});
