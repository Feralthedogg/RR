use super::*;
impl Lowerer {
    pub(crate) fn dotted_name_from_expr(expr: &ast::Expr) -> Option<String> {
        match &expr.kind {
            ast::ExprKind::Name(n) => Some(n.clone()),
            ast::ExprKind::Field { base, name } => {
                let mut s = Self::dotted_name_from_expr(base)?;
                s.push('.');
                s.push_str(name);
                Some(s)
            }
            _ => None,
        }
    }
    pub(crate) fn dotted_name_from_field(base: &ast::Expr, field: &str) -> Option<String> {
        let mut s = Self::dotted_name_from_expr(base)?;
        s.push('.');
        s.push_str(field);
        Some(s)
    }
    pub(crate) fn root_is_unbound_for_dotted(&self, dotted: &str) -> bool {
        let root = dotted.split('.').next().unwrap_or(dotted);
        self.lookup(root).is_none()
    }
    pub(crate) fn lower_dotted_ref(&mut self, dotted: &str, span: Span) -> HirExpr {
        if let Some(lid) = self.lookup(dotted) {
            HirExpr::Local(lid)
        } else if let Some(sym) = self.global_fn_aliases.get(dotted).copied() {
            HirExpr::Global(sym, span)
        } else if let Some(sym) = self.r_import_aliases.get(dotted).copied() {
            HirExpr::Global(sym, span)
        } else if let Some((root, rest)) = dotted.split_once('.')
            && let Some(pkg) = self.r_namespace_aliases.get(root)
        {
            HirExpr::Global(self.intern_symbol(&format!("{}::{}", pkg, rest)), span)
        } else {
            HirExpr::Global(self.intern_symbol(dotted), span)
        }
    }
}
