use super::*;
pub(crate) struct ParsedImport {
    pub(crate) path: String,
    pub(crate) spec: ImportSpec,
    pub(crate) end_fallback: Span,
}

impl<'a> Parser<'a> {
    pub(crate) fn parse_block(&mut self) -> RR<Block> {
        let start = self.current.span;
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        let mut errors: Vec<RRException> = Vec::new();
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::Eof {
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

    pub(crate) fn parse_if_stmt(&mut self) -> RR<Stmt> {
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

    pub(crate) fn parse_stmt_or_block(&mut self) -> RR<Block> {
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

    pub(crate) fn parse_while_stmt(&mut self) -> RR<Stmt> {
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

    pub(crate) fn parse_for_stmt(&mut self) -> RR<Stmt> {
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

    pub(crate) fn parse_return_stmt(&mut self) -> RR<Stmt> {
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

    pub(crate) fn parse_break_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // break
        let end = self.consume_stmt_end(start)?;
        Ok(Stmt {
            kind: StmtKind::Break,
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_next_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // next
        let end = self.consume_stmt_end(start)?;
        Ok(Stmt {
            kind: StmtKind::Next,
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_import_stmt(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // import

        let source = self.parse_import_source_prefix();
        let parsed = self.parse_import_payload(source)?;
        let end = self.consume_stmt_end(parsed.end_fallback)?;

        Ok(Stmt {
            kind: StmtKind::Import {
                source,
                path: parsed.path,
                spec: parsed.spec,
            },
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_import_source_prefix(&mut self) -> ImportSource {
        if let TokenKind::Ident(name) = &self.current.kind
            && name == "r"
        {
            self.advance();
            return ImportSource::RPackage;
        }

        ImportSource::Module
    }

    pub(crate) fn parse_import_payload(&mut self, source: ImportSource) -> RR<ParsedImport> {
        if source != ImportSource::RPackage {
            return self.parse_glob_import("Expected string after import");
        }

        match &self.current.kind {
            TokenKind::Ident(name) if name == "default" => self.parse_r_default_import(),
            TokenKind::LBrace => self.parse_r_named_import(),
            TokenKind::Star => self.parse_r_namespace_import(),
            _ => self.parse_glob_import("Expected string after import"),
        }
    }

    pub(crate) fn parse_r_default_import(&mut self) -> RR<ParsedImport> {
        self.advance(); // default
        self.expect_ident_keyword("from", "Expected 'from' after 'default' in R import")?;
        let (pkg, end_fallback) =
            self.parse_import_string("Expected package string after 'from'")?;

        Ok(ParsedImport {
            path: pkg.clone(),
            spec: ImportSpec::Namespace(pkg),
            end_fallback,
        })
    }

    pub(crate) fn parse_r_named_import(&mut self) -> RR<ParsedImport> {
        let bindings = self.parse_r_import_bindings()?;
        self.expect_ident_keyword("from", "Expected 'from' after R import list")?;
        let (pkg, end_fallback) =
            self.parse_import_string("Expected package string after 'from'")?;

        Ok(ParsedImport {
            path: pkg,
            spec: ImportSpec::Named(bindings),
            end_fallback,
        })
    }

    pub(crate) fn parse_r_import_bindings(&mut self) -> RR<Vec<ImportBinding>> {
        let mut bindings = Vec::new();
        self.advance(); // {

        while self.current.kind != TokenKind::RBrace {
            let imported = self.parse_dotted_ident("in R package import list")?;
            let local = if self.current_is_ident_keyword("as") {
                self.advance();
                Some(self.parse_dotted_ident("after 'as' in R import list")?)
            } else {
                None
            };
            bindings.push(ImportBinding { imported, local });

            if self.current.kind != TokenKind::Comma {
                break;
            }
            self.advance();
        }

        self.expect(TokenKind::RBrace)?;
        Ok(bindings)
    }

    pub(crate) fn parse_r_namespace_import(&mut self) -> RR<ParsedImport> {
        self.advance(); // *
        self.expect_ident_keyword("as", "Expected 'as' after '*' in R namespace import")?;
        let alias = self.parse_dotted_ident("after 'as' in R namespace import")?;
        self.expect_ident_keyword("from", "Expected 'from' after R namespace alias")?;
        let (pkg, end_fallback) =
            self.parse_import_string("Expected package string after 'from'")?;

        Ok(ParsedImport {
            path: pkg,
            spec: ImportSpec::Namespace(alias),
            end_fallback,
        })
    }

    pub(crate) fn parse_glob_import(&mut self, error_message: &str) -> RR<ParsedImport> {
        let (path, end_fallback) = self.parse_import_string(error_message)?;
        Ok(ParsedImport {
            path,
            spec: ImportSpec::Glob,
            end_fallback,
        })
    }

    pub(crate) fn parse_import_string(&mut self, error_message: &str) -> RR<(String, Span)> {
        match &self.current.kind {
            TokenKind::String(value) => {
                let value = value.clone();
                self.advance();
                Ok((value, self.previous_span))
            }
            _ => bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "{}",
                error_message
            ),
        }
    }

    pub(crate) fn expect_ident_keyword(&mut self, keyword: &str, error_message: &str) -> RR<()> {
        if self.current_is_ident_keyword(keyword) {
            self.advance();
            return Ok(());
        }

        bail_at!(
            self.current.span,
            "RR.ParseError",
            RRCode::E0001,
            Stage::Parse,
            "{}",
            error_message
        )
    }

    pub(crate) fn current_is_ident_keyword(&self, keyword: &str) -> bool {
        matches!(&self.current.kind, TokenKind::Ident(name) if name == keyword)
    }

    pub(crate) fn parse_export_modifier(&mut self) -> RR<Stmt> {
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

    pub(crate) fn parse_start_ident_or_expr(&mut self) -> RR<Stmt> {
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
        } else if let Some(op) = helpers::compound_assign_binop(&self.current.kind) {
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

    pub(crate) fn expr_to_lvalue(&self, expr: Expr) -> RR<LValue> {
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

    pub(crate) fn parse_expr(&mut self, precedence: Precedence) -> RR<Expr> {
        let mut left = self.parse_prefix()?;

        while precedence < helpers::token_precedence(&self.current.kind) {
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

    pub(crate) fn parse_name_expr(&mut self, start: Span, name: String) -> Expr {
        self.advance();
        Expr {
            kind: ExprKind::Name(name),
            span: start,
        }
    }

    pub(crate) fn parse_literal_expr(&mut self, start: Span, lit: Lit) -> Expr {
        self.advance();
        Expr {
            kind: ExprKind::Lit(lit),
            span: start,
        }
    }

    pub(crate) fn parse_column_expr(&mut self, start: Span) -> RR<Expr> {
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

    pub(crate) fn parse_unquote_expr(&mut self, start: Span) -> RR<Expr> {
        self.advance();
        let val = self.parse_expr(Precedence::Prefix)?;
        let end = val.span;
        Ok(Expr {
            kind: ExprKind::Unquote(Box::new(val)),
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_parenthesized_expr(&mut self) -> RR<Expr> {
        self.advance();
        let expr = self.parse_expr(Precedence::Lowest)?;
        self.expect(TokenKind::RParen)?;
        Ok(expr)
    }

    pub(crate) fn parse_vector_literal_expr(&mut self, start: Span) -> RR<Expr> {
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

    pub(crate) fn parse_record_literal_expr(&mut self, start: Span) -> RR<Expr> {
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

    pub(crate) fn parse_unary_expr(&mut self, start: Span, op: UnaryOp) -> RR<Expr> {
        self.advance();
        let rhs = self.parse_expr(Precedence::Prefix)?;
        let end = rhs.span;
        Ok(Expr {
            kind: ExprKind::Unary {
                op,
                rhs: Box::new(rhs),
            },
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_formula_expr(&mut self, start: Span) -> RR<Expr> {
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

    pub(crate) fn parse_prefix(&mut self) -> RR<Expr> {
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
            TokenKind::Ident(name) => Ok(self.parse_name_expr(start, name.clone())),
            TokenKind::Int(i) => Ok(self.parse_literal_expr(start, Lit::Int(*i))),
            TokenKind::Float(f) => Ok(self.parse_literal_expr(start, Lit::Float(*f))),
            TokenKind::String(s) => Ok(self.parse_literal_expr(start, Lit::Str(s.clone()))),
            TokenKind::True => Ok(self.parse_literal_expr(start, Lit::Bool(true))),
            TokenKind::False => Ok(self.parse_literal_expr(start, Lit::Bool(false))),
            TokenKind::Null => Ok(self.parse_literal_expr(start, Lit::Null)),
            TokenKind::Na => Ok(self.parse_literal_expr(start, Lit::Na)),
            TokenKind::Fn => self.parse_lambda_expr(),
            TokenKind::UnsafeRBlock { .. } => {
                bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "unsafe r blocks are statements and cannot be used as expressions"
                )
            }
            TokenKind::Match => self.parse_match(),
            TokenKind::At => self.parse_column_expr(start),
            TokenKind::Caret => self.parse_unquote_expr(start),
            TokenKind::LParen => self.parse_parenthesized_expr(),
            TokenKind::Lt => self.parse_fully_qualified_assoc_expr(),
            TokenKind::LBracket => self.parse_vector_literal_expr(start),
            TokenKind::LBrace => self.parse_record_literal_expr(start),
            TokenKind::Minus => self.parse_unary_expr(start, UnaryOp::Neg),
            TokenKind::Bang => self.parse_unary_expr(start, UnaryOp::Not),
            TokenKind::Tilde => self.parse_formula_expr(start),
            _ => bail!(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Unexpected token in prefix: {:?}",
                self.current.kind
            ),
        }
    }

    pub(crate) fn parse_lambda_expr(&mut self) -> RR<Expr> {
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

    pub(crate) fn parse_fully_qualified_assoc_expr(&mut self) -> RR<Expr> {
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
}
