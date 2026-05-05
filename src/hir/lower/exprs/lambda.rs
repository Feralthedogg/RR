use super::*;
impl Lowerer {
    pub(crate) fn collect_lambda_captures(
        &self,
        params: &[ast::FnParam],
        body: &ast::Block,
    ) -> Vec<(String, LocalId)> {
        pub(crate) fn in_scopes(scopes: &[FxHashSet<String>], name: &str) -> bool {
            scopes.iter().rev().any(|s| s.contains(name))
        }

        pub(crate) fn record_capture(
            lowerer: &Lowerer,
            scopes: &[FxHashSet<String>],
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            name: &str,
        ) {
            if in_scopes(scopes, name) {
                return;
            }
            if let Some(lid) = lowerer.lookup(name)
                && seen.insert(name.to_string())
            {
                captures.push((name.to_string(), lid));
            }
        }

        pub(crate) fn collect_pat_binders(p: &ast::Pattern, out: &mut FxHashSet<String>) {
            match &p.kind {
                ast::PatternKind::Bind(n) => {
                    out.insert(n.clone());
                }
                ast::PatternKind::List { items, rest } => {
                    for it in items {
                        collect_pat_binders(it, out);
                    }
                    if let Some(r) = rest {
                        out.insert(r.clone());
                    }
                }
                ast::PatternKind::Record { fields } => {
                    for (_, fp) in fields {
                        collect_pat_binders(fp, out);
                    }
                }
                ast::PatternKind::Wild | ast::PatternKind::Lit(_) => {}
            }
        }

        pub(crate) fn visit_unsafe_r_code(
            lowerer: &Lowerer,
            scopes: &[FxHashSet<String>],
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            code: &str,
        ) {
            let mut ident = String::new();
            let mut quote: Option<char> = None;
            let mut escaped = false;
            let mut in_comment = false;

            let flush_ident =
                |ident: &mut String,
                 seen: &mut FxHashSet<String>,
                 captures: &mut Vec<(String, LocalId)>| {
                    if !ident.is_empty() {
                        record_capture(lowerer, scopes, seen, captures, ident);
                        ident.clear();
                    }
                };

            for ch in code.chars() {
                if in_comment {
                    if ch == '\n' {
                        in_comment = false;
                    }
                    continue;
                }
                if let Some(q) = quote {
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == q {
                        quote = None;
                    }
                    continue;
                }

                match ch {
                    '"' | '\'' | '`' => {
                        flush_ident(&mut ident, seen, captures);
                        quote = Some(ch);
                    }
                    '#' => {
                        flush_ident(&mut ident, seen, captures);
                        in_comment = true;
                    }
                    c if c == '_' || c.is_ascii_alphabetic() => ident.push(c),
                    c if c.is_ascii_digit() && !ident.is_empty() => ident.push(c),
                    _ => flush_ident(&mut ident, seen, captures),
                }
            }
            flush_ident(&mut ident, seen, captures);
        }

        pub(crate) fn visit_expr(
            lowerer: &Lowerer,
            scopes: &mut Vec<FxHashSet<String>>,
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            expr: &ast::Expr,
        ) {
            match &expr.kind {
                ast::ExprKind::Name(n) => record_capture(lowerer, scopes, seen, captures, n),
                ast::ExprKind::Unary { rhs, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, rhs)
                }
                ast::ExprKind::Formula { lhs, rhs } => {
                    if let Some(lhs) = lhs {
                        visit_expr(lowerer, scopes, seen, captures, lhs);
                    }
                    visit_expr(lowerer, scopes, seen, captures, rhs);
                }
                ast::ExprKind::Binary { lhs, rhs, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, lhs);
                    visit_expr(lowerer, scopes, seen, captures, rhs);
                }
                ast::ExprKind::Range { a, b } => {
                    visit_expr(lowerer, scopes, seen, captures, a);
                    visit_expr(lowerer, scopes, seen, captures, b);
                }
                ast::ExprKind::Call {
                    callee,
                    type_args: _,
                    args,
                } => {
                    visit_expr(lowerer, scopes, seen, captures, callee);
                    for a in args {
                        visit_expr(lowerer, scopes, seen, captures, a);
                    }
                }
                ast::ExprKind::NamedArg { value, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, value)
                }
                ast::ExprKind::Index { base, idx } => {
                    visit_expr(lowerer, scopes, seen, captures, base);
                    for i in idx {
                        visit_expr(lowerer, scopes, seen, captures, i);
                    }
                }
                ast::ExprKind::Field { base, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, base)
                }
                ast::ExprKind::VectorLit(xs) => {
                    for x in xs {
                        visit_expr(lowerer, scopes, seen, captures, x);
                    }
                }
                ast::ExprKind::RecordLit(fields) => {
                    for (_, v) in fields {
                        visit_expr(lowerer, scopes, seen, captures, v);
                    }
                }
                ast::ExprKind::Pipe { lhs, rhs_call } => {
                    visit_expr(lowerer, scopes, seen, captures, lhs);
                    visit_expr(lowerer, scopes, seen, captures, rhs_call);
                }
                ast::ExprKind::Try { expr } => visit_expr(lowerer, scopes, seen, captures, expr),
                ast::ExprKind::Unquote(e) => visit_expr(lowerer, scopes, seen, captures, e),
                ast::ExprKind::Match { scrutinee, arms } => {
                    visit_expr(lowerer, scopes, seen, captures, scrutinee);
                    for arm in arms {
                        let mut arm_scope = FxHashSet::default();
                        collect_pat_binders(&arm.pat, &mut arm_scope);
                        scopes.push(arm_scope);
                        if let Some(g) = &arm.guard {
                            visit_expr(lowerer, scopes, seen, captures, g);
                        }
                        visit_expr(lowerer, scopes, seen, captures, &arm.body);
                        scopes.pop();
                    }
                }
                ast::ExprKind::Lambda {
                    params,
                    ret_ty_hint: _,
                    body,
                } => {
                    let mut lambda_scope = FxHashSet::default();
                    for p in params {
                        lambda_scope.insert(p.name.clone());
                    }
                    scopes.push(lambda_scope);
                    visit_block(lowerer, scopes, seen, captures, body);
                    scopes.pop();
                }
                ast::ExprKind::Column(_) | ast::ExprKind::ColRef(_) | ast::ExprKind::Lit(_) => {}
            }
        }

        pub(crate) fn visit_stmt(
            lowerer: &Lowerer,
            scopes: &mut Vec<FxHashSet<String>>,
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            stmt: &ast::Stmt,
        ) {
            match &stmt.kind {
                ast::StmtKind::Let {
                    name,
                    ty_hint: _,
                    init,
                } => {
                    if let Some(e) = init {
                        visit_expr(lowerer, scopes, seen, captures, e);
                    }
                    if let Some(scope) = scopes.last_mut() {
                        scope.insert(name.clone());
                    }
                }
                ast::StmtKind::Assign { target, value } => {
                    visit_expr(lowerer, scopes, seen, captures, value);
                    match &target.kind {
                        ast::LValueKind::Name(n) => {
                            if !in_scopes(scopes, n)
                                && lowerer.lookup(n).is_none()
                                && let Some(scope) = scopes.last_mut()
                            {
                                scope.insert(n.clone());
                            }
                        }
                        ast::LValueKind::Index { base, idx } => {
                            visit_expr(lowerer, scopes, seen, captures, base);
                            for i in idx {
                                visit_expr(lowerer, scopes, seen, captures, i);
                            }
                        }
                        ast::LValueKind::Field { base, .. } => {
                            visit_expr(lowerer, scopes, seen, captures, base);
                        }
                    }
                }
                ast::StmtKind::If {
                    cond,
                    then_blk,
                    else_blk,
                } => {
                    visit_expr(lowerer, scopes, seen, captures, cond);
                    scopes.push(FxHashSet::default());
                    visit_block(lowerer, scopes, seen, captures, then_blk);
                    scopes.pop();
                    if let Some(eb) = else_blk {
                        scopes.push(FxHashSet::default());
                        visit_block(lowerer, scopes, seen, captures, eb);
                        scopes.pop();
                    }
                }
                ast::StmtKind::While { cond, body } => {
                    visit_expr(lowerer, scopes, seen, captures, cond);
                    scopes.push(FxHashSet::default());
                    visit_block(lowerer, scopes, seen, captures, body);
                    scopes.pop();
                }
                ast::StmtKind::For { var, iter, body } => {
                    visit_expr(lowerer, scopes, seen, captures, iter);
                    let mut loop_scope = FxHashSet::default();
                    loop_scope.insert(var.clone());
                    scopes.push(loop_scope);
                    visit_block(lowerer, scopes, seen, captures, body);
                    scopes.pop();
                }
                ast::StmtKind::Return { value } => {
                    if let Some(v) = value {
                        visit_expr(lowerer, scopes, seen, captures, v);
                    }
                }
                ast::StmtKind::ExprStmt { expr } | ast::StmtKind::Expr(expr) => {
                    visit_expr(lowerer, scopes, seen, captures, expr);
                }
                ast::StmtKind::UnsafeRBlock { code, .. } => {
                    visit_unsafe_r_code(lowerer, scopes, seen, captures, code);
                }
                ast::StmtKind::FnDecl { .. }
                | ast::StmtKind::TraitDecl(_)
                | ast::StmtKind::ImplDecl(_)
                | ast::StmtKind::Import { .. }
                | ast::StmtKind::Export(_)
                | ast::StmtKind::Break
                | ast::StmtKind::Next => {}
            }
        }

        pub(crate) fn visit_block(
            lowerer: &Lowerer,
            scopes: &mut Vec<FxHashSet<String>>,
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            block: &ast::Block,
        ) {
            for s in &block.stmts {
                visit_stmt(lowerer, scopes, seen, captures, s);
            }
        }

        let mut scopes: Vec<FxHashSet<String>> =
            vec![params.iter().map(|p| p.name.clone()).collect()];
        let mut seen = FxHashSet::default();
        let mut captures = Vec::new();
        visit_block(self, &mut scopes, &mut seen, &mut captures, body);
        captures
    }
    pub(crate) fn infer_param_type_hint(default: &ast::Expr) -> Option<Ty> {
        match &default.kind {
            ast::ExprKind::Lit(ast::Lit::Int(_)) => Some(Ty::Int),
            ast::ExprKind::Lit(ast::Lit::Float(_)) => Some(Ty::Double),
            ast::ExprKind::Lit(ast::Lit::Bool(_)) => Some(Ty::Logical),
            ast::ExprKind::Lit(ast::Lit::Str(_)) => Some(Ty::Char),
            ast::ExprKind::Lit(ast::Lit::Null) => Some(Ty::Null),
            _ => None,
        }
    }
    pub(crate) fn parse_type_hint_expr(expr: &ast::TypeExpr) -> Option<Ty> {
        match expr {
            ast::TypeExpr::Named(name) => match name.to_ascii_lowercase().as_str() {
                "any" => Some(Ty::Any),
                "null" => Some(Ty::Null),
                "bool" | "boolean" | "logical" => Some(Ty::Logical),
                "int" | "integer" | "i32" | "i64" | "isize" => Some(Ty::Int),
                "float" | "double" | "numeric" | "f32" | "f64" => Some(Ty::Double),
                "str" | "string" | "char" | "character" => Some(Ty::Char),
                _ => None,
            },
            ast::TypeExpr::Generic { base, args } => {
                let base = base.to_ascii_lowercase();
                if base == "vector" && args.len() == 1 {
                    return Some(Ty::Vector(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "matrix" && args.len() == 1 {
                    return Some(Ty::Matrix(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "option" && args.len() == 1 {
                    return Some(Ty::Option(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "list" && args.len() == 1 {
                    return Some(Ty::List(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "box" && args.len() == 1 {
                    return Self::parse_type_hint_expr(&args[0])
                        .map(|inner| Ty::Box(Box::new(inner)));
                }
                None
            }
        }
    }
    pub(crate) fn lower_lambda_expr(
        &mut self,
        params: Vec<ast::FnParam>,
        ret_ty_hint: Option<ast::TypeExpr>,
        body: ast::Block,
        span: Span,
    ) -> RR<HirExpr> {
        let captures = self.collect_lambda_captures(&params, &body);
        let lambda_name = self.alloc_lambda_name();
        let lambda_sym = self.intern_symbol(&lambda_name);
        let fn_id = self.alloc_fn_id();

        let saved_scopes = std::mem::replace(&mut self.scopes, vec![FxHashMap::default()]);
        let saved_local_names = std::mem::take(&mut self.local_names);
        let saved_local_emitted_names = std::mem::take(&mut self.local_emitted_names);
        let saved_local_trait_types = std::mem::take(&mut self.local_trait_types);
        let saved_type_params = std::mem::take(&mut self.current_type_params);
        let saved_where_bounds = std::mem::take(&mut self.current_where_bounds);
        let saved_next_local = self.next_local_id;
        self.next_local_id = 0;

        let mut hir_params: Vec<HirParam> = Vec::new();
        for (cap_name, _) in &captures {
            let _cap_local = self.declare_local(cap_name);
            let cap_sym = self.intern_symbol(cap_name);
            hir_params.push(HirParam {
                name: cap_sym,
                ty: None,
                ty_inferred: false,
                default: None,
                span,
            });
        }
        let mut param_syms = Vec::with_capacity(params.len());
        for p in &params {
            let pid = self.declare_local(&p.name);
            if let Some(ty_hint) = &p.ty_hint {
                self.local_trait_types
                    .insert(pid, Self::ast_type_ref(ty_hint));
            }
            let psym = self.intern_symbol(&p.name);
            param_syms.push(psym);
        }
        for (p, psym) in params.into_iter().zip(param_syms) {
            let explicit_ty_hint = p.ty_hint.as_ref().and_then(Self::parse_type_hint_expr);
            let default_ty_hint = explicit_ty_hint
                .is_none()
                .then(|| p.default.as_ref().and_then(Self::infer_param_type_hint))
                .flatten();
            let ty_inferred = default_ty_hint.is_some();
            let ty_hint = explicit_ty_hint.or(default_ty_hint);
            let default = if let Some(d) = p.default {
                Some(self.lower_expr(d)?)
            } else {
                None
            };
            hir_params.push(HirParam {
                name: psym,
                ty: ty_hint,
                ty_inferred,
                default,
                span: p.span,
            });
        }
        let hir_body = Self::apply_implicit_tail_return(self.lower_block(body)?);
        let lambda_local_names = std::mem::take(&mut self.local_names);

        self.scopes = saved_scopes;
        self.local_names = saved_local_names;
        self.local_emitted_names = saved_local_emitted_names;
        self.local_trait_types = saved_local_trait_types;
        self.current_type_params = saved_type_params;
        self.current_where_bounds = saved_where_bounds;
        self.next_local_id = saved_next_local;

        self.pending_fns.push(HirFn {
            id: fn_id,
            name: lambda_sym,
            type_params: Vec::new(),
            where_bounds: Vec::new(),
            params: hir_params,
            has_varargs: false,
            ret_ty: ret_ty_hint.as_ref().and_then(Self::parse_type_hint_expr),
            ret_ty_inferred: false,
            body: hir_body,
            attrs: HirFnAttrs {
                inline_hint: InlineHint::Default,
                tidy_safe: false,
            },
            span,
            local_names: lambda_local_names,
            public: false,
        });

        if captures.is_empty() {
            return Ok(HirExpr::Global(lambda_sym, span));
        }

        let make_sym = self.intern_symbol("rr_closure_make");
        let mut args = Vec::new();
        args.push(HirArg::Pos(HirExpr::Global(lambda_sym, span)));
        for (_, lid) in captures {
            args.push(HirArg::Pos(HirExpr::Local(lid)));
        }
        Ok(HirExpr::Call(HirCall {
            callee: Box::new(HirExpr::Global(make_sym, span)),
            args,
            span,
        }))
    }
}
