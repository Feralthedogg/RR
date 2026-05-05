use crate::error::{RR, RRCode, Stage};
use crate::bail;
use crate::ir::*;
use crate::syntax::ast::*;
use crate::utils::Span;
use crate::runtime::R_RUNTIME;

pub struct Emitter {
    output: String,
    current_line: u32,
    source_map: Vec<MapEntry>,
    indent: usize,
    with_runtime: bool,
}

#[derive(Debug)]
pub struct MapEntry {
    pub r_line: u32,
    pub rr_span: Span,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            current_line: 1,
            source_map: Vec::new(),
            indent: 0,
            with_runtime: true,
        }
    }

    pub fn emit_program(&mut self, prog: IRProgram, with_runtime: bool) -> RR<(String, Vec<MapEntry>)> {
        self.with_runtime = with_runtime;
        if with_runtime {
            self.write_raw(R_RUNTIME);
        }
        
        for stmt in prog.stmts {
            self.emit_stmt(stmt)?;
        }
        
        Ok((self.output.clone(), self.source_map.drain(..).collect()))
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
        self.current_line += s.chars().filter(|c| *c == '\n').count() as u32;
    }
    
    fn write_raw(&mut self, s: &str) {
        self.output.push_str(s);
        // Keep source-map line tracking accurate when runtime prelude is injected.
        self.current_line += s.chars().filter(|c| *c == '\n').count() as u32;
    }

    fn newline(&mut self) {
        self.output.push('\n');
        self.current_line += 1;
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    fn record_span(&mut self, span: Span) {
        if span.start_line != 0 {
            self.source_map.push(MapEntry {
                r_line: self.current_line,
                rr_span: span,
            });
        }
    }

    fn escape_r_string(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 8);
        for ch in s.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"'  => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(ch),
            }
        }
        out
    }

    fn emit_source_marker(&mut self, span: Span) {
        if self.with_runtime && span.start_line != 0 {
            self.write_indent();
            self.write(&format!("rr_mark({}, {});", span.start_line, span.start_col));
            self.newline();
        }
    }

    fn emit_stmt(&mut self, stmt: IRStmt) -> RR<()> {
        self.record_span(stmt.span);
        self.emit_source_marker(stmt.span);

        match stmt.kind {
            IRStmtKind::Assign { target, value } => self.emit_assign_stmt(target, value)?,
            IRStmtKind::ExprStmt { expr } => self.emit_expr_stmt(expr)?,
            IRStmtKind::FnDecl { name, params, body } => self.emit_fn_decl(name, params, body)?,
            IRStmtKind::If {
                cond,
                then_blk,
                else_blk,
            } => self.emit_if_stmt(cond, then_blk, else_blk)?,
            IRStmtKind::While { cond, body } => self.emit_while_stmt(cond, body)?,
            IRStmtKind::For { var, seq, body } => self.emit_for_stmt(var, seq, body)?,
            IRStmtKind::Return { value } => self.emit_return_stmt(value)?,
        }
        Ok(())
    }

    fn emit_stmt_block(&mut self, body: Vec<IRStmt>) -> RR<()> {
        self.indent += 1;
        for stmt in body {
            self.emit_stmt(stmt)?;
        }
        self.indent -= 1;
        Ok(())
    }

    fn emit_assign_stmt(&mut self, target: IRLValue, value: IRExpr) -> RR<()> {
        self.write_indent();
        self.emit_lvalue(target)?;
        self.write(" <- ");
        self.emit_expr(value)?;
        self.write(";");
        self.newline();
        Ok(())
    }

    fn emit_expr_stmt(&mut self, expr: IRExpr) -> RR<()> {
        self.write_indent();
        self.emit_expr(expr)?;
        self.write(";");
        self.newline();
        Ok(())
    }

    fn emit_fn_decl(&mut self, name: String, params: Vec<String>, body: Vec<IRStmt>) -> RR<()> {
        self.write_indent();
        self.write(&format!("{} <- function({}) {{", name, params.join(", ")));
        self.newline();
        self.emit_stmt_block(body)?;
        self.write_indent();
        self.write("};");
        self.newline();
        Ok(())
    }

    fn emit_if_stmt(
        &mut self,
        cond: IRExpr,
        then_blk: Vec<IRStmt>,
        else_blk: Option<Vec<IRStmt>>,
    ) -> RR<()> {
        self.write_indent();
        self.write("if (rr_truthy1(");
        self.emit_expr(cond)?;
        self.write(")) {");
        self.newline();
        self.emit_stmt_block(then_blk)?;
        if let Some(else_body) = else_blk {
            self.write_indent();
            self.write("} else {");
            self.newline();
            self.emit_stmt_block(else_body)?;
        }
        self.write_indent();
        self.write("}");
        self.newline();
        Ok(())
    }

    fn emit_while_stmt(&mut self, cond: IRExpr, body: Vec<IRStmt>) -> RR<()> {
        self.write_indent();
        self.write("while (rr_truthy1(");
        self.emit_expr(cond)?;
        self.write(")) {");
        self.newline();
        self.emit_stmt_block(body)?;
        self.write_indent();
        self.write("}");
        self.newline();
        Ok(())
    }

    fn emit_for_stmt(&mut self, var: String, seq: IRExpr, body: Vec<IRStmt>) -> RR<()> {
        self.write_indent();
        self.write(&format!("for ({} in ", var));
        self.emit_expr(seq)?;
        self.write(") {");
        self.newline();
        self.emit_stmt_block(body)?;
        self.write_indent();
        self.write("}");
        self.newline();
        Ok(())
    }

    fn emit_return_stmt(&mut self, value: Option<IRExpr>) -> RR<()> {
        self.write_indent();
        self.write("return(");
        if let Some(value) = value {
            self.emit_expr(value)?;
        } else {
            self.write("NULL");
        }
        self.write(");");
        self.newline();
        Ok(())
    }

    fn emit_lvalue(&mut self, lv: IRLValue) -> RR<()> {
        match lv {
            IRLValue::Name(n) => { self.write(&n); Ok(()) },
            IRLValue::Index1D { base, idx } => {
                self.emit_expr(*base)?;                self.write("[");
                
                let mut optimized = false;
                if let IRExprKind::Binary { op: BinOp::Sub, lhs, rhs } = &idx.kind {
                     if let IRExprKind::Lit(Lit::Int(1)) = &rhs.kind {
                         // Fold `(x - 1) + 1L` into `x`.
                         self.write("(");
                         self.emit_expr((**lhs).clone())?;
                         self.write(")");
                         optimized = true;
                     }
                }
                
                if !optimized {
                    if idx.facts.has(Facts::INT_SCALAR | Facts::NON_NEG | Facts::NON_NA) {
                        self.write("(");
                        self.emit_expr(*idx)?;
                        self.write(") + 1L");
                    } else {
                        self.write("rr_index1_write(");
                        self.emit_expr(*idx)?;
                        self.write(", \"index\")");
                    }
                }
                self.write("]");
                Ok(())
            }
            IRLValue::Index2D { base, r, c } => {
                self.emit_expr(*base)?;
                self.write("[rr_i0_write(");
                self.emit_expr(*r)?;
                self.write(", \"row\") + 1L, rr_i0_write(");
                self.emit_expr(*c)?;
                self.write(", \"col\") + 1L]");
                Ok(())
            }
        }
    }

    fn emit_expr(&mut self, expr: IRExpr) -> RR<()> {
        match expr.kind {
            IRExprKind::Lit(l) => match l {
                Lit::Int(i) => self.write(&format!("{}L", i)),
                Lit::Float(f) => self.write(&format!("{}", f)),
                Lit::Str(s) => self.write(&format!("\"{}\"", Self::escape_r_string(&s))),
                Lit::Bool(b) => self.write(if b { "TRUE" } else { "FALSE" }),
                Lit::Null => self.write("NULL"),
                Lit::Na => self.write("NA"),
            },
            IRExprKind::Name(n) => self.write(&n),
            IRExprKind::Unary { op, rhs } => {
                let op_str = match op { UnaryOp::Neg => "-", UnaryOp::Not => "!" };
                self.write(op_str);
                self.write("(");
                self.emit_expr(*rhs)?;
                self.write(")");
            }
            IRExprKind::Binary { op, lhs, rhs } => {
                let op_str = match op {
                    BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%%", BinOp::MatMul => "%*%",
                    BinOp::Eq => "==", BinOp::Ne => "!=", BinOp::Lt => "<", BinOp::Le => "<=", BinOp::Gt => ">", BinOp::Ge => ">=",
                    BinOp::And => "&", BinOp::Or => "|",
                };
                
                let is_arith = matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod);
                let has_vec = lhs.facts.has(Facts::IS_VECTOR) || rhs.facts.has(Facts::IS_VECTOR);
                
                if is_arith && has_vec && self.with_runtime {
                    // Evaluate operands once to preserve side effects and evaluation order.
                    self.write("({ .lhs <- ");
                    self.emit_expr(*lhs.clone())?;
                    self.write("; .rhs <- ");
                    self.emit_expr(*rhs.clone())?;
                    self.write(&format!("; rr_same_len(.lhs, .rhs, \"{}\"); .lhs {} .rhs }})", op_str, op_str));
                } else {
                    self.write("(");
                    self.emit_expr(*lhs)?;
                    self.write(&format!(" {} ", op_str));
                    self.emit_expr(*rhs)?;
                    self.write(")");
                }
            }
            IRExprKind::Call { callee, args } => {
                self.emit_expr(*callee)?;
                self.write("(");
                for (i, arg) in args.into_iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.emit_expr(arg)?;
                }
                self.write(")");
            }
            IRExprKind::RrRange { a, b } => {
                self.write("rr_range(");
                self.emit_expr(*a)?;
                self.write(", ");
                self.emit_expr(*b)?;
                self.write(")");
            }
            IRExprKind::RrIndices { x } => {
                self.write("rr_indices(");
                self.emit_expr(*x)?;
                self.write(")");
            }
            IRExprKind::Index1D { base, idx } => {
                  if idx.facts.has(Facts::INT_SCALAR | Facts::NON_NEG | Facts::NON_NA) {
                       self.emit_expr(*base)?;
                       self.write("[");
                  
                       let mut optimized = false;
                       if let IRExprKind::Binary { op: BinOp::Sub, lhs, rhs } = &idx.kind {
                            if let IRExprKind::Lit(Lit::Int(1)) = &rhs.kind {
                                // Fold `(x - 1) + 1L` into `x`.
                                self.write("(");
                                self.emit_expr((**lhs).clone())?;
                                self.write(")");
                                optimized = true;
                            }
                       }
                  
                       if !optimized {
                            self.write("(");
                            self.emit_expr(*idx)?;
                            self.write(") + 1L");
                       }
                       self.write("]");
                  } else {
                       self.write("rr_index1_read(");
                       self.emit_expr(*base)?;
                       self.write(", ");
                       self.emit_expr(*idx)?;
                       self.write(", \"index\")");
                  }
            }
            IRExprKind::Index2D { base, r, c } => {
                 self.emit_expr(*base)?;
                 self.write("[rr_i0_read(");
                 self.emit_expr(*r)?;
                 self.write(", \"row\") + 1L, rr_i0_read(");
                 self.emit_expr(*c)?;
                 self.write(", \"col\") + 1L]");
            }
            IRExprKind::Slice1D { base, a, b } => {
                self.emit_expr(*base)?;
                self.write("[rr_range(");
                self.emit_expr(*a)?;
                self.write(", ");
                self.emit_expr(*b)?;
                self.write(") + 1L]");
            }
            IRExprKind::VectorLit(elems) => {
                self.write("c(");
                for (i, e) in elems.into_iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.emit_expr(e)?;
                }
                self.write(")");
            }
            IRExprKind::ListLit(fields) => {
                self.write("list(");
                for (i, (n, e)) in fields.into_iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.write(&format!("{}=", n));
                    self.emit_expr(e)?;
                }
                self.write(")");
            }
        }
        Ok(())
    }
}


