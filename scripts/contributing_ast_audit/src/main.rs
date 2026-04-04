use quote::ToTokens;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprMacro, ExprMethodCall, ExprUnsafe, File, FnArg, ItemFn,
    ItemStatic, Local, Pat, Signature, StaticMutability, Stmt, Type,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Finding {
    level: &'static str,
    code: &'static str,
    path: String,
    line: usize,
    message: &'static str,
}

struct AstAudit<'a> {
    path: &'a str,
    findings: Vec<Finding>,
    hash_backed_scopes: Vec<BTreeSet<String>>,
}

impl<'a> AstAudit<'a> {
    fn push(
        &mut self,
        level: &'static str,
        code: &'static str,
        line: usize,
        message: &'static str,
    ) {
        self.findings.push(Finding {
            level,
            code,
            path: self.path.to_string(),
            line,
            message,
        });
    }

    fn line<T: Spanned>(&self, node: &T) -> usize {
        node.span().start().line
    }

    fn push_scope(&mut self) {
        self.hash_backed_scopes.push(BTreeSet::new());
    }

    fn pop_scope(&mut self) {
        self.hash_backed_scopes.pop();
    }

    fn note_hash_backed_name(&mut self, name: &str) {
        if let Some(scope) = self.hash_backed_scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn is_hash_backed_name(&self, name: &str) -> bool {
        self.hash_backed_scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    fn scan_statements(&mut self, stmts: &[Stmt]) {
        for (idx, stmt) in stmts.iter().enumerate() {
            self.note_hash_binding(stmt);
            self.maybe_warn_unsorted_hash_collect(stmt, &stmts[idx + 1..]);
            self.maybe_note_stmt_macros(stmt);
            visit::visit_stmt(self, stmt);
        }
    }

    fn note_signature_hash_params(&mut self, sig: &Signature) {
        for input in &sig.inputs {
            if let FnArg::Typed(typed) = input
                && ty_looks_hash_backed(&typed.ty)
                && let Some(name) = binding_name(&typed.pat)
            {
                self.note_hash_backed_name(&name);
            }
        }
    }

    fn note_hash_binding(&mut self, stmt: &Stmt) {
        let Stmt::Local(local) = stmt else {
            return;
        };
        let Some(name) = local_binding_name(local) else {
            return;
        };
        let typed_hash = local_type(local).is_some_and(ty_looks_hash_backed);
        let init_hash = local
            .init
            .as_ref()
            .is_some_and(|init| expr_looks_hash_backed(&init.expr));
        if typed_hash || init_hash {
            self.note_hash_backed_name(&name);
        }
    }

    fn maybe_warn_unsorted_hash_collect(&mut self, stmt: &Stmt, lookahead: &[Stmt]) {
        let Stmt::Local(local) = stmt else {
            return;
        };
        let Some(dst_name) = local_binding_name(local) else {
            return;
        };
        let Some(init) = local.init.as_ref() else {
            return;
        };
        let Some(src_name) = collected_hash_source_name(&init.expr) else {
            return;
        };
        if !self.is_hash_backed_name(&src_name) {
            return;
        }
        let dest_is_vec_like = local_type(local)
            .map(type_tokens_from_type)
            .is_some_and(|tokens| tokens.contains("Vec<"));
        if !dest_is_vec_like {
            return;
        }
        if lookahead
            .iter()
            .take(5)
            .any(|stmt| stmt_sorts_name(stmt, &dst_name))
        {
            return;
        }
        self.push(
            "warn",
            "ast-hash-order-review",
            self.line(local),
            "AST audit: collected hash-backed keys/values into a vector without a nearby sort",
        );
    }

    fn maybe_note_stmt_macros(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Macro(stmt_macro) => {
                self.note_macro_path(&stmt_macro.mac.path, stmt_macro.span())
            }
            Stmt::Expr(Expr::Macro(expr_macro), _) => {
                self.note_macro_path(&expr_macro.mac.path, expr_macro.span())
            }
            _ => {}
        }
    }

    fn note_macro_path(&mut self, path: &syn::Path, span: proc_macro2::Span) {
        let line = span.start().line;
        if let Some(last) = path.segments.last() {
            match last.ident.to_string().as_str() {
                "panic" => self.push(
                    "error",
                    "ast-production-panic",
                    line,
                    "AST audit: panic! is forbidden on production compiler paths",
                ),
                "dbg" => self.push(
                    "error",
                    "ast-production-dbg",
                    line,
                    "AST audit: dbg!() is forbidden on production compiler paths",
                ),
                _ => {}
            }
        }
    }
}

fn path_segments(expr: &Expr) -> Vec<String> {
    match expr {
        Expr::Path(path) => path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

fn type_tokens(item: &ItemStatic) -> String {
    item.ty.to_token_stream().to_string().replace(' ', "")
}

fn attr_is_inline_always(attr: &Attribute) -> bool {
    attr.path().is_ident("inline") && attr.meta.to_token_stream().to_string().contains("always")
}

fn expr_is_format_like(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(lit) => matches!(lit.lit, syn::Lit::Str(_)),
        Expr::Macro(mac) => mac
            .mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "format"),
        Expr::Reference(reference) => expr_is_format_like(&reference.expr),
        _ => false,
    }
}

fn type_tokens_from_type(ty: &Type) -> String {
    ty.to_token_stream().to_string().replace(' ', "")
}

fn ty_looks_hash_backed(ty: &Type) -> bool {
    let tokens = type_tokens_from_type(ty);
    tokens.contains("HashMap<")
        || tokens.contains("HashSet<")
        || tokens.contains("FxHashMap<")
        || tokens.contains("FxHashSet<")
}

fn expr_looks_hash_backed(expr: &Expr) -> bool {
    let tokens = expr.to_token_stream().to_string().replace(' ', "");
    tokens.contains("HashMap::new(")
        || tokens.contains("HashSet::new(")
        || tokens.contains("FxHashMap::default(")
        || tokens.contains("FxHashSet::default(")
        || tokens.contains("HashMap::default(")
        || tokens.contains("HashSet::default(")
}

fn binding_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(ident) => Some(ident.ident.to_string()),
        Pat::Type(typed) => binding_name(&typed.pat),
        _ => None,
    }
}

