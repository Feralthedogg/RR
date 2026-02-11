use crate::error::RR;
use crate::mir::def::{BinOp, FnIR, Instr, Lit, Terminator, UnaryOp, Value, ValueKind};
use crate::mir::structurizer::{StructuredBlock, Structurizer};
use crate::utils::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MapEntry {
    pub r_line: u32,
    pub rr_span: Span,
}

pub struct MirEmitter {
    backend: RBackend,
}

impl MirEmitter {
    pub fn new() -> Self {
        Self {
            backend: RBackend::new(),
        }
    }

    pub fn emit(&mut self, fn_ir: &FnIR) -> RR<(String, Vec<MapEntry>)> {
        let code = self.backend.emit_function(fn_ir)?;
        Ok((code, self.backend.source_map.clone()))
    }
}

pub struct RBackend {
    output: String,
    indent: usize,
    current_line: u32,
    pub source_map: Vec<MapEntry>,
    // Codegen-time binding: ValueId -> (var name, var version at bind time).
    value_bindings: HashMap<usize, (String, u64)>,
    // Per-variable write version used to invalidate stale bindings.
    var_versions: HashMap<String, u64>,
}

impl RBackend {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            current_line: 1,
            source_map: Vec::new(),
            value_bindings: HashMap::new(),
            var_versions: HashMap::new(),
        }
    }

    pub fn emit_function(&mut self, fn_ir: &FnIR) -> Result<String, crate::error::RRException> {
        if fn_ir
            .values
            .iter()
            .any(|v| matches!(v.kind, ValueKind::Phi { .. }))
        {
            return Err(crate::error::RRException::new(
                "codegen",
                crate::error::RRCode::ICE9001,
                crate::error::Stage::Codegen,
                "Phi should be eliminated before codegen",
            ));
        }
        self.output.clear();
        self.indent = 0;
        self.current_line = 1;
        self.source_map.clear();
        self.value_bindings.clear();
        self.var_versions.clear();

        self.write(&format!("{} <- function(", fn_ir.name));
        let param_strs: Vec<String> = fn_ir.params.iter().map(|p| p.clone()).collect();
        self.write(&param_strs.join(", "));
        self.write(") ");
        self.newline();
        self.write_indent();
        self.write("{\n");
        self.indent += 1;

        if fn_ir.unsupported_dynamic {
            self.write_stmt(&format!(
                "# rr-hybrid-fallback: {}",
                if fn_ir.fallback_reasons.is_empty() {
                    "dynamic runtime feature detected".to_string()
                } else {
                    fn_ir.fallback_reasons.join(", ")
                }
            ));
        }

        let structured = Structurizer::new(fn_ir).build();
        self.emit_structured(&structured, fn_ir)?;

        self.indent -= 1;
        self.write_indent();
        self.write("}\n");

        Ok(self.output.clone())
    }

    fn record_span(&mut self, span: Span) {
        if span.start_line != 0 {
            self.source_map.push(MapEntry {
                r_line: self.current_line,
                rr_span: span,
            });
        }
    }

    fn emit_instr(
        &mut self,
        instr: &Instr,
        values: &[Value],
        params: &[String],
    ) -> Result<(), crate::error::RRException> {
        match instr {
            Instr::Assign { dst, src, span } => {
                let label = format!("assign {}", dst);
                self.emit_mark(*span, Some(label.as_str()));
                let v = if let Some(bound) = self.resolve_bound_value(*src) {
                    bound
                } else {
                    self.resolve_val(*src, values, params, true) // Prefer expression for RHS
                };
                if v != *dst {
                    self.record_span(*span);
                    self.write_stmt(&format!("{} <- {}", dst, v));
                    self.note_var_write(dst);
                    self.bind_value_to_var(*src, dst);
                }
            }
            Instr::Eval { val, span } => {
                self.emit_mark(*span, Some("eval"));
                self.record_span(*span);
                let v = self.resolve_val(*val, values, params, false);
                self.write_stmt(&v);
            }
            Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector,
                is_safe,
                is_na_safe,
                span,
            } => {
                self.emit_mark(*span, Some("store"));
                self.record_span(*span);
                let base_val = self.resolve_val(*base, values, params, false);
                let idx_val = self.resolve_val(*idx, values, params, false);
                let src_val = self.resolve_val(*val, values, params, false);

                if *is_vector {
                    self.write_stmt(&format!("{} <- {}", base_val, src_val));
                    self.bump_base_version_if_named(*base, values);
                } else {
                    if *is_safe && *is_na_safe {
                        self.write_stmt(&format!("{}[{}] <- {}", base_val, idx_val, src_val));
                    } else {
                        let idx_expr = format!("rr_index1_write({}, \"index\")", idx_val);
                        self.write_stmt(&format!("{}[{}] <- {}", base_val, idx_expr, src_val));
                    }
                    // Indexed store mutates the base object; invalidate stale bindings for that variable.
                    self.bump_base_version_if_named(*base, values);
                }
            }
            Instr::StoreIndex2D {
                base,
                r,
                c,
                val,
                span,
            } => {
                self.emit_mark(*span, Some("store2d"));
                self.record_span(*span);
                let base_val = self.resolve_val(*base, values, params, false);
                let r_val = self.resolve_val(*r, values, params, false);
                let c_val = self.resolve_val(*c, values, params, false);
                let src_val = self.resolve_val(*val, values, params, false);
                let r_idx = format!("rr_index1_write({}, \"row\")", r_val);
                let c_idx = format!("rr_index1_write({}, \"col\")", c_val);
                self.write_stmt(&format!(
                    "{}[{}, {}] <- {}",
                    base_val, r_idx, c_idx, src_val
                ));
                self.bump_base_version_if_named(*base, values);
            }
        }
        Ok(())
    }

    fn current_var_version(&self, var: &str) -> u64 {
        *self.var_versions.get(var).unwrap_or(&0)
    }

    fn note_var_write(&mut self, var: &str) {
        let next = self.current_var_version(var) + 1;
        self.var_versions.insert(var.to_string(), next);
    }

    fn bind_value_to_var(&mut self, val_id: usize, var: &str) {
        let version = self.current_var_version(var);
        self.value_bindings
            .insert(val_id, (var.to_string(), version));
    }

    fn resolve_bound_value(&self, val_id: usize) -> Option<String> {
        if let Some((var, version)) = self.value_bindings.get(&val_id) {
            if self.current_var_version(var) == *version {
                return Some(var.clone());
            }
        }
        None
    }

    fn bump_base_version_if_named(&mut self, base: usize, values: &[Value]) {
        if let Some(var) = values[base].origin_var.as_ref() {
            self.note_var_write(var);
        }
    }

    fn emit_term(
        &mut self,
        term: &Terminator,
        values: &[Value],
        params: &[String],
    ) -> Result<(), crate::error::RRException> {
        match term {
            Terminator::Goto(t) => {
                self.write_stmt(&format!("break; # goto {}", t));
            }
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                let c = self.resolve_val(*cond, values, params, false);
                self.write_stmt(&format!(
                    "if (rr_truthy1({}, \"condition\")) {{ # goto {}/{}",
                    c, then_bb, else_bb
                ));
                self.write_stmt("}");
            }
            Terminator::Return(Some(v)) => {
                let val = self.resolve_val(*v, values, params, false);
                self.write_stmt(&format!("return({})", val));
            }
            Terminator::Return(None) => {
                self.write_stmt("return(NULL)");
            }
            Terminator::Unreachable => {
                // Should be unreachable due to skip in emit_function
                self.write_stmt("rr_fail(\"RR.RuntimeError\", \"ICE9001\", \"unreachable code reached\", \"control flow\")");
            }
        }
        Ok(())
    }

    fn emit_structured(
        &mut self,
        node: &StructuredBlock,
        fn_ir: &FnIR,
    ) -> Result<(), crate::error::RRException> {
        match node {
            StructuredBlock::Sequence(items) => {
                for item in items {
                    self.emit_structured(item, fn_ir)?;
                }
            }
            StructuredBlock::BasicBlock(bid) => {
                self.write_indent();
                self.write(&format!("# Block {}\n", bid));
                self.newline();
                let blk = &fn_ir.blocks[*bid];
                for instr in &blk.instrs {
                    self.emit_instr(instr, &fn_ir.values, &fn_ir.params)?;
                }
            }
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            } => {
                // Branch-local codegen state:
                // emitting `then` must not invalidate value bindings for `else`.
                // Otherwise, edge copies like `x <- x` on else-path can be mis-rendered
                // as re-expanded expressions (e.g. `x <- x + dx`).
                let before_bindings = self.value_bindings.clone();
                let before_versions = self.var_versions.clone();

                let cond_span = fn_ir.values[*cond].span;
                self.emit_mark(cond_span, Some("if"));
                self.record_span(cond_span);
                let c = self.resolve_val(*cond, &fn_ir.values, &fn_ir.params, false);
                self.write_stmt(&format!("if (rr_truthy1({}, \"condition\")) {{", c));
                self.indent += 1;
                self.emit_structured(then_body, fn_ir)?;
                self.indent -= 1;
                if let Some(else_body) = else_body {
                    // Reset to pre-if state before emitting else branch.
                    self.value_bindings = before_bindings.clone();
                    self.var_versions = before_versions.clone();
                    self.write_stmt("} else {");
                    self.indent += 1;
                    self.emit_structured(else_body, fn_ir)?;
                    self.indent -= 1;
                    self.write_stmt("}");
                } else {
                    self.write_stmt("}");
                }

                // Join point: drop branch-local expression bindings conservatively.
                self.value_bindings.clear();
                self.var_versions = before_versions;
            }
            StructuredBlock::Loop {
                header,
                cond,
                continue_on_true,
                body,
            } => {
                self.write_stmt("repeat {");
                self.indent += 1;

                self.write_indent();
                self.write(&format!("# LoopHeader {}\n", header));
                self.newline();
                let blk = &fn_ir.blocks[*header];
                for instr in &blk.instrs {
                    self.emit_instr(instr, &fn_ir.values, &fn_ir.params)?;
                }

                let cond_span = fn_ir.values[*cond].span;
                self.emit_mark(cond_span, Some("loop-cond"));
                self.record_span(cond_span);
                let c = self.resolve_val(*cond, &fn_ir.values, &fn_ir.params, false);
                if *continue_on_true {
                    self.write_stmt(&format!("if (!rr_truthy1({}, \"condition\")) break", c));
                } else {
                    self.write_stmt(&format!("if (rr_truthy1({}, \"condition\")) break", c));
                }
                self.emit_structured(body, fn_ir)?;

                self.indent -= 1;
                self.write_stmt("}");
            }
            StructuredBlock::Break => {
                self.write_stmt("break");
            }
            StructuredBlock::Next => {
                self.write_stmt("next");
            }
            StructuredBlock::Return(v) => match v {
                Some(val) => {
                    let r = self.resolve_val(*val, &fn_ir.values, &fn_ir.params, false);
                    self.write_stmt(&format!("return({})", r));
                }
                None => self.write_stmt("return(NULL)"),
            },
        }
        Ok(())
    }

    fn resolve_val(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        prefer_expr: bool,
    ) -> String {
        let val = &values[val_id];

        if !prefer_expr {
            if let Some(bound) = self.resolve_bound_value(val_id) {
                return bound;
            }
        }

        // Strategy:
        // 1. If prefer_expr is false (we are using the value) and it has a name, use the name.
        //    (Except for literals which are better as literals)
        // 2. Otherwise, resolve the expression.

        // Use variable names only for value kinds that are stable to reference by name.
        // For expression values (e.g. Binary/Index), forcing the name can miscompile after
        // CSE/GVN when a value reuses a variable-origin annotation.
        let should_use_name = !prefer_expr
            && val.origin_var.is_some()
            && matches!(
                val.kind,
                ValueKind::Load { .. } | ValueKind::Param { .. } | ValueKind::Call { .. }
            );
        if should_use_name {
            return val.origin_var.as_ref().unwrap().clone();
        }

        match &val.kind {
            ValueKind::Const(lit) => self.emit_lit(lit),
            ValueKind::Phi { .. } => {
                panic!("Phi should have been eliminated before codegen");
            }
            ValueKind::Param { index } => {
                if *index < params.len() {
                    let p = params[*index].clone();
                    p
                } else {
                    format!(".p{}", index)
                }
            }
            ValueKind::Binary { op, lhs, rhs } => {
                let l = self.resolve_val(*lhs, values, params, false);
                let r = self.resolve_val(*rhs, values, params, false);
                if matches!(op, BinOp::Add)
                    && (matches!(values[*lhs].kind, ValueKind::Const(Lit::Str(_)))
                        || matches!(values[*rhs].kind, ValueKind::Const(Lit::Str(_))))
                {
                    return format!("paste0({}, {})", l, r);
                }
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%%",
                    BinOp::MatMul => "%*%",
                    BinOp::Eq => "==",
                    BinOp::Ne => "!=",
                    BinOp::Lt => "<",
                    BinOp::Le => "<=",
                    BinOp::Gt => ">",
                    BinOp::Ge => ">=",
                    BinOp::And => "&",
                    BinOp::Or => "|",
                };
                format!("({} {} {})", l, op_str, r)
            }
            ValueKind::Unary { op, rhs } => {
                let r = self.resolve_val(*rhs, values, params, false);
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                };
                format!("({}({}))", op_str, r)
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                let mut arg_strs: Vec<String> = Vec::with_capacity(args.len());
                for (i, a) in args.iter().enumerate() {
                    let v = self.resolve_val(*a, values, params, false);
                    if let Some(Some(name)) = names.get(i) {
                        arg_strs.push(format!("{} = {}", name, v));
                    } else {
                        arg_strs.push(v);
                    }
                }
                format!("{}({})", callee, arg_strs.join(", "))
            }
            ValueKind::Len { base } => {
                format!("length({})", self.resolve_val(*base, values, params, false))
            }
            ValueKind::Range { start, end } => {
                format!(
                    "{}:{}",
                    self.resolve_val(*start, values, params, false),
                    self.resolve_val(*end, values, params, false)
                )
            }
            ValueKind::Indices { base } => {
                format!(
                    "(seq_along({}) - 1L)",
                    self.resolve_val(*base, values, params, false)
                )
            }
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                let b = self.resolve_val(*base, values, params, false);
                let i = self.resolve_val(*idx, values, params, false);
                if *is_safe && *is_na_safe {
                    format!("{}[{}]", b, i)
                } else {
                    format!("rr_index1_read({}, {}, \"index\")", b, i)
                }
            }
            ValueKind::Index2D { base, r, c } => {
                let b = self.resolve_val(*base, values, params, false);
                let rr = self.resolve_val(*r, values, params, false);
                let cc = self.resolve_val(*c, values, params, false);
                format!(
                    "{}[rr_index1_write({}, \"row\"), rr_index1_write({}, \"col\")]",
                    b, rr, cc
                )
            }
            ValueKind::Load { var } => var.clone(),
        }
    }

    fn emit_lit(&self, lit: &Lit) -> String {
        match lit {
            Lit::Int(i) => format!("{}L", i),
            Lit::Float(f) => f.to_string(),
            Lit::Str(s) => format!("\"{}\"", s),
            Lit::Bool(true) => "TRUE".to_string(),
            Lit::Bool(false) => "FALSE".to_string(),
            Lit::Null => "NULL".to_string(),
            Lit::Na => "NA".to_string(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    fn newline(&mut self) {
        self.output.push('\n');
        self.current_line += 1;
    }

    fn write_stmt(&mut self, s: &str) {
        self.write_indent();
        self.write(s);
        self.newline();
    }

    fn emit_mark(&mut self, span: Span, label: Option<&str>) {
        if span.start_line == 0 {
            return;
        }
        self.write_indent();
        self.write(&format!(
            "rr_mark({}, {});",
            span.start_line, span.start_col
        ));
        if let Some(lbl) = label {
            self.write(&format!(
                " # rr:{}:{} {}",
                span.start_line, span.start_col, lbl
            ));
        } else {
            self.write(&format!(" # rr:{}:{}", span.start_line, span.start_col));
        }
        self.newline();
    }
}
