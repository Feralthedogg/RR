#![no_main]

use rustc_hash::FxHashMap;
use std::collections::HashMap;

use RR::codegen::mir_emit::MirEmitter;
use RR::hir::def::{HirItem, HirProgram, ModuleId};
use RR::hir::desugar::Desugarer;
use RR::hir::lower::Lowerer;
use RR::mir::lower_hir::MirLowerer;
use RR::mir::opt::TachyonEngine;
use RR::syntax::parse::Parser;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else {
        return;
    };

    let mut parser = Parser::new(src);
    let ast = match parser.parse_program() {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut lowerer = Lowerer::new();
    let (hir_mod, symbols) = match lowerer.lower_module(ast, ModuleId(0)) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut known_fn_arities: FxHashMap<String, usize> = FxHashMap::default();
    for item in &hir_mod.items {
        if let HirItem::Fn(f) = item {
            if let Some(name) = symbols.get(&f.name).cloned() {
                known_fn_arities.insert(name, f.params.len());
            }
        }
    }
    let known_fn_arities_std: HashMap<String, usize> = known_fn_arities
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    let hir_prog = HirProgram {
        modules: vec![hir_mod],
    };
    let mut desugarer = Desugarer::new();
    let desugared = match desugarer.desugar_program(hir_prog) {
        Ok(v) => v,
        Err(_) => return,
    };

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
                let Ok(fn_ir) = mir_lowerer.lower_fn(f) else {
                    continue;
                };
                all_fns.insert(fn_name, fn_ir);
            }
        }
    }

    if all_fns.is_empty() {
        return;
    }

    if RR::mir::semantics::validate_program(&all_fns).is_err() {
        return;
    }
    if RR::mir::semantics::validate_runtime_safety(&all_fns).is_err() {
        return;
    }

    let mut optimized = all_fns;
    TachyonEngine::new().stabilize_for_codegen(&mut optimized);

    let mut emitter = MirEmitter::new();
    for fn_ir in optimized.values() {
        let _ = emitter.emit(fn_ir);
    }
});
