use super::*;
impl Lowerer {
    pub(crate) fn lower_call_args(&mut self, args: Vec<ast::Expr>) -> RR<Vec<HirArg>> {
        let mut hargs = Vec::with_capacity(args.len());
        for arg in args {
            match arg.kind {
                ast::ExprKind::NamedArg { name, value } => {
                    let sym = self.intern_symbol(&name);
                    hargs.push(HirArg::Named {
                        name: sym,
                        value: self.lower_expr(*value)?,
                    });
                }
                _ => hargs.push(HirArg::Pos(self.lower_expr(arg)?)),
            }
        }
        Ok(hargs)
    }
    pub(crate) fn hir_binop(op: ast::BinOp) -> HirBinOp {
        match op {
            ast::BinOp::Add => HirBinOp::Add,
            ast::BinOp::Sub => HirBinOp::Sub,
            ast::BinOp::Mul => HirBinOp::Mul,
            ast::BinOp::Div => HirBinOp::Div,
            ast::BinOp::Mod => HirBinOp::Mod,
            ast::BinOp::MatMul => HirBinOp::MatMul,
            ast::BinOp::Eq => HirBinOp::Eq,
            ast::BinOp::Ne => HirBinOp::Ne,
            ast::BinOp::Lt => HirBinOp::Lt,
            ast::BinOp::Le => HirBinOp::Le,
            ast::BinOp::Gt => HirBinOp::Gt,
            ast::BinOp::Ge => HirBinOp::Ge,
            ast::BinOp::And => HirBinOp::And,
            ast::BinOp::Or => HirBinOp::Or,
        }
    }
    pub(crate) fn operator_trait_for_binop(op: ast::BinOp) -> Option<(&'static str, &'static str)> {
        match op {
            ast::BinOp::Add => Some(("Add", "add")),
            ast::BinOp::Sub => Some(("Sub", "sub")),
            ast::BinOp::Mul => Some(("Mul", "mul")),
            ast::BinOp::Div => Some(("Div", "div")),
            ast::BinOp::Mod => Some(("Mod", "mod")),
            ast::BinOp::MatMul => Some(("MatMul", "matmul")),
            ast::BinOp::Eq
            | ast::BinOp::Ne
            | ast::BinOp::Lt
            | ast::BinOp::Le
            | ast::BinOp::Gt
            | ast::BinOp::Ge
            | ast::BinOp::And
            | ast::BinOp::Or => None,
        }
    }
    pub(crate) fn operator_trait_for_unop(
        op: ast::UnaryOp,
    ) -> Option<(&'static str, &'static str)> {
        match op {
            ast::UnaryOp::Neg => Some(("Neg", "neg")),
            ast::UnaryOp::Not | ast::UnaryOp::Formula => None,
        }
    }
}
