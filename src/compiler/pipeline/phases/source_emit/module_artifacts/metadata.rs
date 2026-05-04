use super::*;
pub(crate) fn desugar_single_module(
    module: crate::hir::def::HirModule,
) -> crate::error::RR<crate::hir::def::HirModule> {
    let mut desugarer = crate::hir::desugar::Desugarer::new();
    let mut program = desugarer.desugar_program(crate::hir::def::HirProgram {
        modules: vec![module],
    })?;
    program.modules.pop().ok_or_else(|| {
        InternalCompilerError::new(Stage::Lower, "desugarer produced no module").into_exception()
    })
}

pub(crate) fn collect_public_symbols_from_module(
    module: &crate::hir::def::HirModule,
    symbol_map: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> Vec<String> {
    let mut names = Vec::new();
    for item in &module.items {
        if let crate::hir::def::HirItem::Fn(f) = item
            && f.public
            && f.type_params.is_empty()
            && let Some(name) = symbol_map.get(&f.name)
        {
            names.push(name.clone());
        }
    }
    names.sort();
    names.dedup();
    names
}

pub(crate) fn collect_public_function_arities(
    module: &crate::hir::def::HirModule,
    symbol_map: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for item in &module.items {
        if let crate::hir::def::HirItem::Fn(f) = item
            && f.public
            && f.type_params.is_empty()
            && let Some(name) = symbol_map.get(&f.name)
        {
            out.push((name.clone(), f.params.len()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

pub(crate) fn collect_emit_roots(
    module: &crate::hir::def::HirModule,
    symbol_map: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> Vec<String> {
    let mut out = collect_public_symbols_from_module(module, symbol_map);
    if module
        .items
        .iter()
        .any(|item| matches!(item, crate::hir::def::HirItem::Stmt(_)))
    {
        out.push(format!("Sym_top_{}", module.id.0));
    }
    out.sort();
    out.dedup();
    out
}
