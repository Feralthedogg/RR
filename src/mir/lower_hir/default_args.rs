use super::*;
impl<'a> MirLowerer<'a> {
    // Most math/data builtins are treated as intrinsics by later passes.
    // Only the small scalar-indexing group is allowed to shadow base R names.
    pub(crate) fn allow_user_builtin_shadowing(name: &str) -> bool {
        matches!(name, "length" | "floor" | "round" | "ceiling" | "trunc")
    }

    pub(crate) fn function_name_suggestion_candidates() -> &'static [&'static str] {
        &[
            "length",
            "seq_along",
            "seq_len",
            "c",
            "list",
            "sum",
            "mean",
            "var",
            "prod",
            "min",
            "max",
            "abs",
            "sqrt",
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            "atan2",
            "sinh",
            "cosh",
            "tanh",
            "log",
            "log10",
            "log2",
            "exp",
            "sign",
            "gamma",
            "lgamma",
            "floor",
            "ceiling",
            "trunc",
            "round",
            "pmax",
            "pmin",
            "print",
            "paste",
            "paste0",
            "sprintf",
            "cat",
            "names",
            "rownames",
            "colnames",
            "sort",
            "order",
            "match",
            "unique",
            "duplicated",
            "anyDuplicated",
            "any",
            "all",
            "which",
            "is.na",
            "is.finite",
            "numeric",
            "character",
            "logical",
            "integer",
            "double",
            "rep",
            "rep.int",
            "vector",
            "matrix",
            "dim",
            "dimnames",
            "nrow",
            "ncol",
            "colSums",
            "rowSums",
            "crossprod",
            "tcrossprod",
            "t",
            "diag",
            "rbind",
            "cbind",
            "library",
            "require",
            "plot",
            "lines",
            "legend",
            "png",
            "dev.off",
            "eval",
            "parse",
            "get",
            "assign",
            "exists",
            "mget",
            "rm",
            "ls",
            "parent.frame",
            "environment",
            "sys.frame",
            "sys.call",
            "do.call",
        ]
    }

    pub(crate) fn suggest_function_name(&self, name: &str) -> Option<String> {
        did_you_mean(
            name,
            self.known_functions.keys().cloned().chain(
                Self::function_name_suggestion_candidates()
                    .iter()
                    .map(|name| (*name).to_string()),
            ),
        )
    }

    pub(crate) fn render_default_lit(lit: &hir::HirLit) -> String {
        match lit {
            hir::HirLit::Int(i) => format!("{i}L"),
            hir::HirLit::Double(f) => {
                let mut rendered = f.to_string();
                if f.is_finite() && !rendered.contains(['.', 'e', 'E']) {
                    rendered.push_str(".0");
                }
                rendered
            }
            hir::HirLit::Char(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            hir::HirLit::Bool(true) => "TRUE".to_string(),
            hir::HirLit::Bool(false) => "FALSE".to_string(),
            hir::HirLit::NA => "NA".to_string(),
            hir::HirLit::Null => "NULL".to_string(),
        }
    }

    pub(crate) fn render_default_unop(op: &hir::HirUnOp) -> &'static str {
        match op {
            hir::HirUnOp::Not => "!",
            hir::HirUnOp::Neg => "-",
        }
    }

    pub(crate) fn render_default_binop(op: &hir::HirBinOp) -> &'static str {
        match op {
            hir::HirBinOp::Add => "+",
            hir::HirBinOp::Sub => "-",
            hir::HirBinOp::Mul => "*",
            hir::HirBinOp::Div => "/",
            hir::HirBinOp::Mod => "%%",
            hir::HirBinOp::MatMul => "%*%",
            hir::HirBinOp::And => "&",
            hir::HirBinOp::Or => "|",
            hir::HirBinOp::Eq => "==",
            hir::HirBinOp::Ne => "!=",
            hir::HirBinOp::Lt => "<",
            hir::HirBinOp::Le => "<=",
            hir::HirBinOp::Gt => ">",
            hir::HirBinOp::Ge => ">=",
        }
    }

    pub(crate) fn render_default_arg(&self, arg: &hir::HirArg) -> RR<String> {
        match arg {
            hir::HirArg::Pos(expr) => self.render_default_expr(expr),
            hir::HirArg::Named { name, value } => {
                let rendered_name = self
                    .symbols
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| format!("arg_{}", name.0));
                Ok(format!(
                    "{rendered_name} = {}",
                    self.render_default_expr(value)?
                ))
            }
        }
    }

    pub(crate) fn render_default_expr(&self, expr: &hir::HirExpr) -> RR<String> {
        match expr {
            hir::HirExpr::Local(local) => self.var_names.get(local).cloned().ok_or_else(|| {
                InternalCompilerError::new(
                    Stage::Mir,
                    format!(
                        "missing local name while rendering default argument: {:?}",
                        local
                    ),
                )
                .into_exception()
            }),
            hir::HirExpr::Global(sym, _) => Ok(self
                .symbols
                .get(sym)
                .cloned()
                .unwrap_or_else(|| format!("Sym_{}", sym.0))),
            hir::HirExpr::Lit(lit) => Ok(Self::render_default_lit(lit)),
            hir::HirExpr::Unary { op, expr } => Ok(format!(
                "{}({})",
                Self::render_default_unop(op),
                self.render_default_expr(expr)?
            )),
            hir::HirExpr::Binary { op, lhs, rhs } => Ok(format!(
                "({} {} {})",
                self.render_default_expr(lhs)?,
                Self::render_default_binop(op),
                self.render_default_expr(rhs)?
            )),
            hir::HirExpr::Call(call) => {
                let callee = self.render_default_expr(call.callee.as_ref())?;
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.render_default_arg(arg))
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("{callee}({})", args.join(", ")))
            }
            hir::HirExpr::Index { base, index } => {
                let base = self.render_default_expr(base)?;
                let index = index
                    .iter()
                    .map(|expr| self.render_default_expr(expr))
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("{base}[{}]", index.join(", ")))
            }
            hir::HirExpr::Field { base, name } => {
                let base = self.render_default_expr(base)?;
                let name = self
                    .symbols
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| format!("field_{}", name.0));
                Ok(format!("{base}[[\"{name}\"]]"))
            }
            hir::HirExpr::IfExpr {
                cond,
                then_expr,
                else_expr,
            } => Ok(format!(
                "if ({}) {} else {}",
                self.render_default_expr(cond)?,
                self.render_default_expr(then_expr)?,
                self.render_default_expr(else_expr)?
            )),
            hir::HirExpr::ListLit(fields) => {
                let rendered = fields
                    .iter()
                    .map(|(name, value)| {
                        let rendered_name = self
                            .symbols
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| format!("field_{}", name.0));
                        Ok(format!(
                            "{rendered_name} = {}",
                            self.render_default_expr(value)?
                        ))
                    })
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("list({})", rendered.join(", ")))
            }
            hir::HirExpr::VectorLit(values) => {
                let rendered = values
                    .iter()
                    .map(|value| self.render_default_expr(value))
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("c({})", rendered.join(", ")))
            }
            hir::HirExpr::Range { start, end } => Ok(format!(
                "{}:{}",
                self.render_default_expr(start)?,
                self.render_default_expr(end)?
            )),
            unsupported => Err(RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1002,
                crate::error::Stage::Mir,
                format!(
                    "default argument expression is not yet supported in MIR lowering: {:?}",
                    unsupported
                ),
            )),
        }
    }
}
