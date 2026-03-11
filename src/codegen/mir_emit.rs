use crate::error::RR;
use crate::mir::def::{
    BinOp, FnIR, Instr, IntrinsicOp, Lit, Terminator, UnaryOp, Value, ValueKind,
};
use crate::mir::flow::Facts;
use crate::mir::structurizer::{StructuredBlock, Structurizer};
use crate::typeck::{PrimTy, ShapeTy, TypeTerm};
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

#[derive(Debug, Clone)]
struct TypedParallelWrapperPlan {
    impl_name: String,
    slice_param_slots: Vec<usize>,
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
        self.emitted_temp_names_scratch.clear();

        let wrapper_plan = Self::typed_parallel_wrapper_plan(fn_ir);
        if let Some(plan) = wrapper_plan.as_ref() {
            self.emit_function_named(fn_ir, &plan.impl_name)?;
            self.newline();
            self.emit_typed_parallel_wrapper(fn_ir, plan);
        } else {
            self.emit_function_named(fn_ir, fn_ir.name.as_str())?;
        }
        Self::prune_dead_cse_temps(&mut self.output);

        Ok((
            std::mem::take(&mut self.output),
            std::mem::take(&mut self.source_map),
        ))
    }

    fn emit_function_named(
        &mut self,
        fn_ir: &FnIR,
        emitted_name: &str,
    ) -> Result<(), crate::error::RRException> {
        self.value_bindings.clear();
        self.var_versions.clear();
        self.value_binding_log.clear();
        self.var_version_log.clear();
        self.branch_snapshot_depth = 0;
        self.expr_use_counts_scratch.clear();
        self.expr_path_scratch.clear();
        self.emitted_ids_scratch.clear();
        self.emitted_temp_names_scratch.clear();

        self.write(emitted_name);
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
        self.write("{");
        self.newline();
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
        self.write("}");
        self.newline();
        Ok(())
    }

    fn emit_typed_parallel_wrapper(&mut self, fn_ir: &FnIR, plan: &TypedParallelWrapperPlan) {
        self.write_stmt("# rr-typed-parallel-wrapper");
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
        self.write("{");
        self.newline();
        self.indent += 1;

        let slice_slots = plan
            .slice_param_slots
            .iter()
            .map(|slot| format!("{}L", slot + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let arg_list = if fn_ir.params.is_empty() {
            String::new()
        } else {
            format!(", {}", fn_ir.params.join(", "))
        };
        self.write_stmt(&format!(
            "return(rr_parallel_typed_vec_call(\"{}\", {}, c({}){}))",
            fn_ir.name, plan.impl_name, slice_slots, arg_list
        ));

        self.indent -= 1;
        self.write_indent();
        self.write("}");
        self.newline();
    }

    fn typed_parallel_wrapper_plan(fn_ir: &FnIR) -> Option<TypedParallelWrapperPlan> {
        if fn_ir.unsupported_dynamic || fn_ir.opaque_interop {
            return None;
        }
        if !Self::typed_parallel_returns_vector(fn_ir) {
            return None;
        }
        if !Self::typed_parallel_cfg_is_straight_line(fn_ir) {
            return None;
        }

        let bindings = Self::collect_typed_parallel_local_bindings(fn_ir)?;
        let slice_param_slots = Self::typed_parallel_slice_param_slots(fn_ir, &bindings);
        if slice_param_slots.is_empty() {
            return None;
        }

        let ret = Self::typed_parallel_return_value(fn_ir)?;
        if !Self::is_typed_parallel_safe_value(fn_ir, ret, &bindings, &mut FxHashSet::default()) {
            return None;
        }

        Some(TypedParallelWrapperPlan {
            impl_name: format!("{}__typed_impl", fn_ir.name),
            slice_param_slots,
        })
    }

    fn typed_parallel_returns_vector(fn_ir: &FnIR) -> bool {
        matches!(fn_ir.ret_term_hint.as_ref(), Some(TypeTerm::Vector(_)))
            || matches!(fn_ir.inferred_ret_term, TypeTerm::Vector(_))
    }

    fn typed_parallel_slice_param_slots(
        fn_ir: &FnIR,
        bindings: &FxHashMap<String, usize>,
    ) -> Vec<usize> {
        let mut slots = Vec::new();
        for idx in 0..fn_ir.params.len() {
            if Self::typed_parallel_param_is_vector(fn_ir, idx, bindings) {
                slots.push(idx);
            }
        }
        slots
    }

    fn typed_parallel_param_is_vector(
        fn_ir: &FnIR,
        idx: usize,
        bindings: &FxHashMap<String, usize>,
    ) -> bool {
        if fn_ir
            .param_ty_hints
            .get(idx)
            .is_some_and(|ty| ty.shape == ShapeTy::Vector)
        {
            return true;
        }
        if matches!(fn_ir.param_term_hints.get(idx), Some(TypeTerm::Vector(_))) {
            return true;
        }
        fn_ir.values.iter().any(|value| {
            matches!(value.kind, ValueKind::Param { index } if index == idx)
                && (value.value_ty.shape == ShapeTy::Vector
                    || matches!(value.value_term, TypeTerm::Vector(_)))
        }) || fn_ir.values.iter().any(|value| {
            (value.value_ty.shape == ShapeTy::Vector
                || matches!(value.value_term, TypeTerm::Vector(_)))
                && Self::typed_parallel_value_param_slot(
                    fn_ir,
                    value.id,
                    bindings,
                    &mut FxHashSet::default(),
                ) == Some(idx)
        })
    }

    fn typed_parallel_value_param_slot(
        fn_ir: &FnIR,
        vid: usize,
        bindings: &FxHashMap<String, usize>,
        seen: &mut FxHashSet<usize>,
    ) -> Option<usize> {
        if !seen.insert(vid) {
            return None;
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Param { index } => Some(*index),
            ValueKind::Load { var } => bindings
                .get(var)
                .copied()
                .and_then(|src| Self::typed_parallel_value_param_slot(fn_ir, src, bindings, seen)),
            _ => None,
        }
    }

    fn typed_parallel_cfg_is_straight_line(fn_ir: &FnIR) -> bool {
        let mut returns = 0usize;
        for bb in &fn_ir.blocks {
            if bb
                .instrs
                .iter()
                .any(|ins| !matches!(ins, Instr::Assign { .. }))
            {
                return false;
            }
            match bb.term {
                Terminator::Goto(target) => {
                    if target <= bb.id {
                        return false;
                    }
                }
                Terminator::Return(Some(_)) => returns += 1,
                Terminator::Unreachable => {}
                _ => return false,
            }
        }
        returns == 1
    }

    fn collect_typed_parallel_local_bindings(fn_ir: &FnIR) -> Option<FxHashMap<String, usize>> {
        let mut bindings = FxHashMap::default();
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    return None;
                };
                if let Some(prev) = bindings.insert(dst.clone(), *src)
                    && prev != *src
                {
                    return None;
                }
            }
        }
        Some(bindings)
    }

    fn typed_parallel_return_value(fn_ir: &FnIR) -> Option<usize> {
        let mut ret = None;
        for bb in &fn_ir.blocks {
            let Terminator::Return(Some(value)) = bb.term else {
                continue;
            };
            if ret.replace(value).is_some() {
                return None;
            }
        }
        ret
    }

    fn is_typed_parallel_safe_value(
        fn_ir: &FnIR,
        vid: usize,
        bindings: &FxHashMap<String, usize>,
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let safe = match &fn_ir.values[vid].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } => true,
            ValueKind::Load { var } => {
                Self::is_typed_parallel_safe_load(fn_ir, var, bindings, seen)
            }
            ValueKind::Unary { rhs, .. } => {
                Self::is_typed_parallel_safe_value(fn_ir, *rhs, bindings, seen)
            }
            ValueKind::Binary { op, lhs, rhs } => {
                Self::is_typed_parallel_safe_binop(*op)
                    && Self::is_typed_parallel_safe_value(fn_ir, *lhs, bindings, seen)
                    && Self::is_typed_parallel_safe_value(fn_ir, *rhs, bindings, seen)
            }
            ValueKind::Intrinsic { op, args } => {
                Self::is_typed_parallel_safe_intrinsic(*op)
                    && args
                        .iter()
                        .all(|arg| Self::is_typed_parallel_safe_value(fn_ir, *arg, bindings, seen))
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => Self::is_typed_parallel_safe_call(fn_ir, callee, args, names, bindings, seen),
            ValueKind::Phi { .. }
            | ValueKind::Len { .. }
            | ValueKind::Indices { .. }
            | ValueKind::Range { .. }
            | ValueKind::Index1D { .. }
            | ValueKind::Index2D { .. }
            | ValueKind::Index3D { .. }
            | ValueKind::RSymbol { .. } => false,
        };
        seen.remove(&vid);
        safe
    }

    fn is_typed_parallel_safe_load(
        fn_ir: &FnIR,
        var: &str,
        bindings: &FxHashMap<String, usize>,
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if fn_ir.params.iter().any(|param| param == var) {
            return true;
        }
        bindings
            .get(var)
            .is_some_and(|src| Self::is_typed_parallel_safe_value(fn_ir, *src, bindings, seen))
    }

    fn is_typed_parallel_safe_binop(op: BinOp) -> bool {
        !matches!(op, BinOp::MatMul)
    }

    fn is_typed_parallel_safe_intrinsic(op: IntrinsicOp) -> bool {
        matches!(
            op,
            IntrinsicOp::VecAddF64
                | IntrinsicOp::VecSubF64
                | IntrinsicOp::VecMulF64
                | IntrinsicOp::VecDivF64
                | IntrinsicOp::VecAbsF64
                | IntrinsicOp::VecLogF64
                | IntrinsicOp::VecSqrtF64
                | IntrinsicOp::VecPmaxF64
                | IntrinsicOp::VecPminF64
        )
    }

    fn is_typed_parallel_safe_call(
        fn_ir: &FnIR,
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        bindings: &FxHashMap<String, usize>,
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if names.iter().any(|name| name.is_some()) {
            return false;
        }
        if !matches!(callee, "abs" | "log" | "sqrt" | "pmax" | "pmin") {
            return false;
        }
        args.iter()
            .all(|arg| Self::is_typed_parallel_safe_value(fn_ir, *arg, bindings, seen))
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
                    if !matches!(&values[*src].kind, ValueKind::Load { var } if var != dst) {
                        self.bind_value_to_var(*src, dst);
                    }
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
                let base_val = self.resolve_mutable_base(*base, values, params);
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
                let base_val = self.resolve_mutable_base(*base, values, params);
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
            Instr::StoreIndex3D {
                base,
                i,
                j,
                k,
                val,
                span,
            } => {
                self.emit_mark(*span, Some("store3d"));
                self.record_span(*span);
                let base_val = self.resolve_mutable_base(*base, values, params);
                let i_val = self.resolve_val(*i, values, params, false);
                let j_val = self.resolve_val(*j, values, params, false);
                let k_val = self.resolve_val(*k, values, params, false);
                let src_val = self.resolve_val(*val, values, params, false);
                let i_idx = if Self::can_elide_index_wrapper(*i, values) {
                    i_val
                } else {
                    format!("rr_index1_write({}, \"dim1\")", i_val)
                };
                let j_idx = if Self::can_elide_index_wrapper(*j, values) {
                    j_val
                } else {
                    format!("rr_index1_write({}, \"dim2\")", j_val)
                };
                let k_idx = if Self::can_elide_index_wrapper(*k, values) {
                    k_val
                } else {
                    format!("rr_index1_write({}, \"dim3\")", k_val)
                };
                self.write_stmt(&format!(
                    "{}[{}, {}, {}] <- {}",
                    base_val, i_idx, j_idx, k_idx, src_val
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

    fn resolve_mutable_base(&self, val_id: usize, values: &[Value], params: &[String]) -> String {
        if let Some(bound) = self.resolve_bound_value(val_id) {
            return bound;
        }
        if let Some(origin_var) = values[val_id].origin_var.as_ref() {
            return origin_var.clone();
        }
        self.resolve_val(val_id, values, params, false)
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
            ValueKind::Index3D { base, i, j, k } => {
                visit(*base);
                visit(*i);
                visit(*j);
                visit(*k);
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
            // Keep hoisted temps local to the statement that emitted them.
            self.note_var_write(&temp);
        }
        self.emitted_temp_names_scratch = temps;
    }

    fn prune_dead_cse_temps(output: &mut String) {
        let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
        if lines.is_empty() {
            return;
        }

        loop {
            let temp_defs: Vec<(usize, String, String)> = lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| {
                    let (name, indent) = Self::extract_cse_assign_name(line)?;
                    Some((idx, name, indent))
                })
                .collect();
            if temp_defs.is_empty() {
                break;
            }

            let mut changed = false;
            for (idx, name, indent) in temp_defs {
                let is_live = lines.iter().enumerate().any(|(other_idx, line)| {
                    other_idx != idx && Self::line_contains_symbol(line, &name)
                });
                if !is_live {
                    lines[idx] = format!("{}# rr-cse-pruned", indent);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let mut rebuilt = lines.join("\n");
        rebuilt.push('\n');
        *output = rebuilt;
    }

    fn extract_cse_assign_name(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with(".__rr_cse_") {
            return None;
        }
        let (name, _) = trimmed.split_once(" <- ")?;
        Some((
            name.to_string(),
            line[..line.len() - trimmed.len()].to_string(),
        ))
    }

    fn line_contains_symbol(line: &str, symbol: &str) -> bool {
        let mut search_from = 0;
        while let Some(rel_idx) = line[search_from..].find(symbol) {
            let idx = search_from + rel_idx;
            let before = line[..idx].chars().next_back();
            let after = line[idx + symbol.len()..].chars().next();
            let boundary_ok = before.is_none_or(|ch| !Self::is_symbol_char(ch))
                && after.is_none_or(|ch| !Self::is_symbol_char(ch));
            if boundary_ok {
                return true;
            }
            search_from = idx + symbol.len();
        }
        false
    }

    fn is_symbol_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
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
                | ValueKind::Index3D { .. }
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
            && matches!(val.kind, ValueKind::Load { .. } | ValueKind::Param { .. });
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
            ValueKind::Index3D { base, i, j, k } => {
                self.resolve_index3d_expr(*base, *i, *j, *k, values, params)
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

    fn resolve_index3d_expr(
        &self,
        base: usize,
        i: usize,
        j: usize,
        k: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let b = self.resolve_val(base, values, params, false);
        let i_val = self.resolve_val(i, values, params, false);
        let j_val = self.resolve_val(j, values, params, false);
        let k_val = self.resolve_val(k, values, params, false);
        let i_idx = if Self::can_elide_index_wrapper(i, values) {
            i_val
        } else {
            format!("rr_index1_write({}, \"dim1\")", i_val)
        };
        let j_idx = if Self::can_elide_index_wrapper(j, values) {
            j_val
        } else {
            format!("rr_index1_write({}, \"dim2\")", j_val)
        };
        let k_idx = if Self::can_elide_index_wrapper(k, values) {
            k_val
        } else {
            format!("rr_index1_write({}, \"dim3\")", k_val)
        };
        format!("{}[{}, {}, {}]", b, i_idx, j_idx, k_idx)
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
        let _ = label;
        self.write(&format!(
            "rr_mark({}, {});",
            span.start_line, span.start_col
        ));
        self.newline();
    }
}

#[cfg(test)]
mod tests {
    use super::RBackend;
    use crate::mir::def::{FnIR, Instr, Terminator, ValueKind};
    use crate::mir::flow::Facts;
    use crate::typeck::{NaTy, PrimTy, ShapeTy, TypeState, TypeTerm};
    use crate::utils::Span;

    #[test]
    fn prune_dead_cse_temps_removes_unused_chain() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  .__rr_cse_1 <- (a + b)",
            "  .__rr_cse_2 <- (.__rr_cse_1 * c)",
            "  x <- 1",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(!output.contains(".__rr_cse_1 <-"));
        assert!(!output.contains(".__rr_cse_2 <-"));
        assert!(output.contains("# rr-cse-pruned"));
    }

    #[test]
    fn prune_dead_cse_temps_keeps_live_temp() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  .__rr_cse_1 <- (a + b)",
            "  x <- .__rr_cse_1",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(output.contains(".__rr_cse_1 <- (a + b)"));
        assert!(!output.contains("# rr-cse-pruned"));
    }

    #[test]
    fn invalidate_emitted_cse_temps_drops_stale_binding() {
        let mut backend = RBackend::new();
        backend.note_var_write(".__rr_cse_7");
        backend.bind_value_to_var(7, ".__rr_cse_7");
        backend
            .emitted_temp_names_scratch
            .push(".__rr_cse_7".to_string());

        assert_eq!(
            backend.resolve_bound_value(7).as_deref(),
            Some(".__rr_cse_7")
        );

        backend.invalidate_emitted_cse_temps();

        assert!(backend.resolve_bound_value(7).is_none());
        assert!(backend.emitted_temp_names_scratch.is_empty());
    }

    #[test]
    fn typed_parallel_wrapper_tracks_vector_local_back_to_param_slot() {
        let mut fn_ir = FnIR::new("scale".to_string(), vec!["a".to_string()]);
        fn_ir.ret_term_hint = Some(TypeTerm::Vector(Box::new(TypeTerm::Double)));
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let param = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::dummy(),
            Facts::empty(),
            Some("a".to_string()),
        );
        let load_v = fn_ir.add_value(
            ValueKind::Load {
                var: "v".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("v".to_string()),
        );
        fn_ir.values[load_v].value_ty = TypeState {
            prim: PrimTy::Double,
            shape: ShapeTy::Vector,
            na: NaTy::Maybe,
            len_sym: None,
        };
        fn_ir.values[load_v].value_term = TypeTerm::Vector(Box::new(TypeTerm::Double));

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "v".to_string(),
            src: param,
            span: Span::dummy(),
        });
        fn_ir.blocks[entry].term = Terminator::Return(Some(load_v));

        let plan =
            RBackend::typed_parallel_wrapper_plan(&fn_ir).expect("wrapper plan should exist");
        assert_eq!(plan.slice_param_slots, vec![0]);
    }
}
