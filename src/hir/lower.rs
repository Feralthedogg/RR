use crate::error::{RR, RRCode, RRException, Stage};
use crate::hir::def::*;
use crate::syntax::ast;
use crate::utils::Span;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;

pub struct Lowerer {
    // Symbol resolution state
    scopes: Vec<HashMap<String, LocalId>>,
    next_local_id: u32,
    next_sym_id: u32,

    // Context flags
    in_tidy: bool,

    // Mapping for current function's locals
    local_names: HashMap<LocalId, String>,

    // Global Symbol Table
    symbols: HashMap<SymbolId, String>,
    symbols_rev: HashMap<String, SymbolId>,
    // Collected lowering warnings (reported by caller)
    warnings: Vec<String>,
    // If true, assignment to undeclared names is an error.
    strict_let: bool,
    // If true, emit warnings for implicit declarations.
    warn_implicit_decl: bool,
    // Lambda-lifted synthetic functions.
    pending_fns: Vec<HirFn>,
    // Top-level aliases of function values, e.g. `let f = fn(...) { ... }`.
    // Used so function bodies can resolve `f(...)` directly to lifted symbols.
    global_fn_aliases: HashMap<String, SymbolId>,
}

impl Lowerer {
    pub fn new() -> Self {
        let strict_let = Self::env_truthy("RR_STRICT_LET") || Self::env_truthy("RR_STRICT_ASSIGN");
        let warn_implicit_decl = Self::env_truthy("RR_WARN_IMPLICIT_DECL");
        Self {
            scopes: vec![HashMap::new()], // Global scope?
            next_local_id: 0,
            next_sym_id: 0,
            in_tidy: false,
            local_names: HashMap::new(),
            symbols: HashMap::new(),
            symbols_rev: HashMap::new(),
            warnings: Vec::new(),
            strict_let,
            warn_implicit_decl,
            pending_fns: Vec::new(),
            global_fn_aliases: HashMap::new(),
        }
    }

    fn env_truthy(key: &str) -> bool {
        match env::var(key) {
            Ok(v) => matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => false,
        }
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    // Persistent symbol table
    pub fn get_symbols(&self) -> HashMap<SymbolId, String> {
        self.symbols.clone()
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        let emitted_name = if self.local_names.values().any(|n| n == name) {
            format!("{}_{}", name, id.0)
        } else {
            name.to_string()
        };
        self.local_names.insert(id, emitted_name);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
        id
    }

    fn lookup(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }

    fn intern_symbol(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.symbols_rev.get(name) {
            return id;
        }
        let id = SymbolId(self.next_sym_id);
        self.next_sym_id += 1;
        let owned = name.to_string();
        self.symbols.insert(id, owned.clone());
        self.symbols_rev.insert(owned, id);
        id
    }

    fn alloc_fn_id(&mut self) -> FnId {
        let id = FnId(self.next_sym_id);
        self.next_sym_id += 1;
        id
    }

    fn alloc_lambda_name(&mut self) -> String {
        let n = self.next_sym_id;
        self.next_sym_id += 1;
        format!("__lambda_{}", n)
    }

    fn flush_pending_fns(&mut self, items: &mut Vec<HirItem>) {
        if self.pending_fns.is_empty() {
            return;
        }
        for f in self.pending_fns.drain(..) {
            items.push(HirItem::Fn(f));
        }
    }

    fn apply_implicit_tail_return(mut body: HirBlock) -> HirBlock {
        if body
            .stmts
            .iter()
            .any(|s| matches!(s, HirStmt::Return { .. }))
        {
            return body;
        }
        if let Some(last) = body.stmts.pop() {
            match last {
                HirStmt::Expr { expr, span } => {
                    body.stmts.push(HirStmt::Return {
                        value: Some(expr),
                        span,
                    });
                }
                other => body.stmts.push(other),
            }
        }
        body
    }

    fn dotted_name_from_expr(expr: &ast::Expr) -> Option<String> {
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

    fn dotted_name_from_field(base: &ast::Expr, field: &str) -> Option<String> {
        let mut s = Self::dotted_name_from_expr(base)?;
        s.push('.');
        s.push_str(field);
        Some(s)
    }

    fn root_is_unbound_for_dotted(&self, dotted: &str) -> bool {
        let root = dotted.split('.').next().unwrap_or(dotted);
        self.lookup(root).is_none()
    }

    fn lower_dotted_ref(&mut self, dotted: &str) -> HirExpr {
        if let Some(lid) = self.lookup(dotted) {
            HirExpr::Local(lid)
        } else if let Some(sym) = self.global_fn_aliases.get(dotted).copied() {
            HirExpr::Global(sym)
        } else {
            HirExpr::Global(self.intern_symbol(dotted))
        }
    }

    fn resolve_or_declare_local_for_assign(&mut self, name: &str, span: Span) -> RR<LocalId> {
        if let Some(lid) = self.lookup(name) {
            return Ok(lid);
        }
        if self.strict_let {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1001,
                Stage::Lower,
                format!("assignment to undeclared variable '{}'", name),
            )
            .at(span)
            .note("Declare it first with `let` before assignment."));
        }
        let lid = self.declare_local(name);
        if self.warn_implicit_decl {
            let where_msg = if span.start_line > 0 {
                format!("{}:{}", span.start_line, span.start_col)
            } else {
                "unknown".to_string()
            };
            self.warnings.push(format!(
                "implicit declaration via assignment: '{}' at {} (treated as `let {} = ...;`). Set RR_STRICT_LET=1 to forbid this.",
                name, where_msg, name
            ));
        }
        Ok(lid)
    }

