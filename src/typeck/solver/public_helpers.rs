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

fn compute_reachable(fn_ir: &FnIR) -> Vec<bool> {
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

fn is_arg_compatible(expected: TypeState, got: TypeState) -> bool {
    if expected.prim == PrimTy::Any || got.prim == PrimTy::Any {
        return true;
    }
    if expected.prim == got.prim {
        return true;
    }
    // Numeric widening accepted in strict call checking.
    matches!((expected.prim, got.prim), (PrimTy::Double, PrimTy::Int))
}
