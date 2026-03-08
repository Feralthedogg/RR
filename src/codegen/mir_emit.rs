use crate::error::RR;
use crate::mir::def::{
    BinOp, FnIR, Instr, IntrinsicOp, Lit, Terminator, UnaryOp, Value, ValueKind,
};
use crate::mir::flow::Facts;
use crate::mir::structurizer::{StructuredBlock, Structurizer};
use crate::typeck::{PrimTy, ShapeTy};
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntry {
    pub r_line: u32,
    pub rr_span: Span,
}

pub struct MirEmitter {
    backend: RBackend,
}

impl Default for MirEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl MirEmitter {
    pub fn new() -> Self {
        Self {
            backend: RBackend::new(),
        }
    }

    pub fn emit(&mut self, fn_ir: &FnIR) -> RR<(String, Vec<MapEntry>)> {
        self.backend.emit_function(fn_ir)
    }
}

#[derive(Debug)]
struct ValueBindingUndo {
    val_id: usize,
    prev: Option<(String, u64)>,
}

#[derive(Debug)]
struct VarVersionUndo {
    var: String,
    prev: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
struct BranchSnapshot {
    value_binding_log_len: usize,
    var_version_log_len: usize,
}

pub struct RBackend {
    output: String,
    indent: usize,
    current_line: u32,
    pub source_map: Vec<MapEntry>,
    // Codegen-time binding: ValueId -> (var name, var version at bind time).
    value_bindings: FxHashMap<usize, (String, u64)>,
    // Per-variable write version used to invalidate stale bindings.
    var_versions: FxHashMap<String, u64>,
    value_binding_log: Vec<ValueBindingUndo>,
    var_version_log: Vec<VarVersionUndo>,
    branch_snapshot_depth: usize,
    expr_use_counts_scratch: FxHashMap<usize, usize>,
    expr_path_scratch: FxHashSet<usize>,
    emitted_ids_scratch: FxHashSet<usize>,
    emitted_temp_names_scratch: Vec<String>,
}

impl Default for RBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RBackend {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            current_line: 1,
            source_map: Vec::new(),
            value_bindings: FxHashMap::default(),
            var_versions: FxHashMap::default(),
            value_binding_log: Vec::new(),
            var_version_log: Vec::new(),
            branch_snapshot_depth: 0,
            expr_use_counts_scratch: FxHashMap::default(),
            expr_path_scratch: FxHashSet::default(),
            emitted_ids_scratch: FxHashSet::default(),
            emitted_temp_names_scratch: Vec::new(),
        }
    }

    pub fn emit_function(
        &mut self,
        fn_ir: &FnIR,
    ) -> Result<(String, Vec<MapEntry>), crate::error::RRException> {
        if let Err(err) = crate::mir::verify::verify_emittable_ir(fn_ir) {
            return Err(crate::error::RRException::new(
                "codegen",
                crate::error::RRCode::ICE9001,
                crate::error::Stage::Codegen,
                err.to_string(),
            ));
        }
        self.output.clear();
        self.indent = 0;
        self.current_line = 1;
        self.source_map.clear();
        self.value_bindings.clear();
        self.var_versions.clear();
        self.value_binding_log.clear();
        self.var_version_log.clear();
        self.branch_snapshot_depth = 0;
        self.expr_use_counts_scratch.clear();
        self.expr_path_scratch.clear();
        self.emitted_ids_scratch.clear();

        self.write(fn_ir.name.as_str());
        self.write(" <- function(");
        for (idx, param) in fn_ir.params.iter().enumerate() {
            if idx > 0 {
                self.write(", ");
            }
            self.write(param);
        }
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
        if fn_ir.opaque_interop {
            self.write_stmt(&format!(
                "# rr-opaque-interop: {}",
                if fn_ir.opaque_reasons.is_empty() {
                    "package/runtime interop requires conservative optimization".to_string()
                } else {
                    fn_ir.opaque_reasons.join(", ")
                }
            ));
        }

        let structured = Structurizer::new(fn_ir).build();
        self.emit_structured(&structured, fn_ir)?;

        self.indent -= 1;
        self.write_indent();
        self.write("}\n");

        Ok((
            std::mem::take(&mut self.output),
            std::mem::take(&mut self.source_map),
        ))
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
                self.emitted_temp_names_scratch.clear();
                let v = if let Some(bound) = self.resolve_bound_value(*src) {
                    bound
                } else {
                    // Probe the RHS first; if assignment is a no-op (`dst <- dst`),
                    // skip CSE temp emission to avoid dead temporaries.
                    let preview = self.resolve_val(*src, values, params, true);
                    if preview == *dst {
                        preview
                    } else {
                        self.emit_common_subexpr_temps(*src, values, params);
                        self.resolve_val(*src, values, params, true) // Prefer expression for RHS
                    }
                };
                if v != *dst {
                    self.record_span(*span);
                    self.write_stmt(&format!("{} <- {}", dst, v));
                    self.note_var_write(dst);
                    self.bind_value_to_var(*src, dst);
                }
                self.invalidate_emitted_cse_temps();
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
                    let idx_elidable = Self::can_elide_index_wrapper(*idx, values);
                    if (*is_safe && *is_na_safe) || idx_elidable {
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
                let r_idx = if Self::can_elide_index_wrapper(*r, values) {
                    r_val
                } else {
                    format!("rr_index1_write({}, \"row\")", r_val)
                };
                let c_idx = if Self::can_elide_index_wrapper(*c, values) {
                    c_val
                } else {
                    format!("rr_index1_write({}, \"col\")", c_val)
                };
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
        self.log_var_version_change(var);
        self.var_versions.insert(var.to_string(), next);
    }

    fn bind_value_to_var(&mut self, val_id: usize, var: &str) {
        let version = self.current_var_version(var);
        self.log_value_binding_change(val_id);
        self.value_bindings
            .insert(val_id, (var.to_string(), version));
    }

    fn resolve_bound_value(&self, val_id: usize) -> Option<String> {
        if let Some((var, version)) = self.value_bindings.get(&val_id)
            && self.current_var_version(var) == *version
        {
            return Some(var.clone());
        }
        None
    }

    fn bump_base_version_if_named(&mut self, base: usize, values: &[Value]) {
        if let Some(var) = values[base].origin_var.as_ref() {
            self.note_var_write(var);
        }
    }

    fn begin_branch_snapshot(&mut self) -> BranchSnapshot {
        self.branch_snapshot_depth += 1;
        BranchSnapshot {
            value_binding_log_len: self.value_binding_log.len(),
            var_version_log_len: self.var_version_log.len(),
        }
    }

    fn rollback_branch_snapshot(&mut self, snapshot: BranchSnapshot) {
        while self.value_binding_log.len() > snapshot.value_binding_log_len {
            let Some(undo) = self.value_binding_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.value_bindings.insert(undo.val_id, prev);
            } else {
                self.value_bindings.remove(&undo.val_id);
            }
        }
        while self.var_version_log.len() > snapshot.var_version_log_len {
            let Some(undo) = self.var_version_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.var_versions.insert(undo.var, prev);
            } else {
                self.var_versions.remove(&undo.var);
            }
        }
    }

    fn end_branch_snapshot(&mut self) {
        if self.branch_snapshot_depth > 0 {
            self.branch_snapshot_depth -= 1;
        }
    }

    fn log_value_binding_change(&mut self, val_id: usize) {
        if self.branch_snapshot_depth == 0 {
            return;
        }
        self.value_binding_log.push(ValueBindingUndo {
            val_id,
            prev: self.value_bindings.get(&val_id).cloned(),
        });
    }

    fn log_var_version_change(&mut self, var: &str) {
        if self.branch_snapshot_depth == 0 {
            return;
        }
        self.var_version_log.push(VarVersionUndo {
            var: var.to_string(),
            prev: self.var_versions.get(var).copied(),
        });
    }

    fn emit_common_subexpr_temps(&mut self, root: usize, values: &[Value], params: &[String]) {
        let mut counts = std::mem::take(&mut self.expr_use_counts_scratch);
        let mut path = std::mem::take(&mut self.expr_path_scratch);
        let mut emitted_ids = std::mem::take(&mut self.emitted_ids_scratch);
        let mut temps = std::mem::take(&mut self.emitted_temp_names_scratch);
        counts.clear();
        path.clear();
        emitted_ids.clear();
        temps.clear();

        Self::collect_expr_use_counts(root, values, &mut counts, &mut path);
        if !counts.values().any(|c| *c > 1) {
            self.expr_use_counts_scratch = counts;
            self.expr_path_scratch = path;
            self.emitted_ids_scratch = emitted_ids;
            self.emitted_temp_names_scratch = temps;
            return;
        }

        path.clear();
        self.emit_hoisted_subexprs_dfs(
            root,
            root,
            values,
            params,
            &counts,
            &mut emitted_ids,
            &mut path,
            &mut temps,
        );
        self.expr_use_counts_scratch = counts;
        self.expr_path_scratch = path;
        self.emitted_ids_scratch = emitted_ids;
        self.emitted_temp_names_scratch = temps;
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_hoisted_subexprs_dfs(
        &mut self,
        vid: usize,
        root: usize,
        values: &[Value],
        params: &[String],
        counts: &FxHashMap<usize, usize>,
        emitted_ids: &mut FxHashSet<usize>,
        path: &mut FxHashSet<usize>,
        temps: &mut Vec<String>,
    ) {
        if !path.insert(vid) {
            return;
        }
        Self::for_each_expr_child(vid, values, |child| {
            self.emit_hoisted_subexprs_dfs(
                child,
                root,
                values,
                params,
                counts,
                emitted_ids,
                path,
                temps,
            );
        });
        path.remove(&vid);

        if vid == root {
            return;
        }
        let uses = counts.get(&vid).copied().unwrap_or(0);
        if !Self::should_hoist_common_subexpr(vid, uses, values) {
            return;
        }
        if !emitted_ids.insert(vid) {
            return;
        }
        if self.resolve_bound_value(vid).is_some() {
            return;
        }

        let temp = format!(".__rr_cse_{}", vid);
        let expr = self.resolve_val(vid, values, params, true);
        self.write_stmt(&format!("{} <- {}", temp, expr));
        self.note_var_write(&temp);
        self.bind_value_to_var(vid, &temp);
        temps.push(temp);
    }

    fn collect_expr_use_counts(
        root: usize,
        values: &[Value],
        counts: &mut FxHashMap<usize, usize>,
        path: &mut FxHashSet<usize>,
    ) {
        *counts.entry(root).or_insert(0) += 1;
        if !path.insert(root) {
            return;
        }
        Self::for_each_expr_child(root, values, |child| {
            Self::collect_expr_use_counts(child, values, counts, path);
        });
        path.remove(&root);
    }

    fn for_each_expr_child<F>(vid: usize, values: &[Value], mut visit: F)
    where
        F: FnMut(usize),
    {
        match &values[vid].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                visit(*lhs);
                visit(*rhs);
            }
            ValueKind::Unary { rhs, .. } => visit(*rhs),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    visit(*arg);
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => visit(*base),
            ValueKind::Range { start, end } => {
                visit(*start);
                visit(*end);
            }
            ValueKind::Index1D { base, idx, .. } => {
                visit(*base);
                visit(*idx);
            }
            ValueKind::Index2D { base, r, c } => {
                visit(*base);
                visit(*r);
                visit(*c);
            }
            ValueKind::Phi { args } => {
                for (value, _) in args {
                    visit(*value);
                }
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
        }
    }

    fn invalidate_emitted_cse_temps(&mut self) {
        let mut temps = std::mem::take(&mut self.emitted_temp_names_scratch);
        for temp in temps.drain(..) {
            // Keep hoisted temps local to this assignment to avoid stale bindings.
            self.note_var_write(&temp);
        }
        self.emitted_temp_names_scratch = temps;
    }

    fn should_hoist_common_subexpr(vid: usize, uses: usize, values: &[Value]) -> bool {
        if uses <= 1 || values[vid].origin_var.is_some() {
            return false;
        }
        matches!(
            values[vid].kind,
            ValueKind::Call { .. }
                | ValueKind::Intrinsic { .. }
                | ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
                | ValueKind::Range { .. }
                | ValueKind::Len { .. }
                | ValueKind::Indices { .. }
        ) || match &values[vid].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                !Self::is_const_like_leaf(*lhs, values) || !Self::is_const_like_leaf(*rhs, values)
            }
            ValueKind::Unary { rhs, .. } => !Self::is_const_like_leaf(*rhs, values),
            _ => false,
        }
    }

    fn is_const_like_leaf(vid: usize, values: &[Value]) -> bool {
        matches!(values[vid].kind, ValueKind::Const(_))
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
                let c = self.resolve_cond(*cond, values, params);
                self.write_stmt(&format!("if ({}) {{ # goto {}/{}", c, then_bb, else_bb));
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
                let snapshot = self.begin_branch_snapshot();

                let cond_span = fn_ir.values[*cond].span;
                self.emit_mark(cond_span, Some("if"));
                self.record_span(cond_span);
                let c = self.resolve_cond(*cond, &fn_ir.values, &fn_ir.params);
                self.write_stmt(&format!("if ({}) {{", c));
                self.indent += 1;
                self.emit_structured(then_body, fn_ir)?;
                self.indent -= 1;
                if let Some(else_body) = else_body {
                    // Reset to pre-if state before emitting else branch.
                    self.rollback_branch_snapshot(snapshot);
                    self.write_stmt("} else {");
                    self.indent += 1;
                    self.emit_structured(else_body, fn_ir)?;
                    self.indent -= 1;
                    self.write_stmt("}");
                } else {
                    self.write_stmt("}");
                }

                // Join point: drop branch-local expression bindings conservatively.
                self.rollback_branch_snapshot(snapshot);
                self.end_branch_snapshot();
                self.value_bindings.clear();
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
                let c = self.resolve_cond(*cond, &fn_ir.values, &fn_ir.params);
                if *continue_on_true {
                    self.write_stmt(&format!("if (!{}) break", c));
                } else {
                    self.write_stmt(&format!("if ({}) break", c));
                }
                self.emit_structured(body, fn_ir)?;

                self.indent -= 1;
                self.write_stmt("}");

                // Loop bodies may execute an unknown number of times (including zero).
                // Drop expression/value bindings after emitting a loop to avoid leaking
                // single-iteration assumptions into post-loop value resolution.
                self.value_bindings.clear();
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

        if !prefer_expr && let Some(bound) = self.resolve_bound_value(val_id) {
            return bound;
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
        if should_use_name && let Some(origin_var) = &val.origin_var {
            return origin_var.clone();
        }

        match &val.kind {
            ValueKind::Const(lit) => self.emit_lit(lit),
            ValueKind::Phi { .. } => {
                // Keep codegen non-panicking on unexpected IR; emit an explicit ICE trap.
                "rr_fail(\"RR.InternalError\", \"ICE9001\", \"phi reached codegen\", \"codegen\")"
                    .to_string()
            }
            ValueKind::Param { index } => self.resolve_param(*index, params),
            ValueKind::Binary { op, lhs, rhs } => {
                self.resolve_binary_expr(val, *op, *lhs, *rhs, values, params)
            }
            ValueKind::Unary { op, rhs } => self.resolve_unary_expr(*op, *rhs, values, params),
            ValueKind::Call {
                callee,
                args,
                names,
            } => self.resolve_call_expr(callee, args, names, values, params),
            ValueKind::Intrinsic { op, args } => {
                self.resolve_intrinsic_expr(*op, args, values, params)
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
            } => self.resolve_index1d_expr(*base, *idx, *is_safe, *is_na_safe, values, params),
            ValueKind::Index2D { base, r, c } => {
                self.resolve_index2d_expr(*base, *r, *c, values, params)
            }
            ValueKind::Load { var } => var.clone(),
            ValueKind::RSymbol { name } => name.clone(),
        }
    }

    fn resolve_param(&self, index: usize, params: &[String]) -> String {
        if index < params.len() {
            params[index].clone()
        } else {
            format!(".p{}", index)
        }
    }

    fn resolve_binary_expr(
        &self,
        val: &Value,
        op: BinOp,
        lhs: usize,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let l = self.resolve_val(lhs, values, params, false);
        let r = self.resolve_val(rhs, values, params, false);
        if matches!(op, BinOp::Add)
            && (matches!(values[lhs].kind, ValueKind::Const(Lit::Str(_)))
                || matches!(values[rhs].kind, ValueKind::Const(Lit::Str(_))))
        {
            return format!("paste0({}, {})", l, r);
        }
        let ty = val.value_ty;
        if ty.shape == ShapeTy::Vector && ty.prim == PrimTy::Double {
            match op {
                BinOp::Add => return format!("rr_parallel_vec_add_f64({}, {})", l, r),
                BinOp::Sub => return format!("rr_parallel_vec_sub_f64({}, {})", l, r),
                BinOp::Mul => return format!("rr_parallel_vec_mul_f64({}, {})", l, r),
                BinOp::Div => return format!("rr_parallel_vec_div_f64({}, {})", l, r),
                _ => {}
            }
        }
        format!("({} {} {})", l, Self::binary_op_str(op), r)
    }

    fn resolve_unary_expr(
        &self,
        op: UnaryOp,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let r = self.resolve_val(rhs, values, params, false);
        format!("({}({}))", Self::unary_op_str(op), r)
    }

    fn resolve_call_expr(
        &self,
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
        params: &[String],
    ) -> String {
        if let Some((base, idx)) = Self::floor_index_read_components(callee, args, names, values) {
            let b = self.resolve_val(base, values, params, false);
            let i = self.resolve_val(idx, values, params, false);
            return format!("rr_index1_read_idx({}, {}, \"index\")", b, i);
        }
        if Self::can_elide_identity_floor_call(callee, args, names, values) {
            return self.resolve_val(args[0], values, params, false);
        }
        let arg_list = self.build_named_arg_list(args, names, values, params);
        format!("{}({})", callee, arg_list)
    }

    fn resolve_intrinsic_expr(
        &self,
        op: IntrinsicOp,
        args: &[usize],
        values: &[Value],
        params: &[String],
    ) -> String {
        let arg_list = self.build_plain_arg_list(args, values, params);
        format!("{}({})", Self::intrinsic_helper(op), arg_list)
    }

    fn resolve_index1d_expr(
        &self,
        base: usize,
        idx: usize,
        is_safe: bool,
        is_na_safe: bool,
        values: &[Value],
        params: &[String],
    ) -> String {
        let b = self.resolve_val(base, values, params, false);
        let i = self.resolve_val(idx, values, params, false);
        if (is_safe && is_na_safe) || Self::can_elide_index_wrapper(idx, values) {
            format!("{}[{}]", b, i)
        } else {
            format!("rr_index1_read({}, {}, \"index\")", b, i)
        }
    }

    fn resolve_index2d_expr(
        &self,
        base: usize,
        r: usize,
        c: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let b = self.resolve_val(base, values, params, false);
        let rr = self.resolve_val(r, values, params, false);
        let cc = self.resolve_val(c, values, params, false);
        let r_idx = if Self::can_elide_index_wrapper(r, values) {
            rr
        } else {
            format!("rr_index1_write({}, \"row\")", rr)
        };
        let c_idx = if Self::can_elide_index_wrapper(c, values) {
            cc
        } else {
            format!("rr_index1_write({}, \"col\")", cc)
        };
        format!("{}[{}, {}]", b, r_idx, c_idx)
    }

    fn build_named_arg_list(
        &self,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
        params: &[String],
    ) -> String {
        let mut out = String::new();
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            let value = self.resolve_val(*a, values, params, false);
            if let Some(Some(name)) = names.get(i) {
                out.push_str(name);
                out.push_str(" = ");
                out.push_str(&value);
            } else {
                out.push_str(&value);
            }
        }
        out
    }

    fn build_plain_arg_list(&self, args: &[usize], values: &[Value], params: &[String]) -> String {
        let mut out = String::new();
        for (idx, arg) in args.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push_str(&self.resolve_val(*arg, values, params, false));
        }
        out
    }

    fn intrinsic_helper(op: IntrinsicOp) -> &'static str {
        match op {
            IntrinsicOp::VecAddF64 => "rr_intrinsic_vec_add_f64",
            IntrinsicOp::VecSubF64 => "rr_intrinsic_vec_sub_f64",
            IntrinsicOp::VecMulF64 => "rr_intrinsic_vec_mul_f64",
            IntrinsicOp::VecDivF64 => "rr_intrinsic_vec_div_f64",
            IntrinsicOp::VecAbsF64 => "rr_intrinsic_vec_abs_f64",
            IntrinsicOp::VecLogF64 => "rr_intrinsic_vec_log_f64",
            IntrinsicOp::VecSqrtF64 => "rr_intrinsic_vec_sqrt_f64",
            IntrinsicOp::VecPmaxF64 => "rr_intrinsic_vec_pmax_f64",
            IntrinsicOp::VecPminF64 => "rr_intrinsic_vec_pmin_f64",
            IntrinsicOp::VecSumF64 => "rr_intrinsic_vec_sum_f64",
            IntrinsicOp::VecMeanF64 => "rr_intrinsic_vec_mean_f64",
        }
    }

    fn binary_op_str(op: BinOp) -> &'static str {
        match op {
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
        }
    }

    fn unary_op_str(op: UnaryOp) -> &'static str {
        match op {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        }
    }

    fn resolve_cond(&self, cond: usize, values: &[Value], params: &[String]) -> String {
        let c = self.resolve_val(cond, values, params, false);
        if values[cond].value_ty.is_logical_scalar_non_na() {
            c
        } else {
            format!("rr_truthy1({}, \"condition\")", c)
        }
    }

    fn can_elide_identity_floor_call(
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
    ) -> bool {
        if !matches!(callee, "floor" | "ceiling" | "trunc") {
            return false;
        }
        if args.len() != 1 || names.len() > 1 {
            return false;
        }
        if names
            .first()
            .and_then(std::option::Option::as_ref)
            .is_some()
        {
            return false;
        }
        values
            .get(args[0])
            .map(|v| v.value_ty.is_int_scalar_non_na() || v.facts.has(Facts::INT_SCALAR))
            .unwrap_or(false)
    }

    fn floor_index_read_components(
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
    ) -> Option<(usize, usize)> {
        if !matches!(callee, "floor" | "ceiling" | "trunc") {
            return None;
        }
        if args.len() != 1 || names.len() > 1 {
            return None;
        }
        if names
            .first()
            .and_then(std::option::Option::as_ref)
            .is_some()
        {
            return None;
        }
        let inner = *args.first()?;
        match &values.get(inner)?.kind {
            ValueKind::Index1D { base, idx, .. } => Some((*base, *idx)),
            ValueKind::Call {
                callee: inner_callee,
                args: inner_args,
                names: inner_names,
            } if matches!(
                inner_callee.as_str(),
                "rr_index1_read" | "rr_index1_read_strict" | "rr_index1_read_floor"
            ) && (inner_args.len() == 2 || inner_args.len() == 3)
                && inner_names.iter().take(2).all(std::option::Option::is_none) =>
            {
                Some((inner_args[0], inner_args[1]))
            }
            _ => None,
        }
    }

    fn can_elide_index_wrapper(idx: usize, values: &[Value]) -> bool {
        let Some(v) = values.get(idx) else {
            return false;
        };
        if v.facts
            .has(Facts::ONE_BASED | Facts::INT_SCALAR | Facts::NON_NA)
        {
            return true;
        }
        match &v.kind {
            ValueKind::Const(Lit::Int(n)) => *n >= 1,
            ValueKind::Const(Lit::Float(f))
                if f.is_finite()
                    && (*f - f.trunc()).abs() < f64::EPSILON
                    && *f >= 1.0
                    && *f <= i64::MAX as f64 =>
            {
                true
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } if callee == "rr_index1_read_idx"
                && (args.len() == 2 || args.len() == 3)
                && names.iter().take(2).all(std::option::Option::is_none) =>
            {
                true
            }
            _ => false,
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
