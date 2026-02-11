use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use RR::hir::def::{HirItem, HirProgram, ModuleId};
use RR::hir::desugar::Desugarer;
use RR::hir::lower::Lowerer;
use RR::mir::lower_hir::MirLowerer;
use RR::mir::opt::TachyonEngine;
use RR::syntax::parse::Parser;

fn build_mir_without_semantic_gate(src: &str) -> HashMap<String, RR::mir::FnIR> {
    let mut parser = Parser::new(src);
    let ast = parser
        .parse_program()
        .expect("parse must succeed for regression input");

    let mut lowerer = Lowerer::new();
    let (hir_mod, symbols) = lowerer
        .lower_module(ast, ModuleId(0))
        .expect("hir lowering must succeed for regression input");

    let mut known_fn_arities: FxHashMap<String, usize> = FxHashMap::default();
    for item in &hir_mod.items {
        if let HirItem::Fn(f) = item
            && let Some(name) = symbols.get(&f.name).cloned()
        {
            known_fn_arities.insert(name, f.params.len());
        }
    }
    let known_fn_arities_std: HashMap<String, usize> = known_fn_arities
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    let mut desugarer = Desugarer::new();
    let desugared = desugarer
        .desugar_program(HirProgram {
            modules: vec![hir_mod],
        })
        .expect("desugar must succeed for regression input");

    let mut all_fns = HashMap::new();
    for module in desugared.modules {
        for item in module.items {
            if let HirItem::Fn(f) = item {
                let fn_name = format!("Sym_{}", f.name.0);
                let params: Vec<String> = f
                    .params
                    .iter()
                    .filter_map(|p| symbols.get(&p.name).cloned())
                    .collect();
                if params.len() != f.params.len() {
                    continue;
                }
                let var_names = f.local_names.clone().into_iter().collect();
                let mir_lowerer = MirLowerer::new(
                    fn_name.clone(),
                    params,
                    var_names,
                    &symbols,
                    &known_fn_arities_std,
                );
                if let Ok(fn_ir) = mir_lowerer.lower_fn(f) {
                    all_fns.insert(fn_name, fn_ir);
                }
            }
        }
    }
    all_fns
}

#[test]
fn invalid_mir_from_undefined_variable_is_rejected_without_panic() {
    let src = r#"
fn inc(x) {
  return x + 1L;
}

fn main() {
  let x = seq_len(8L);
  let y = seq_len(4L);
  for (i in 1L..length(x)) {
    y[i] = inc(p[i]);
  }
  return sum(y);
}
"#;

    let all_fns = build_mir_without_semantic_gate(src);
    assert!(!all_fns.is_empty(), "expected lowered functions");
    assert!(
        all_fns
            .values()
            .any(|f| RR::mir::verify::verify_ir(f).is_err()),
        "regression setup must contain invalid MIR before optimization"
    );

    let mut stabilized = all_fns.clone();
    let stabilize = catch_unwind(AssertUnwindSafe(|| {
        TachyonEngine::new().stabilize_for_codegen(&mut stabilized);
    }));
    assert!(
        stabilize.is_ok(),
        "stabilize_for_codegen must not panic on invalid MIR"
    );
    assert!(
        stabilized.values().any(|f| {
            f.unsupported_dynamic
                && f.fallback_reasons
                    .iter()
                    .any(|r| r.contains("invalid MIR at PrepareForCodegen/Start"))
        }),
        "invalid MIR should be marked as unsupported_dynamic with a reason"
    );

    let mut optimized = all_fns;
    let optimize = catch_unwind(AssertUnwindSafe(|| {
        let _ = TachyonEngine::new().run_program_with_stats(&mut optimized);
    }));
    assert!(
        optimize.is_ok(),
        "run_program_with_stats must not panic on invalid MIR"
    );
}
