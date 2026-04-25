use crate::error::{RR, RRCode, RRCtx, RRException, Stage};
use crate::syntax::ast::*;
use crate::syntax::lex::Lexer;
use crate::syntax::token::*;
use crate::utils::Span;
use crate::{bail, bail_at};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    peek: Token,
    previous_span: Span,
}

#[derive(PartialEq, PartialOrd)]
enum Precedence {
    Lowest,
    Formula,    // ~
    Pipe,       // |>
    LogicOr,    // ||
    LogicAnd,   // &&
    Equality,   // == !=
    Comparison, // < > <= >=
    Range,      // ..
    Sum,        // + -
    Product,    // * / %
    Prefix,     // -X !X
    Call,       // ( [
    Try,        // ? (Postfix)
}

impl<'a> Parser<'a> {
    fn dotted_segment_name(kind: &TokenKind) -> Option<String> {
        match kind {
            TokenKind::Ident(n) => Some(n.clone()),
            TokenKind::Match => Some("match".to_string()),
            // Allow common R-style dotted names like `is.na` / `is.null`.
            TokenKind::Na => Some("na".to_string()),
            TokenKind::Null => Some("null".to_string()),
            TokenKind::True => Some("true".to_string()),
            TokenKind::False => Some("false".to_string()),
            // Keep R-style selectors such as `utils.getAnywhere("x").where`
            // usable after reserving `where` for trait bounds.
            TokenKind::Where => Some("where".to_string()),
            _ => None,
        }
    }

