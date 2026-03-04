#![no_main]

mod common;

use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use libfuzzer_sys::{Corpus, fuzz_target};

fn synthesize_type_program(data: &[u8]) -> String {
    let a = (data.first().copied().unwrap_or(3) % 7 + 1) as i64;
    let b = (data.get(1).copied().unwrap_or(5) % 9 + 1) as i64;
    let n = (data.get(2).copied().unwrap_or(8) % 12 + 1) as i64;
    let flag = if data.get(3).copied().unwrap_or(0) % 2 == 0 {
        "TRUE"
    } else {
        "FALSE"
    };
    let ret_left = if data.get(4).copied().unwrap_or(0) % 2 == 0 {
        "sum(y)"
    } else {
        "mean(y)"
    };
    let ret_right = if data.get(5).copied().unwrap_or(0) % 2 == 0 {
        "mean(y)"
    } else {
        "sum(y)"
    };

    format!(
        r#"
fn ok_count(xs: list<vector<float>>) -> int {{
  return length(xs);
}}

fn mixed(xs: list<box<float>>, n: int, flag: bool) -> float {{
  let x = seq_len(n);
  let y = abs((x * {a}L) + {b}L);
  if (flag) {{
    return {ret_left};
  }} else {{
    return {ret_right};
  }}
}}

fn main() -> float {{
  let seed = list(c(1.0, 2.0), c(3.0));
  print(ok_count(seed));
  return mixed(seed, {n}L, {flag});
}}

print(main());
"#
    )
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.is_empty() {
        return Corpus::Reject;
    }
    let src = synthesize_type_program(data);

    let Some(all_fns) = common::build_mir(&src) else {
        return Corpus::Reject;
    };
    if all_fns.is_empty() {
        return Corpus::Reject;
    }

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

    Corpus::Keep
});