    fn collect_lambda_captures(
        &self,
        params: &[ast::FnParam],
        body: &ast::Block,
    ) -> Vec<(String, LocalId)> {
        fn in_scopes(scopes: &[HashSet<String>], name: &str) -> bool {
            scopes.iter().rev().any(|s| s.contains(name))
        }

        fn record_capture(
            lowerer: &Lowerer,
            scopes: &[HashSet<String>],
            seen: &mut HashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            name: &str,
        ) {
            if in_scopes(scopes, name) {
                return;
            }
            if let Some(lid) = lowerer.lookup(name) {
                if seen.insert(name.to_string()) {
                    captures.push((name.to_string(), lid));
                }
            }
        }

        fn collect_pat_binders(p: &ast::Pattern, out: &mut HashSet<String>) {
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

        fn visit_expr(
            lowerer: &Lowerer,
            scopes: &mut Vec<HashSet<String>>,
            seen: &mut HashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            expr: &ast::Expr,
        ) {
            match &expr.kind {
                ast::ExprKind::Name(n) => record_capture(lowerer, scopes, seen, captures, n),
                ast::ExprKind::Unary { rhs, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, rhs)
                }
                ast::ExprKind::Binary { lhs, rhs, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, lhs);
                    visit_expr(lowerer, scopes, seen, captures, rhs);
                }
                ast::ExprKind::Range { a, b } => {
                    visit_expr(lowerer, scopes, seen, captures, a);
                    visit_expr(lowerer, scopes, seen, captures, b);
                }
                ast::ExprKind::Call { callee, args } => {
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
                        let mut arm_scope = HashSet::new();
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
                    let mut lambda_scope = HashSet::new();
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

        fn visit_stmt(
            lowerer: &Lowerer,
            scopes: &mut Vec<HashSet<String>>,
            seen: &mut HashSet<String>,
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
                            if !in_scopes(scopes, n) && lowerer.lookup(n).is_none() {
                                if let Some(scope) = scopes.last_mut() {
                                    scope.insert(n.clone());
                                }
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
                    scopes.push(HashSet::new());
                    visit_block(lowerer, scopes, seen, captures, then_blk);
                    scopes.pop();
                    if let Some(eb) = else_blk {
                        scopes.push(HashSet::new());
                        visit_block(lowerer, scopes, seen, captures, eb);
                        scopes.pop();
                    }
                }
                ast::StmtKind::While { cond, body } => {
                    visit_expr(lowerer, scopes, seen, captures, cond);
                    scopes.push(HashSet::new());
                    visit_block(lowerer, scopes, seen, captures, body);
                    scopes.pop();
                }
                ast::StmtKind::For { var, iter, body } => {
                    visit_expr(lowerer, scopes, seen, captures, iter);
                    let mut loop_scope = HashSet::new();
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
                ast::StmtKind::FnDecl { .. }
                | ast::StmtKind::Import(_)
                | ast::StmtKind::Export(_)
                | ast::StmtKind::Break
                | ast::StmtKind::Next => {}
            }
        }

        fn visit_block(
            lowerer: &Lowerer,
            scopes: &mut Vec<HashSet<String>>,
            seen: &mut HashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            block: &ast::Block,
        ) {
            for s in &block.stmts {
                visit_stmt(lowerer, scopes, seen, captures, s);
            }
        }

        let mut scopes: Vec<HashSet<String>> =
            vec![params.iter().map(|p| p.name.clone()).collect()];
        let mut seen = HashSet::new();
        let mut captures = Vec::new();
        visit_block(self, &mut scopes, &mut seen, &mut captures, body);
        captures
    }

    fn infer_param_type_hint(default: &ast::Expr) -> Option<Ty> {
        match &default.kind {
            ast::ExprKind::Lit(ast::Lit::Int(_)) => Some(Ty::Int),
            ast::ExprKind::Lit(ast::Lit::Float(_)) => Some(Ty::Double),
            ast::ExprKind::Lit(ast::Lit::Bool(_)) => Some(Ty::Logical),
            ast::ExprKind::Lit(ast::Lit::Str(_)) => Some(Ty::Char),
            ast::ExprKind::Lit(ast::Lit::Null) => Some(Ty::Null),
            _ => None,
        }
    }

    fn parse_type_hint_name(name: &str) -> Option<Ty> {
        match name.to_ascii_lowercase().as_str() {
            "any" => Some(Ty::Any),
            "null" => Some(Ty::Null),
            "bool" | "boolean" | "logical" => Some(Ty::Logical),
            "int" | "integer" | "i32" | "i64" | "isize" => Some(Ty::Int),
            "float" | "double" | "numeric" | "f32" | "f64" => Some(Ty::Double),
            "str" | "string" | "char" | "character" => Some(Ty::Char),
            _ => None,
        }
    }

    fn lower_lambda_expr(
        &mut self,
        params: Vec<ast::FnParam>,
        ret_ty_hint: Option<String>,
        body: ast::Block,
        span: Span,
    ) -> RR<HirExpr> {
        let captures = self.collect_lambda_captures(&params, &body);
        let lambda_name = self.alloc_lambda_name();
        let lambda_sym = self.intern_symbol(&lambda_name);
        let fn_id = self.alloc_fn_id();

        let saved_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
        let saved_local_names = std::mem::take(&mut self.local_names);
        let saved_next_local = self.next_local_id;
        self.next_local_id = 0;

        let mut hir_params: Vec<HirParam> = Vec::new();
        for (cap_name, _) in &captures {
            let _cap_local = self.declare_local(cap_name);
            let cap_sym = self.intern_symbol(cap_name);
            hir_params.push(HirParam {
                name: cap_sym,
                ty: None,
                default: None,
                span,
            });
        }
        let mut param_syms = Vec::with_capacity(params.len());
        for p in &params {
            let _pid = self.declare_local(&p.name);
            let psym = self.intern_symbol(&p.name);
            param_syms.push(psym);
        }
        for (p, psym) in params.into_iter().zip(param_syms.into_iter()) {
            let ty_hint = p
                .ty_hint
                .as_deref()
                .and_then(Self::parse_type_hint_name)
                .or_else(|| p.default.as_ref().and_then(Self::infer_param_type_hint));
            let default = if let Some(d) = p.default {
                Some(self.lower_expr(d)?)
            } else {
                None
            };
            hir_params.push(HirParam {
                name: psym,
                ty: ty_hint,
                default,
                span: p.span,
            });
        }
        let hir_body = Self::apply_implicit_tail_return(self.lower_block(body)?);
        let lambda_local_names = std::mem::take(&mut self.local_names);

        self.scopes = saved_scopes;
        self.local_names = saved_local_names;
        self.next_local_id = saved_next_local;

        self.pending_fns.push(HirFn {
            id: fn_id,
            name: lambda_sym,
            params: hir_params,
            has_varargs: false,
            ret_ty: ret_ty_hint.as_deref().and_then(Self::parse_type_hint_name),
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
            return Ok(HirExpr::Global(lambda_sym));
        }

        let make_sym = self.intern_symbol("rr_closure_make");
        let mut args = Vec::new();
        args.push(HirArg::Pos(HirExpr::Global(lambda_sym)));
        for (_, lid) in captures {
            args.push(HirArg::Pos(HirExpr::Local(lid)));
        }
        Ok(HirExpr::Call(HirCall {
            callee: Box::new(HirExpr::Global(make_sym)),
            args,
            span,
        }))
    }

    pub fn lower_module(
        &mut self,
        prog: ast::Program,
        mod_id: ModuleId,
    ) -> RR<(HirModule, HashMap<SymbolId, String>)> {
        let mut items = Vec::new();
        // println!("Lowering module {:?} with {} statements", mod_id, prog.stmts.len());
        for stmt in prog.stmts {
            // Top level statements become Module Items (Fn) or Stmt Items
            // For now, treat everything as Item::Stmt unless it's a FnDecl
            match stmt.kind {
                ast::StmtKind::FnDecl {
                    name,
                    params,
                    ret_ty_hint,
                    body,
                } => {
                    let fn_item = self.lower_fn(name, params, ret_ty_hint, body, stmt.span)?;
                    items.push(HirItem::Fn(fn_item));
                    self.flush_pending_fns(&mut items);
                }
                ast::StmtKind::Import(path) => {
                    let import = HirImport {
                        module: path,
                        spec: HirImportSpec::Glob,
                        span: stmt.span,
                    };
                    items.push(HirItem::Import(import));
                    self.flush_pending_fns(&mut items);
                }
                ast::StmtKind::Export(fndecl) => {
                    let mut fn_item = self.lower_fn(
                        fndecl.name,
                        fndecl.params,
                        fndecl.ret_ty_hint,
                        fndecl.body,
                        stmt.span,
                    )?;
                    fn_item.public = true;
                    items.push(HirItem::Fn(fn_item));
                    self.flush_pending_fns(&mut items);
                }
                _ => {
                    let s = self.lower_stmt(stmt)?;
                    items.push(HirItem::Stmt(s));
                    self.flush_pending_fns(&mut items);
                }
            }
        }

        let symbols = self.symbols.clone();
        Ok((
            HirModule {
                id: mod_id,
                path: vec![],
                items,
            },
            symbols,
        ))
    }

    fn lower_fn(
        &mut self,
        name: String,
        params: Vec<ast::FnParam>,
        ret_ty_hint: Option<String>,
        body: ast::Block,
        span: Span,
    ) -> RR<HirFn> {
        let fn_id = self.alloc_fn_id();
        let sym_name = self.intern_symbol(&name);

        // Isolate function-local scope from module-level bindings.
        // Names not declared in the function should lower as globals.
        let saved_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
        let saved_local_names = std::mem::take(&mut self.local_names);
        let saved_next_local = self.next_local_id;
        self.next_local_id = 0;

        self.enter_scope();
        let mut hir_params: Vec<HirParam> = Vec::new();
        let mut param_syms = Vec::with_capacity(params.len());
        for p in &params {
            let _pid = self.declare_local(&p.name);
            let psym = self.intern_symbol(&p.name);
            param_syms.push(psym);
        }
        for (p, psym) in params.into_iter().zip(param_syms.into_iter()) {
            let ty_hint = p
                .ty_hint
                .as_deref()
                .and_then(Self::parse_type_hint_name)
                .or_else(|| p.default.as_ref().and_then(Self::infer_param_type_hint));
            let default = if let Some(d) = p.default {
                Some(self.lower_expr(d)?)
            } else {
                None
            };
            hir_params.push(HirParam {
                name: psym,
                ty: ty_hint,
                default,
                span: p.span,
            });
        }

        let hir_body = Self::apply_implicit_tail_return(self.lower_block(body)?);
        self.exit_scope();

        let local_names = self.local_names.drain().collect();

        self.scopes = saved_scopes;
        self.local_names = saved_local_names;
        self.next_local_id = saved_next_local;

        // These variables are not defined in the original code,
        // but the instruction implies they should be used as variables.
        // To maintain syntactic correctness, we'll define them with their original literal values.
        let has_varargs = false;
        let ret_ty = ret_ty_hint.as_deref().and_then(Self::parse_type_hint_name);

        Ok(HirFn {
            id: fn_id,
            name: sym_name,
            params: hir_params,
            has_varargs,
            ret_ty,
            body: hir_body,
            attrs: HirFnAttrs {
                inline_hint: InlineHint::Default,
                tidy_safe: false,
            },
            span,
            local_names,
            public: false, // Default private
        })
    }

    fn lower_block(&mut self, block: ast::Block) -> RR<HirBlock> {
        let mut stmts = Vec::new();
        for s in block.stmts {
            stmts.push(self.lower_stmt(s)?);
        }
        Ok(HirBlock {
            stmts,
            span: block.span,
        })
    }

    fn lower_stmt(&mut self, stmt: ast::Stmt) -> RR<HirStmt> {
        match stmt.kind {
            ast::StmtKind::Let {
                name,
                ty_hint,
                init,
            } => {
                let lid = self.declare_local(&name);
                let sym = self.intern_symbol(&name);
                let val = if let Some(e) = init {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                if self.scopes.len() == 1
                    && let Some(HirExpr::Global(global_sym)) = &val
                    && self
                        .symbols
                        .get(global_sym)
                        .map(|s| s.starts_with("__lambda_"))
                        .unwrap_or(false)
                {
                    self.global_fn_aliases.insert(name.clone(), *global_sym);
                }
                Ok(HirStmt::Let {
                    local: lid,
                    name: sym,
                    ty: ty_hint.as_deref().and_then(Self::parse_type_hint_name),
                    init: val,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Assign { target, value } => {
                let lhs = self.lower_lvalue(target)?;
                let rhs = self.lower_expr(value)?;
                if self.scopes.len() == 1
                    && let HirLValue::Local(_lid) = &lhs
                    && let HirExpr::Global(global_sym) = &rhs
                    && self
                        .symbols
                        .get(global_sym)
                        .map(|s| s.starts_with("__lambda_"))
                        .unwrap_or(false)
                    && let Some(name) = self.local_name_of_lvalue(&lhs)
                {
                    self.global_fn_aliases.insert(name, *global_sym);
                }
                Ok(HirStmt::Assign {
                    target: lhs,
                    value: rhs,
                    span: stmt.span,
                })
            }
            ast::StmtKind::If {
                cond,
                then_blk,
                else_blk,
            } => {
                let c = self.lower_expr(cond)?;
                let t = self.lower_block(then_blk)?;
                let e = if let Some(blk) = else_blk {
                    Some(self.lower_block(blk)?)
                } else {
                    None
                };
                Ok(HirStmt::If {
                    cond: c,
                    then_blk: t,
                    else_blk: e,
                    span: stmt.span,
                })
            }
            ast::StmtKind::While { cond, body } => {
                let c = self.lower_expr(cond)?;
                let b = self.lower_block(body)?;
                Ok(HirStmt::While {
                    cond: c,
                    body: b,
                    span: stmt.span,
                })
            }
            ast::StmtKind::For { var, iter, body } => {
                let iter_expr = self.lower_expr(iter)?;
                self.enter_scope();
                let lid = self.declare_local(&var);

                // Canonicalize known iterator forms for better downstream optimization.
                let iter_kind = match iter_expr {
                    HirExpr::Range { start, end } => HirForIter::Range {
                        var: lid,
                        start: *start,
                        end: *end,
                        inclusive: true,
                    },
                    HirExpr::Call(call) => {
                        let one_arg = call.args.len() == 1;
                        match (&*call.callee, one_arg) {
                            (HirExpr::Global(sym), true) => {
                                let name = self.symbols.get(sym).cloned().unwrap_or_default();
                                let arg_expr = match call.args[0].clone() {
                                    HirArg::Pos(e) => e,
                                    HirArg::Named { value, .. } => value,
                                };
                                if name == "seq_len" {
                                    HirForIter::SeqLen {
                                        var: lid,
                                        len: arg_expr,
                                    }
                                } else if name == "seq_along" {
                                    HirForIter::SeqAlong {
                                        var: lid,
                                        xs: arg_expr,
                                    }
                                } else {
                                    HirForIter::SeqAlong {
                                        var: lid,
                                        xs: HirExpr::Call(call),
                                    }
                                }
                            }
                            _ => HirForIter::SeqAlong {
                                var: lid,
                                xs: HirExpr::Call(call),
                            },
                        }
                    }
                    other => HirForIter::SeqAlong {
                        var: lid,
                        xs: other,
                    },
                };

                let body_hir = self.lower_block(body)?;
                self.exit_scope();
                Ok(HirStmt::For {
                    iter: iter_kind,
                    body: body_hir,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Return { value } => {
                let v = if let Some(e) = value {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                Ok(HirStmt::Return {
                    value: v,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Break => Ok(HirStmt::Break { span: stmt.span }),
            ast::StmtKind::Next => Ok(HirStmt::Next { span: stmt.span }),
            ast::StmtKind::ExprStmt { expr } => Ok(HirStmt::Expr {
                expr: self.lower_expr(expr)?,
                span: stmt.span,
            }),
            _ => Err(RRException::new(
                "Feature.NotImpl",
                RRCode::E9999,
                Stage::Lower,
                "Stmt kind not supported".to_string(),
            )),
        }
    }

    fn lower_expr(&mut self, expr: ast::Expr) -> RR<HirExpr> {
        match expr.kind {
            ast::ExprKind::Lit(l) => {
                let hl = match l {
                    ast::Lit::Int(i) => HirLit::Int(i),
                    ast::Lit::Float(f) => HirLit::Double(f),
                    ast::Lit::Str(s) => HirLit::Char(s),
                    ast::Lit::Bool(b) => HirLit::Bool(b),
                    ast::Lit::Na => HirLit::NA,
                    ast::Lit::Null => HirLit::Null,
                };
                Ok(HirExpr::Lit(hl))
            }
            ast::ExprKind::Name(n) => {
                if let Some(lid) = self.lookup(&n) {
                    Ok(HirExpr::Local(lid))
                } else if let Some(sym) = self.global_fn_aliases.get(&n).copied() {
                    Ok(HirExpr::Global(sym))
                } else {
                    Ok(HirExpr::Global(self.intern_symbol(&n)))
                }
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                let hop = match op {
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
                };
                Ok(HirExpr::Binary {
                    op: hop,
                    lhs: Box::new(self.lower_expr(*lhs)?),
                    rhs: Box::new(self.lower_expr(*rhs)?),
                })
            }
            ast::ExprKind::Unary { op, rhs } => {
                let hop = match op {
                    ast::UnaryOp::Not => HirUnOp::Not,
                    ast::UnaryOp::Neg => HirUnOp::Neg,
                };
                Ok(HirExpr::Unary {
                    op: hop,
                    expr: Box::new(self.lower_expr(*rhs)?),
                })
            }
            ast::ExprKind::Lambda {
                params,
                ret_ty_hint,
                body,
            } => self.lower_lambda_expr(params, ret_ty_hint, body, expr.span),
            ast::ExprKind::Call { callee, args } => {
                let dotted_callee = Self::dotted_name_from_expr(&callee);
                let c = if let Some(dotted) =
                    dotted_callee.filter(|d| self.root_is_unbound_for_dotted(d))
                {
                    self.lower_dotted_ref(&dotted)
                } else {
                    self.lower_expr(*callee)?
                };
                let mut hargs = Vec::new();
                for a in args {
                    match a.kind {
                        ast::ExprKind::NamedArg { name, value } => {
                            let sym = self.intern_symbol(&name);
                            hargs.push(HirArg::Named {
                                name: sym,
                                value: self.lower_expr(*value)?,
                            });
                        }
                        _ => hargs.push(HirArg::Pos(self.lower_expr(a)?)),
                    }
                }
                Ok(HirExpr::Call(HirCall {
                    callee: Box::new(c),
                    args: hargs,
                    span: expr.span,
                }))
            }
            ast::ExprKind::Pipe { lhs, rhs_call } => {
                let lhs_h = self.lower_expr(*lhs)?;
                match rhs_call.kind {
                    ast::ExprKind::Call { callee, args } => {
                        let c = self.lower_expr(*callee)?;
                        let mut hargs = Vec::with_capacity(args.len() + 1);
                        hargs.push(HirArg::Pos(lhs_h));
                        for a in args {
                            match a.kind {
                                ast::ExprKind::NamedArg { name, value } => {
                                    let sym = self.intern_symbol(&name);
                                    hargs.push(HirArg::Named {
                                        name: sym,
                                        value: self.lower_expr(*value)?,
                                    });
                                }
                                _ => hargs.push(HirArg::Pos(self.lower_expr(a)?)),
                            }
                        }
                        Ok(HirExpr::Call(HirCall {
                            callee: Box::new(c),
                            args: hargs,
                            span: expr.span,
                        }))
                    }
                    ast::ExprKind::Try { expr: inner } => match inner.kind {
                        ast::ExprKind::Call { callee, args } => {
                            let c = self.lower_expr(*callee)?;
                            let mut hargs = Vec::with_capacity(args.len() + 1);
                            hargs.push(HirArg::Pos(lhs_h));
                            for a in args {
                                match a.kind {
                                    ast::ExprKind::NamedArg { name, value } => {
                                        let sym = self.intern_symbol(&name);
                                        hargs.push(HirArg::Named {
                                            name: sym,
                                            value: self.lower_expr(*value)?,
                                        });
                                    }
                                    _ => hargs.push(HirArg::Pos(self.lower_expr(a)?)),
                                }
                            }
                            let call = HirExpr::Call(HirCall {
                                callee: Box::new(c),
                                args: hargs,
                                span: expr.span,
                            });
                            Ok(HirExpr::Try(Box::new(call)))
                        }
                        _ => Err(RRException::new(
                            "RR.ParseError",
                            RRCode::E0001,
                            Stage::Lower,
                            "RHS of |> must be call or call?".to_string(),
                        )),
                    },
                    _ => Err(RRException::new(
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Lower,
                        "RHS of |> must be call".to_string(),
                    )),
                }
            }
            ast::ExprKind::Field { base, name } => {
                if let Some(dotted) = Self::dotted_name_from_field(&base, &name)
                    .filter(|d| self.root_is_unbound_for_dotted(d))
                {
                    return Ok(self.lower_dotted_ref(&dotted));
                }
                let b = self.lower_expr(*base)?;
                let sym = self.intern_symbol(&name);
                Ok(HirExpr::Field {
                    base: Box::new(b),
                    name: sym,
                })
            }
            // v6 features
            ast::ExprKind::Match { scrutinee, arms } => {
                let s = self.lower_expr(*scrutinee)?;
                let mut harms = Vec::new();
                for arm in arms {
                    self.enter_scope(); // Arm scope
                    let pat = self.lower_pattern(arm.pat)?;
                    let guard = if let Some(g) = arm.guard {
                        Some(self.lower_expr(*g)?)
                    } else {
                        None
                    };
                    let body = self.lower_expr(*arm.body)?;
                    self.exit_scope();

                    harms.push(HirMatchArm {
                        pat,
                        guard,
                        body,
                        span: arm.span,
                    });
                }
                Ok(HirExpr::Match {
                    scrut: Box::new(s),
                    arms: harms,
                })
            }
            ast::ExprKind::Try { expr: e } => Ok(HirExpr::Try(Box::new(self.lower_expr(*e)?))),
            ast::ExprKind::Column(n) => Ok(HirExpr::Column(n)),
            ast::ExprKind::ColRef(n) => Ok(HirExpr::Column(n)),
            ast::ExprKind::Unquote(e) => {
                let inner = self.lower_expr(*e)?;
                Ok(HirExpr::Unquote(Box::new(inner)))
            }
            ast::ExprKind::Index { base, idx } => {
                let b = self.lower_expr(*base)?;
                let mut indices = Vec::new();
                for i in idx {
                    indices.push(self.lower_expr(i)?);
                }
                Ok(HirExpr::Index {
                    base: Box::new(b),
                    index: indices,
                })
            }
            ast::ExprKind::Range { a, b } => {
                let start = self.lower_expr(*a)?;
                let end = self.lower_expr(*b)?;
                Ok(HirExpr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                })
            }
            ast::ExprKind::VectorLit(elems) => {
                let mut helems = Vec::new();
                for e in elems {
                    helems.push(self.lower_expr(e)?);
                }
                Ok(HirExpr::VectorLit(helems))
            }
            ast::ExprKind::RecordLit(fields) => {
                let mut hfields = Vec::new();
                for (k, v) in fields {
                    let sym = self.intern_symbol(&k);
                    hfields.push((sym, self.lower_expr(v)?));
                }
                Ok(HirExpr::ListLit(hfields))
            }
            _ => Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!("unsupported expression in HIR lowering: {:?}", expr.kind),
            )
            .at(expr.span)
            .push_frame("hir::lower::lower_expr/1", Some(expr.span))),
        }
    }

    fn lower_pattern(&mut self, pat: ast::Pattern) -> RR<HirPat> {
        match pat.kind {
            ast::PatternKind::Wild => Ok(HirPat::Wild),
            ast::PatternKind::Lit(l) => {
                let hl = match l {
                    ast::Lit::Int(i) => HirLit::Int(i),
                    ast::Lit::Float(f) => HirLit::Double(f),
                    ast::Lit::Str(s) => HirLit::Char(s),
                    ast::Lit::Bool(b) => HirLit::Bool(b),
                    ast::Lit::Na => HirLit::NA,
                    ast::Lit::Null => HirLit::Null,
                };
                Ok(HirPat::Lit(hl))
            }
            ast::PatternKind::Bind(n) => {
                let lid = self.declare_local(&n);
                let sym = self.intern_symbol(&n);
                Ok(HirPat::Bind {
                    name: sym,
                    local: lid,
                })
            }
            ast::PatternKind::List { items, rest } => {
                let mut hitems = Vec::new();
                for i in items {
                    hitems.push(self.lower_pattern(i)?);
                }

                let hrest = if let Some(n) = rest {
                    let lid = self.declare_local(&n);
                    let sym = self.intern_symbol(&n);
                    Some((sym, lid))
                } else {
                    None
                };
                Ok(HirPat::List {
                    items: hitems,
                    rest: hrest,
                })
            }
            ast::PatternKind::Record { fields } => {
                let mut hfields = Vec::new();
                for (name, p) in fields {
                    let sym = self.intern_symbol(&name);
                    let hp = self.lower_pattern(p)?;
                    hfields.push((sym, hp));
                }
                Ok(HirPat::Record { fields: hfields })
            }
        }
    }

    fn lower_lvalue(&mut self, lval: ast::LValue) -> RR<HirLValue> {
        let lv_span = lval.span;
        match lval.kind {
            ast::LValueKind::Name(n) => {
                let lid = self.resolve_or_declare_local_for_assign(&n, lv_span)?;
                Ok(HirLValue::Local(lid))
            }
            ast::LValueKind::Index { base, idx } => {
                let b = self.lower_expr(base)?;
                let mut indices = Vec::new();
                for i in idx {
                    indices.push(self.lower_expr(i)?);
                }
                Ok(HirLValue::Index {
                    base: b,
                    index: indices,
                })
            }
            ast::LValueKind::Field { base, name } => {
                if let Some(dotted) = Self::dotted_name_from_field(&base, &name)
                    .filter(|d| self.root_is_unbound_for_dotted(d))
                {
                    let lid = self.resolve_or_declare_local_for_assign(&dotted, lv_span)?;
                    return Ok(HirLValue::Local(lid));
                }
                let b = self.lower_expr(base)?;
                let sym = self.intern_symbol(&name);
                Ok(HirLValue::Field { base: b, name: sym })
            }
        }
    }

    fn local_name_of_lvalue(&self, lval: &HirLValue) -> Option<String> {
        match lval {
            HirLValue::Local(id) => self.local_names.get(id).cloned(),
            _ => None,
        }
    }
}