fn local_binding_name(local: &Local) -> Option<String> {
    binding_name(&local.pat)
}

fn local_type(local: &Local) -> Option<&Type> {
    match &local.pat {
        Pat::Type(typed) => Some(&typed.ty),
        _ => None,
    }
}

fn path_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        Expr::Paren(paren) => path_name(&paren.expr),
        Expr::Reference(reference) => path_name(&reference.expr),
        _ => None,
    }
}

fn collected_hash_source_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::MethodCall(call) if call.method == "collect" => {
            collected_hash_source_name(&call.receiver)
        }
        Expr::MethodCall(call)
            if matches!(
                call.method.to_string().as_str(),
                "cloned" | "copied" | "iter" | "into_iter"
            ) =>
        {
            collected_hash_source_name(&call.receiver)
        }
        Expr::MethodCall(call) if matches!(call.method.to_string().as_str(), "keys" | "values") => {
            path_name(&call.receiver)
        }
        Expr::Paren(paren) => collected_hash_source_name(&paren.expr),
        Expr::Reference(reference) => collected_hash_source_name(&reference.expr),
        _ => None,
    }
}

fn stmt_sorts_name(stmt: &Stmt, name: &str) -> bool {
    let text = stmt.to_token_stream().to_string().replace(' ', "");
    text.contains(&format!("{name}.sort(")) || text.contains(&format!("{name}.sort_unstable("))
}

