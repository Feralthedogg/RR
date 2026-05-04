use super::*;
impl<'a> Parser<'a> {
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

    pub(crate) fn advance(&mut self) {
        self.previous_span = self.current.span;
        self.current = self.peek.clone();
        self.peek = self.lexer.next_token();
    }

    pub(crate) fn expect(&mut self, kind: TokenKind) -> RR<()> {
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

    pub(crate) fn parse_dotted_ident(&mut self, err_context: &str) -> RR<String> {
        let mut out = match helpers::dotted_segment_name(&self.current.kind) {
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
            if helpers::dotted_segment_name(&self.peek.kind).is_none() {
                break;
            }
            self.advance(); // dot
            let Some(seg) = helpers::dotted_segment_name(&self.current.kind) else {
                break;
            };
            out.push('.');
            out.push_str(&seg);
            self.advance();
        }
        Ok(out)
    }

    pub(crate) fn parse_type_path_ident(&mut self, err_context: &str) -> RR<String> {
        let mut out = self.parse_dotted_ident(err_context)?;
        while self.current.kind == TokenKind::DoubleColon {
            if helpers::dotted_segment_name(&self.peek.kind).is_none() {
                break;
            }
            self.advance(); // ::
            let Some(seg) = helpers::dotted_segment_name(&self.current.kind) else {
                break;
            };
            out.push_str("::");
            out.push_str(&seg);
            self.advance();
        }
        Ok(out)
    }

    pub(crate) fn parse_fn_params(&mut self) -> RR<Vec<FnParam>> {
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

    pub(crate) fn parse_type_expr(&mut self, ctx: &str) -> RR<TypeExpr> {
        if self.current.kind == TokenKind::Lt {
            return self.parse_fully_qualified_type_projection(ctx);
        }

        if let TokenKind::Int(value) = self.current.kind {
            self.advance();
            return Ok(TypeExpr::Named(format!("#{value}")));
        }

        let base = self.parse_type_path_ident(ctx)?;
        if base == "dyn"
            && let Some(trait_name) = helpers::dotted_segment_name(&self.current.kind)
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

    pub(crate) fn parse_as_keyword(&mut self, ctx: &str) -> RR<()> {
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

    pub(crate) fn parse_fully_qualified_type_projection(&mut self, ctx: &str) -> RR<TypeExpr> {
        self.expect(TokenKind::Lt)?;
        let receiver_ty = self.parse_type_expr(ctx)?;
        self.parse_as_keyword("inside fully-qualified associated type")?;
        let trait_name = self.parse_type_path_ident("after 'as' in associated type")?;
        self.expect(TokenKind::Gt)?;
        self.expect(TokenKind::DoubleColon)?;
        let assoc_name = self.parse_dotted_ident("after '::' in associated type")?;
        let base = format!(
            "<{} as {}>::{}",
            helpers::type_expr_key(&receiver_ty),
            trait_name,
            assoc_name
        );
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

    pub(crate) fn parse_optional_type_params(&mut self) -> RR<Vec<String>> {
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

    pub(crate) fn parse_optional_for_lifetime_binder(&mut self) -> RR<()> {
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

    pub(crate) fn parse_optional_where_bounds(&mut self) -> RR<Vec<TraitBound>> {
        if self.current.kind != TokenKind::Where {
            return Ok(Vec::new());
        }
        self.advance(); // where
        let mut bounds = Vec::new();
        loop {
            self.parse_optional_for_lifetime_binder()?;
            let type_name = helpers::type_expr_key(&self.parse_type_expr("in where clause")?);
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

    pub(crate) fn parse_call_args(&mut self) -> RR<(Vec<Expr>, Span)> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        if self.current.kind != TokenKind::RParen {
            loop {
                if let Some(name) = helpers::call_arg_name(&self.current.kind) {
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

    pub(crate) fn parse_explicit_type_args(&mut self) -> RR<Vec<TypeExpr>> {
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

    pub(crate) fn is_stmt_start(kind: &TokenKind) -> bool {
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
                | TokenKind::UnsafeRBlock { .. }
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

    pub(crate) fn current_is_structural_stmt_end(&self) -> bool {
        matches!(
            self.current.kind,
            TokenKind::RBrace | TokenKind::Eof | TokenKind::Else
        )
    }

    pub(crate) fn current_starts_stmt_on_new_line_after(&self, span: Span) -> bool {
        span.end_line > 0
            && Self::is_stmt_start(&self.current.kind)
            && self.current.span.start_line > span.end_line
    }

    pub(crate) fn current_starts_stmt_on_same_line_after(&self, span: Span) -> bool {
        span.end_line > 0
            && Self::is_stmt_start(&self.current.kind)
            && self.current.span.start_line == span.end_line
    }

    pub(crate) fn consume_stmt_end(&mut self, fallback: Span) -> RR<Span> {
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

    pub(crate) fn recover_stmt_boundary(&mut self) {
        while self.current.kind != TokenKind::Eof {
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
        while self.current.kind != TokenKind::Eof {
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

    pub(crate) fn parse_stmt(&mut self) -> RR<Stmt> {
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
            TokenKind::UnsafeRBlock { .. } => self.parse_unsafe_r_block_stmt(),
            _ => self.parse_start_ident_or_expr(),
        }
    }

    pub(crate) fn parse_unsafe_r_block_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        let TokenKind::UnsafeRBlock { code, read_only } = self.current.kind.clone() else {
            return Err(InternalCompilerError::new(
                Stage::Parse,
                "unsafe R block parser called on a non-unsafe token",
            )
            .at(start)
            .into_exception());
        };
        self.advance();
        let end = self.consume_stmt_end(start)?;
        Ok(Stmt {
            kind: StmtKind::UnsafeRBlock { code, read_only },
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_let_stmt(&mut self) -> RR<Stmt> {
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

    pub(crate) fn parse_fn_decl(&mut self) -> RR<Stmt> {
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

    pub(crate) fn parse_trait_method_sig(&mut self) -> RR<TraitMethodSig> {
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

    pub(crate) fn parse_trait_supertraits(&mut self) -> RR<Vec<String>> {
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

    pub(crate) fn parse_trait_assoc_type(&mut self) -> RR<TraitAssocType> {
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

    pub(crate) fn parse_impl_assoc_type(&mut self) -> RR<ImplAssocType> {
        let start = self.current.span;
        self.advance(); // type
        let name = self.parse_dotted_ident("after associated type")?;
        let name = if self.current.kind == TokenKind::Lt {
            self.advance();
            let mut args = Vec::new();
            loop {
                args.push(helpers::type_expr_key(
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

    pub(crate) fn current_is_const_keyword(&self) -> bool {
        matches!(&self.current.kind, TokenKind::Ident(name) if name == "const")
    }

    pub(crate) fn parse_trait_assoc_const(&mut self) -> RR<TraitAssocConst> {
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

    pub(crate) fn parse_impl_assoc_const(&mut self) -> RR<ImplAssocConst> {
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

    pub(crate) fn parse_trait_decl(&mut self) -> RR<Stmt> {
        self.parse_trait_decl_with_visibility(false)
    }

    pub(crate) fn parse_trait_decl_with_visibility(&mut self, public: bool) -> RR<Stmt> {
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
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::Eof {
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

    pub(crate) fn parse_impl_decl(&mut self) -> RR<Stmt> {
        self.parse_impl_decl_with_visibility(false)
    }

    pub(crate) fn parse_impl_decl_with_visibility(&mut self, public: bool) -> RR<Stmt> {
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
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::Eof {
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
            let stmt_span = stmt.span;
            let StmtKind::FnDecl {
                name,
                type_params,
                params,
                ret_ty_hint,
                where_bounds,
                body,
            } = stmt.kind
            else {
                return Err(InternalCompilerError::new(
                    Stage::Parse,
                    "function parser returned a non-function statement",
                )
                .at(stmt_span)
                .into_exception());
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
}
