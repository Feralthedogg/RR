use super::*;
pub fn hir_ty_to_type_state(ty: &Ty) -> TypeState {
    from_hir_ty(ty)
}

pub fn hir_ty_to_type_term(ty: &Ty) -> TypeTerm {
    from_hir_ty_term(ty)
}

pub fn hir_ty_to_type_term_with_symbols(
    ty: &Ty,
    symbols: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> TypeTerm {
    term_from_hir_ty_with_symbols(ty, symbols)
}

pub(crate) fn compute_reachable(fn_ir: &FnIR) -> Vec<bool> {
    let mut reachable = vec![false; fn_ir.blocks.len()];
    if fn_ir.entry >= fn_ir.blocks.len() {
        return reachable;
    }
    let mut work = VecDeque::new();
    reachable[fn_ir.entry] = true;
    work.push_back(fn_ir.entry);

    while let Some(bb) = work.pop_front() {
        match fn_ir.blocks[bb].term {
            Terminator::Goto(t) => {
                if t < fn_ir.blocks.len() && !reachable[t] {
                    reachable[t] = true;
                    work.push_back(t);
                }
            }
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                if then_bb < fn_ir.blocks.len() && !reachable[then_bb] {
                    reachable[then_bb] = true;
                    work.push_back(then_bb);
                }
                if else_bb < fn_ir.blocks.len() && !reachable[else_bb] {
                    reachable[else_bb] = true;
                    work.push_back(else_bb);
                }
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }

    reachable
}

pub(crate) fn fn_ir_has_user_type_contract(fn_ir: &FnIR) -> bool {
    if fn_ir.ret_hint_span.is_some() {
        return true;
    }
    if fn_ir.param_hint_spans.len() == fn_ir.params.len() {
        return fn_ir.param_hint_spans.iter().any(Option::is_some);
    }

    // Hand-built MIR tests predate `param_hint_spans`; keep treating their
    // non-default hint vectors as explicit contracts.
    fn_ir.param_ty_hints.iter().any(|ty| !ty.is_unknown())
        || fn_ir.param_term_hints.iter().any(|term| !term.is_any())
}

pub(crate) fn param_slot_has_user_type_contract(fn_ir: &FnIR, slot: usize) -> bool {
    if fn_ir.param_hint_spans.len() == fn_ir.params.len() {
        return fn_ir
            .param_hint_spans
            .get(slot)
            .copied()
            .flatten()
            .is_some();
    }

    let has_ty_hint = fn_ir
        .param_ty_hints
        .get(slot)
        .is_some_and(|ty| !ty.is_unknown());
    let has_term_hint = fn_ir
        .param_term_hints
        .get(slot)
        .is_some_and(|term| !term.is_any());
    has_ty_hint || has_term_hint
}

pub(crate) fn is_arg_compatible(
    expected: TypeState,
    got: TypeState,
    explicit_param_hint: bool,
) -> bool {
    if expected.prim == PrimTy::Any || got.prim == PrimTy::Any {
        return true;
    }
    if expected.prim == got.prim {
        return true;
    }
    if !explicit_param_hint
        && matches!(
            (expected.prim, got.prim),
            (PrimTy::Int | PrimTy::Double, PrimTy::Int | PrimTy::Double)
        )
    {
        return true;
    }
    // Numeric widening accepted in strict call checking.
    matches!((expected.prim, got.prim), (PrimTy::Double, PrimTy::Int))
}
