use super::*;
pub(crate) fn is_generalizable_expr(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Lit(_) => true,
        HirExpr::Unary { expr, .. } | HirExpr::Some(expr) | HirExpr::Ok(expr) => {
            is_generalizable_expr(expr)
        }
        HirExpr::Binary { lhs, rhs, .. } => {
            is_generalizable_expr(lhs) && is_generalizable_expr(rhs)
        }
        HirExpr::VectorLit(values) => values.iter().all(is_generalizable_expr),
        HirExpr::ListLit(fields) => fields.iter().all(|(_, value)| is_generalizable_expr(value)),
        HirExpr::None => true,
        HirExpr::Err(_)
        | HirExpr::Try(_)
        | HirExpr::Local(_)
        | HirExpr::Global(_, _)
        | HirExpr::Call(_)
        | HirExpr::TidyCall(_)
        | HirExpr::Index { .. }
        | HirExpr::Field { .. }
        | HirExpr::Block(_)
        | HirExpr::IfExpr { .. }
        | HirExpr::Match { .. }
        | HirExpr::Range { .. }
        | HirExpr::Unquote(_)
        | HirExpr::Column(_) => false,
    }
}

pub(crate) fn ret_hint_safe_for_fn(f: &HirFn) -> bool {
    ret_hint_safe_block(&f.body)
}

pub(crate) fn ret_hint_safe_block(block: &HirBlock) -> bool {
    block.stmts.iter().all(|stmt| match stmt {
        HirStmt::Let { init, .. } => init.as_ref().is_none_or(is_generalizable_expr),
        HirStmt::Expr { expr, .. } => ret_expr_safe(expr),
        HirStmt::Return { value, .. } => value.as_ref().is_none_or(ret_expr_safe),
        HirStmt::If {
            cond,
            then_blk,
            else_blk,
            ..
        } => {
            ret_expr_safe(cond)
                && ret_hint_safe_block(then_blk)
                && else_blk.as_ref().is_none_or(ret_hint_safe_block)
        }
        HirStmt::Assign { .. }
        | HirStmt::While { .. }
        | HirStmt::For { .. }
        | HirStmt::Break { .. }
        | HirStmt::Next { .. }
        | HirStmt::UnsafeRBlock { .. } => false,
    })
}

pub(crate) fn ret_expr_safe(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Local(_) | HirExpr::Lit(_) | HirExpr::None => true,
        HirExpr::Unary { expr, .. }
        | HirExpr::Some(expr)
        | HirExpr::Ok(expr)
        | HirExpr::Err(expr)
        | HirExpr::Try(expr)
        | HirExpr::Unquote(expr) => ret_expr_safe(expr),
        HirExpr::Binary { lhs, rhs, .. } => ret_expr_safe(lhs) && ret_expr_safe(rhs),
        HirExpr::IfExpr {
            cond,
            then_expr,
            else_expr,
        } => ret_expr_safe(cond) && ret_expr_safe(then_expr) && ret_expr_safe(else_expr),
        HirExpr::VectorLit(values) => values.iter().all(ret_expr_safe),
        HirExpr::ListLit(fields) => fields.iter().all(|(_, value)| ret_expr_safe(value)),
        HirExpr::Range { start, end } => ret_expr_safe(start) && ret_expr_safe(end),
        HirExpr::Global(_, _)
        | HirExpr::Call(_)
        | HirExpr::TidyCall(_)
        | HirExpr::Index { .. }
        | HirExpr::Field { .. }
        | HirExpr::Block(_)
        | HirExpr::Match { .. }
        | HirExpr::Column(_) => false,
    }
}

pub(crate) fn param_local_id(
    f: &HirFn,
    symbols: &FxHashMap<SymbolId, String>,
    param: SymbolId,
) -> Option<LocalId> {
    let name = symbol_name(symbols, param)?;
    f.local_names
        .iter()
        .find_map(|(local, local_name)| (local_name == &name).then_some(*local))
}

