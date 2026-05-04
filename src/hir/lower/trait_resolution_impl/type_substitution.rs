use super::*;
impl Lowerer {
    pub(crate) fn type_ref_to_ast_type(ty: &HirTypeRef) -> ast::TypeExpr {
        match ty {
            HirTypeRef::Named(name) => ast::TypeExpr::Named(name.clone()),
            HirTypeRef::Generic { base, args } => ast::TypeExpr::Generic {
                base: base.clone(),
                args: args.iter().map(Self::type_ref_to_ast_type).collect(),
            },
        }
    }
    pub(crate) fn const_int_from_type_ref(ty: &HirTypeRef) -> Option<i64> {
        let HirTypeRef::Named(name) = ty else {
            return None;
        };
        name.strip_prefix('#')?.parse().ok()
    }
    pub(crate) fn substitute_type_expr(
        ty: ast::TypeExpr,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::TypeExpr {
        match ty {
            ast::TypeExpr::Named(name) => subst
                .get(&name)
                .map(Self::type_ref_to_ast_type)
                .unwrap_or(ast::TypeExpr::Named(name)),
            ast::TypeExpr::Generic { base, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| Self::substitute_type_expr(arg, subst))
                    .collect::<Vec<_>>();
                let key = format!(
                    "{}<{}>",
                    base,
                    args.iter()
                        .map(Self::type_expr_key_for_subst)
                        .collect::<Vec<_>>()
                        .join(",")
                );
                subst
                    .get(&key)
                    .map(Self::type_ref_to_ast_type)
                    .unwrap_or(ast::TypeExpr::Generic { base, args })
            }
        }
    }
    pub(crate) fn type_expr_key_for_subst(ty: &ast::TypeExpr) -> String {
        match ty {
            ast::TypeExpr::Named(name) => name.clone(),
            ast::TypeExpr::Generic { base, args } => format!(
                "{}<{}>",
                base,
                args.iter()
                    .map(Self::type_expr_key_for_subst)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
    pub(crate) fn substitute_fn_param_type(
        mut param: ast::FnParam,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::FnParam {
        param.ty_hint = param
            .ty_hint
            .map(|ty| Self::substitute_type_expr(ty, subst));
        if let Some(default) = param.default {
            param.default = Some(Self::substitute_expr_type_hints(default, subst));
        }
        param
    }
    pub(crate) fn substitute_stmt_type_hints(
        mut stmt: ast::Stmt,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::Stmt {
        stmt.kind = match stmt.kind {
            ast::StmtKind::Let {
                name,
                ty_hint,
                init,
            } => ast::StmtKind::Let {
                name,
                ty_hint: ty_hint.map(|ty| Self::substitute_type_expr(ty, subst)),
                init: init.map(|expr| Self::substitute_expr_type_hints(expr, subst)),
            },
            ast::StmtKind::Assign { target, value } => ast::StmtKind::Assign {
                target,
                value: Self::substitute_expr_type_hints(value, subst),
            },
            ast::StmtKind::FnDecl {
                name,
                type_params,
                params,
                ret_ty_hint,
                where_bounds,
                body,
            } => ast::StmtKind::FnDecl {
                name,
                type_params,
                params: params
                    .into_iter()
                    .map(|param| Self::substitute_fn_param_type(param, subst))
                    .collect(),
                ret_ty_hint: ret_ty_hint.map(|ty| Self::substitute_type_expr(ty, subst)),
                where_bounds,
                body: Self::substitute_block_type_hints(body, subst),
            },
            ast::StmtKind::If {
                cond,
                then_blk,
                else_blk,
            } => ast::StmtKind::If {
                cond: Self::substitute_expr_type_hints(cond, subst),
                then_blk: Self::substitute_block_type_hints(then_blk, subst),
                else_blk: else_blk.map(|blk| Self::substitute_block_type_hints(blk, subst)),
            },
            ast::StmtKind::While { cond, body } => ast::StmtKind::While {
                cond: Self::substitute_expr_type_hints(cond, subst),
                body: Self::substitute_block_type_hints(body, subst),
            },
            ast::StmtKind::For { var, iter, body } => ast::StmtKind::For {
                var,
                iter: Self::substitute_expr_type_hints(iter, subst),
                body: Self::substitute_block_type_hints(body, subst),
            },
            ast::StmtKind::Return { value } => ast::StmtKind::Return {
                value: value.map(|expr| Self::substitute_expr_type_hints(expr, subst)),
            },
            ast::StmtKind::ExprStmt { expr } => ast::StmtKind::ExprStmt {
                expr: Self::substitute_expr_type_hints(expr, subst),
            },
            ast::StmtKind::Expr(expr) => {
                ast::StmtKind::Expr(Self::substitute_expr_type_hints(expr, subst))
            }
            other => other,
        };
        stmt
    }
    pub(crate) fn substitute_block_type_hints(
        mut block: ast::Block,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::Block {
        block.stmts = block
            .stmts
            .into_iter()
            .map(|stmt| Self::substitute_stmt_type_hints(stmt, subst))
            .collect();
        block
    }
    pub(crate) fn substitute_expr_type_hints(
        mut expr: ast::Expr,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::Expr {
        expr.kind = match expr.kind {
            ast::ExprKind::Name(name) => subst
                .get(&name)
                .and_then(Self::const_int_from_type_ref)
                .map(|value| ast::ExprKind::Lit(ast::Lit::Int(value)))
                .unwrap_or(ast::ExprKind::Name(name)),
            ast::ExprKind::Unary { op, rhs } => ast::ExprKind::Unary {
                op,
                rhs: Box::new(Self::substitute_expr_type_hints(*rhs, subst)),
            },
            ast::ExprKind::Formula { lhs, rhs } => ast::ExprKind::Formula {
                lhs: lhs.map(|expr| Box::new(Self::substitute_expr_type_hints(*expr, subst))),
                rhs: Box::new(Self::substitute_expr_type_hints(*rhs, subst)),
            },
            ast::ExprKind::Binary { op, lhs, rhs } => ast::ExprKind::Binary {
                op,
                lhs: Box::new(Self::substitute_expr_type_hints(*lhs, subst)),
                rhs: Box::new(Self::substitute_expr_type_hints(*rhs, subst)),
            },
            ast::ExprKind::Range { a, b } => ast::ExprKind::Range {
                a: Box::new(Self::substitute_expr_type_hints(*a, subst)),
                b: Box::new(Self::substitute_expr_type_hints(*b, subst)),
            },
            ast::ExprKind::Call {
                callee,
                type_args,
                args,
            } => ast::ExprKind::Call {
                callee: Box::new(Self::substitute_expr_type_hints(*callee, subst)),
                type_args: type_args
                    .into_iter()
                    .map(|ty| Self::substitute_type_expr(ty, subst))
                    .collect(),
                args: args
                    .into_iter()
                    .map(|arg| Self::substitute_expr_type_hints(arg, subst))
                    .collect(),
            },
            ast::ExprKind::NamedArg { name, value } => ast::ExprKind::NamedArg {
                name,
                value: Box::new(Self::substitute_expr_type_hints(*value, subst)),
            },
            ast::ExprKind::Index { base, idx } => ast::ExprKind::Index {
                base: Box::new(Self::substitute_expr_type_hints(*base, subst)),
                idx: idx
                    .into_iter()
                    .map(|idx| Self::substitute_expr_type_hints(idx, subst))
                    .collect(),
            },
            ast::ExprKind::Field { base, name } => ast::ExprKind::Field {
                base: Box::new(Self::substitute_expr_type_hints(*base, subst)),
                name,
            },
            ast::ExprKind::VectorLit(xs) => ast::ExprKind::VectorLit(
                xs.into_iter()
                    .map(|expr| Self::substitute_expr_type_hints(expr, subst))
                    .collect(),
            ),
            ast::ExprKind::RecordLit(fields) => ast::ExprKind::RecordLit(
                fields
                    .into_iter()
                    .map(|(name, value)| (name, Self::substitute_expr_type_hints(value, subst)))
                    .collect(),
            ),
            ast::ExprKind::Pipe { lhs, rhs_call } => ast::ExprKind::Pipe {
                lhs: Box::new(Self::substitute_expr_type_hints(*lhs, subst)),
                rhs_call: Box::new(Self::substitute_expr_type_hints(*rhs_call, subst)),
            },
            ast::ExprKind::Try { expr } => ast::ExprKind::Try {
                expr: Box::new(Self::substitute_expr_type_hints(*expr, subst)),
            },
            ast::ExprKind::Unquote(expr) => {
                ast::ExprKind::Unquote(Box::new(Self::substitute_expr_type_hints(*expr, subst)))
            }
            ast::ExprKind::Match { scrutinee, arms } => ast::ExprKind::Match {
                scrutinee: Box::new(Self::substitute_expr_type_hints(*scrutinee, subst)),
                arms: arms
                    .into_iter()
                    .map(|mut arm| {
                        if let Some(guard) = arm.guard {
                            arm.guard =
                                Some(Box::new(Self::substitute_expr_type_hints(*guard, subst)));
                        }
                        arm.body = Box::new(Self::substitute_expr_type_hints(*arm.body, subst));
                        arm
                    })
                    .collect(),
            },
            ast::ExprKind::Lambda {
                params,
                ret_ty_hint,
                body,
            } => ast::ExprKind::Lambda {
                params: params
                    .into_iter()
                    .map(|param| Self::substitute_fn_param_type(param, subst))
                    .collect(),
                ret_ty_hint: ret_ty_hint.map(|ty| Self::substitute_type_expr(ty, subst)),
                body: Self::substitute_block_type_hints(body, subst),
            },
            other => other,
        };
        expr
    }
}