    fn call_arg_name(kind: &TokenKind) -> Option<String> {
        match kind {
            TokenKind::Ident(name) => Some(name.clone()),
            // `where` is a trait-bound keyword in declarations, but R APIs also
            // use it as a named argument, e.g. `methods.findUnique(where = env)`.
            TokenKind::Where => Some("where".to_string()),
            _ => None,
        }
    }

    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token();
        let peek = lexer.next_token();
        Self {
            lexer,
            current,
            peek,
            previous_span: Span::default(),
        }
    }

    fn advance(&mut self) {
        self.previous_span = self.current.span;
        self.current = self.peek.clone();
        self.peek = self.lexer.next_token();
    }

    fn expect(&mut self, kind: TokenKind) -> RR<()> {
        if std::mem::discriminant(&self.current.kind) == std::mem::discriminant(&kind) {
            self.advance();
            Ok(())
        } else {
            bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected {:?}, got {:?}",
                kind,
                self.current.kind
            );
        }
    }

    fn parse_dotted_ident(&mut self, err_context: &str) -> RR<String> {
        let mut out = match Self::dotted_segment_name(&self.current.kind) {
            Some(name) => name,
            None => bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected identifier {}",
                err_context
            ),
        };
        self.advance();
        while self.current.kind == TokenKind::Dot {
            if Self::dotted_segment_name(&self.peek.kind).is_none() {
                break;
            }
            self.advance(); // dot
            let Some(seg) = Self::dotted_segment_name(&self.current.kind) else {
                break;
            };
            out.push('.');
            out.push_str(&seg);
            self.advance();
        }
        Ok(out)
    }

    fn parse_type_path_ident(&mut self, err_context: &str) -> RR<String> {
        let mut out = self.parse_dotted_ident(err_context)?;
        while self.current.kind == TokenKind::DoubleColon {
            if Self::dotted_segment_name(&self.peek.kind).is_none() {
                break;
            }
            self.advance(); // ::
            let Some(seg) = Self::dotted_segment_name(&self.current.kind) else {
                break;
            };
            out.push_str("::");
            out.push_str(&seg);
            self.advance();
        }
        Ok(out)
    }

    fn type_expr_key(expr: &TypeExpr) -> String {
        match expr {
            TypeExpr::Named(name) => name.clone(),
            TypeExpr::Generic { base, args } => {
                let args = args
                    .iter()
                    .map(Self::type_expr_key)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{base}<{args}>")
            }
        }
    }

    fn parse_fn_params(&mut self) -> RR<Vec<FnParam>> {
        let mut params = Vec::new();
        if self.current.kind == TokenKind::RParen {
            return Ok(params);
        }
        loop {
            let p_start = self.current.span;
            let name = self.parse_dotted_ident("in parameter list")?;
            let ty_hint = if self.current.kind == TokenKind::Colon {
                self.advance();
                Some(self.parse_type_expr("in parameter type annotation")?)
            } else {
                None
            };
            let default = if self.current.kind == TokenKind::Assign {
                self.advance();
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };
            let p_end = default
                .as_ref()
                .map(|e| e.span)
                .unwrap_or(self.current.span);
            params.push(FnParam {
                name,
                ty_hint,
                default,
                span: p_start.merge(p_end),
            });
            if self.current.kind == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }
        Ok(params)
    }

    fn parse_type_expr(&mut self, ctx: &str) -> RR<TypeExpr> {
        if self.current.kind == TokenKind::Lt {
            return self.parse_fully_qualified_type_projection(ctx);
        }

        if let TokenKind::Int(value) = self.current.kind {
            self.advance();
            return Ok(TypeExpr::Named(format!("#{value}")));
        }

        let base = self.parse_type_path_ident(ctx)?;
        if base == "dyn"
            && let Some(trait_name) = Self::dotted_segment_name(&self.current.kind)
        {
            self.advance();
            return Ok(TypeExpr::Named(format!("dyn {trait_name}")));
        }
        if self.current.kind != TokenKind::Lt {
            return Ok(TypeExpr::Named(base));
        }

        self.advance(); // <
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type_expr("in type parameter list")?);
            if self.current.kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }
        if self.current.kind != TokenKind::Gt {
            bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected '>' to close generic type '{}'",
                base
            );
        }
        self.advance(); // >
        Ok(TypeExpr::Generic { base, args })
    }

    fn parse_as_keyword(&mut self, ctx: &str) -> RR<()> {
        match &self.current.kind {
            TokenKind::Ident(name) if name == "as" => {
                self.advance();
                Ok(())
            }
            _ => bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected 'as' {}",
                ctx
            ),
        }
    }

    fn parse_fully_qualified_type_projection(&mut self, ctx: &str) -> RR<TypeExpr> {
        self.expect(TokenKind::Lt)?;
        let receiver_ty = self.parse_type_expr(ctx)?;
        self.parse_as_keyword("inside fully-qualified associated type")?;
        let _trait_name = self.parse_type_path_ident("after 'as' in associated type")?;
        self.expect(TokenKind::Gt)?;
        self.expect(TokenKind::DoubleColon)?;
        let assoc_name = self.parse_dotted_ident("after '::' in associated type")?;
        let base = format!("{}::{}", Self::type_expr_key(&receiver_ty), assoc_name);
        if self.current.kind != TokenKind::Lt {
            return Ok(TypeExpr::Named(base));
        }
        self.advance();
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type_expr("in associated type projection arguments")?);
            if self.current.kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(TokenKind::Gt)?;
        Ok(TypeExpr::Generic { base, args })
    }

    fn parse_optional_type_params(&mut self) -> RR<Vec<String>> {
        if self.current.kind != TokenKind::Lt {
            return Ok(Vec::new());
        }
        self.advance(); // <
        let mut params = Vec::new();
        loop {
            if matches!(&self.current.kind, TokenKind::Ident(name) if name == "const") {
                self.advance();
            }
            let name = match &self.current.kind {
                TokenKind::Ident(name) => name.clone(),
                _ => {
                    bail_at!(
                        self.current.span,
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Parse,
                        "Expected type parameter name"
                    );
                }
            };
            self.advance();
            if !name.starts_with('\'') {
                params.push(name);
            }
            if self.current.kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(TokenKind::Gt)?;
        Ok(params)
    }

    fn parse_optional_for_lifetime_binder(&mut self) -> RR<()> {
        if self.current.kind != TokenKind::For {
            return Ok(());
        }
        self.advance(); // for
        self.expect(TokenKind::Lt)?;
        loop {
            match &self.current.kind {
                TokenKind::Ident(name) if name.starts_with('\'') => self.advance(),
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected lifetime parameter in higher-ranked bound"
                ),
            }
            if self.current.kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(TokenKind::Gt)?;
        Ok(())
    }

    fn parse_optional_where_bounds(&mut self) -> RR<Vec<TraitBound>> {
        if self.current.kind != TokenKind::Where {
            return Ok(Vec::new());
        }
        self.advance(); // where
        let mut bounds = Vec::new();
        loop {
            self.parse_optional_for_lifetime_binder()?;
            let type_name = Self::type_expr_key(&self.parse_type_expr("in where clause")?);
            self.expect(TokenKind::Colon)?;
            let mut trait_names = Vec::new();
            loop {
                trait_names.push(self.parse_dotted_ident("after ':' in where clause")?);
                if self.current.kind == TokenKind::Plus {
                    self.advance();
                    continue;
                }
                break;
            }
            bounds.push(TraitBound {
                type_name,
                trait_names,
            });
            if self.current.kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }
        Ok(bounds)
    }

    fn token_precedence(kind: &TokenKind) -> Precedence {
        match kind {
            TokenKind::Tilde => Precedence::Formula,
            TokenKind::Pipe => Precedence::Pipe,
            TokenKind::Or => Precedence::LogicOr,
            TokenKind::And => Precedence::LogicAnd,
            TokenKind::Eq | TokenKind::Ne => Precedence::Equality,
            TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge => Precedence::Comparison,
            TokenKind::DotDot => Precedence::Range,
            TokenKind::Plus | TokenKind::Minus => Precedence::Sum,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent | TokenKind::MatMul => {
                Precedence::Product
            }
            TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot | TokenKind::DoubleColon => {
                Precedence::Call
            }
            TokenKind::Question => Precedence::Try,
            _ => Precedence::Lowest,
        }
    }

    fn compound_assign_binop(kind: &TokenKind) -> Option<BinOp> {
        match kind {
            TokenKind::PlusAssign => Some(BinOp::Add),
            TokenKind::MinusAssign => Some(BinOp::Sub),
            TokenKind::StarAssign => Some(BinOp::Mul),
            TokenKind::SlashAssign => Some(BinOp::Div),
            TokenKind::PercentAssign => Some(BinOp::Mod),
            _ => None,
        }
    }

    fn parse_call_args(&mut self) -> RR<(Vec<Expr>, Span)> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        if self.current.kind != TokenKind::RParen {
            loop {
                if let Some(name) = Self::call_arg_name(&self.current.kind) {
                    if matches!(self.peek.kind, TokenKind::Assign) {
                        let arg_start = self.current.span;
                        let arg_name = name;
                        self.advance(); // ident
                        self.expect(TokenKind::Assign)?; // '='
                        let value = self.parse_expr(Precedence::Lowest)?;
                        let arg_span = arg_start.merge(value.span);
                        args.push(Expr {
                            kind: ExprKind::NamedArg {
                                name: arg_name,
                                value: Box::new(value),
                            },
                            span: arg_span,
                        });
                    } else {
                        args.push(self.parse_expr(Precedence::Lowest)?);
                    }
                } else {
                    args.push(self.parse_expr(Precedence::Lowest)?);
                }
                if self.current.kind == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        let end = self.current.span;
        self.expect(TokenKind::RParen)?;
        Ok((args, end))
    }

    fn parse_explicit_type_args(&mut self) -> RR<Vec<TypeExpr>> {
        self.expect(TokenKind::DoubleColon)?;
        self.expect(TokenKind::Lt)?;
        let mut type_args = Vec::new();
        loop {
            type_args.push(self.parse_type_expr("in explicit call type arguments")?);
            if self.current.kind == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }
        self.expect(TokenKind::Gt)?;
        Ok(type_args)
    }

    fn is_stmt_start(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Let
                | TokenKind::Fn
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Next
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Ident(_)
                | TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::String(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Na
                | TokenKind::Match
                | TokenKind::Lt
                | TokenKind::At
                | TokenKind::Tilde
                | TokenKind::Caret
                | TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::LBrace
                | TokenKind::Minus
                | TokenKind::Bang
        )
    }

    fn current_is_structural_stmt_end(&self) -> bool {
        matches!(
            self.current.kind,
            TokenKind::RBrace | TokenKind::EOF | TokenKind::Else
        )
    }

    fn current_starts_stmt_on_new_line_after(&self, span: Span) -> bool {
        span.end_line > 0
            && Self::is_stmt_start(&self.current.kind)
            && self.current.span.start_line > span.end_line
    }

    fn current_starts_stmt_on_same_line_after(&self, span: Span) -> bool {
        span.end_line > 0
            && Self::is_stmt_start(&self.current.kind)
            && self.current.span.start_line == span.end_line
    }

    fn consume_stmt_end(&mut self, fallback: Span) -> RR<Span> {
        if let TokenKind::Invalid(msg) = &self.current.kind {
            bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "{}",
                msg
            );
        }
        if self.current_is_structural_stmt_end()
            || self.current_starts_stmt_on_new_line_after(fallback)
        {
            return Ok(fallback);
        }
        if self.current_starts_stmt_on_same_line_after(fallback) {
            bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "statements must be separated by a newline or '}}' before {:?}",
                self.current.kind
            );
        }
        bail_at!(
            self.current.span,
            "RR.ParseError",
            RRCode::E0001,
            Stage::Parse,
            "Expected statement boundary, got {:?}",
            self.current.kind
        );
    }

    fn recover_stmt_boundary(&mut self) {
        while self.current.kind != TokenKind::EOF {
            if matches!(self.current.kind, TokenKind::RBrace | TokenKind::Else) {
                break;
            }
            if Self::is_stmt_start(&self.current.kind) {
                break;
            }
            self.advance();
        }
    }

    pub fn parse_program(&mut self) -> RR<Program> {
        let mut stmts = Vec::new();
        let mut errors: Vec<RRException> = Vec::new();
        while self.current.kind != TokenKind::EOF {
            if let TokenKind::Invalid(msg) = &self.current.kind {
                errors.push(
                    RRException::new(
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Parse,
                        msg.to_string(),
                    )
                    .at(self.current.span)
                    .push_frame("Parser.parse_program/1", Some(self.current.span)),
                );
                self.recover_stmt_boundary();
                continue;
            }
            match self
                .parse_stmt()
                .ctx("Parser.parse_stmt/1", Some(self.current.span))
            {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => {
                    errors.push(e);
                    let before = self.current.span;
                    self.recover_stmt_boundary();
                    if self.current.span == before
                        && matches!(self.current.kind, TokenKind::RBrace | TokenKind::Else)
                    {
                        // Skip only structural boundaries that cannot start a valid statement.
                        self.advance();
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(Program { stmts })
        } else if errors.len() == 1 {
            Err(errors.remove(0))
        } else {
            Err(RRException::aggregate(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                format!("parse failed: {} error(s)", errors.len()),
                errors,
            ))
        }
    }

    fn parse_stmt(&mut self) -> RR<Stmt> {
        let _start_span = self.current.span;
        match self.current.kind {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Fn => {
                if self.peek.kind == TokenKind::LParen {
                    self.parse_start_ident_or_expr()
                } else {
                    self.parse_fn_decl()
                }
            }
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Next => self.parse_next_stmt(),
            TokenKind::Import => self.parse_import_stmt(),
            TokenKind::Export => self.parse_export_modifier(),
            TokenKind::Trait => self.parse_trait_decl(),
            TokenKind::Impl => self.parse_impl_decl(),
            _ => self.parse_start_ident_or_expr(),
        }
    }

    fn parse_let_stmt(&mut self) -> RR<Stmt> {
        // let ident = expr ;
        let start = self.current.span;
        self.advance(); // let

        let name = self.parse_dotted_ident("after let")?;
        let ty_hint = if self.current.kind == TokenKind::Colon {
            self.advance();
            Some(self.parse_type_expr("after ':' in let binding")?)
        } else {
            None
        };

        self.expect(TokenKind::Assign)?;
        let init = self.parse_expr(Precedence::Lowest)?;
        let end = self.consume_stmt_end(init.span)?;

        Ok(Stmt {
            kind: StmtKind::Let {
                name,
                ty_hint,
                init: Some(init),
            },
            span: start.merge(end),
        })
    }

    fn parse_fn_decl(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // fn
        let name = self.parse_dotted_ident("for fn")?;
        let type_params = self.parse_optional_type_params()?;

        self.expect(TokenKind::LParen)?;
        let params = self.parse_fn_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_ty_hint = if self.current.kind == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_expr("after function return arrow")?)
        } else {
            None
        };
        let where_bounds = self.parse_optional_where_bounds()?;

        let body = if self.current.kind == TokenKind::LBrace {
            self.parse_block()?
        } else if self.current.kind == TokenKind::Assign {
            // Expression-bodied function: fn f(a, b) = a + b
            self.advance();
            let expr = self.parse_expr(Precedence::Lowest)?;
            let end = self.consume_stmt_end(expr.span)?;
            let stmt = Stmt {
                kind: StmtKind::ExprStmt { expr: expr.clone() },
                span: expr.span,
            };
            Block {
                stmts: vec![stmt],
                span: start.merge(end),
            }
        } else {
            bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected function body ('{{ ... }}' or '= expr'), got {:?}",
                self.current.kind
            );
        };

        Ok(Stmt {
            kind: StmtKind::FnDecl {
                name,
                type_params,
                params,
                ret_ty_hint,
                where_bounds,
                body: body.clone(),
            },
            span: start.merge(body.span),
        })
    }

    fn parse_trait_method_sig(&mut self) -> RR<TraitMethodSig> {
        let start = self.current.span;
        self.advance(); // fn
        let name = self.parse_dotted_ident("for trait method")?;
        if self.current.kind == TokenKind::Lt {
            bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Trait method type parameters are not supported in this trait slice"
            );
        }
        self.expect(TokenKind::LParen)?;
        let params = self.parse_fn_params()?;
        self.expect(TokenKind::RParen)?;
        let fallback = self.previous_span;
        let ret_ty_hint = if self.current.kind == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_expr("after trait method return arrow")?)
        } else {
            None
        };
        let where_bounds = self.parse_optional_where_bounds()?;
        let mut default_body = None;
        let end = if self.current.kind == TokenKind::LBrace {
            let body = self.parse_block()?;
            let span = body.span;
            default_body = Some(body);
            span
        } else if self.current.kind == TokenKind::Assign {
            self.advance();
            let expr = self.parse_expr(Precedence::Lowest)?;
            let end = self.consume_stmt_end(expr.span)?;
            default_body = Some(Block {
                stmts: vec![Stmt {
                    kind: StmtKind::ExprStmt { expr: expr.clone() },
                    span: expr.span,
                }],
                span: start.merge(end),
            });
            end
        } else {
            let end_fallback = ret_ty_hint
                .as_ref()
                .map(|_| self.previous_span)
                .unwrap_or(fallback);
            self.consume_stmt_end(end_fallback)?
        };
        Ok(TraitMethodSig {
            name,
            params,
            ret_ty_hint,
            where_bounds,
            default_body,
            span: start.merge(end),
        })
    }

    fn parse_trait_supertraits(&mut self) -> RR<Vec<String>> {
        if self.current.kind != TokenKind::Colon {
            return Ok(Vec::new());
        }
        self.advance(); // :
        let mut out = Vec::new();
        loop {
            out.push(self.parse_dotted_ident("after ':' in trait supertrait list")?);
            if self.current.kind == TokenKind::Plus {
                self.advance();
                continue;
            }
            break;
        }
        Ok(out)
    }

    fn parse_trait_assoc_type(&mut self) -> RR<TraitAssocType> {
        let start = self.current.span;
        self.advance(); // type
        let name = self.parse_dotted_ident("after associated type")?;
        let type_params = self.parse_optional_type_params()?;
        let end = self.consume_stmt_end(self.previous_span)?;
        Ok(TraitAssocType {
            name,
            type_params,
            span: start.merge(end),
        })
    }

    fn parse_impl_assoc_type(&mut self) -> RR<ImplAssocType> {
        let start = self.current.span;
        self.advance(); // type
        let name = self.parse_dotted_ident("after associated type")?;
        let name = if self.current.kind == TokenKind::Lt {
            self.advance();
            let mut args = Vec::new();
            loop {
                args.push(Self::type_expr_key(
                    &self.parse_type_expr("in associated type arguments")?,
                ));
                if self.current.kind == TokenKind::Comma {
                    self.advance();
                    continue;
                }
                break;
            }
            self.expect(TokenKind::Gt)?;
            format!("{}<{}>", name, args.join(","))
        } else {
            name
        };
        self.expect(TokenKind::Assign)?;
        let ty = self.parse_type_expr("after associated type '='")?;
        let end = self.consume_stmt_end(self.previous_span)?;
        Ok(ImplAssocType {
            name,
            ty,
            span: start.merge(end),
        })
    }

    fn current_is_const_keyword(&self) -> bool {
        matches!(&self.current.kind, TokenKind::Ident(name) if name == "const")
    }

    fn parse_trait_assoc_const(&mut self) -> RR<TraitAssocConst> {
        let start = self.current.span;
        self.advance(); // const
        let name = self.parse_dotted_ident("after associated const")?;
        self.expect(TokenKind::Colon)?;
        let ty_hint = self.parse_type_expr("after associated const ':'")?;
        let mut default = None;
        let fallback = self.previous_span;
        let end = if self.current.kind == TokenKind::Assign {
            self.advance();
            let expr = self.parse_expr(Precedence::Lowest)?;
            let end = self.consume_stmt_end(expr.span)?;
            default = Some(expr);
            end
        } else {
            self.consume_stmt_end(fallback)?
        };
        Ok(TraitAssocConst {
            name,
            ty_hint,
            default,
            span: start.merge(end),
        })
    }

    fn parse_impl_assoc_const(&mut self) -> RR<ImplAssocConst> {
        let start = self.current.span;
        self.advance(); // const
        let name = self.parse_dotted_ident("after associated const")?;
        self.expect(TokenKind::Colon)?;
        let ty_hint = self.parse_type_expr("after associated const ':'")?;
        self.expect(TokenKind::Assign)?;
        let value = self.parse_expr(Precedence::Lowest)?;
        let end = self.consume_stmt_end(value.span)?;
        Ok(ImplAssocConst {
            name,
            ty_hint,
            value,
            span: start.merge(end),
        })
    }

    fn parse_trait_decl(&mut self) -> RR<Stmt> {
        self.parse_trait_decl_with_visibility(false)
    }

    fn parse_trait_decl_with_visibility(&mut self, public: bool) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // trait
        let name = self.parse_dotted_ident("after trait")?;
        let type_params = self.parse_optional_type_params()?;
        let supertraits = self.parse_trait_supertraits()?;
        let where_bounds = self.parse_optional_where_bounds()?;
        self.expect(TokenKind::LBrace)?;
        let mut assoc_types = Vec::new();
        let mut assoc_consts = Vec::new();
        let mut methods = Vec::new();
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::EOF {
            if matches!(&self.current.kind, TokenKind::Ident(name) if name == "type") {
                assoc_types.push(self.parse_trait_assoc_type()?);
                continue;
            }
            if self.current_is_const_keyword() {
                assoc_consts.push(self.parse_trait_assoc_const()?);
                continue;
            }
            if self.current.kind != TokenKind::Fn {
                bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected trait method signature, associated type, or associated const"
                );
            }
            methods.push(self.parse_trait_method_sig()?);
        }
        let end = self.current.span;
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt {
            kind: StmtKind::TraitDecl(TraitDecl {
                name,
                type_params,
                supertraits,
                where_bounds,
                assoc_types,
                assoc_consts,
                methods,
                public,
            }),
            span: start.merge(end),
        })
    }

    fn parse_impl_decl(&mut self) -> RR<Stmt> {
        self.parse_impl_decl_with_visibility(false)
    }

    fn parse_impl_decl_with_visibility(&mut self, public: bool) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // impl
        let type_params = self.parse_optional_type_params()?;
        let negative = if self.current.kind == TokenKind::Bang {
            self.advance();
            true
        } else {
            false
        };
        let trait_name = self.parse_dotted_ident("after impl")?;
        self.expect(TokenKind::For)?;
        let for_ty = self.parse_type_expr("after impl Trait for")?;
        let where_bounds = self.parse_optional_where_bounds()?;
        self.expect(TokenKind::LBrace)?;
        let mut assoc_types = Vec::new();
        let mut assoc_consts = Vec::new();
        let mut methods = Vec::new();
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::EOF {
            if matches!(&self.current.kind, TokenKind::Ident(name) if name == "type") {
                assoc_types.push(self.parse_impl_assoc_type()?);
                continue;
            }
            if self.current_is_const_keyword() {
                assoc_consts.push(self.parse_impl_assoc_const()?);
                continue;
            }
            if self.current.kind != TokenKind::Fn {
                bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected function, associated type, or associated const inside impl block"
                );
            }
            let stmt = self.parse_fn_decl()?;
            let StmtKind::FnDecl {
                name,
                type_params,
                params,
                ret_ty_hint,
                where_bounds,
                body,
            } = stmt.kind
            else {
                unreachable!("parse_fn_decl always returns a function declaration");
            };
            methods.push(FnDecl {
                name,
                type_params,
                params,
                ret_ty_hint,
                where_bounds,
                body,
                public: false,
            });
        }
        let end = self.current.span;
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt {
            kind: StmtKind::ImplDecl(ImplDecl {
                trait_name,
                type_params,
                negative,
                for_ty,
                where_bounds,
                assoc_types,
                assoc_consts,
                methods,
                public,
            }),
            span: start.merge(end),
        })
    }

    fn parse_block(&mut self) -> RR<Block> {
        let start = self.current.span;
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        let mut errors: Vec<RRException> = Vec::new();
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::EOF {
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => {
                    errors.push(e);
                    let before = self.current.span;
                    self.recover_stmt_boundary();
                    if self.current.span == before
                        && matches!(self.current.kind, TokenKind::RBrace | TokenKind::Else)
                    {
                        // Skip only structural boundaries that cannot start a valid statement.
                        self.advance();
                    }
                }
            }
        }
        let end = self.current.span;
        if self.current.kind == TokenKind::RBrace {
            self.advance();
        } else {
            errors.push(
                RRException::new(
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected RBrace, got EOF".to_string(),
                )
                .at(self.current.span)
                .push_frame("Parser.parse_block/1", Some(self.current.span)),
            );
        }

        if errors.is_empty() {
            Ok(Block {
                stmts,
                span: start.merge(end),
            })
        } else if errors.len() == 1 {
            Err(errors.remove(0))
        } else {
            Err(RRException::aggregate(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                format!("block parse failed: {} error(s)", errors.len()),
                errors,
            ))
        }
    }

    fn parse_if_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // if
        let cond = if self.current.kind == TokenKind::LParen {
            self.advance();
            let c = self.parse_expr(Precedence::Lowest)?;
            self.expect(TokenKind::RParen)?;
            c
        } else {
            // R++ style: if cond { ... } / if cond stmt
            self.parse_expr(Precedence::Lowest)?
        };
        let then_blk = self.parse_stmt_or_block()?;
        let else_blk = if self.current.kind == TokenKind::Else {
            self.advance();
            Some(self.parse_stmt_or_block()?)
        } else {
            None
        };

        let end_span = else_blk.as_ref().map(|b| b.span).unwrap_or(then_blk.span);
        Ok(Stmt {
            kind: StmtKind::If {
                cond,
                then_blk,
                else_blk,
            },
            span: start.merge(end_span),
        })
    }

    fn parse_stmt_or_block(&mut self) -> RR<Block> {
        if self.current.kind == TokenKind::LBrace {
            return self.parse_block();
        }
        let stmt = self.parse_stmt()?;
        let span = stmt.span;
        Ok(Block {
            stmts: vec![stmt],
            span,
        })
    }

    fn parse_while_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // while
        let cond = if self.current.kind == TokenKind::LParen {
            self.advance();
            let c = self.parse_expr(Precedence::Lowest)?;
            self.expect(TokenKind::RParen)?;
            c
        } else {
            // R++ style: while cond { ... } / while cond stmt
            self.parse_expr(Precedence::Lowest)?
        };
        let body = self.parse_stmt_or_block()?;
        Ok(Stmt {
            kind: StmtKind::While {
                cond,
                body: body.clone(),
            },
            span: start.merge(body.span),
        })
    }

    fn parse_for_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // for
        let has_paren = self.current.kind == TokenKind::LParen;
        if has_paren {
            self.advance();
        }
        let var = match &self.current.kind {
            TokenKind::Ident(n) => n.clone(),
            _ => bail!(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected identifier in for"
            ),
        };
        self.advance();
        self.expect(TokenKind::In)?;
        let iter = self.parse_expr(Precedence::Lowest)?;
        if has_paren {
            self.expect(TokenKind::RParen)?;
        }
        let body = self.parse_stmt_or_block()?;
        Ok(Stmt {
            kind: StmtKind::For {
                var,
                iter,
                body: body.clone(),
            },
            span: start.merge(body.span),
        })
    }

    fn parse_return_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // return
        // `return` without a value is allowed only at a structural boundary or
        // when the next statement starts on a later line.
        let value = if self.current_is_structural_stmt_end()
            || self.current_starts_stmt_on_new_line_after(start)
        {
            None
        } else {
            Some(self.parse_expr(Precedence::Lowest)?)
        };
        let end_fallback = value.as_ref().map(|v| v.span).unwrap_or(start);
        let end = self.consume_stmt_end(end_fallback)?;
        Ok(Stmt {
            kind: StmtKind::Return { value },
            span: start.merge(end),
        })
    }

    fn parse_break_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // break
        let end = self.consume_stmt_end(start)?;
        Ok(Stmt {
            kind: StmtKind::Break,
            span: start.merge(end),
        })
    }

    fn parse_next_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // next
        let end = self.consume_stmt_end(start)?;
        Ok(Stmt {
            kind: StmtKind::Next,
            span: start.merge(end),
        })
    }

    fn parse_import_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // import

        let mut source = ImportSource::Module;
        if let TokenKind::Ident(name) = &self.current.kind
            && name == "r"
        {
            source = ImportSource::RPackage;
            self.advance();
        }

        let (path, spec, end_fallback) = if source == ImportSource::RPackage
            && matches!(&self.current.kind, TokenKind::Ident(name) if name == "default")
        {
            self.advance();
            match &self.current.kind {
                TokenKind::Ident(name) if name == "from" => self.advance(),
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected 'from' after 'default' in R import"
                ),
            }
            let pkg = match &self.current.kind {
                TokenKind::String(s) => {
                    let pkg = s.clone();
                    self.advance();
                    pkg
                }
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected package string after 'from'"
                ),
            };
            let alias = pkg.clone();
            (pkg, ImportSpec::Namespace(alias), self.previous_span)
        } else if source == ImportSource::RPackage && self.current.kind == TokenKind::LBrace {
            let mut bindings = Vec::new();
            self.advance(); // {
            while self.current.kind != TokenKind::RBrace {
                let imported = self.parse_dotted_ident("in R package import list")?;
                let local = match &self.current.kind {
                    TokenKind::Ident(name) if name == "as" => {
                        self.advance();
                        Some(self.parse_dotted_ident("after 'as' in R import list")?)
                    }
                    _ => None,
                };
                bindings.push(ImportBinding { imported, local });
                if self.current.kind == TokenKind::Comma {
                    self.advance();
                    continue;
                }
                break;
            }
            self.expect(TokenKind::RBrace)?;
            match &self.current.kind {
                TokenKind::Ident(name) if name == "from" => self.advance(),
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected 'from' after R import list"
                ),
            }
            let pkg = match &self.current.kind {
                TokenKind::String(s) => {
                    let pkg = s.clone();
                    self.advance();
                    pkg
                }
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected package string after 'from'"
                ),
            };
            (pkg, ImportSpec::Named(bindings), self.previous_span)
        } else if source == ImportSource::RPackage && self.current.kind == TokenKind::Star {
            self.advance(); // *
            match &self.current.kind {
                TokenKind::Ident(name) if name == "as" => self.advance(),
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected 'as' after '*' in R namespace import"
                ),
            }
            let alias = self.parse_dotted_ident("after 'as' in R namespace import")?;
            match &self.current.kind {
                TokenKind::Ident(name) if name == "from" => self.advance(),
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected 'from' after R namespace alias"
                ),
            }
            let pkg = match &self.current.kind {
                TokenKind::String(s) => {
                    let pkg = s.clone();
                    self.advance();
                    pkg
                }
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected package string after 'from'"
                ),
            };
            (pkg, ImportSpec::Namespace(alias), self.previous_span)
        } else {
            let path = match &self.current.kind {
                TokenKind::String(s) => {
                    let path = s.clone();
                    self.advance();
                    path
                }
                _ => bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Expected string after import"
                ),
            };
            (path, ImportSpec::Glob, self.previous_span)
        };

        let end = self.consume_stmt_end(end_fallback)?;

        Ok(Stmt {
            kind: StmtKind::Import { source, path, spec },
            span: start.merge(end),
        })
    }

    fn parse_export_modifier(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // export

        if self.current.kind == TokenKind::Trait {
            let mut stmt = self.parse_trait_decl_with_visibility(true)?;
            stmt.span = start.merge(stmt.span);
            return Ok(stmt);
        }

        if self.current.kind == TokenKind::Impl {
            let mut stmt = self.parse_impl_decl_with_visibility(true)?;
            stmt.span = start.merge(stmt.span);
            return Ok(stmt);
        }

        // Expect fn declaration
        let stmt = self.parse_fn_decl()?;

        if let StmtKind::FnDecl {
            name,
            type_params,
            params,
            ret_ty_hint,
            where_bounds,
            body,
        } = stmt.kind
        {
            Ok(Stmt {
                kind: StmtKind::Export(FnDecl {
                    name,
                    type_params,
                    params,
                    ret_ty_hint,
                    where_bounds,
                    body,
                    public: true,
                }),
                span: start.merge(stmt.span),
            })
        } else {
            bail_at!(
                start,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected function, trait, or impl after export"
            );
        }
    }

    fn parse_start_ident_or_expr(&mut self) -> RR<Stmt> {
        // Can be Assign (x = ...) or ExprStmt (x or call(x))
        let start = self.current.span;
        let expr = self.parse_expr(Precedence::Lowest)?;

        if self.current.kind == TokenKind::Colon {
            // Typed declaration sugar: x: int = expr
            let name = match expr.kind {
                ExprKind::Name(n) => n,
                _ => {
                    bail_at!(
                        expr.span,
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Parse,
                        "Typed declaration target must be a plain name"
                    );
                }
            };
            self.advance(); // :
            let ty_hint = self.parse_type_expr("after ':' in typed declaration")?;
            self.expect(TokenKind::Assign)?;
            let value = self.parse_expr(Precedence::Lowest)?;
            let end = self.consume_stmt_end(value.span)?;
            Ok(Stmt {
                kind: StmtKind::Let {
                    name,
                    ty_hint: Some(ty_hint),
                    init: Some(value),
                },
                span: start.merge(end),
            })
        } else if self.current.kind == TokenKind::Assign {
            // It's an assignment
            self.advance();
            let value = self.parse_expr(Precedence::Lowest)?;
            let end = self.consume_stmt_end(value.span)?;

            // Convert expr to LValue
            let lvalue = self.expr_to_lvalue(expr)?;
            Ok(Stmt {
                kind: StmtKind::Assign {
                    target: lvalue,
                    value,
                },
                span: start.merge(end),
            })
        } else if let Some(op) = Self::compound_assign_binop(&self.current.kind) {
            // Compound assignment sugar:
            //   x += y       -> x = x + y
            //   a[i] += y    -> a[i] = a[i] + y
            //   rec.x += y   -> rec.x = rec.x + y
            let lhs_expr = expr.clone();
            self.advance();
            let rhs = self.parse_expr(Precedence::Lowest)?;
            let value_span = lhs_expr.span.merge(rhs.span);
            let end = self.consume_stmt_end(rhs.span)?;
            let lvalue = self.expr_to_lvalue(expr)?;
            let value = Expr {
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs_expr),
                    rhs: Box::new(rhs),
                },
                span: value_span,
            };
            Ok(Stmt {
                kind: StmtKind::Assign {
                    target: lvalue,
                    value,
                },
                span: start.merge(end),
            })
        } else {
            let end = self.consume_stmt_end(expr.span)?;
            Ok(Stmt {
                kind: StmtKind::ExprStmt { expr },
                span: start.merge(end),
            })
        }
    }

    fn expr_to_lvalue(&self, expr: Expr) -> RR<LValue> {
        match expr.kind {
            ExprKind::Name(n) => Ok(LValue {
                kind: LValueKind::Name(n),
                span: expr.span,
            }),
            ExprKind::Index { base, idx } => Ok(LValue {
                kind: LValueKind::Index { base: *base, idx },
                span: expr.span,
            }),
            ExprKind::Field { base, name } => Ok(LValue {
                kind: LValueKind::Field { base: *base, name },
                span: expr.span,
            }),
            _ => bail!(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Parse,
                "Invalid lvalue: {:?}",
                expr
            ),
        }
    }

    fn parse_expr(&mut self, precedence: Precedence) -> RR<Expr> {
        let mut left = self.parse_prefix()?;

        while precedence < Self::token_precedence(&self.current.kind) {
            // A leading `<Type as Trait>::Item` can start a new statement.
            // Do not swallow it as a cross-line comparison continuation.
            if self.current.span.start_line > left.span.end_line
                && matches!(self.current.kind, TokenKind::Lt)
            {
                break;
            }
            // R-style newline statement termination:
            // don't continue postfix chains across a line break.
            if self.current.span.start_line > left.span.end_line
                && matches!(
                    self.current.kind,
                    TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot
                )
            {
                break;
            }
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> RR<Expr> {
        let start = self.current.span;
        match &self.current.kind {
            TokenKind::Invalid(msg) => {
                bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "{}",
                    msg
                )
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Name(name),
                    span: start,
                })
            }
            TokenKind::Int(i) => {
                let i = *i;
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Lit(Lit::Int(i)),
                    span: start,
                })
            }
            TokenKind::Float(f) => {
                let f = *f;
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Lit(Lit::Float(f)),
                    span: start,
                })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Lit(Lit::Str(s)),
                    span: start,
                })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Lit(Lit::Bool(true)),
                    span: start,
                })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Lit(Lit::Bool(false)),
                    span: start,
                })
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Lit(Lit::Null),
                    span: start,
                })
            }
            TokenKind::Na => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Lit(Lit::Na),
                    span: start,
                })
            }
            TokenKind::Fn => self.parse_lambda_expr(),

            TokenKind::Match => self.parse_match(),
            TokenKind::At => {
                self.advance();
                let name = match &self.current.kind {
                    TokenKind::Ident(n) => n.clone(),
                    _ => bail_at!(
                        self.current.span,
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Parse,
                        "Expected identifier after @"
                    ),
                };
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Column(name),
                    span: start.merge(self.current.span),
                })
            }
            TokenKind::Caret => {
                self.advance();
                let val = self.parse_expr(Precedence::Prefix)?;
                let end = val.span;
                Ok(Expr {
                    kind: ExprKind::Unquote(Box::new(val)),
                    span: start.merge(end),
                })
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr(Precedence::Lowest)?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::Lt => self.parse_fully_qualified_assoc_expr(),
            TokenKind::LBracket => {
                // Vector Decl: [1, 2]
                self.advance();
                let mut elems = Vec::new();
                if self.current.kind != TokenKind::RBracket {
                    loop {
                        elems.push(self.parse_expr(Precedence::Lowest)?);
                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                let end = self.current.span;
                self.expect(TokenKind::RBracket)?;
                Ok(Expr {
                    kind: ExprKind::VectorLit(elems),
                    span: start.merge(end),
                })
            }
            TokenKind::LBrace => {
                // Record Decl: { a: 1, b: 2 }
                self.advance();
                let mut fields = Vec::new();
                if self.current.kind != TokenKind::RBrace {
                    loop {
                        let name = match &self.current.kind {
                            TokenKind::Ident(n) => n.clone(),
                            _ => bail!(
                                "RR.ParseError",
                                RRCode::E0001,
                                Stage::Parse,
                                "Expected field name in record"
                            ),
                        };
                        self.advance();
                        self.expect(TokenKind::Colon)?;
                        let val = self.parse_expr(Precedence::Lowest)?;
                        fields.push((name, val));

                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                let end = self.current.span;
                self.expect(TokenKind::RBrace)?;
                Ok(Expr {
                    kind: ExprKind::RecordLit(fields),
                    span: start.merge(end),
                })
            }
            TokenKind::Minus => {
                self.advance();
                let rhs = self.parse_expr(Precedence::Prefix)?;
                let end = rhs.span;
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Neg,
                        rhs: Box::new(rhs),
                    },
                    span: start.merge(end),
                })
            }
            TokenKind::Bang => {
                self.advance();
                let rhs = self.parse_expr(Precedence::Prefix)?;
                let end = rhs.span;
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Not,
                        rhs: Box::new(rhs),
                    },
                    span: start.merge(end),
                })
            }
            TokenKind::Tilde => {
                self.advance();
                let rhs = self.parse_expr(Precedence::Formula)?;
                let end = rhs.span;
                Ok(Expr {
                    kind: ExprKind::Formula {
                        lhs: None,
                        rhs: Box::new(rhs),
                    },
                    span: start.merge(end),
                })
            }
            _ => bail!(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Unexpected token in prefix: {:?}",
                self.current.kind
            ),
        }
    }

    fn parse_lambda_expr(&mut self) -> RR<Expr> {
        let start = self.current.span;
        self.advance(); // fn
        self.expect(TokenKind::LParen)?;
        let params = self.parse_fn_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_ty_hint = if self.current.kind == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_expr("after lambda return arrow")?)
        } else {
            None
        };
        let body = if self.current.kind == TokenKind::LBrace {
            self.parse_block()?
        } else if self.current.kind == TokenKind::Assign {
            // Expression-bodied lambda: function(a, b) = a + b
            self.advance();
            let expr = self.parse_expr(Precedence::Lowest)?;
            let stmt = Stmt {
                kind: StmtKind::ExprStmt { expr: expr.clone() },
                span: expr.span,
            };
            Block {
                stmts: vec![stmt],
                span: start.merge(expr.span),
            }
        } else {
            bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected lambda body ('{{ ... }}' or '= expr'), got {:?}",
                self.current.kind
            );
        };
        Ok(Expr {
            kind: ExprKind::Lambda {
                params,
                ret_ty_hint,
                body: body.clone(),
            },
            span: start.merge(body.span),
        })
    }

    fn parse_fully_qualified_assoc_expr(&mut self) -> RR<Expr> {
        let start = self.current.span;
        self.expect(TokenKind::Lt)?;
        let receiver_ty = self.parse_type_expr("inside fully-qualified associated item")?;
        self.parse_as_keyword("inside fully-qualified associated item")?;
        let trait_name = self.parse_type_path_ident("after 'as' in associated item")?;
        self.expect(TokenKind::Gt)?;
        self.expect(TokenKind::DoubleColon)?;
        let item_name = self.parse_dotted_ident("after '::' in associated item")?;

        let callee_span = start.merge(self.current.span);
        let callee = Expr {
            kind: ExprKind::Field {
                base: Box::new(Expr {
                    kind: ExprKind::Name(trait_name),
                    span: start,
                }),
                name: item_name,
            },
            span: callee_span,
        };
        let (args, end) = if self.current.kind == TokenKind::LParen {
            self.parse_call_args()?
        } else {
            (Vec::new(), callee_span)
        };
        Ok(Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                type_args: vec![receiver_ty],
                args,
            },
            span: start.merge(end),
        })
    }

    fn parse_infix(&mut self, left: Expr) -> RR<Expr> {
        let start = left.span;
        let kind = self.current.kind.clone();

        match kind {
            TokenKind::LParen => {
                let (args, end) = self.parse_call_args()?;
                Ok(Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(left),
                        type_args: Vec::new(),
                        args,
                    },
                    span: start.merge(end),
                })
            }
            TokenKind::DoubleColon => {
                if self.peek.kind == TokenKind::Lt {
                    let type_args = self.parse_explicit_type_args()?;
                    let (args, end) = self.parse_call_args()?;
                    Ok(Expr {
                        kind: ExprKind::Call {
                            callee: Box::new(left),
                            type_args,
                            args,
                        },
                        span: start.merge(end),
                    })
                } else {
                    self.advance(); // ::
                    let name = match Self::dotted_segment_name(&self.current.kind) {
                        Some(name) => name,
                        None => bail_at!(
                            self.current.span,
                            "RR.ParseError",
                            RRCode::E0001,
                            Stage::Parse,
                            "Expected identifier after '::'"
                        ),
                    };
                    let end = self.current.span;
                    self.advance();
                    Ok(Expr {
                        kind: ExprKind::Field {
                            base: Box::new(left),
                            name,
                        },
                        span: start.merge(end),
                    })
                }
            }
            TokenKind::LBracket => {
                // Index
                self.advance();
                let mut idx = Vec::new();
                loop {
                    idx.push(self.parse_expr(Precedence::Lowest)?);
                    if self.current.kind == TokenKind::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let end = self.current.span;
                self.expect(TokenKind::RBracket)?;
                Ok(Expr {
                    kind: ExprKind::Index {
                        base: Box::new(left),
                        idx,
                    },
                    span: start.merge(end),
                })
            }
            TokenKind::Dot => {
                self.advance(); // consume '.'
                let (name, end) = match Self::dotted_segment_name(&self.current.kind) {
                    Some(n) => (n, self.current.span),
                    _ => bail!(
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Parse,
                        "Expected field name after '.', got {:?}",
                        self.current.kind
                    ),
                };
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Field {
                        base: Box::new(left),
                        name,
                    },
                    span: start.merge(end),
                })
            }
            TokenKind::DotDot => {
                self.advance();
                let right = self.parse_expr(Precedence::Range)?;
                let end = right.span;
                Ok(Expr {
                    kind: ExprKind::Range {
                        a: Box::new(left),
                        b: Box::new(right),
                    },
                    span: start.merge(end),
                })
            }
            TokenKind::Pipe => {
                self.advance();
                let rhs = self.parse_expr(Precedence::Pipe)?;
                if let ExprKind::Call { .. } = rhs.kind {
                    let end = rhs.span;
                    Ok(Expr {
                        kind: ExprKind::Pipe {
                            lhs: Box::new(left),
                            rhs_call: Box::new(rhs),
                        },
                        span: start.merge(end),
                    })
                } else if let ExprKind::Try { .. } = rhs.kind {
                    // Allow pipe to try? x |> f?
                    // If rhs is Try(Call(..)), valid?
                    // The parser parsed rhs as Try(Call).
                    let end = rhs.span;
                    Ok(Expr {
                        kind: ExprKind::Pipe {
                            lhs: Box::new(left),
                            rhs_call: Box::new(rhs),
                        },
                        span: start.merge(end),
                    })
                } else {
                    bail!(
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Parse,
                        "RHS of |> must be a function call (got {:?})",
                        rhs.kind
                    );
                }
            }
            TokenKind::Question => {
                // Postfix Try ?
                let end = self.current.span;
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Try {
                        expr: Box::new(left),
                    },
                    span: start.merge(end),
                })
            }
            TokenKind::Tilde => {
                self.advance();
                let right = self.parse_expr(Precedence::Formula)?;
                let end = right.span;
                Ok(Expr {
                    kind: ExprKind::Formula {
                        lhs: Some(Box::new(left)),
                        rhs: Box::new(right),
                    },
                    span: start.merge(end),
                })
            }
            _ => {
                // Binary Op
                let op = match kind {
                    TokenKind::Plus => BinOp::Add,
                    TokenKind::Minus => BinOp::Sub,
                    TokenKind::Star => BinOp::Mul,
                    TokenKind::Slash => BinOp::Div,
                    TokenKind::Percent => BinOp::Mod,
                    TokenKind::MatMul => BinOp::MatMul,
                    TokenKind::Eq => BinOp::Eq,
                    TokenKind::Ne => BinOp::Ne,
                    TokenKind::Lt => BinOp::Lt,
                    TokenKind::Le => BinOp::Le,
                    TokenKind::Gt => BinOp::Gt,
                    TokenKind::Ge => BinOp::Ge,
                    TokenKind::And => BinOp::And,
                    TokenKind::Or => BinOp::Or,
                    _ => bail!(
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Parse,
                        "Unknown infix op: {:?}",
                        kind
                    ),
                };
                let prec = Self::token_precedence(&kind);
                self.advance();
                let right = self.parse_expr(prec)?;
                let end = right.span;
                Ok(Expr {
                    kind: ExprKind::Binary {
                        op,
                        lhs: Box::new(left),
                        rhs: Box::new(right),
                    },
                    span: start.merge(end),
                })
            }
        }
    }

    fn parse_match(&mut self) -> RR<Expr> {
        let start = self.current.span;
        self.advance(); // match
        self.expect(TokenKind::LParen)?;
        let scrutinee = self.parse_expr(Precedence::Lowest)?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::EOF {
            arms.push(self.parse_match_arm()?);
        }
        let end = self.current.span;
        self.expect(TokenKind::RBrace)?;

        Ok(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: start.merge(end),
        })
    }

    fn parse_match_arm(&mut self) -> RR<MatchArm> {
        let pat = self.parse_pattern()?;

        let guard = if self.current.kind == TokenKind::If {
            self.advance();
            Some(Box::new(self.parse_expr(Precedence::Lowest)?))
        } else {
            None
        };

        self.expect(TokenKind::Arrow)?;

        let body = self.parse_expr(Precedence::Lowest)?;

        // Allow a trailing comma between match arms.
        if self.current.kind == TokenKind::Comma {
            self.advance();
        }

        let span = pat.span();
        Ok(MatchArm {
            pat,
            guard,
            body: Box::new(body),
            span,
        })
    }

    fn parse_pattern(&mut self) -> RR<Pattern> {
        let start = self.current.span;
        let kind = match &self.current.kind {
            TokenKind::Ident(n) => {
                let name = n.clone();
                self.advance();
                if name == "_" {
                    PatternKind::Wild
                } else {
                    PatternKind::Bind(name)
                }
            }
            TokenKind::Int(i) => {
                let l = Lit::Int(*i);
                self.advance();
                PatternKind::Lit(l)
            }
            TokenKind::Float(f) => {
                let l = Lit::Float(*f);
                self.advance();
                PatternKind::Lit(l)
            }
            TokenKind::String(s) => {
                let l = Lit::Str(s.clone());
                self.advance();
                PatternKind::Lit(l)
            }
            TokenKind::True => {
                self.advance();
                PatternKind::Lit(Lit::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                PatternKind::Lit(Lit::Bool(false))
            }
            TokenKind::Na => {
                self.advance();
                PatternKind::Lit(Lit::Na)
            }
            TokenKind::Null => {
                self.advance();
                PatternKind::Lit(Lit::Null)
            }

            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                let mut rest = None;

                if self.current.kind != TokenKind::RBracket {
                    loop {
                        if self.current.kind == TokenKind::DotDot {
                            self.advance();
                            if let TokenKind::Ident(n) = &self.current.kind {
                                rest = Some(n.clone());
                                self.advance();
                            }
                            // Rest must be last, consume comma if present
                            if self.current.kind == TokenKind::Comma {
                                self.advance();
                            }
                            // Expect end
                            if self.current.kind != TokenKind::RBracket {
                                bail_at!(
                                    self.current.span,
                                    "RR.ParseError",
                                    RRCode::E0001,
                                    Stage::Parse,
                                    "Spread .. must be last in pattern"
                                );
                            }
                            break;
                        } else {
                            items.push(self.parse_pattern()?);
                        }

                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket)?;
                PatternKind::List { items, rest }
            }
            TokenKind::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                if self.current.kind != TokenKind::RBrace {
                    loop {
                        if self.current.kind == TokenKind::DotDot {
                            bail_at!(
                                self.current.span,
                                "RR.ParseError",
                                RRCode::E0001,
                                Stage::Parse,
                                "record rest pattern (`..`) is not supported"
                            );
                        }
                        let field_name = match &self.current.kind {
                            TokenKind::Ident(n) => n.clone(),
                            _ => bail!(
                                "RR.ParseError",
                                RRCode::E0001,
                                Stage::Parse,
                                "Expected field name in record pattern"
                            ),
                        };
                        self.advance();
                        self.expect(TokenKind::Colon)?;
                        let field_pat = self.parse_pattern()?;
                        fields.push((field_name, field_pat));

                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBrace)?;
                PatternKind::Record { fields }
            }
            _ => bail!(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected pattern"
            ),
        };
        // Pattern span currently uses the start token span.
        Ok(Pattern { kind, span: start })
    }
}