pub(crate) fn symbol_name(symbols: &FxHashMap<SymbolId, String>, sym: SymbolId) -> Option<String> {
    symbols.get(&sym).cloned()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hir::def::{FnId, HirBlock, HirFnAttrs, HirModule, InlineHint, ModuleId};
    use crate::hir::def::{HirBinOp, HirItem, HirLit, HirProgram, Ty};
    use crate::utils::Span;

    #[test]
    fn unifier_rejects_recursive_type() {
        let mut subst = Subst::new();
        let err = unify(
            &HmTy::Var(0),
            &HmTy::Vector(Box::new(HmTy::Var(0))),
            &mut subst,
        )
        .unwrap_err();
        assert!(matches!(err, UnifyError::Occurs { var: 0, .. }));
    }

    #[test]
    fn hm_pass_fills_literal_let_and_return_hints() {
        let mut symbols = FxHashMap::default();
        symbols.insert(SymbolId(1), "demo".to_string());
        symbols.insert(SymbolId(2), "x".to_string());
        let local = LocalId(1);
        let mut local_names = FxHashMap::default();
        local_names.insert(local, "x".to_string());
        let mut program = HirProgram {
            modules: vec![HirModule {
                id: ModuleId(0),
                path: Vec::new(),
                items: vec![HirItem::Fn(HirFn {
                    id: FnId(1),
                    name: SymbolId(1),
                    type_params: Vec::new(),
                    where_bounds: Vec::new(),
                    params: Vec::new(),
                    has_varargs: false,
                    ret_ty: None,
                    ret_ty_inferred: false,
                    body: HirBlock {
                        stmts: vec![
                            HirStmt::Let {
                                local,
                                name: SymbolId(2),
                                ty: None,
                                init: Some(HirExpr::Lit(HirLit::Double(1.0))),
                                span: Span::default(),
                            },
                            HirStmt::Expr {
                                expr: HirExpr::Binary {
                                    op: HirBinOp::Add,
                                    lhs: Box::new(HirExpr::Local(local)),
                                    rhs: Box::new(HirExpr::Lit(HirLit::Double(2.0))),
                                },
                                span: Span::default(),
                            },
                        ],
                        span: Span::default(),
                    },
                    attrs: HirFnAttrs {
                        inline_hint: InlineHint::Default,
                        tidy_safe: false,
                    },
                    span: Span::default(),
                    local_names,
                    public: false,
                })],
            }],
        };

        apply_hm_hints(&mut program, &symbols);
        let HirItem::Fn(f) = &program.modules[0].items[0] else {
            panic!("expected fn");
        };
        assert_eq!(f.ret_ty, Some(Ty::Double));
        assert!(f.ret_ty_inferred);
        let HirStmt::Let { ty, .. } = &f.body.stmts[0] else {
            panic!("expected let");
        };
        assert_eq!(*ty, Some(Ty::Double));
    }

    #[test]
    fn hm_pass_closes_polymorphic_function_scheme() {
        let mut symbols = FxHashMap::default();
        symbols.insert(SymbolId(1), "id".to_string());
        symbols.insert(SymbolId(2), "x".to_string());
        let local = LocalId(1);
        let mut local_names = FxHashMap::default();
        local_names.insert(local, "x".to_string());
        let mut program = HirProgram {
            modules: vec![HirModule {
                id: ModuleId(0),
                path: Vec::new(),
                items: vec![HirItem::Fn(HirFn {
                    id: FnId(1),
                    name: SymbolId(1),
                    type_params: Vec::new(),
                    where_bounds: Vec::new(),
                    params: vec![crate::hir::def::HirParam {
                        name: SymbolId(2),
                        ty: None,
                        ty_inferred: false,
                        default: None,
                        span: Span::default(),
                    }],
                    has_varargs: false,
                    ret_ty: None,
                    ret_ty_inferred: false,
                    body: HirBlock {
                        stmts: vec![HirStmt::Expr {
                            expr: HirExpr::Local(local),
                            span: Span::default(),
                        }],
                        span: Span::default(),
                    },
                    attrs: HirFnAttrs {
                        inline_hint: InlineHint::Default,
                        tidy_safe: false,
                    },
                    span: Span::default(),
                    local_names,
                    public: false,
                })],
            }],
        };

        let globals = apply_hm_hints(&mut program, &symbols);
        let scheme = globals.get("id").expect("id scheme");
        assert_eq!(scheme.vars.len(), 1);
        let HmTy::Function(params, ret) = &scheme.ty else {
            panic!("expected function scheme");
        };
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], **ret);
        assert_eq!(params[0], HmTy::Var(scheme.vars[0]));
    }

    #[test]
    fn hm_builtin_is_na_preserves_vector_shape() {
        assert_eq!(
            infer_builtin_call_hint("is.na", &[HmTy::Vector(Box::new(HmTy::Int))]),
            Some(HmTy::Vector(Box::new(HmTy::Logical)))
        );
        assert_eq!(
            infer_builtin_call_hint("base::is.finite", &[HmTy::Double]),
            Some(HmTy::Logical)
        );
    }

    #[test]
    fn hm_binary_logical_results_preserve_vector_shape() {
        assert_eq!(
            binary_result(HirBinOp::Gt, &HmTy::Vector(Box::new(HmTy::Int)), &HmTy::Int),
            HmTy::Vector(Box::new(HmTy::Logical))
        );
        assert_eq!(
            binary_result(
                HirBinOp::Or,
                &HmTy::Vector(Box::new(HmTy::Logical)),
                &HmTy::Logical,
            ),
            HmTy::Vector(Box::new(HmTy::Logical))
        );
        assert_eq!(
            binary_result(HirBinOp::Eq, &HmTy::Int, &HmTy::Int),
            HmTy::Logical
        );
        assert_eq!(
            binary_result(
                HirBinOp::Lt,
                &HmTy::Matrix(Box::new(HmTy::Double)),
                &HmTy::Double,
            ),
            HmTy::Matrix(Box::new(HmTy::Logical))
        );

        let mut env = InferEnv::new(FxHashMap::default());
        assert_eq!(
            env.infer_binary(
                &HirBinOp::And,
                HmTy::Vector(Box::new(HmTy::Logical)),
                HmTy::Vector(Box::new(HmTy::Logical)),
            ),
            HmTy::Vector(Box::new(HmTy::Logical))
        );
    }

    #[test]
    fn hm_binary_numeric_constraint_fills_param_hint() {
        let mut symbols = FxHashMap::default();
        symbols.insert(SymbolId(1), "inc".to_string());
        symbols.insert(SymbolId(2), "x".to_string());
        let local = LocalId(1);
        let mut local_names = FxHashMap::default();
        local_names.insert(local, "x".to_string());
        let mut program = HirProgram {
            modules: vec![HirModule {
                id: ModuleId(0),
                path: Vec::new(),
                items: vec![HirItem::Fn(HirFn {
                    id: FnId(1),
                    name: SymbolId(1),
                    type_params: Vec::new(),
                    where_bounds: Vec::new(),
                    params: vec![crate::hir::def::HirParam {
                        name: SymbolId(2),
                        ty: None,
                        ty_inferred: false,
                        default: None,
                        span: Span::default(),
                    }],
                    has_varargs: false,
                    ret_ty: None,
                    ret_ty_inferred: false,
                    body: HirBlock {
                        stmts: vec![HirStmt::Expr {
                            expr: HirExpr::Binary {
                                op: HirBinOp::Add,
                                lhs: Box::new(HirExpr::Local(local)),
                                rhs: Box::new(HirExpr::Lit(HirLit::Int(1))),
                            },
                            span: Span::default(),
                        }],
                        span: Span::default(),
                    },
                    attrs: HirFnAttrs {
                        inline_hint: InlineHint::Default,
                        tidy_safe: false,
                    },
                    span: Span::default(),
                    local_names,
                    public: false,
                })],
            }],
        };

        apply_hm_hints(&mut program, &symbols);
        let HirItem::Fn(f) = &program.modules[0].items[0] else {
            panic!("expected fn");
        };
        assert_eq!(f.params[0].ty, Some(Ty::Int));
        assert!(f.params[0].ty_inferred);
        assert_eq!(f.ret_ty, Some(Ty::Int));
        assert!(f.ret_ty_inferred);
    }

    #[test]
    fn hm_mod_constraint_widens_unannotated_param_hint() {
        let mut symbols = FxHashMap::default();
        symbols.insert(SymbolId(1), "bucket".to_string());
        symbols.insert(SymbolId(2), "x".to_string());
        let local = LocalId(1);
        let mut local_names = FxHashMap::default();
        local_names.insert(local, "x".to_string());
        let mut program = HirProgram {
            modules: vec![HirModule {
                id: ModuleId(0),
                path: Vec::new(),
                items: vec![HirItem::Fn(HirFn {
                    id: FnId(1),
                    name: SymbolId(1),
                    type_params: Vec::new(),
                    where_bounds: Vec::new(),
                    params: vec![crate::hir::def::HirParam {
                        name: SymbolId(2),
                        ty: None,
                        ty_inferred: false,
                        default: None,
                        span: Span::default(),
                    }],
                    has_varargs: false,
                    ret_ty: None,
                    ret_ty_inferred: false,
                    body: HirBlock {
                        stmts: vec![HirStmt::Expr {
                            expr: HirExpr::Binary {
                                op: HirBinOp::Mod,
                                lhs: Box::new(HirExpr::Local(local)),
                                rhs: Box::new(HirExpr::Lit(HirLit::Int(2))),
                            },
                            span: Span::default(),
                        }],
                        span: Span::default(),
                    },
                    attrs: HirFnAttrs {
                        inline_hint: InlineHint::Default,
                        tidy_safe: false,
                    },
                    span: Span::default(),
                    local_names,
                    public: false,
                })],
            }],
        };

        apply_hm_hints(&mut program, &symbols);
        let HirItem::Fn(f) = &program.modules[0].items[0] else {
            panic!("expected fn");
        };
        assert_eq!(f.params[0].ty, Some(Ty::Double));
        assert!(f.params[0].ty_inferred);
        assert_eq!(f.ret_ty, Some(Ty::Double));
        assert!(f.ret_ty_inferred);
    }
}
