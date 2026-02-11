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
}

#[derive(PartialEq, PartialOrd)]
enum Precedence {
    Lowest,
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
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token();
        let peek = lexer.next_token();
        Self {
            lexer,
            current,
            peek,
        }
    }

    fn advance(&mut self) {
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
        let mut out = match &self.current.kind {
            TokenKind::Ident(n) => n.clone(),
            _ => bail_at!(
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
            if !matches!(self.peek.kind, TokenKind::Ident(_)) {
                break;
            }
            self.advance(); // dot
            let seg = match &self.current.kind {
                TokenKind::Ident(n) => n.clone(),
                _ => break,
            };
            out.push('.');
            out.push_str(&seg);
            self.advance();
        }
        Ok(out)
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
                Some(self.parse_type_name("in parameter type annotation")?)
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

    fn parse_type_name(&mut self, ctx: &str) -> RR<String> {
        self.parse_dotted_ident(ctx)
    }

    fn token_precedence(kind: &TokenKind) -> Precedence {
        match kind {
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
            TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot => Precedence::Call,
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
                | TokenKind::Ident(_)
                | TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::String(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Na
                | TokenKind::Match
                | TokenKind::At
                | TokenKind::Caret
                | TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::LBrace
                | TokenKind::Minus
                | TokenKind::Bang
        )
    }

    fn can_end_stmt_here(&self) -> bool {
        matches!(
            self.current.kind,
            TokenKind::RBrace | TokenKind::EOF | TokenKind::Else
        ) || Self::is_stmt_start(&self.current.kind)
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
        if self.current.kind == TokenKind::Semicolon {
            let semi = self.current.span;
            self.advance();
            return Ok(semi);
        }
        if matches!(
            self.current.kind,
            TokenKind::RBrace | TokenKind::EOF | TokenKind::Else
        ) {
            return Ok(fallback);
        }
        if Self::is_stmt_start(&self.current.kind) {
            let same_line =
                fallback.end_line > 0 && self.current.span.start_line == fallback.end_line;
            if same_line {
                bail_at!(
                    self.current.span,
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    "Missing ';' before {:?} on the same line",
                    self.current.kind
                );
            }
            return Ok(fallback);
        }
        bail_at!(
            self.current.span,
            "RR.ParseError",
            RRCode::E0001,
            Stage::Parse,
            "Expected statement separator, got {:?}",
            self.current.kind
        );
    }

    fn recover_stmt_boundary(&mut self) {
        while self.current.kind != TokenKind::EOF {
            if self.current.kind == TokenKind::Semicolon {
                self.advance();
                break;
            }
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
                        format!("{}", msg),
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
                    if self.current.span == before && self.current.kind != TokenKind::EOF {
                        // Prevent infinite recovery loops on the same token.
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
            Some(self.parse_type_name("after ':' in let binding")?)
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

        self.expect(TokenKind::LParen)?;
        let params = self.parse_fn_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_ty_hint = if self.current.kind == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_name("after function return arrow")?)
        } else {
            None
        };

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
                params,
                ret_ty_hint,
                body: body.clone(),
            },
            span: start.merge(body.span),
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
                    if self.current.span == before && self.current.kind != TokenKind::EOF {
                        // Prevent infinite recovery loops inside blocks.
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
        // `return` without value is allowed only at explicit/structural statement end.
        // Using `is_stmt_start` here breaks `return x + 1` when semicolons are optional.
        let value = if matches!(
            self.current.kind,
            TokenKind::Semicolon | TokenKind::RBrace | TokenKind::EOF | TokenKind::Else
        ) {
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

        let path = match &self.current.kind {
            TokenKind::String(s) => s.clone(),
            _ => bail_at!(
                self.current.span,
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                "Expected string after import"
            ),
        };
        self.advance();

        // Optional semicolon?
        if self.current.kind == TokenKind::Semicolon {
            self.advance();
        }

        Ok(Stmt {
            kind: StmtKind::Import(path),
            span: start.merge(self.current.span),
        })
    }

    fn parse_export_modifier(&mut self) -> RR<Stmt> {
        let start = self.current.span;
        self.advance(); // export

        // Expect fn declaration
        let stmt = self.parse_fn_decl()?;

        if let StmtKind::FnDecl {
            name,
            params,
            ret_ty_hint,
            body,
        } = stmt.kind
        {
            Ok(Stmt {
                kind: StmtKind::Export(FnDecl {
                    name,
                    params,
                    ret_ty_hint,
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
                "Expected function after export"
            );
        }
    }

    fn parse_start_ident_or_expr(&mut self) -> RR<Stmt> {
        // Can be Assign (x = ...) or ExprStmt (x; or call(x);)
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
            let ty_hint = self.parse_type_name("after ':' in typed declaration")?;
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

        while self.current.kind != TokenKind::Semicolon
            && precedence < Self::token_precedence(&self.current.kind)
        {
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
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Neg,
                        rhs: Box::new(rhs),
                    },
                    span: start,
                })
            }
            TokenKind::Bang => {
                self.advance();
                let rhs = self.parse_expr(Precedence::Prefix)?;
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Not,
                        rhs: Box::new(rhs),
                    },
                    span: start,
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
            Some(self.parse_type_name("after lambda return arrow")?)
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

    fn parse_infix(&mut self, left: Expr) -> RR<Expr> {
        let start = left.span;
        let kind = self.current.kind.clone();

        match kind {
            TokenKind::LParen => {
                // Call
                self.advance();
                let mut args = Vec::new();
                if self.current.kind != TokenKind::RParen {
                    loop {
                        if let TokenKind::Ident(name) = &self.current.kind {
                            if matches!(self.peek.kind, TokenKind::Assign) {
                                let arg_start = self.current.span;
                                let arg_name = name.clone();
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
                Ok(Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(left),
                        args,
                    },
                    span: start.merge(end),
                })
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
                let (name, end) = match &self.current.kind {
                    TokenKind::Ident(n) => (n.clone(), self.current.span),
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
                self.advance();
                let end = self.current.span; // Approx
                Ok(Expr {
                    kind: ExprKind::Try {
                        expr: Box::new(left),
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
