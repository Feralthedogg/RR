use super::*;
impl<'a> Parser<'a> {
    pub(crate) fn parse_infix(&mut self, left: Expr) -> RR<Expr> {
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
                    let name = match helpers::dotted_segment_name(&self.current.kind) {
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
                let (name, end) = match helpers::dotted_segment_name(&self.current.kind) {
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
                let prec = helpers::token_precedence(&kind);
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

    pub(crate) fn parse_match(&mut self) -> RR<Expr> {
        let start = self.current.span;
        self.advance(); // match
        self.expect(TokenKind::LParen)?;
        let scrutinee = self.parse_expr(Precedence::Lowest)?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::Eof {
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

    pub(crate) fn parse_match_arm(&mut self) -> RR<MatchArm> {
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

    pub(crate) fn parse_pattern(&mut self) -> RR<Pattern> {
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