impl<'ast, 'a> Visit<'ast> for AstAudit<'a> {
    fn visit_item_static(&mut self, node: &'ast ItemStatic) {
        if matches!(node.mutability, StaticMutability::Mut(_)) {
            self.push(
                "error",
                "ast-static-mut",
                self.line(node),
                "AST audit: core compiler path contains static mut",
            );
        }

        let ty = type_tokens(node);
        if ty.contains("OnceLock<Mutex")
            || ty.contains("LazyLock<Mutex")
            || ty.contains("OnceLock<RwLock")
            || ty.contains("LazyLock<RwLock")
            || ty.contains("OnceLock<RefCell")
            || ty.contains("LazyLock<RefCell")
            || ty.contains("OnceLock<Cell")
            || ty.contains("LazyLock<Cell")
        {
            self.push(
                "warn",
                "ast-mutable-global-review",
                self.line(node),
                "AST audit: review mutable global state to confirm it cannot affect compilation results",
            );
        }
        visit::visit_item_static(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.push_scope();
        self.note_signature_hash_params(&node.sig);
        if node.sig.unsafety.is_some() {
            self.findings.push(Finding {
                level: "warn",
                code: "ast-unsafe-review",
                path: self.path.to_string(),
                line: self.line(node),
                message: "AST audit: unsafe fn detected; confirm adjacent SAFETY comment and narrow scope",
            });
        }
        for attr in &node.attrs {
            self.visit_attribute(attr);
        }
        self.scan_statements(&node.block.stmts);
        self.pop_scope();
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.push_scope();
        self.scan_statements(&node.stmts);
        self.pop_scope();
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        let segments = path_segments(&node.func);
        let line = self.line(node);
        match segments.as_slice() {
            [single] if single == "thread_rng" => self.push(
                "error",
                "ast-nondeterministic-rng",
                line,
                "AST audit: runtime randomness is not allowed on deterministic compiler paths",
            ),
            [left, right] if left == "rand" && right == "thread_rng" => self.push(
                "error",
                "ast-nondeterministic-rng",
                line,
                "AST audit: runtime randomness is not allowed on deterministic compiler paths",
            ),
            [single] if single == "getrandom" => self.push(
                "error",
                "ast-nondeterministic-rng",
                line,
                "AST audit: runtime randomness is not allowed on deterministic compiler paths",
            ),
            [left, right] if left == "thread" && right == "spawn" => self.push(
                "warn",
                "ast-thread-spawn-review",
                line,
                "AST audit: thread::spawn detected; review scheduler, determinism, and shutdown behavior",
            ),
            [one, two, three] if one == "std" && two == "thread" && three == "spawn" => self.push(
                "warn",
                "ast-thread-spawn-review",
                line,
                "AST audit: thread::spawn detected; review scheduler, determinism, and shutdown behavior",
            ),
            [left, right] if left == "Command" && right == "new" => self.push(
                "warn",
                "ast-process-command-review",
                line,
                "AST audit: Command::new detected; review hermeticity, cwd, and environment handling",
            ),
            [one, two, three, four]
                if one == "std" && two == "process" && three == "Command" && four == "new" =>
            {
                self.push(
                    "warn",
                    "ast-process-command-review",
                    line,
                    "AST audit: Command::new detected; review hermeticity, cwd, and environment handling",
                )
            }
            [left, right] if left == "SystemTime" && right == "now" => self.push(
                "warn",
                "ast-wall-clock-review",
                line,
                "AST audit: review wall-clock access to confirm it cannot affect deterministic compilation results",
            ),
            [left, right] if (left == "env" || left == "std") && right == "current_dir" => self.push(
                "warn",
                "ast-current-dir-review",
                line,
                "AST audit: review current_dir usage to confirm cwd-dependent paths are normalized before affecting compilation results",
            ),
            [one, two, three] if one == "std" && two == "env" && three == "current_dir" => self.push(
                "warn",
                "ast-current-dir-review",
                line,
                "AST audit: review current_dir usage to confirm cwd-dependent paths are normalized before affecting compilation results",
            ),
            [left, right] if (left == "env" || left == "std") && right == "temp_dir" => self.push(
                "warn",
                "ast-temp-dir-review",
                line,
                "AST audit: review temp_dir usage to confirm environment-specific paths stay outside correctness-affecting artifacts",
            ),
            [one, two, three] if one == "std" && two == "env" && three == "temp_dir" => self.push(
                "warn",
                "ast-temp-dir-review",
                line,
                "AST audit: review temp_dir usage to confirm environment-specific paths stay outside correctness-affecting artifacts",
            ),
            _ => {}
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        self.note_macro_path(&node.mac.path, node.span());
        visit::visit_expr_macro(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();
        let line = self.line(node);
        match method.as_str() {
            "unwrap" | "unwrap_err" => self.push(
                "error",
                "ast-production-unwrap",
                line,
                "AST audit: unwrap-style method call detected on production compiler path",
            ),
            "expect" if node.args.first().is_some_and(expr_is_format_like) => self.push(
                "error",
                "ast-production-unwrap",
                line,
                "AST audit: expect-style method call detected on production compiler path",
            ),
            "for_each" | "try_for_each" => self.push(
                "warn",
                "ast-for-each-review",
                line,
                "AST audit: review chained for_each/try_for_each for hidden side effects",
            ),
            _ => {}
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast ExprUnsafe) {
        self.push(
            "warn",
            "ast-unsafe-review",
            self.line(node),
            "AST audit: unsafe block detected; confirm adjacent SAFETY comment and narrow scope",
        );
        visit::visit_expr_unsafe(self, node);
    }

    fn visit_attribute(&mut self, node: &'ast Attribute) {
        if attr_is_inline_always(node) {
            self.push(
                "error",
                "ast-inline-always",
                self.line(node),
                "AST audit: #[inline(always)] requires benchmark-backed justification",
            );
        }
        visit::visit_attribute(self, node);
    }
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn load_paths(root: &Path, list_path: &Path) -> Result<Vec<PathBuf>, String> {
    let raw = fs::read_to_string(list_path)
        .map_err(|err| format!("failed to read file list '{}': {err}", list_path.display()))?;
    Ok(raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let path = PathBuf::from(line.trim());
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        })
        .collect())
}

fn parse_file(text: &str, path: &Path) -> Result<File, String> {
    syn::parse_file(text).map_err(|err| format!("failed to parse '{}': {err}", path.display()))
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "usage: rr-contributing-ast-audit <root> <file-list>".to_string())?;
    let list_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "usage: rr-contributing-ast-audit <root> <file-list>".to_string())?;
    if args.next().is_some() {
        return Err("usage: rr-contributing-ast-audit <root> <file-list>".to_string());
    }

    let mut findings = Vec::new();
    for path in load_paths(&root, &list_path)? {
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") || !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read '{}': {err}", path.display()))?;
        let parsed = parse_file(&text, &path)?;
        let shown = display_path(&root, &path);
        let mut visitor = AstAudit {
            path: &shown,
            findings: Vec::new(),
            hash_backed_scopes: Vec::new(),
        };
        visitor.visit_file(&parsed);
        findings.extend(visitor.findings);
    }

    findings.sort();
    for finding in findings {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            finding.level, finding.code, finding.path, finding.line, finding.message
        );
    }
    Ok(())
}
