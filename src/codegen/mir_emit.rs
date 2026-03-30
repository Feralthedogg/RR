use crate::error::RR;
use crate::mir::def::{
    BinOp, BlockId, FnIR, Instr, IntrinsicOp, Lit, Terminator, UnaryOp, Value, ValueKind,
};
use crate::mir::flow::Facts;
use crate::mir::structurizer::{StructuredBlock, Structurizer};
use crate::typeck::{LenSym, PrimTy, ShapeTy, TypeTerm};
use crate::utils::Span;
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

const IDENT_PATTERN: &str = r"(?:[A-Za-z_][A-Za-z0-9._]*|\.[A-Za-z_][A-Za-z0-9._]*)";

fn compile_regex(pattern: String) -> Option<Regex> {
    Regex::new(&pattern).ok()
}

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
        Self::with_options(FxHashSet::default(), true)
    }

    pub fn with_fresh_result_calls(known_fresh_result_calls: FxHashSet<String>) -> Self {
        Self::with_options(known_fresh_result_calls, true)
    }

    pub fn with_options(
        known_fresh_result_calls: FxHashSet<String>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_analysis_options(
            known_fresh_result_calls,
            FxHashMap::default(),
            direct_builtin_vector_math,
        )
    }

    pub fn with_analysis_options(
        known_fresh_result_calls: FxHashSet<String>,
        seq_len_param_end_slots_by_fn: FxHashMap<String, FxHashMap<usize, usize>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_shared_analysis_options(
            Arc::new(known_fresh_result_calls),
            Arc::new(seq_len_param_end_slots_by_fn),
            direct_builtin_vector_math,
        )
    }

    pub fn with_shared_analysis_options(
        known_fresh_result_calls: Arc<FxHashSet<String>>,
        seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self {
            backend: RBackend::with_shared_analysis_options(
                known_fresh_result_calls,
                seq_len_param_end_slots_by_fn,
                direct_builtin_vector_math,
            ),
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

#[derive(Debug)]
struct VarValueBindingUndo {
    var: String,
    prev: Option<(usize, u64)>,
}

#[derive(Debug)]
struct LastAssignedValueUndo {
    var: String,
    prev: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct BranchSnapshot {
    value_binding_log_len: usize,
    var_version_log_len: usize,
    var_value_binding_log_len: usize,
    last_assigned_value_log_len: usize,
}

#[derive(Debug, Clone)]
struct TypedParallelWrapperPlan {
    impl_name: String,
    slice_param_slots: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScalarLoopCmp {
    Lt,
    Le,
}

#[derive(Clone, Debug)]
struct ActiveScalarLoopIndex {
    var: String,
    start_min: i64,
    cmp: ScalarLoopCmp,
}

pub struct RBackend {
    output: String,
    indent: usize,
    current_line: u32,
    current_fn_name: String,
    pub source_map: Vec<MapEntry>,
    // Codegen-time binding: ValueId -> (var name, var version at bind time).
    value_bindings: FxHashMap<usize, (String, u64)>,
    // Per-variable write version used to invalidate stale bindings.
    var_versions: FxHashMap<String, u64>,
    // Reverse binding: variable -> (ValueId, variable version at bind time).
    var_value_bindings: FxHashMap<String, (usize, u64)>,
    last_assigned_value_ids: FxHashMap<String, usize>,
    value_binding_log: Vec<ValueBindingUndo>,
    var_version_log: Vec<VarVersionUndo>,
    var_value_binding_log: Vec<VarValueBindingUndo>,
    last_assigned_value_log: Vec<LastAssignedValueUndo>,
    branch_snapshot_depth: usize,
    expr_use_counts_scratch: FxHashMap<usize, usize>,
    expr_path_scratch: FxHashSet<usize>,
    emitted_ids_scratch: FxHashSet<usize>,
    emitted_temp_names_scratch: Vec<String>,
    recent_whole_assign_bases: FxHashSet<String>,
    known_full_end_exprs: FxHashMap<String, String>,
    len_sym_end_exprs: FxHashMap<LenSym, String>,
    active_loop_known_full_end_exprs: Vec<FxHashMap<String, String>>,
    active_loop_mutated_vars: Vec<FxHashSet<String>>,
    active_scalar_loop_indices: Vec<ActiveScalarLoopIndex>,
    known_fresh_result_calls: Arc<FxHashSet<String>>,
    seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
    current_seq_len_param_end_slots: FxHashMap<usize, usize>,
    direct_builtin_vector_math: bool,
}

impl Default for RBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RBackend {
    pub fn new() -> Self {
        Self::with_options(FxHashSet::default(), true)
    }

    pub fn with_fresh_result_calls(known_fresh_result_calls: FxHashSet<String>) -> Self {
        Self::with_options(known_fresh_result_calls, true)
    }

    pub fn with_options(
        known_fresh_result_calls: FxHashSet<String>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_analysis_options(
            known_fresh_result_calls,
            FxHashMap::default(),
            direct_builtin_vector_math,
        )
    }

    pub fn with_analysis_options(
        known_fresh_result_calls: FxHashSet<String>,
        seq_len_param_end_slots_by_fn: FxHashMap<String, FxHashMap<usize, usize>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_shared_analysis_options(
            Arc::new(known_fresh_result_calls),
            Arc::new(seq_len_param_end_slots_by_fn),
            direct_builtin_vector_math,
        )
    }

    pub fn with_shared_analysis_options(
        known_fresh_result_calls: Arc<FxHashSet<String>>,
        seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            current_line: 1,
            current_fn_name: String::new(),
            source_map: Vec::new(),
            value_bindings: FxHashMap::default(),
            var_versions: FxHashMap::default(),
            var_value_bindings: FxHashMap::default(),
            last_assigned_value_ids: FxHashMap::default(),
            value_binding_log: Vec::new(),
            var_version_log: Vec::new(),
            var_value_binding_log: Vec::new(),
            last_assigned_value_log: Vec::new(),
            branch_snapshot_depth: 0,
            expr_use_counts_scratch: FxHashMap::default(),
            expr_path_scratch: FxHashSet::default(),
            emitted_ids_scratch: FxHashSet::default(),
            emitted_temp_names_scratch: Vec::new(),
            recent_whole_assign_bases: FxHashSet::default(),
            known_full_end_exprs: FxHashMap::default(),
            len_sym_end_exprs: FxHashMap::default(),
            active_loop_known_full_end_exprs: Vec::new(),
            active_loop_mutated_vars: Vec::new(),
            active_scalar_loop_indices: Vec::new(),
            known_fresh_result_calls,
            seq_len_param_end_slots_by_fn,
            current_seq_len_param_end_slots: FxHashMap::default(),
            direct_builtin_vector_math,
        }
    }

    fn in_active_loop_mutated_context(&self, var: &str) -> bool {
        self.active_loop_mutated_vars
            .iter()
            .rev()
            .any(|vars| vars.contains(var))
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
        self.var_value_bindings.clear();
        self.last_assigned_value_ids.clear();
        self.value_binding_log.clear();
        self.var_version_log.clear();
        self.var_value_binding_log.clear();
        self.last_assigned_value_log.clear();
        self.branch_snapshot_depth = 0;
        self.expr_use_counts_scratch.clear();
        self.expr_path_scratch.clear();
        self.emitted_ids_scratch.clear();
        self.emitted_temp_names_scratch.clear();
        self.recent_whole_assign_bases.clear();
        self.known_full_end_exprs.clear();
        self.len_sym_end_exprs.clear();
        self.active_loop_known_full_end_exprs.clear();
        self.active_loop_mutated_vars.clear();
        self.active_scalar_loop_indices.clear();
        self.current_seq_len_param_end_slots.clear();

        let wrapper_plan = Self::typed_parallel_wrapper_plan(fn_ir);
        if let Some(plan) = wrapper_plan.as_ref() {
            self.emit_function_named(fn_ir, &plan.impl_name)?;
            self.newline();
            self.emit_typed_parallel_wrapper(fn_ir, plan);
        } else {
            self.emit_function_named(fn_ir, fn_ir.name.as_str())?;
        }
        Self::rewrite_safe_scalar_loop_index_helpers(&mut self.output);
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
        self.current_fn_name = fn_ir.name.clone();
        self.value_bindings.clear();
        self.var_versions.clear();
        self.var_value_bindings.clear();
        self.last_assigned_value_ids.clear();
        self.value_binding_log.clear();
        self.var_version_log.clear();
        self.var_value_binding_log.clear();
        self.last_assigned_value_log.clear();
        self.branch_snapshot_depth = 0;
        self.expr_use_counts_scratch.clear();
        self.expr_path_scratch.clear();
        self.emitted_ids_scratch.clear();
        self.emitted_temp_names_scratch.clear();
        self.recent_whole_assign_bases.clear();
        self.known_full_end_exprs.clear();
        self.len_sym_end_exprs.clear();
        self.active_loop_known_full_end_exprs.clear();
        self.active_loop_mutated_vars.clear();
        self.active_scalar_loop_indices.clear();
        self.current_seq_len_param_end_slots = self
            .seq_len_param_end_slots_by_fn
            .get(fn_ir.name.as_str())
            .cloned()
            .unwrap_or_default();

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
        if !Self::typed_parallel_returns_slice_like(fn_ir) {
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

    fn typed_parallel_returns_slice_like(fn_ir: &FnIR) -> bool {
        matches!(
            fn_ir.ret_term_hint.as_ref(),
            Some(TypeTerm::Vector(_))
                | Some(TypeTerm::VectorLen(_, _))
                | Some(TypeTerm::Matrix(_))
                | Some(TypeTerm::ArrayDim(_, _))
        ) || matches!(
            fn_ir.inferred_ret_term,
            TypeTerm::Vector(_)
                | TypeTerm::VectorLen(_, _)
                | TypeTerm::Matrix(_)
                | TypeTerm::ArrayDim(_, _)
        )
    }

    fn typed_parallel_slice_param_slots(
        fn_ir: &FnIR,
        bindings: &FxHashMap<String, usize>,
    ) -> Vec<usize> {
        let mut slots = Vec::new();
        for idx in 0..fn_ir.params.len() {
            if Self::typed_parallel_param_is_slice_like(fn_ir, idx, bindings) {
                slots.push(idx);
            }
        }
        slots
    }

    fn typed_parallel_param_is_slice_like(
        fn_ir: &FnIR,
        idx: usize,
        bindings: &FxHashMap<String, usize>,
    ) -> bool {
        if fn_ir
            .param_ty_hints
            .get(idx)
            .is_some_and(|ty| matches!(ty.shape, ShapeTy::Vector | ShapeTy::Matrix))
        {
            return true;
        }
        if matches!(
            fn_ir.param_term_hints.get(idx),
            Some(TypeTerm::Vector(_))
                | Some(TypeTerm::VectorLen(_, _))
                | Some(TypeTerm::Matrix(_))
                | Some(TypeTerm::ArrayDim(_, _))
        ) {
            return true;
        }
        fn_ir.values.iter().any(|value| {
            matches!(value.kind, ValueKind::Param { index } if index == idx)
                && (matches!(value.value_ty.shape, ShapeTy::Vector | ShapeTy::Matrix)
                    || matches!(
                        value.value_term,
                        TypeTerm::Vector(_)
                            | TypeTerm::VectorLen(_, _)
                            | TypeTerm::Matrix(_)
                            | TypeTerm::ArrayDim(_, _)
                    ))
        }) || fn_ir.values.iter().any(|value| {
            (matches!(value.value_ty.shape, ShapeTy::Vector | ShapeTy::Matrix)
                || matches!(
                    value.value_term,
                    TypeTerm::Vector(_)
                        | TypeTerm::VectorLen(_, _)
                        | TypeTerm::Matrix(_)
                        | TypeTerm::ArrayDim(_, _)
                ))
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
                if let Some(partial_slice_stmt) =
                    self.try_render_constant_safe_partial_self_assign(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&partial_slice_stmt);
                    self.note_var_write(dst);
                    self.invalidate_var_binding(dst);
                    self.last_assigned_value_ids.remove(dst);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(row_slice_stmt) =
                    self.try_render_safe_idx_cube_row_slice_assign(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&row_slice_stmt);
                    self.note_var_write(dst);
                    self.invalidate_var_binding(dst);
                    self.last_assigned_value_ids.remove(dst);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(whole_range_rhs) =
                    self.try_resolve_whole_range_self_assign_rhs(dst, *src, values, params)
                {
                    if whole_range_rhs == *dst {
                        self.invalidate_emitted_cse_temps();
                        return Ok(());
                    }
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {whole_range_rhs}"));
                    self.note_var_write(dst);
                    self.recent_whole_assign_bases.insert(dst.clone());
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.last_assigned_value_ids.insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(whole_range_rhs) =
                    self.try_resolve_whole_range_call_map_rhs(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {whole_range_rhs}"));
                    self.note_var_write(dst);
                    self.recent_whole_assign_bases.insert(dst.clone());
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.last_assigned_value_ids.insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(whole_range_rhs) =
                    self.try_resolve_whole_auto_call_map_rhs(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {whole_range_rhs}"));
                    self.note_var_write(dst);
                    self.recent_whole_assign_bases.insert(dst.clone());
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.last_assigned_value_ids.insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                let preserve_loop_seed = self.in_active_loop_mutated_context(dst);
                let same_origin_self_assign = values[*src].origin_var.as_deref()
                    == Some(dst.as_str())
                    && self.resolve_bound_value_id(dst) != Some(*src);
                let stale_origin_probe = if same_origin_self_assign {
                    None
                } else {
                    self.resolve_stale_origin_var(*src, &values[*src], values)
                };
                let bound_probe = if same_origin_self_assign {
                    None
                } else {
                    self.resolve_bound_value(*src)
                };
                let mutated_whole_range_copy_probe =
                    self.try_resolve_mutated_whole_range_copy_alias(*src, values, params);
                let allow_last_assigned_skip = !matches!(
                    values[*src].kind,
                    ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
                );
                let stale_self_fresh_replay = !preserve_loop_seed
                    && self.is_fresh_mutable_aggregate_value(&values[*src])
                    && values[*src].origin_var.as_deref() == Some(dst.as_str())
                    && self.resolve_bound_value_id(dst) != Some(*src)
                    && (self
                        .value_bindings
                        .get(src)
                        .is_some_and(|(bound_var, version)| {
                            bound_var == dst && self.current_var_version(dst) != *version
                        })
                        || self.resolve_bound_value_id(dst).is_some_and(|current| {
                            !self.is_fresh_mutable_aggregate_value(&values[current])
                        }));
                let stale_same_origin_fresh_without_live_binding = !preserve_loop_seed
                    && self.is_fresh_mutable_aggregate_value(&values[*src])
                    && values[*src].origin_var.as_deref() == Some(dst.as_str())
                    && self.resolve_bound_value_id(dst).is_none()
                    && self.current_var_version(dst) > 0;
                if stale_self_fresh_replay {
                    if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
                        let binding = self.value_bindings.get(src).cloned();
                        eprintln!(
                            "RR_DEBUG_EMIT_ASSIGN skip=stale_self_fresh_replay fn={} dst={} src={} kind={:?} current_bound={:?} origin={:?} binding={:?} current_version={}",
                            self.current_fn_name,
                            dst,
                            src,
                            values[*src].kind,
                            self.resolve_bound_value_id(dst),
                            values[*src].origin_var,
                            binding,
                            self.current_var_version(dst),
                        );
                    }
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if stale_same_origin_fresh_without_live_binding {
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if allow_last_assigned_skip
                    && let Some(prev_src) = self.last_assigned_value_ids.get(dst).copied()
                    && values[prev_src].kind == values[*src].kind
                {
                    if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
                        eprintln!(
                            "RR_DEBUG_EMIT_ASSIGN skip=last_assigned_same_kind fn={} dst={} src={} prev_src={} kind={:?}",
                            self.current_fn_name, dst, src, prev_src, values[*src].kind,
                        );
                    }
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if !preserve_loop_seed {
                    if matches!(
                        values[*src].kind,
                        ValueKind::Call { ref callee, .. }
                            if self.call_is_known_fresh_allocation(callee)
                    ) && values[*src].origin_var.as_deref() == Some(dst.as_str())
                        && self
                            .resolve_stale_origin_var(*src, &values[*src], values)
                            .as_deref()
                            == Some(dst.as_str())
                        && self
                            .resolve_bound_value_id(dst)
                            .is_some_and(|current_val_id| {
                                values[current_val_id].kind == values[*src].kind
                            })
                    {
                        self.invalidate_emitted_cse_temps();
                        return Ok(());
                    }
                    if self.resolve_bound_value_id(dst) == Some(*src) {
                        self.invalidate_emitted_cse_temps();
                        return Ok(());
                    }
                    if let Some(current_val_id) = self.resolve_bound_value_id(dst)
                        && current_val_id != *src
                    {
                        if values[current_val_id].kind == values[*src].kind {
                            self.invalidate_emitted_cse_temps();
                            return Ok(());
                        }
                        let src_expr = self.resolve_val(*src, values, params, true);
                        let current_expr = self.resolve_val(current_val_id, values, params, true);
                        if src_expr == current_expr {
                            self.invalidate_emitted_cse_temps();
                            return Ok(());
                        }
                    }
                }
                let v = if let Some(alias_var) = mutated_whole_range_copy_probe.clone() {
                    alias_var
                } else if !matches!(
                    values[*src].kind,
                    ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
                ) && let Some(origin_var) = stale_origin_probe.clone()
                {
                    origin_var
                } else if !matches!(values[*src].kind, ValueKind::Const(_))
                    && let Some(bound) = bound_probe.clone()
                {
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
                let v =
                    self.rewrite_known_one_based_full_range_alias_reads(v.as_str(), values, params);
                if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
                    eprintln!(
                        "RR_DEBUG_EMIT_ASSIGN fn={} dst={} src={} kind={:?} rendered={} skip={}",
                        self.current_fn_name,
                        dst,
                        src,
                        values[*src].kind,
                        v,
                        v == *dst
                    );
                }
                if v != *dst {
                    self.record_span(*span);
                    self.write_stmt(&format!("{} <- {}", dst, v));
                    self.note_var_write(dst);
                    self.recent_whole_assign_bases.insert(dst.clone());
                    if !matches!(&values[*src].kind, ValueKind::Load { var } if var != dst) {
                        self.bind_value_to_var(*src, dst);
                    }
                    if !matches!(&values[*src].kind, ValueKind::Load { .. }) {
                        self.bind_var_to_value(dst, *src);
                    }
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.last_assigned_value_ids.insert(dst.clone(), *src);
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
                    if let Some(base_name) = Self::named_mutable_base_expr(
                        *base,
                        values,
                        &self.value_bindings,
                        &self.var_versions,
                    ) {
                        self.bind_value_to_var(*val, &base_name);
                        if !matches!(&values[*val].kind, ValueKind::Load { .. }) {
                            self.bind_var_to_value(&base_name, *val);
                        }
                        self.recent_whole_assign_bases.insert(base_name.clone());
                        self.remember_known_full_end_expr(&base_name, *val, values, params);
                    }
                } else {
                    let idx_elidable = self.can_elide_index_expr(*idx, values, params);
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
                let r_idx = if self.can_elide_index_expr(*r, values, params) {
                    r_val
                } else {
                    format!("rr_index1_write({}, \"row\")", r_val)
                };
                let c_idx = if self.can_elide_index_expr(*c, values, params) {
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
                let i_idx = if self.can_elide_index_expr(*i, values, params) {
                    i_val
                } else {
                    format!("rr_index1_write({}, \"dim1\")", i_val)
                };
                let j_idx = if self.can_elide_index_expr(*j, values, params) {
                    j_val
                } else {
                    format!("rr_index1_write({}, \"dim2\")", j_val)
                };
                let k_idx = if self.can_elide_index_expr(*k, values, params) {
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

    fn bind_var_to_value(&mut self, var: &str, val_id: usize) {
        let version = self.current_var_version(var);
        self.log_var_value_binding_change(var);
        self.var_value_bindings
            .insert(var.to_string(), (val_id, version));
    }

    fn resolve_bound_value(&self, val_id: usize) -> Option<String> {
        if let Some((var, version)) = self.value_bindings.get(&val_id)
            && self.current_var_version(var) == *version
        {
            return Some(var.clone());
        }
        None
    }

    fn resolve_bound_value_id(&self, var: &str) -> Option<usize> {
        self.var_value_bindings
            .get(var)
            .filter(|(_, version)| self.current_var_version(var) == *version)
            .map(|(val_id, _)| *val_id)
    }

    fn can_elide_index_expr(&self, idx: usize, values: &[Value], params: &[String]) -> bool {
        if Self::can_elide_index_wrapper(idx, values) {
            return true;
        }
        for ctx in self.active_scalar_loop_indices.iter().rev() {
            if self
                .loop_index_offset(idx, ctx, values, &mut FxHashSet::default())
                .is_some_and(|offset| Self::loop_context_allows_offset(ctx, offset))
            {
                return true;
            }
        }
        let rendered = self.resolve_val(idx, values, params, false);
        for ctx in self.active_scalar_loop_indices.iter().rev() {
            if Self::rendered_loop_index_offset(&rendered, ctx)
                .is_some_and(|offset| Self::loop_context_allows_offset(ctx, offset))
            {
                return true;
            }
        }
        self.resolve_bound_value_id(&rendered)
            .is_some_and(|bound| Self::can_elide_index_wrapper(bound, values))
    }

    fn resolve_temp_bound_value_id(&self, var: &str) -> Option<usize> {
        self.resolve_bound_value_id(var).or_else(|| {
            (var.starts_with(".__rr_cse_") || var.starts_with(".tachyon_exprmap"))
                .then(|| self.var_value_bindings.get(var).map(|(val_id, _)| *val_id))
                .flatten()
        })
    }

    fn resolve_readonly_arg_alias_name(&self, var: &str, values: &[Value]) -> Option<String> {
        let stripped = var.strip_prefix(".arg_")?;
        if stripped.is_empty() || self.current_var_version(var) > 1 {
            return None;
        }
        let bound = self.resolve_temp_bound_value_id(var)?;
        matches!(
            values.get(bound).map(|v| &v.kind),
            Some(ValueKind::Param { .. })
        )
        .then(|| stripped.to_string())
    }

    fn rewrite_live_readonly_arg_aliases(&self, expr: String, values: &[Value]) -> String {
        let mut out = expr;
        let mut aliases: Vec<(String, String)> = self
            .var_value_bindings
            .keys()
            .filter_map(|var| {
                self.resolve_readonly_arg_alias_name(var, values)
                    .map(|alias| (var.clone(), alias))
            })
            .collect();
        aliases.sort_by(|(lhs_a, _), (lhs_b, _)| lhs_b.len().cmp(&lhs_a.len()));
        for (from, to) in aliases {
            let Some(re) = compile_regex(format!(r"\b{}\b", regex::escape(&from))) else {
                continue;
            };
            out = re.replace_all(&out, to.as_str()).to_string();
        }
        out
    }

    fn known_full_end_expr_for_var(&self, var: &str) -> Option<&str> {
        self.known_full_end_exprs
            .get(var)
            .map(String::as_str)
            .or_else(|| {
                self.active_loop_known_full_end_exprs
                    .iter()
                    .rev()
                    .find_map(|frame| frame.get(var).map(String::as_str))
            })
    }

    fn remember_known_full_end_expr(
        &mut self,
        var: &str,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) {
        if let Some(end_expr) = self.known_full_end_expr_for_value(val_id, values, params) {
            self.known_full_end_exprs
                .insert(var.to_string(), end_expr.clone());
            if let Some(sym) = values.get(val_id).and_then(|value| value.value_ty.len_sym) {
                self.len_sym_end_exprs.insert(sym, end_expr);
            }
        } else {
            self.known_full_end_exprs.remove(var);
        }
    }

    fn known_full_end_expr_for_value(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        self.known_full_end_expr_for_value_impl(val_id, values, params, &mut FxHashSet::default())
    }

    fn resolve_known_full_end_expr_with_seen(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> Option<String> {
        self.known_full_end_expr_for_value_impl(val_id, values, params, seen)
            .or_else(|| {
                let rendered =
                    self.resolve_bound_temp_expr(val_id, values, params, &mut FxHashSet::default());
                (!rendered.is_empty()).then_some(rendered)
            })
    }

    fn known_full_end_expr_for_value_impl(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> Option<String> {
        if !seen.insert(val_id) {
            return None;
        }
        let value = values.get(val_id)?;
        match &value.kind {
            ValueKind::Param { index } => self
                .current_seq_len_param_end_slots
                .get(index)
                .map(|end_index| self.resolve_param(*end_index, params))
                .or_else(|| Some(self.resolve_param(*index, params))),
            ValueKind::Load { var } => self
                .resolve_bound_value_id(var)
                .and_then(|bound| {
                    self.known_full_end_expr_for_value_impl(bound, values, params, seen)
                })
                .or_else(|| self.known_full_end_expr_for_var(var).map(str::to_string)),
            ValueKind::Len { base } => {
                self.known_full_end_expr_for_value_impl(*base, values, params, seen)
            }
            ValueKind::Call { callee, args, .. }
                if self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .is_some() =>
            {
                let len_idx = self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .unwrap_or(0);
                self.resolve_known_full_end_expr_with_seen(args[len_idx], values, params, seen)
            }
            ValueKind::Call { callee, args, .. }
                if callee == "rr_assign_slice" && args.len() >= 4 =>
            {
                if self.value_is_known_one(args[1], values) {
                    self.resolve_known_full_end_expr_with_seen(args[2], values, params, seen)
                } else {
                    None
                }
            }
            ValueKind::Call { callee, args, .. }
                if callee == "rr_call_map_slice_auto" && args.len() >= 7 =>
            {
                if self.value_is_known_one(args[1], values) {
                    self.resolve_known_full_end_expr_with_seen(args[2], values, params, seen)
                } else {
                    None
                }
            }
            ValueKind::Call { callee, args, .. }
                if callee == "rr_call_map_whole_auto" && !args.is_empty() =>
            {
                Self::named_mutable_base_expr(
                    args[0],
                    values,
                    &self.value_bindings,
                    &self.var_versions,
                )
                .and_then(|var| {
                    self.known_full_end_expr_for_var(var.as_str())
                        .map(str::to_string)
                })
            }
            ValueKind::Binary { lhs, rhs, .. } if !self.value_is_scalar_shape(val_id, values) => {
                let lhs_end = self.known_full_end_expr_for_value_impl(*lhs, values, params, seen);
                let rhs_end = self.known_full_end_expr_for_value_impl(*rhs, values, params, seen);
                self.merge_known_full_end_exprs(lhs_end, rhs_end, *lhs, *rhs, values)
            }
            ValueKind::Unary { rhs, .. } if !self.value_is_scalar_shape(val_id, values) => {
                self.known_full_end_expr_for_value_impl(*rhs, values, params, seen)
            }
            ValueKind::Intrinsic { op, args } if !self.value_is_scalar_shape(val_id, values) => {
                match (op, args.as_slice()) {
                    (
                        IntrinsicOp::VecAddF64
                        | IntrinsicOp::VecSubF64
                        | IntrinsicOp::VecMulF64
                        | IntrinsicOp::VecDivF64
                        | IntrinsicOp::VecPmaxF64
                        | IntrinsicOp::VecPminF64,
                        [lhs, rhs],
                    ) => {
                        let lhs_end =
                            self.known_full_end_expr_for_value_impl(*lhs, values, params, seen);
                        let rhs_end =
                            self.known_full_end_expr_for_value_impl(*rhs, values, params, seen);
                        self.merge_known_full_end_exprs(lhs_end, rhs_end, *lhs, *rhs, values)
                    }
                    (
                        IntrinsicOp::VecAbsF64
                        | IntrinsicOp::VecLogF64
                        | IntrinsicOp::VecSqrtF64
                        | IntrinsicOp::VecSumF64
                        | IntrinsicOp::VecMeanF64,
                        [arg],
                    ) => self.known_full_end_expr_for_value_impl(*arg, values, params, seen),
                    _ => None,
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } if names.iter().all(|name| name.is_none())
                && !self.value_is_scalar_shape(val_id, values) =>
            {
                match (callee.as_str(), args.as_slice()) {
                    ("abs" | "log" | "sqrt" | "floor" | "ceiling" | "trunc", [arg]) => {
                        self.known_full_end_expr_for_value_impl(*arg, values, params, seen)
                    }
                    ("pmax" | "pmin", [lhs, rhs]) => {
                        let lhs_end =
                            self.known_full_end_expr_for_value_impl(*lhs, values, params, seen);
                        let rhs_end =
                            self.known_full_end_expr_for_value_impl(*rhs, values, params, seen);
                        self.merge_known_full_end_exprs(lhs_end, rhs_end, *lhs, *rhs, values)
                    }
                    _ => value
                        .value_ty
                        .len_sym
                        .and_then(|sym| self.len_sym_end_exprs.get(&sym).cloned()),
                }
            }
            _ => value
                .value_ty
                .len_sym
                .and_then(|sym| self.len_sym_end_exprs.get(&sym).cloned()),
        }
    }

    fn resolve_known_full_end_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        self.known_full_end_expr_for_value_impl(val_id, values, params, &mut FxHashSet::default())
            .or_else(|| {
                let rendered =
                    self.resolve_bound_temp_expr(val_id, values, params, &mut FxHashSet::default());
                (!rendered.is_empty()).then_some(rendered)
            })
    }

    fn fresh_allocation_len_arg_index(
        &self,
        callee: &str,
        args: &[usize],
        values: &[Value],
    ) -> Option<usize> {
        let argc = args.len();
        match callee {
            "numeric" | "seq_len" => Some(0),
            "rep.int" if argc >= 2 => Some(1),
            "vector" if argc >= 2 => Some(1),
            "vector" if argc >= 1 => Some(0),
            _ if self.known_fresh_result_calls.contains(callee)
                && argc == 3
                && self.value_can_be_allocator_scalar_arg(args[0], values)
                && self.value_can_be_allocator_scalar_arg(args[1], values)
                && matches!(self.const_int_value(args[2], values), Some(tag) if (0..=4).contains(&tag)) =>
            {
                Some(0)
            }
            _ => None,
        }
    }

    fn merge_known_full_end_exprs(
        &self,
        lhs_end: Option<String>,
        rhs_end: Option<String>,
        lhs: usize,
        rhs: usize,
        values: &[Value],
    ) -> Option<String> {
        match (lhs_end, rhs_end) {
            (Some(lhs_end), Some(rhs_end)) if lhs_end == rhs_end => Some(lhs_end),
            (Some(lhs_end), None)
                if !self.value_is_scalar_shape(lhs, values)
                    && self.value_is_scalar_shape(rhs, values) =>
            {
                Some(lhs_end)
            }
            (None, Some(rhs_end))
                if self.value_is_scalar_shape(lhs, values)
                    && !self.value_is_scalar_shape(rhs, values) =>
            {
                Some(rhs_end)
            }
            _ => None,
        }
    }

    fn value_is_scalar_shape(&self, value_id: usize, values: &[Value]) -> bool {
        values.get(value_id).is_some_and(|value| {
            value.value_ty.shape == ShapeTy::Scalar
                || value.facts.has(Facts::INT_SCALAR)
                || value.facts.has(Facts::BOOL_SCALAR)
                || matches!(value.kind, ValueKind::Const(_) | ValueKind::Len { .. })
        })
    }

    fn value_can_be_allocator_scalar_arg(&self, value_id: usize, values: &[Value]) -> bool {
        values.get(value_id).is_some_and(|value| {
            if self.value_is_scalar_shape(value_id, values) {
                return true;
            }
            !matches!(value.value_ty.shape, ShapeTy::Vector | ShapeTy::Matrix)
                && !matches!(
                    value.value_term,
                    TypeTerm::Vector(_)
                        | TypeTerm::VectorLen(_, _)
                        | TypeTerm::Matrix(_)
                        | TypeTerm::MatrixDim(_, _, _)
                        | TypeTerm::ArrayDim(_, _)
                        | TypeTerm::DataFrame(_)
                        | TypeTerm::DataFrameNamed(_)
                        | TypeTerm::NamedList(_)
                        | TypeTerm::List(_)
                        | TypeTerm::Boxed(_)
                        | TypeTerm::Union(_)
                )
        })
    }

    fn whole_dest_end_matches_known_var(
        &self,
        var: &str,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> bool {
        let end_rendered = self.resolve_val(end, values, params, false);
        let end_canonical = self.resolve_known_full_end_expr(end, values, params);
        self.known_full_end_expr_for_var(var)
            .is_some_and(|known| known == end_rendered || end_canonical.as_deref() == Some(known))
    }

    fn known_full_end_bound_for_var(&self, var: &str, values: &[Value]) -> Option<i64> {
        self.resolve_bound_value_id(var)
            .and_then(|bound| self.known_full_end_bound_for_value(bound, values))
    }

    fn known_full_end_bound_for_value(&self, val_id: usize, values: &[Value]) -> Option<i64> {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Call { callee, args, .. })
                if self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .is_some() =>
            {
                let len_idx = self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .unwrap_or(0);
                self.const_index_int_value(args[len_idx], values)
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_assign_slice" && args.len() >= 4 =>
            {
                self.const_index_int_value(args[2], values)
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_call_map_slice_auto" && args.len() >= 7 =>
            {
                self.const_index_int_value(args[2], values)
            }
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.known_full_end_bound_for_value(bound, values)),
            _ => None,
        }
    }

    fn invalidate_var_binding(&mut self, var: &str) {
        // Keep stale reverse bindings around so later codegen can still recover
        // "mutated descendant" aliases after loops or indexed stores. Live use
        // sites already gate on the tracked version, so leaving the stale entry
        // here does not make it a valid current binding again.
        self.recent_whole_assign_bases.remove(var);
    }

    fn invalidate_var_bindings<'a, I>(&mut self, vars: I)
    where
        I: IntoIterator<Item = &'a String>,
    {
        for var in vars {
            self.invalidate_var_binding(var);
        }
    }

    fn named_written_base(base: usize, values: &[Value]) -> Option<String> {
        if let Some(var) = values[base].origin_var.as_ref() {
            return Some(var.clone());
        }
        match &values[base].kind {
            ValueKind::Load { var } => Some(var.clone()),
            _ => None,
        }
    }

    fn collect_mutated_vars(node: &StructuredBlock, fn_ir: &FnIR, out: &mut FxHashSet<String>) {
        match node {
            StructuredBlock::Sequence(items) => {
                for item in items {
                    Self::collect_mutated_vars(item, fn_ir, out);
                }
            }
            StructuredBlock::BasicBlock(bid) => {
                for instr in &fn_ir.blocks[*bid].instrs {
                    match instr {
                        Instr::Assign { dst, .. } => {
                            out.insert(dst.clone());
                        }
                        Instr::StoreIndex1D { base, .. }
                        | Instr::StoreIndex2D { base, .. }
                        | Instr::StoreIndex3D { base, .. } => {
                            if let Some(var) = Self::named_written_base(*base, &fn_ir.values) {
                                out.insert(var);
                            }
                        }
                        Instr::Eval { .. } => {}
                    }
                }
            }
            StructuredBlock::If {
                then_body,
                else_body,
                ..
            } => {
                Self::collect_mutated_vars(then_body, fn_ir, out);
                if let Some(else_body) = else_body {
                    Self::collect_mutated_vars(else_body, fn_ir, out);
                }
            }
            StructuredBlock::Loop { header, body, .. } => {
                for instr in &fn_ir.blocks[*header].instrs {
                    match instr {
                        Instr::Assign { dst, .. } => {
                            out.insert(dst.clone());
                        }
                        Instr::StoreIndex1D { base, .. }
                        | Instr::StoreIndex2D { base, .. }
                        | Instr::StoreIndex3D { base, .. } => {
                            if let Some(var) = Self::named_written_base(*base, &fn_ir.values) {
                                out.insert(var);
                            }
                        }
                        Instr::Eval { .. } => {}
                    }
                }
                Self::collect_mutated_vars(body, fn_ir, out);
            }
            StructuredBlock::Break | StructuredBlock::Next | StructuredBlock::Return(_) => {}
        }
    }

    fn collect_loop_invariant_scalar_candidates_from_instrs(
        &self,
        instrs: &[Instr],
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
        visited: &mut FxHashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        for instr in instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *src,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
                Instr::StoreIndex1D { idx, val, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *idx,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *val,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
                Instr::StoreIndex2D { r, c, val, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *r,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *c,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *val,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
                Instr::StoreIndex3D { i, j, k, val, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *i,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *j,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *k,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *val,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
            }
        }
    }

    fn collect_loop_invariant_scalar_candidates_from_block(
        &self,
        node: &StructuredBlock,
        fn_ir: &FnIR,
        loop_mutated_vars: &FxHashSet<String>,
        visited: &mut FxHashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        match node {
            StructuredBlock::Sequence(items) => {
                for item in items {
                    self.collect_loop_invariant_scalar_candidates_from_block(
                        item,
                        fn_ir,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
            }
            StructuredBlock::BasicBlock(bid) => {
                self.collect_loop_invariant_scalar_candidates_from_instrs(
                    &fn_ir.blocks[*bid].instrs,
                    &fn_ir.values,
                    loop_mutated_vars,
                    visited,
                    out,
                );
                match fn_ir.blocks[*bid].term {
                    Terminator::If { cond, .. } | Terminator::Return(Some(cond)) => {
                        self.collect_loop_invariant_scalar_candidates_from_value(
                            cond,
                            &fn_ir.values,
                            loop_mutated_vars,
                            visited,
                            out,
                        );
                    }
                    _ => {}
                }
            }
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            } => {
                self.collect_loop_invariant_scalar_candidates_from_value(
                    *cond,
                    &fn_ir.values,
                    loop_mutated_vars,
                    visited,
                    out,
                );
                self.collect_loop_invariant_scalar_candidates_from_block(
                    then_body,
                    fn_ir,
                    loop_mutated_vars,
                    visited,
                    out,
                );
                if let Some(else_body) = else_body {
                    self.collect_loop_invariant_scalar_candidates_from_block(
                        else_body,
                        fn_ir,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
            }
            StructuredBlock::Loop { cond, body, .. } => {
                self.collect_loop_invariant_scalar_candidates_from_value(
                    *cond,
                    &fn_ir.values,
                    loop_mutated_vars,
                    visited,
                    out,
                );
                self.collect_loop_invariant_scalar_candidates_from_block(
                    body,
                    fn_ir,
                    loop_mutated_vars,
                    visited,
                    out,
                );
            }
            StructuredBlock::Break | StructuredBlock::Next | StructuredBlock::Return(_) => {}
        }
    }

    fn collect_loop_invariant_scalar_candidates_from_value(
        &self,
        val_id: usize,
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
        visited: &mut FxHashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        if !visited.insert(val_id) {
            return;
        }
        Self::for_each_expr_child(val_id, values, |child| {
            self.collect_loop_invariant_scalar_candidates_from_value(
                child,
                values,
                loop_mutated_vars,
                visited,
                out,
            );
        });
        if self.is_loop_invariant_scalar_expr_candidate(val_id, values, loop_mutated_vars) {
            out.push(val_id);
        }
    }

    fn is_loop_invariant_scalar_expr_candidate(
        &self,
        val_id: usize,
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
    ) -> bool {
        if !self.value_is_scalar_shape(val_id, values) {
            return false;
        }
        match values.get(val_id).map(|value| &value.kind) {
            Some(ValueKind::Unary { op, rhs }) => {
                !matches!(op, UnaryOp::Formula)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        &mut FxHashSet::default(),
                    )
            }
            Some(ValueKind::Binary { op, lhs, rhs }) => {
                !matches!(op, BinOp::MatMul)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *lhs,
                        values,
                        loop_mutated_vars,
                        &mut FxHashSet::default(),
                    )
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        &mut FxHashSet::default(),
                    )
            }
            _ => false,
        }
    }

    fn value_depends_only_on_loop_invariant_inputs(
        &self,
        val_id: usize,
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(val_id) {
            return true;
        }
        match values.get(val_id).map(|value| &value.kind) {
            Some(ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. }) => true,
            Some(ValueKind::Load { var }) => !loop_mutated_vars.contains(var),
            Some(ValueKind::Unary { op, rhs }) => {
                !matches!(op, UnaryOp::Formula)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        seen,
                    )
            }
            Some(ValueKind::Binary { op, lhs, rhs }) => {
                !matches!(op, BinOp::MatMul)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *lhs,
                        values,
                        loop_mutated_vars,
                        seen,
                    )
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        seen,
                    )
            }
            _ => false,
        }
    }

    fn emit_loop_invariant_scalar_hoists(
        &mut self,
        header: BlockId,
        cond: usize,
        body: &StructuredBlock,
        fn_ir: &FnIR,
        loop_mutated_vars: &FxHashSet<String>,
    ) {
        let mut candidates = Vec::new();
        let mut visited = FxHashSet::default();
        self.collect_loop_invariant_scalar_candidates_from_instrs(
            &fn_ir.blocks[header].instrs,
            &fn_ir.values,
            loop_mutated_vars,
            &mut visited,
            &mut candidates,
        );
        self.collect_loop_invariant_scalar_candidates_from_value(
            cond,
            &fn_ir.values,
            loop_mutated_vars,
            &mut visited,
            &mut candidates,
        );
        self.collect_loop_invariant_scalar_candidates_from_block(
            body,
            fn_ir,
            loop_mutated_vars,
            &mut visited,
            &mut candidates,
        );

        for val_id in candidates {
            if self.resolve_bound_value(val_id).is_some() {
                continue;
            }
            let temp_name = format!("licm_{val_id}");
            let expr = self.resolve_val(val_id, &fn_ir.values, &fn_ir.params, false);
            self.write_stmt(&format!("{temp_name} <- {expr}"));
            self.note_var_write(&temp_name);
            self.bind_value_to_var(val_id, &temp_name);
            self.bind_var_to_value(&temp_name, val_id);
        }
    }

    fn resolve_stale_origin_var(
        &self,
        val_id: usize,
        val: &Value,
        _values: &[Value],
    ) -> Option<String> {
        let is_self_update_call =
            matches!(&val.kind, ValueKind::Call { callee, .. } if callee == "rr_assign_slice");
        if let Some((bound_var, version)) = self.value_bindings.get(&val_id) {
            let current_version = self.current_var_version(bound_var);
            if *version != current_version {
                if is_self_update_call {
                    return None;
                }
                return Some(bound_var.clone());
            }
        }

        let origin_var = val.origin_var.as_ref()?;
        let current_version = self.current_var_version(origin_var);

        if let Some((current_val_id, version)) = self.var_value_bindings.get(origin_var)
            && *version == current_version
            && *current_val_id != val_id
        {
            if is_self_update_call {
                return None;
            }
            return Some(origin_var.clone());
        }

        if !is_self_update_call && current_version > 0 && self.is_fresh_mutable_aggregate_value(val)
        {
            return Some(origin_var.clone());
        }

        None
    }

    fn resolve_stale_fresh_clone_var(
        &self,
        val_id: usize,
        val: &Value,
        values: &[Value],
    ) -> Option<String> {
        if val.origin_var.is_some() || !self.is_fresh_mutable_aggregate_value(val) {
            return None;
        }
        let mut best: Option<(&str, usize)> = None;
        for (other_val_id, (var, version)) in &self.value_bindings {
            if *other_val_id == val_id {
                continue;
            }
            if self.current_var_version(var) == *version {
                continue;
            }
            let Some(other) = values.get(*other_val_id) else {
                continue;
            };
            if other.kind == val.kind {
                match best {
                    None => best = Some((var.as_str(), *other_val_id)),
                    Some((best_var, best_id))
                        if (var.as_str(), *other_val_id) < (best_var, best_id) =>
                    {
                        best = Some((var.as_str(), *other_val_id));
                    }
                    Some(_) => {}
                }
            }
        }
        best.map(|(var, _)| var.to_string())
    }

    fn call_is_known_fresh_allocation(&self, callee: &str) -> bool {
        matches!(
            callee,
            "rep.int" | "numeric" | "vector" | "matrix" | "seq_len"
        ) || self.known_fresh_result_calls.contains(callee)
    }

    fn is_fresh_mutable_aggregate_value(&self, val: &Value) -> bool {
        matches!(
            &val.kind,
            ValueKind::Call { callee, .. }
                if self.call_is_known_fresh_allocation(callee)
        )
    }

    fn should_prefer_stale_var_over_expr(val: &Value) -> bool {
        !matches!(val.value_ty.shape, ShapeTy::Scalar)
            || matches!(
                val.value_term,
                TypeTerm::Any
                    | TypeTerm::Vector(_)
                    | TypeTerm::VectorLen(_, _)
                    | TypeTerm::Matrix(_)
                    | TypeTerm::MatrixDim(_, _, _)
                    | TypeTerm::ArrayDim(_, _)
                    | TypeTerm::DataFrame(_)
                    | TypeTerm::DataFrameNamed(_)
                    | TypeTerm::NamedList(_)
                    | TypeTerm::List(_)
                    | TypeTerm::Boxed(_)
                    | TypeTerm::Option(_)
                    | TypeTerm::Union(_)
            )
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

    fn resolve_read_base(&self, val_id: usize, values: &[Value], params: &[String]) -> String {
        if let Some(bound) = self.resolve_bound_value(val_id) {
            return bound;
        }
        if let ValueKind::Call { callee, .. } = &values[val_id].kind
            && callee.contains("::")
            && let Some(origin_var) = values[val_id].origin_var.as_ref()
        {
            return origin_var.clone();
        }
        self.resolve_val(val_id, values, params, false)
    }

    fn begin_branch_snapshot(&mut self) -> BranchSnapshot {
        self.branch_snapshot_depth += 1;
        BranchSnapshot {
            value_binding_log_len: self.value_binding_log.len(),
            var_version_log_len: self.var_version_log.len(),
            var_value_binding_log_len: self.var_value_binding_log.len(),
            last_assigned_value_log_len: self.last_assigned_value_log.len(),
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
        while self.var_value_binding_log.len() > snapshot.var_value_binding_log_len {
            let Some(undo) = self.var_value_binding_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.var_value_bindings.insert(undo.var, prev);
            } else {
                self.var_value_bindings.remove(&undo.var);
            }
        }
        while self.last_assigned_value_log.len() > snapshot.last_assigned_value_log_len {
            let Some(undo) = self.last_assigned_value_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.last_assigned_value_ids.insert(undo.var, prev);
            } else {
                self.last_assigned_value_ids.remove(&undo.var);
            }
        }
    }

    fn end_branch_snapshot(&mut self) {
        if self.branch_snapshot_depth > 0 {
            self.branch_snapshot_depth -= 1;
        }
    }

    fn join_branch_var_value_bindings(
        &mut self,
        then_var_versions: &FxHashMap<String, u64>,
        then_var_value_bindings: &FxHashMap<String, (usize, u64)>,
        else_var_versions: &FxHashMap<String, u64>,
        else_var_value_bindings: &FxHashMap<String, (usize, u64)>,
    ) {
        let mut vars = FxHashSet::default();
        vars.extend(then_var_versions.keys().cloned());
        vars.extend(else_var_versions.keys().cloned());
        vars.extend(then_var_value_bindings.keys().cloned());
        vars.extend(else_var_value_bindings.keys().cloned());

        for var in vars {
            let pre_version = self.var_versions.get(&var).copied().unwrap_or(0);
            let then_version = then_var_versions.get(&var).copied().unwrap_or(pre_version);
            let else_version = else_var_versions.get(&var).copied().unwrap_or(pre_version);
            let joined_version = then_version.max(else_version);

            let then_binding = then_var_value_bindings.get(&var).copied();
            let else_binding = else_var_value_bindings.get(&var).copied();

            if let (Some((then_val_id, _)), Some((else_val_id, _))) = (then_binding, else_binding)
                && then_val_id == else_val_id
            {
                self.log_var_version_change(&var);
                self.var_versions.insert(var.clone(), joined_version);
                self.log_var_value_binding_change(&var);
                self.var_value_bindings
                    .insert(var.clone(), (then_val_id, joined_version));
                continue;
            }

            if joined_version != pre_version || then_binding != else_binding {
                self.log_var_version_change(&var);
                self.var_versions.insert(var.clone(), joined_version);
                self.log_var_value_binding_change(&var);
                self.var_value_bindings.remove(&var);
            }
        }
    }

    fn join_branch_last_assigned_values(
        &mut self,
        then_last_assigned: &FxHashMap<String, usize>,
        else_last_assigned: &FxHashMap<String, usize>,
    ) {
        let mut vars = FxHashSet::default();
        vars.extend(self.last_assigned_value_ids.keys().cloned());
        vars.extend(then_last_assigned.keys().cloned());
        vars.extend(else_last_assigned.keys().cloned());
        for var in vars {
            let pre = self.last_assigned_value_ids.get(&var).copied();
            let then = then_last_assigned.get(&var).copied().or(pre);
            let else_ = else_last_assigned.get(&var).copied().or(pre);
            self.log_last_assigned_value_change(&var);
            if then == else_ {
                if let Some(val_id) = then {
                    self.last_assigned_value_ids.insert(var, val_id);
                } else {
                    self.last_assigned_value_ids.remove(&var);
                }
            } else {
                self.last_assigned_value_ids.remove(&var);
            }
        }
    }

    fn join_branch_known_full_end_exprs(
        &mut self,
        pre_known_full_end_exprs: &FxHashMap<String, String>,
        then_known_full_end_exprs: &FxHashMap<String, String>,
        else_known_full_end_exprs: &FxHashMap<String, String>,
    ) {
        let mut vars = FxHashSet::default();
        vars.extend(pre_known_full_end_exprs.keys().cloned());
        vars.extend(then_known_full_end_exprs.keys().cloned());
        vars.extend(else_known_full_end_exprs.keys().cloned());

        for var in vars {
            let pre = pre_known_full_end_exprs.get(&var);
            let then = then_known_full_end_exprs.get(&var).or(pre);
            let else_ = else_known_full_end_exprs.get(&var).or(pre);
            match (then, else_) {
                (Some(lhs), Some(rhs)) if lhs == rhs => {
                    self.known_full_end_exprs.insert(var, lhs.clone());
                }
                _ => {
                    self.known_full_end_exprs.remove(&var);
                }
            }
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

    fn log_var_value_binding_change(&mut self, var: &str) {
        if self.branch_snapshot_depth == 0 {
            return;
        }
        self.var_value_binding_log.push(VarValueBindingUndo {
            var: var.to_string(),
            prev: self.var_value_bindings.get(var).copied(),
        });
    }

    fn log_last_assigned_value_change(&mut self, var: &str) {
        if self.branch_snapshot_depth == 0 {
            return;
        }
        self.last_assigned_value_log.push(LastAssignedValueUndo {
            var: var.to_string(),
            prev: self.last_assigned_value_ids.get(var).copied(),
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
        if Self::should_prefer_stale_var_over_expr(&values[vid])
            && (self
                .resolve_stale_origin_var(vid, &values[vid], values)
                .is_some()
                || self
                    .resolve_stale_fresh_clone_var(vid, &values[vid], values)
                    .is_some())
        {
            return;
        }

        let temp = format!(".__rr_cse_{}", vid);
        let expr = self.rewrite_known_one_based_full_range_alias_reads(
            &self.resolve_val(vid, values, params, true),
            values,
            params,
        );
        self.write_stmt(&format!("{} <- {}", temp, expr));
        self.note_var_write(&temp);
        self.bind_value_to_var(vid, &temp);
        self.bind_var_to_value(&temp, vid);
        self.remember_known_full_end_expr(&temp, vid, values, params);
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
        let function_scope_ends = Self::function_scope_ends(&lines);

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
                let scope_end = function_scope_ends[idx];
                let is_live = lines
                    .iter()
                    .enumerate()
                    .take(scope_end + 1)
                    .skip(idx + 1)
                    .any(|(_, line)| Self::line_contains_symbol(line, &name));
                if !is_live {
                    lines[idx] = format!("{}# rr-cse-pruned", indent);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let seed_defs: Vec<(usize, String, String)> = lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| {
                let (name, indent, rhs) = Self::extract_dead_loop_seed_assign(line)?;
                Some((idx, name, indent)).filter(|_| rhs == "1L" || rhs == "1" || rhs == "1.0")
            })
            .collect();
        for (idx, name, indent) in seed_defs {
            let scope_end = function_scope_ends[idx];
            let Some(next_idx) = lines
                .iter()
                .enumerate()
                .take(scope_end + 1)
                .skip(idx + 1)
                .find_map(|(line_idx, line)| {
                    let trimmed = line.trim();
                    (!trimmed.is_empty() && trimmed != "# rr-cse-pruned").then_some(line_idx)
                })
            else {
                continue;
            };
            if Self::line_contains_symbol(&lines[next_idx], &name) {
                continue;
            }
            let is_live_after = lines
                .iter()
                .enumerate()
                .take(scope_end + 1)
                .skip(next_idx + 1)
                .any(|(_, line)| Self::line_contains_symbol(line, &name));
            if !is_live_after {
                lines[idx] = format!("{indent}# rr-cse-pruned");
            }
        }

        loop {
            let init_defs: Vec<(usize, String, String)> = lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| {
                    let (name, indent, rhs) = Self::extract_plain_assign(line)?;
                    Self::is_prunable_dead_init_rhs(rhs.as_str()).then_some((idx, name, indent))
                })
                .collect();
            if init_defs.is_empty() {
                break;
            }

            let mut changed = false;
            for (idx, name, indent) in init_defs {
                let scope_end = function_scope_ends[idx];
                let has_later_use = Self::has_later_symbol_use(&lines, idx, scope_end, &name);
                if !has_later_use
                    || Self::is_dead_pre_loop_init_overwritten_before_use(
                        &lines, idx, scope_end, &name,
                    )
                    || Self::find_dead_overwrite_without_intervening_use(
                        &lines, idx, scope_end, &name,
                    )
                    .is_some()
                {
                    lines[idx] = format!("{indent}# rr-cse-pruned");
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let mut compacted = Vec::with_capacity(lines.len());
        for line in lines {
            let trimmed = line.trim();
            let prev_same_pruned = compacted
                .last()
                .is_some_and(|prev: &String| prev == &line && prev.trim() == "# rr-cse-pruned");
            if trimmed == "# rr-cse-pruned" && prev_same_pruned {
                continue;
            }
            compacted.push(line);
        }

        let mut rebuilt = compacted.join("\n");
        rebuilt.push('\n');
        *output = rebuilt;
    }

    fn function_scope_ends(lines: &[String]) -> Vec<usize> {
        let mut ends: Vec<usize> = (0..lines.len()).collect();
        let mut idx = 0usize;
        while idx < lines.len() {
            if !lines[idx].contains(" <- function(") {
                idx += 1;
                continue;
            }
            let start = idx;
            let mut depth = 0isize;
            let mut saw_open = false;
            let mut end = start;
            for (j, line) in lines.iter().enumerate().skip(start) {
                for ch in line.chars() {
                    match ch {
                        '{' => {
                            depth += 1;
                            saw_open = true;
                        }
                        '}' => {
                            depth -= 1;
                        }
                        _ => {}
                    }
                }
                end = j;
                if saw_open && depth <= 0 {
                    break;
                }
            }
            for entry in ends.iter_mut().take(end + 1).skip(start) {
                *entry = end;
            }
            idx = end + 1;
        }
        ends
    }

    fn extract_cse_assign_name(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim_start();
        if !(trimmed.starts_with(".__rr_cse_")
            || trimmed.starts_with(".tachyon_callmap_arg")
            || trimmed.starts_with(".tachyon_exprmap"))
        {
            return None;
        }
        let (name, _) = trimmed.split_once(" <- ")?;
        Some((
            name.to_string(),
            line[..line.len() - trimmed.len()].to_string(),
        ))
    }

    fn extract_dead_loop_seed_assign(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("i") {
            return None;
        }
        let (name, rhs) = trimmed.split_once(" <- ")?;
        if !(name == "i" || name.starts_with("i_")) {
            return None;
        }
        Some((
            name.to_string(),
            line[..line.len() - trimmed.len()].to_string(),
            rhs.trim().to_string(),
        ))
    }

    fn extract_plain_assign(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (name, rhs) = trimmed.split_once(" <- ")?;
        if name.is_empty() || !name.chars().all(Self::is_symbol_char) {
            return None;
        }
        Some((
            name.to_string(),
            line[..line.len() - trimmed.len()].to_string(),
            rhs.trim().to_string(),
        ))
    }

    fn is_prunable_dead_init_rhs(rhs: &str) -> bool {
        rhs.starts_with("rep.int(")
            || rhs.starts_with("numeric(")
            || rhs.starts_with("integer(")
            || rhs.starts_with("logical(")
            || rhs.starts_with("character(")
            || rhs.starts_with("vector(")
            || rhs.starts_with("matrix(")
            || rhs.starts_with("Sym_17(")
            || matches!(
                rhs,
                "0" | "0L" | "0.0" | "1" | "1L" | "1.0" | "TRUE" | "FALSE"
            )
    }

    fn has_later_symbol_use(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
        symbol: &str,
    ) -> bool {
        lines
            .iter()
            .enumerate()
            .take(scope_end + 1)
            .skip(start_idx + 1)
            .any(|(_, line)| Self::line_contains_symbol(line, symbol))
    }

    fn is_dead_pre_loop_init_overwritten_before_use(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
        symbol: &str,
    ) -> bool {
        let mut loop_start = None;
        for (idx, line) in lines
            .iter()
            .enumerate()
            .take(scope_end + 1)
            .skip(start_idx + 1)
        {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" || trimmed.starts_with('#') {
                continue;
            }
            if trimmed == "repeat {" {
                loop_start = Some(idx);
                break;
            }
            if Self::line_contains_symbol(line, symbol) || Self::line_breaks_straight_line(trimmed)
            {
                return false;
            }
        }
        let Some(loop_start) = loop_start else {
            return false;
        };
        let Some(loop_end) = Self::block_end_for_open_brace(lines, loop_start, scope_end) else {
            return false;
        };
        for line in lines.iter().take(loop_end).skip(loop_start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" || trimmed.starts_with('#') {
                continue;
            }
            if !Self::line_contains_symbol(line, symbol) {
                continue;
            }
            let Some((assigned, _, rhs)) = Self::extract_plain_assign(line) else {
                return false;
            };
            if assigned != symbol || Self::line_contains_symbol(rhs.as_str(), symbol) {
                return false;
            }
            return !Self::has_later_symbol_use(lines, loop_end, scope_end, symbol);
        }
        false
    }

    fn block_end_for_open_brace(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
    ) -> Option<usize> {
        let mut depth = 0isize;
        let mut saw_open = false;
        for (idx, line) in lines.iter().enumerate().take(scope_end + 1).skip(start_idx) {
            for ch in line.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        saw_open = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if saw_open && depth <= 0 {
                return Some(idx);
            }
        }
        None
    }

    fn find_dead_overwrite_without_intervening_use(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
        symbol: &str,
    ) -> Option<usize> {
        for (idx, line) in lines
            .iter()
            .enumerate()
            .take(scope_end + 1)
            .skip(start_idx + 1)
        {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" || trimmed.starts_with('#') {
                continue;
            }
            if Self::line_breaks_straight_line(trimmed) {
                return None;
            }
            if !Self::line_contains_symbol(line, symbol) {
                continue;
            }
            let (assigned, _, rhs) = Self::extract_plain_assign(line)?;
            if assigned != symbol {
                return None;
            }
            if Self::line_contains_symbol(rhs.as_str(), symbol) {
                return None;
            }
            return Some(idx);
        }
        None
    }

    fn line_breaks_straight_line(trimmed: &str) -> bool {
        trimmed == "{"
            || trimmed == "}"
            || trimmed.starts_with("if ")
            || trimmed.starts_with("if(")
            || trimmed.starts_with("if (")
            || trimmed.starts_with("else")
            || trimmed.starts_with("repeat")
            || trimmed.starts_with("next")
            || trimmed.starts_with("break")
            || trimmed.starts_with("return(")
            || trimmed.starts_with("return (")
            || trimmed.starts_with("return ")
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
                let mut idx = 0usize;
                while idx < items.len() {
                    if let Some(consumed) =
                        self.try_emit_full_range_conditional_loop_sequence(&items[idx..], fn_ir)
                    {
                        idx += consumed;
                        continue;
                    }
                    if idx + 1 < items.len()
                        && let StructuredBlock::BasicBlock(init_bb) = &items[idx]
                        && let StructuredBlock::Loop {
                            cond,
                            continue_on_true,
                            ..
                        } = &items[idx + 1]
                        && *continue_on_true
                        && let Some(ctx) = self
                            .extract_scalar_loop_index_context_from_init_bb(*init_bb, *cond, fn_ir)
                            .or_else(|| {
                                self.extract_scalar_loop_index_context_from_live_binding(
                                    *cond, fn_ir,
                                )
                            })
                    {
                        self.emit_structured(&items[idx], fn_ir)?;
                        self.active_scalar_loop_indices.push(ctx);
                        self.emit_structured(&items[idx + 1], fn_ir)?;
                        self.active_scalar_loop_indices.pop();
                        idx += 2;
                        continue;
                    }
                    self.emit_structured(&items[idx], fn_ir)?;
                    idx += 1;
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
                let pre_if_known_full_end_exprs = self.known_full_end_exprs.clone();

                let cond_span = fn_ir.values[*cond].span;
                self.emit_mark(cond_span, Some("if"));
                self.record_span(cond_span);
                let c = self.resolve_cond(*cond, &fn_ir.values, &fn_ir.params);
                self.write_stmt(&format!("if ({}) {{", c));
                self.indent += 1;
                self.emit_structured(then_body, fn_ir)?;
                self.indent -= 1;
                let then_var_versions = self.var_versions.clone();
                let then_var_value_bindings = self.var_value_bindings.clone();
                let then_last_assigned = self.last_assigned_value_ids.clone();
                let then_known_full_end_exprs = self.known_full_end_exprs.clone();
                if let Some(else_body) = else_body {
                    // Reset to pre-if state before emitting else branch.
                    self.rollback_branch_snapshot(snapshot);
                    self.known_full_end_exprs = pre_if_known_full_end_exprs.clone();
                    self.write_stmt("} else {");
                    self.indent += 1;
                    self.emit_structured(else_body, fn_ir)?;
                    self.indent -= 1;
                    let else_var_versions = self.var_versions.clone();
                    let else_var_value_bindings = self.var_value_bindings.clone();
                    let else_last_assigned = self.last_assigned_value_ids.clone();
                    let else_known_full_end_exprs = self.known_full_end_exprs.clone();
                    self.write_stmt("}");
                    self.rollback_branch_snapshot(snapshot);
                    self.known_full_end_exprs = pre_if_known_full_end_exprs.clone();
                    self.join_branch_var_value_bindings(
                        &then_var_versions,
                        &then_var_value_bindings,
                        &else_var_versions,
                        &else_var_value_bindings,
                    );
                    self.join_branch_last_assigned_values(&then_last_assigned, &else_last_assigned);
                    self.join_branch_known_full_end_exprs(
                        &pre_if_known_full_end_exprs,
                        &then_known_full_end_exprs,
                        &else_known_full_end_exprs,
                    );
                } else {
                    self.write_stmt("}");
                    self.rollback_branch_snapshot(snapshot);
                    self.known_full_end_exprs = pre_if_known_full_end_exprs;
                }

                // Join point: drop branch-local expression bindings conservatively.
                self.end_branch_snapshot();
                self.value_bindings.clear();
                self.recent_whole_assign_bases.clear();
            }
            StructuredBlock::Loop {
                header,
                cond,
                continue_on_true,
                body,
            } => {
                let pre_loop_value_bindings = self.value_bindings.clone();
                let pre_loop_var_value_bindings = self.var_value_bindings.clone();
                let mut loop_mutated_vars = FxHashSet::default();
                Self::collect_mutated_vars(body, fn_ir, &mut loop_mutated_vars);
                for instr in &fn_ir.blocks[*header].instrs {
                    match instr {
                        Instr::Assign { dst, .. } => {
                            loop_mutated_vars.insert(dst.clone());
                        }
                        Instr::StoreIndex1D { base, .. }
                        | Instr::StoreIndex2D { base, .. }
                        | Instr::StoreIndex3D { base, .. } => {
                            if let Some(var) = Self::named_written_base(*base, &fn_ir.values) {
                                loop_mutated_vars.insert(var);
                            }
                        }
                        Instr::Eval { .. } => {}
                    }
                }
                let pre_loop_known_full_end_exprs = loop_mutated_vars
                    .iter()
                    .filter_map(|var| {
                        self.known_full_end_expr_for_var(var)
                            .map(|expr| (var.clone(), expr.to_string()))
                    })
                    .collect::<FxHashMap<_, _>>();
                self.invalidate_var_bindings(loop_mutated_vars.iter());
                self.active_loop_known_full_end_exprs
                    .push(pre_loop_known_full_end_exprs.clone());
                self.active_loop_mutated_vars
                    .push(loop_mutated_vars.clone());
                self.emit_loop_invariant_scalar_hoists(
                    *header,
                    *cond,
                    body.as_ref(),
                    fn_ir,
                    &loop_mutated_vars,
                );
                let scalar_loop_ctx = self
                    .extract_scalar_loop_index_context_from_init_bb(*header, *cond, fn_ir)
                    .or_else(|| {
                        self.extract_scalar_loop_index_context_from_live_binding(*cond, fn_ir)
                    });
                if let Some(ctx) = scalar_loop_ctx.clone() {
                    self.active_scalar_loop_indices.push(ctx);
                }
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
                let fallback_idx_var = match fn_ir.values.get(*cond).map(|v| &v.kind) {
                    Some(ValueKind::Binary {
                        op: BinOp::Le, lhs, ..
                    }) => self.extract_loop_index_var(*lhs, &fn_ir.values),
                    Some(ValueKind::Binary {
                        op: BinOp::Ge, rhs, ..
                    }) => self.extract_loop_index_var(*rhs, &fn_ir.values),
                    _ => None,
                };
                if let Some(idx_var) = fallback_idx_var
                    && !loop_mutated_vars.contains(&idx_var)
                {
                    self.write_stmt(&format!("{idx_var} <- ({idx_var} + 1L)"));
                }

                self.indent -= 1;
                self.write_stmt("}");

                // Loop bodies may execute an unknown number of times (including zero).
                // Restore pre-loop bindings: values computed inside the loop are unsafe to
                // reference after the loop, but pre-loop bindings remain valid for unchanged
                // vars and become useful stale-origin hints for mutated aggregates.
                self.value_bindings = pre_loop_value_bindings;
                self.var_value_bindings = pre_loop_var_value_bindings;
                self.last_assigned_value_ids.clear();
                self.invalidate_var_bindings(loop_mutated_vars.iter());
                for var in &loop_mutated_vars {
                    self.known_full_end_exprs.remove(var);
                }
                self.known_full_end_exprs
                    .extend(pre_loop_known_full_end_exprs);
                self.recent_whole_assign_bases.clear();
                self.active_loop_known_full_end_exprs.pop();
                self.active_loop_mutated_vars.pop();
                if scalar_loop_ctx.is_some() {
                    self.active_scalar_loop_indices.pop();
                }
            }
            StructuredBlock::Break => {
                self.write_stmt("break");
            }
            StructuredBlock::Next => {
                self.write_stmt("next");
            }
            StructuredBlock::Return(v) => match v {
                Some(val) => {
                    if std::env::var_os("RR_DEBUG_RETURN").is_some() {
                        eprintln!(
                            "RR_DEBUG_RETURN fn={} val={} kind={:?} bound={:?} stale={:?}",
                            fn_ir.name,
                            val,
                            fn_ir.values[*val].kind,
                            self.resolve_bound_value(*val),
                            self.resolve_stale_origin_var(*val, &fn_ir.values[*val], &fn_ir.values)
                        );
                    }
                    if let ValueKind::Call {
                        callee,
                        args,
                        names,
                    } = &fn_ir.values[*val].kind
                        && callee == "rr_assign_slice"
                        && !args.is_empty()
                        && let Some(base_var) = Self::named_mutable_base_expr(
                            args[0],
                            &fn_ir.values,
                            &self.value_bindings,
                            &self.var_versions,
                        )
                    {
                        if self.resolve_bound_value(*val).as_deref() == Some(base_var.as_str()) {
                            self.write_stmt(&format!("return({base_var})"));
                            return Ok(());
                        }
                        let call_expr = self.resolve_call_expr(
                            &fn_ir.values[*val],
                            callee,
                            args,
                            names,
                            &fn_ir.values,
                            &fn_ir.params,
                        );
                        self.write_stmt(&format!("{base_var} <- {call_expr}"));
                        self.write_stmt(&format!("return({base_var})"));
                        return Ok(());
                    }
                    if let Some(bound) = self.resolve_bound_value(*val) {
                        self.write_stmt(&format!("return({bound})"));
                        return Ok(());
                    }
                    let r = self.resolve_val(*val, &fn_ir.values, &fn_ir.params, false);
                    self.write_stmt(&format!("return({})", r));
                }
                None => self.write_stmt("return(NULL)"),
            },
        }
        Ok(())
    }

    fn try_emit_full_range_conditional_loop_sequence(
        &mut self,
        items: &[StructuredBlock],
        fn_ir: &FnIR,
    ) -> Option<usize> {
        if items.len() < 2 {
            return None;
        }
        let StructuredBlock::BasicBlock(init_bb) = items.first()? else {
            return None;
        };
        let StructuredBlock::Loop {
            header: _,
            cond,
            continue_on_true,
            body,
        } = items.get(1)?
        else {
            return None;
        };
        if !continue_on_true {
            return None;
        }
        let init_block = &fn_ir.blocks[*init_bb];
        let [
            Instr::Assign {
                dst: idx_var, src, ..
            },
        ] = init_block.instrs.as_slice()
        else {
            return None;
        };
        if !idx_var.starts_with("i_") || !self.value_is_known_one(*src, &fn_ir.values) {
            return None;
        }
        if items[2..]
            .iter()
            .any(|item| self.structured_uses_var(item, fn_ir, idx_var))
        {
            return None;
        }
        let (guard_var, end_val) = self.extract_full_range_loop_guard(*cond, idx_var, fn_ir)?;
        if guard_var != *idx_var {
            return None;
        }
        let (branch_cond, then_bb, else_bb, incr_bb) =
            self.extract_conditional_loop_shape(body.as_ref())?;
        let (dest_var, then_val) =
            self.extract_conditional_loop_store(then_bb, idx_var, end_val, fn_ir)?;
        let (else_dest_var, else_val) =
            self.extract_conditional_loop_store(else_bb, idx_var, end_val, fn_ir)?;
        if dest_var != else_dest_var || !self.loop_increment_matches(incr_bb, idx_var, fn_ir) {
            return None;
        }

        let cond_expr = self.resolve_full_range_loop_expr(
            branch_cond,
            idx_var,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        )?;
        let then_expr = self.resolve_full_range_loop_expr(
            then_val,
            idx_var,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        )?;
        let else_expr = self.resolve_full_range_loop_expr(
            else_val,
            idx_var,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        )?;

        let cond_span = fn_ir.values[branch_cond].span;
        self.emit_mark(cond_span, Some("loop-vector-ifelse"));
        self.record_span(cond_span);
        self.write_stmt(&format!(
            "{dest_var} <- ifelse(({cond_expr}), {then_expr}, {else_expr})"
        ));
        self.note_var_write(&dest_var);
        self.recent_whole_assign_bases.insert(dest_var);
        self.last_assigned_value_ids.clear();
        Some(2)
    }

    fn extract_full_range_loop_guard(
        &self,
        cond: usize,
        expected_idx_var: &str,
        fn_ir: &FnIR,
    ) -> Option<(String, usize)> {
        let ValueKind::Binary { op, lhs, rhs } = fn_ir.values.get(cond)?.kind else {
            return None;
        };
        match op {
            BinOp::Le => {
                let idx_var = self.extract_loop_index_var(lhs, &fn_ir.values)?;
                if idx_var == expected_idx_var {
                    Some((idx_var, rhs))
                } else {
                    None
                }
            }
            BinOp::Ge => {
                let idx_var = self.extract_loop_index_var(rhs, &fn_ir.values)?;
                if idx_var == expected_idx_var {
                    Some((idx_var, lhs))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn extract_loop_index_var(&self, value_id: usize, values: &[Value]) -> Option<String> {
        match values.get(value_id).map(|v| &v.kind) {
            Some(ValueKind::Load { var }) if var == "i" || var.starts_with("i_") => {
                Some(var.clone())
            }
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.extract_loop_index_var(bound, values)),
            _ => None,
        }
    }

    fn extract_conditional_loop_shape(
        &self,
        body: &StructuredBlock,
    ) -> Option<(usize, usize, usize, usize)> {
        let StructuredBlock::Sequence(items) = body else {
            return None;
        };
        let [
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            },
            StructuredBlock::BasicBlock(incr_bb),
            StructuredBlock::Next,
        ] = items.as_slice()
        else {
            return None;
        };
        let then_bb = self.single_basic_block(then_body.as_ref())?;
        let else_bb = self.single_basic_block(else_body.as_ref()?.as_ref())?;
        Some((*cond, then_bb, else_bb, *incr_bb))
    }

    fn single_basic_block(&self, node: &StructuredBlock) -> Option<usize> {
        match node {
            StructuredBlock::BasicBlock(bb) => Some(*bb),
            StructuredBlock::Sequence(items) if items.len() == 1 => {
                self.single_basic_block(&items[0])
            }
            _ => None,
        }
    }

    fn extract_conditional_loop_store(
        &self,
        bb: usize,
        idx_var: &str,
        end_val: usize,
        fn_ir: &FnIR,
    ) -> Option<(String, usize)> {
        let block = &fn_ir.blocks[bb];
        let [Instr::StoreIndex1D { base, idx, val, .. }] = block.instrs.as_slice() else {
            return None;
        };
        if !self.value_matches_loop_index(*idx, idx_var, &fn_ir.values, &mut FxHashSet::default()) {
            return None;
        }
        if !self.value_is_full_dest_end(
            *base,
            end_val,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        ) {
            return None;
        }
        let dest_var = Self::named_mutable_base_expr(
            *base,
            &fn_ir.values,
            &self.value_bindings,
            &self.var_versions,
        )?;
        Some((dest_var, *val))
    }

    fn value_matches_loop_index(
        &self,
        value_id: usize,
        idx_var: &str,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(value_id) {
            return false;
        }
        match values.get(value_id).map(|v| &v.kind) {
            Some(ValueKind::Load { var }) if var == idx_var => true,
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .is_some_and(|bound| self.value_matches_loop_index(bound, idx_var, values, seen)),
            _ => false,
        }
    }

    fn loop_increment_matches(&self, bb: usize, idx_var: &str, fn_ir: &FnIR) -> bool {
        let block = &fn_ir.blocks[bb];
        let [Instr::Assign { dst, src, .. }] = block.instrs.as_slice() else {
            return false;
        };
        if dst != idx_var {
            return false;
        }
        match fn_ir.values.get(*src).map(|v| &v.kind) {
            Some(ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            }) => {
                (self.value_matches_loop_index(
                    *lhs,
                    idx_var,
                    &fn_ir.values,
                    &mut FxHashSet::default(),
                ) && self.value_is_known_one(*rhs, &fn_ir.values))
                    || (self.value_matches_loop_index(
                        *rhs,
                        idx_var,
                        &fn_ir.values,
                        &mut FxHashSet::default(),
                    ) && self.value_is_known_one(*lhs, &fn_ir.values))
            }
            _ => false,
        }
    }

    fn known_small_positive_scalar(&self, value_id: usize, values: &[Value]) -> Option<i64> {
        let value = values.get(value_id)?;
        if value.facts.has(Facts::INT_SCALAR | Facts::NON_NA) && value.facts.interval.min >= 1 {
            let max = value.facts.interval.max;
            let min = value.facts.interval.min;
            if min == max {
                return Some(min);
            }
        }
        match &value.kind {
            ValueKind::Const(Lit::Int(i)) if *i >= 1 => Some(*i),
            ValueKind::Const(Lit::Float(f))
                if f.is_finite()
                    && (*f - f.trunc()).abs() < f64::EPSILON
                    && *f >= 1.0
                    && *f <= i64::MAX as f64 =>
            {
                Some(*f as i64)
            }
            _ => None,
        }
    }

    fn extract_scalar_loop_index_context_from_init_bb(
        &self,
        init_bb: usize,
        cond: usize,
        fn_ir: &FnIR,
    ) -> Option<ActiveScalarLoopIndex> {
        let block = &fn_ir.blocks[init_bb];
        let ValueKind::Binary { op, lhs, rhs } = fn_ir.values.get(cond)?.kind else {
            return None;
        };
        let (idx_var, cmp) = match op {
            BinOp::Lt => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Le => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            BinOp::Gt => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Ge => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            _ => return None,
        };

        let start_min = block
            .instrs
            .iter()
            .filter_map(|instr| match instr {
                Instr::Assign { dst, src, .. } if *dst == idx_var => {
                    self.known_small_positive_scalar(*src, &fn_ir.values)
                }
                _ => None,
            })
            .next()?;

        Some(ActiveScalarLoopIndex {
            var: idx_var,
            start_min,
            cmp,
        })
    }

    fn extract_scalar_loop_index_context_from_live_binding(
        &self,
        cond: usize,
        fn_ir: &FnIR,
    ) -> Option<ActiveScalarLoopIndex> {
        let ValueKind::Binary { op, lhs, rhs } = fn_ir.values.get(cond)?.kind else {
            return None;
        };
        let (idx_var, cmp) = match op {
            BinOp::Lt => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Le => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            BinOp::Gt => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Ge => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            _ => return None,
        };
        let bound = self.resolve_bound_value_id(&idx_var)?;
        let start_min = self.known_small_positive_scalar(bound, &fn_ir.values)?;
        Some(ActiveScalarLoopIndex {
            var: idx_var,
            start_min,
            cmp,
        })
    }

    fn loop_index_offset(
        &self,
        value_id: usize,
        ctx: &ActiveScalarLoopIndex,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> Option<i64> {
        if !seen.insert(value_id) {
            return None;
        }
        match values.get(value_id).map(|v| &v.kind)? {
            ValueKind::Load { var } if var == &ctx.var => Some(0),
            ValueKind::Load { var } => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.loop_index_offset(bound, ctx, values, seen)),
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                if let Some(base) = self.loop_index_offset(*lhs, ctx, values, seen)
                    && let Some(delta) = self.known_small_positive_scalar(*rhs, values)
                {
                    return Some(base.saturating_add(delta));
                }
                if let Some(base) = self.loop_index_offset(*rhs, ctx, values, seen)
                    && let Some(delta) = self.known_small_positive_scalar(*lhs, values)
                {
                    return Some(base.saturating_add(delta));
                }
                None
            }
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs,
                rhs,
            } => {
                let base = self.loop_index_offset(*lhs, ctx, values, seen)?;
                let delta = self.known_small_positive_scalar(*rhs, values)?;
                Some(base.saturating_sub(delta))
            }
            _ => None,
        }
    }

    fn loop_context_allows_offset(ctx: &ActiveScalarLoopIndex, offset: i64) -> bool {
        if ctx.start_min.saturating_add(offset) < 1 {
            return false;
        }
        if offset <= 0 {
            return true;
        }
        matches!(ctx.cmp, ScalarLoopCmp::Lt) && offset <= 1
    }

    fn rendered_loop_index_offset(expr: &str, ctx: &ActiveScalarLoopIndex) -> Option<i64> {
        let mut compact = expr
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        if compact == ctx.var {
            return Some(0);
        }
        if compact.starts_with('(') && compact.ends_with(')') {
            compact = compact[1..compact.len() - 1].to_string();
        }
        let minus_one = format!("{}-1", ctx.var);
        if compact == minus_one || compact == format!("{minus_one}L") {
            return Some(-1);
        }
        let plus_one = format!("{}+1", ctx.var);
        if compact == plus_one || compact == format!("{plus_one}L") {
            return Some(1);
        }
        None
    }

    fn resolve_full_range_loop_expr(
        &self,
        val_id: usize,
        idx_var: &str,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> Option<String> {
        if !seen.insert(val_id) {
            return None;
        }
        let value = values.get(val_id)?;
        match values.get(val_id).map(|v| &v.kind)? {
            ValueKind::Const(lit) => Some(self.emit_lit(lit)),
            ValueKind::Param { index } => Some(self.resolve_param(*index, params)),
            ValueKind::Load { var } if var == idx_var => None,
            ValueKind::Load { var } if var.starts_with('.') => self
                .resolve_bound_value_id(var)
                .filter(|bound| *bound != val_id)
                .and_then(|bound| {
                    self.resolve_full_range_loop_expr(bound, idx_var, values, params, seen)
                }),
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Binary { op, lhs, rhs } => {
                let l = self.resolve_full_range_loop_expr(*lhs, idx_var, values, params, seen)?;
                let r = self.resolve_full_range_loop_expr(*rhs, idx_var, values, params, seen)?;
                if matches!(
                    op,
                    BinOp::Eq
                        | BinOp::Ne
                        | BinOp::Lt
                        | BinOp::Le
                        | BinOp::Gt
                        | BinOp::Ge
                        | BinOp::Add
                        | BinOp::Sub
                        | BinOp::Mul
                        | BinOp::Div
                        | BinOp::Mod
                        | BinOp::And
                        | BinOp::Or
                ) {
                    Some(format!("({l} {} {r})", Self::binary_op_str(*op)))
                } else {
                    None
                }
            }
            ValueKind::Unary { op, rhs } => {
                let r = self.resolve_full_range_loop_expr(*rhs, idx_var, values, params, seen)?;
                Some(format!("({}({}))", Self::unary_op_str(*op), r))
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                if callee.starts_with("rr_") || names.iter().any(|name| name.is_some()) {
                    return None;
                }
                let rendered_args: Option<Vec<String>> = args
                    .iter()
                    .map(|arg| {
                        self.resolve_full_range_loop_expr(*arg, idx_var, values, params, seen)
                    })
                    .collect();
                let rendered_args = rendered_args?;
                if !self.direct_builtin_vector_math
                    && value.value_ty.shape == ShapeTy::Vector
                    && value.value_ty.prim == PrimTy::Double
                {
                    match (callee.as_str(), rendered_args.as_slice()) {
                        ("abs", [arg]) => return Some(format!("rr_intrinsic_vec_abs_f64({arg})")),
                        ("log", [arg]) => return Some(format!("rr_intrinsic_vec_log_f64({arg})")),
                        ("sqrt", [arg]) => {
                            return Some(format!("rr_intrinsic_vec_sqrt_f64({arg})"));
                        }
                        ("pmax", [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})"));
                        }
                        ("pmin", [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})"));
                        }
                        _ => {}
                    }
                }
                Some(format!("{}({})", callee, rendered_args.join(", ")))
            }
            ValueKind::Intrinsic { op, args } => {
                let rendered_args: Option<Vec<String>> = args
                    .iter()
                    .map(|arg| {
                        self.resolve_full_range_loop_expr(*arg, idx_var, values, params, seen)
                    })
                    .collect();
                let rendered_args = rendered_args?;
                if !self.direct_builtin_vector_math {
                    match (op, rendered_args.as_slice()) {
                        (IntrinsicOp::VecAbsF64, [arg]) => {
                            return Some(format!("rr_intrinsic_vec_abs_f64({arg})"));
                        }
                        (IntrinsicOp::VecLogF64, [arg]) => {
                            return Some(format!("rr_intrinsic_vec_log_f64({arg})"));
                        }
                        (IntrinsicOp::VecSqrtF64, [arg]) => {
                            return Some(format!("rr_intrinsic_vec_sqrt_f64({arg})"));
                        }
                        (IntrinsicOp::VecPmaxF64, [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})"));
                        }
                        (IntrinsicOp::VecPminF64, [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})"));
                        }
                        _ => {}
                    }
                }
                match (op, rendered_args.as_slice()) {
                    (IntrinsicOp::VecAddF64, [lhs, rhs]) => Some(format!("({lhs} + {rhs})")),
                    (IntrinsicOp::VecSubF64, [lhs, rhs]) => Some(format!("({lhs} - {rhs})")),
                    (IntrinsicOp::VecMulF64, [lhs, rhs]) => Some(format!("({lhs} * {rhs})")),
                    (IntrinsicOp::VecDivF64, [lhs, rhs]) => Some(format!("({lhs} / {rhs})")),
                    (IntrinsicOp::VecAbsF64, [arg]) => Some(format!("abs({arg})")),
                    (IntrinsicOp::VecLogF64, [arg]) => Some(format!("log({arg})")),
                    (IntrinsicOp::VecSqrtF64, [arg]) => Some(format!("sqrt({arg})")),
                    (IntrinsicOp::VecPmaxF64, [lhs, rhs]) => Some(format!("pmax({lhs}, {rhs})")),
                    (IntrinsicOp::VecPminF64, [lhs, rhs]) => Some(format!("pmin({lhs}, {rhs})")),
                    _ => None,
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                if self.value_matches_loop_index(*idx, idx_var, values, &mut FxHashSet::default()) {
                    Some(self.resolve_read_base(*base, values, params))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn structured_uses_var(&self, node: &StructuredBlock, fn_ir: &FnIR, var: &str) -> bool {
        match node {
            StructuredBlock::Sequence(items) => items
                .iter()
                .any(|item| self.structured_uses_var(item, fn_ir, var)),
            StructuredBlock::BasicBlock(bb) => {
                let block = &fn_ir.blocks[*bb];
                block.instrs.iter().any(|instr| match instr {
                    Instr::Assign { dst, src, .. } => {
                        dst == var
                            || self.value_mentions_var(
                                *src,
                                &fn_ir.values,
                                var,
                                &mut FxHashSet::default(),
                            )
                    }
                    Instr::Eval { val, .. } => {
                        self.value_mentions_var(*val, &fn_ir.values, var, &mut FxHashSet::default())
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        self.value_mentions_var(
                            *base,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *idx,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *val,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        )
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        self.value_mentions_var(
                            *base,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *r,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *c,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *val,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        )
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        self.value_mentions_var(
                            *base,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *i,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *j,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *k,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *val,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        )
                    }
                }) || match block.term {
                    Terminator::If { cond, .. } => {
                        self.value_mentions_var(cond, &fn_ir.values, var, &mut FxHashSet::default())
                    }
                    Terminator::Return(Some(val)) => {
                        self.value_mentions_var(val, &fn_ir.values, var, &mut FxHashSet::default())
                    }
                    _ => false,
                }
            }
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            } => {
                self.value_mentions_var(*cond, &fn_ir.values, var, &mut FxHashSet::default())
                    || self.structured_uses_var(then_body, fn_ir, var)
                    || else_body
                        .as_ref()
                        .is_some_and(|body| self.structured_uses_var(body, fn_ir, var))
            }
            StructuredBlock::Loop { cond, body, .. } => {
                self.value_mentions_var(*cond, &fn_ir.values, var, &mut FxHashSet::default())
                    || self.structured_uses_var(body, fn_ir, var)
            }
            StructuredBlock::Return(Some(val)) => {
                self.value_mentions_var(*val, &fn_ir.values, var, &mut FxHashSet::default())
            }
            StructuredBlock::Break | StructuredBlock::Next | StructuredBlock::Return(None) => false,
        }
    }

    fn value_mentions_var(
        &self,
        value_id: usize,
        values: &[Value],
        var: &str,
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(value_id) {
            return false;
        }
        match values.get(value_id).map(|v| &v.kind) {
            Some(ValueKind::Load { var: load_var }) => load_var == var,
            Some(ValueKind::Phi { args }) => args
                .iter()
                .any(|(arg, _)| self.value_mentions_var(*arg, values, var, seen)),
            Some(ValueKind::Len { base }) | Some(ValueKind::Indices { base }) => {
                self.value_mentions_var(*base, values, var, seen)
            }
            Some(ValueKind::Range { start, end }) => {
                self.value_mentions_var(*start, values, var, seen)
                    || self.value_mentions_var(*end, values, var, seen)
            }
            Some(ValueKind::Binary { lhs, rhs, .. }) => {
                self.value_mentions_var(*lhs, values, var, seen)
                    || self.value_mentions_var(*rhs, values, var, seen)
            }
            Some(ValueKind::Unary { rhs, .. }) => self.value_mentions_var(*rhs, values, var, seen),
            Some(ValueKind::Call { args, .. }) | Some(ValueKind::Intrinsic { args, .. }) => args
                .iter()
                .any(|arg| self.value_mentions_var(*arg, values, var, seen)),
            Some(ValueKind::Index1D { base, idx, .. }) => {
                self.value_mentions_var(*base, values, var, seen)
                    || self.value_mentions_var(*idx, values, var, seen)
            }
            Some(ValueKind::Index2D { base, r, c }) => {
                self.value_mentions_var(*base, values, var, seen)
                    || self.value_mentions_var(*r, values, var, seen)
                    || self.value_mentions_var(*c, values, var, seen)
            }
            Some(ValueKind::Index3D { base, i, j, k }) => {
                self.value_mentions_var(*base, values, var, seen)
                    || self.value_mentions_var(*i, values, var, seen)
                    || self.value_mentions_var(*j, values, var, seen)
                    || self.value_mentions_var(*k, values, var, seen)
            }
            _ => false,
        }
    }

    fn named_mutable_base_expr(
        val_id: usize,
        values: &[Value],
        value_bindings: &FxHashMap<usize, (String, u64)>,
        var_versions: &FxHashMap<String, u64>,
    ) -> Option<String> {
        if let Some((var, version)) = value_bindings.get(&val_id)
            && var_versions.get(var).copied().unwrap_or(0) == *version
        {
            return Some(var.clone());
        }
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Load { .. } | ValueKind::Param { .. }) => {
                values.get(val_id).and_then(|v| v.origin_var.clone())
            }
            _ => None,
        }
    }

    fn resolve_val(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        prefer_expr: bool,
    ) -> String {
        let val = &values[val_id];

        if !prefer_expr
            && let ValueKind::Load { var } = &val.kind
            && self
                .active_scalar_loop_indices
                .iter()
                .rev()
                .any(|ctx| ctx.var == *var)
        {
            return var.clone();
        }

        if !prefer_expr && Self::should_prefer_stale_var_over_expr(val) {
            if let Some(origin_var) = self.resolve_stale_origin_var(val_id, val, values) {
                return origin_var;
            }
            if let Some(origin_var) = self.resolve_stale_fresh_clone_var(val_id, val, values) {
                return origin_var;
            }
        }

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
            ValueKind::Const(lit) => self.emit_lit_with_value(lit, val),
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
            } => self.resolve_call_expr(val, callee, args, names, values, params),
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
            ValueKind::Load { var } => self
                .resolve_readonly_arg_alias_name(var, values)
                .unwrap_or_else(|| var.clone()),
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
        let mut l = self.resolve_val(lhs, values, params, false);
        let mut r = self.resolve_val(rhs, values, params, false);
        if let Some(origin_var) = self.resolve_live_const_origin_var(lhs, values) {
            l = origin_var;
        }
        if let Some(origin_var) = self.resolve_live_const_origin_var(rhs, values) {
            r = origin_var;
        }
        if matches!(op, BinOp::Mul | BinOp::Div | BinOp::Mod) {
            if let Some(origin_var) = values[lhs].origin_var.as_deref()
                && matches!(values[lhs].kind, ValueKind::Const(_))
                && r == origin_var
            {
                l = origin_var.to_string();
            }
            if let Some(origin_var) = values[rhs].origin_var.as_deref()
                && matches!(values[rhs].kind, ValueKind::Const(_))
                && l == origin_var
            {
                r = origin_var.to_string();
            }
        }
        if matches!(op, BinOp::Add)
            && (matches!(values[lhs].kind, ValueKind::Const(Lit::Str(_)))
                || matches!(values[rhs].kind, ValueKind::Const(Lit::Str(_))))
        {
            return format!("paste0({}, {})", l, r);
        }
        let ty = val.value_ty;
        if self.direct_builtin_vector_math
            && ty.shape == ShapeTy::Vector
            && ty.prim == PrimTy::Double
        {
            return format!("({} {} {})", l, Self::binary_op_str(op), r);
        }
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

    fn resolve_live_const_origin_var(&self, val_id: usize, values: &[Value]) -> Option<String> {
        let val = values.get(val_id)?;
        let ValueKind::Const(_) = &val.kind else {
            return None;
        };
        let origin_var = val.origin_var.as_ref()?;
        let (bound_val_id, version) = *self.var_value_bindings.get(origin_var)?;
        if self.current_var_version(origin_var) != version {
            return None;
        }
        let bound = values.get(bound_val_id)?;
        if bound.kind == val.kind {
            return Some(origin_var.clone());
        }
        None
    }

    fn resolve_unary_expr(
        &self,
        op: UnaryOp,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        if matches!(op, UnaryOp::Neg) {
            match values.get(rhs).map(|value| &value.kind) {
                Some(ValueKind::Const(Lit::Int(v))) => {
                    if let Some(negated) = v.checked_neg() {
                        return format!("{negated}L");
                    }
                }
                Some(ValueKind::Const(Lit::Float(v))) => {
                    return self.emit_float_lit(-v);
                }
                _ => {}
            }
        }
        let r = self.resolve_val(rhs, values, params, false);
        format!("({}({}))", Self::unary_op_str(op), r)
    }

    fn resolve_call_expr(
        &self,
        val: &Value,
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
        params: &[String],
    ) -> String {
        if matches!(callee, "rr_index1_read" | "rr_index1_read_floor")
            && (args.len() == 2 || args.len() == 3)
            && names.iter().take(2).all(std::option::Option::is_none)
            && self.can_elide_index_expr(args[1], values, params)
        {
            let base = self.resolve_val(args[0], values, params, false);
            let idx = self.resolve_val(args[1], values, params, false);
            return format!("{}[{}]", base, idx);
        }
        if callee == "rr_index1_write"
            && (args.len() == 1 || args.len() == 2)
            && names
                .first()
                .and_then(std::option::Option::as_ref)
                .is_none()
            && self.can_elide_index_expr(args[0], values, params)
        {
            return self.resolve_val(args[0], values, params, false);
        }
        if matches!(callee, "rr_index1_read_vec" | "rr_index1_read_vec_floor")
            && args.len() >= 2
            && names.iter().take(2).all(std::option::Option::is_none)
        {
            let base = args[0];
            let idx = args[1];
            if let Some(end_expr) = self.known_full_end_expr_for_value(base, values, params)
                && self.value_is_one_based_full_range_alias(
                    idx,
                    end_expr.as_str(),
                    values,
                    params,
                    &mut FxHashSet::default(),
                )
            {
                return self.resolve_val(base, values, params, false);
            }
        }
        if let Some((base, idx)) = Self::floor_index_read_components(callee, args, names, values) {
            if let Some(end_expr) = self.known_full_end_expr_for_value(base, values, params)
                && self.value_is_one_based_full_range_alias(
                    idx,
                    end_expr.as_str(),
                    values,
                    params,
                    &mut FxHashSet::default(),
                )
            {
                return self.resolve_val(base, values, params, false);
            }
            let b = self.resolve_val(base, values, params, false);
            let i = self.resolve_val(idx, values, params, false);
            return format!("rr_index1_read_idx({}, {}, \"index\")", b, i);
        }
        if Self::can_elide_identity_floor_call(callee, args, names, values) {
            return self.resolve_val(args[0], values, params, false);
        }
        if !self.direct_builtin_vector_math
            && val.value_ty.shape == ShapeTy::Vector
            && val.value_ty.prim == PrimTy::Double
            && names.iter().all(Option::is_none)
        {
            let resolved: Vec<String> = args
                .iter()
                .map(|arg| self.resolve_val(*arg, values, params, false))
                .collect();
            match (callee, resolved.as_slice()) {
                ("abs", [arg]) => return format!("rr_intrinsic_vec_abs_f64({arg})"),
                ("log", [arg]) => return format!("rr_intrinsic_vec_log_f64({arg})"),
                ("sqrt", [arg]) => return format!("rr_intrinsic_vec_sqrt_f64({arg})"),
                ("pmax", [lhs, rhs]) => return format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})"),
                ("pmin", [lhs, rhs]) => return format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})"),
                _ => {}
            }
        }
        if callee == "rr_idx_cube_vec_i" && args.len() == 4 && names.iter().all(Option::is_none) {
            let rendered_args = [
                self.resolve_rr_idx_cube_vec_arg_expr(args[0], values, params),
                self.resolve_rr_idx_cube_vec_arg_expr(args[1], values, params),
                self.resolve_rr_idx_cube_vec_arg_expr(args[2], values, params),
                self.resolve_val(args[3], values, params, false),
            ];
            return format!(
                "rr_idx_cube_vec_i({}, {}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2], rendered_args[3]
            );
        }
        let arg_list = self.build_named_arg_list(args, names, values, params);
        format!("{}({})", callee, arg_list)
    }

    fn resolve_rr_idx_cube_vec_arg_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        self.try_resolve_singleton_replace_expr(val_id, values, params)
            .or_else(|| {
                self.try_render_singleton_assign_call_with_scalar_rhs(val_id, values, params)
            })
            .unwrap_or_else(|| self.resolve_val(val_id, values, params, false))
    }

    fn try_resolve_singleton_replace_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(val_id)?.kind else {
            return None;
        };
        if *callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let start_expr =
            self.resolve_bound_temp_expr(args[1], values, params, &mut FxHashSet::default());
        let end_expr =
            self.resolve_bound_temp_expr(args[2], values, params, &mut FxHashSet::default());
        if start_expr != end_expr {
            return None;
        }
        let boundary_ok = self.value_is_known_one(args[1], values)
            || self.value_is_full_dest_end(
                args[0],
                args[2],
                values,
                params,
                &mut FxHashSet::default(),
            )
            || self
                .resolve_named_mutable_base_var(args[0], values, params)
                .is_some_and(|base_var| {
                    self.whole_dest_end_matches_known_var(
                        base_var.as_str(),
                        args[2],
                        values,
                        params,
                    )
                });
        if !boundary_ok {
            return None;
        }
        let scalar = self.resolve_singleton_assign_scalar_expr(args[3], values, params)?;
        let base = self.resolve_bound_temp_expr(args[0], values, params, &mut FxHashSet::default());
        Some(format!("replace({}, {}, {})", base, start_expr, scalar))
    }

    fn resolve_singleton_assign_scalar_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Call { callee, args, .. })
                if *callee == "rep.int"
                    && args.len() >= 2
                    && self.value_is_known_one(args[1], values) =>
            {
                Some(self.resolve_bound_temp_expr(
                    args[0],
                    values,
                    params,
                    &mut FxHashSet::default(),
                ))
            }
            _ if self.value_is_scalar_shape(val_id, values) => Some(self.resolve_bound_temp_expr(
                val_id,
                values,
                params,
                &mut FxHashSet::default(),
            )),
            _ => None,
        }
    }

    fn try_render_singleton_assign_call_with_scalar_rhs(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(val_id)?.kind else {
            return None;
        };
        if *callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let start_expr = self.resolve_val(args[1], values, params, false);
        let end_expr = self.resolve_val(args[2], values, params, false);
        if start_expr != end_expr {
            return None;
        }
        let scalar = self.resolve_singleton_assign_scalar_expr(args[3], values, params)?;
        let base = self.resolve_bound_temp_expr(args[0], values, params, &mut FxHashSet::default());
        let start_expr =
            self.resolve_bound_temp_expr(args[1], values, params, &mut FxHashSet::default());
        let end_expr =
            self.resolve_bound_temp_expr(args[2], values, params, &mut FxHashSet::default());
        Some(format!(
            "rr_assign_slice({}, {}, {}, {})",
            base, start_expr, end_expr, scalar
        ))
    }

    fn try_resolve_whole_range_self_assign_rhs(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
            return None;
        };
        if callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let base_var = self.resolve_named_mutable_base_var(args[0], values, params)?;
        if base_var != dst {
            return None;
        }
        if !self.value_is_known_one(args[1], values) {
            return None;
        }
        if !self.value_is_full_dest_end(args[0], args[2], values, params, &mut FxHashSet::default())
            && !self.whole_dest_end_matches_known_var(dst, args[2], values, params)
        {
            return None;
        }
        Some(self.normalize_whole_range_vector_expr(
            self.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default()),
            args[1],
            args[2],
            values,
            params,
        ))
    }

    fn try_render_constant_safe_partial_self_assign(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
            return None;
        };
        if callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let base_var = self.resolve_named_mutable_base_var(args[0], values, params)?;
        if base_var != dst {
            if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
                eprintln!(
                    "RR_DEBUG_PARTIAL_SLICE skip=base_mismatch fn={} dst={} base_var={} src={}",
                    self.current_fn_name, dst, base_var, src
                );
            }
            return None;
        }
        let Some(start) = self.const_index_int_value(args[1], values) else {
            if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
                eprintln!(
                    "RR_DEBUG_PARTIAL_SLICE skip=start_nonconst fn={} dst={} start_expr={}",
                    self.current_fn_name,
                    dst,
                    self.resolve_val(args[1], values, params, false)
                );
            }
            return None;
        };
        let Some(end) = self.const_index_int_value(args[2], values) else {
            if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
                eprintln!(
                    "RR_DEBUG_PARTIAL_SLICE skip=end_nonconst fn={} dst={} end_expr={}",
                    self.current_fn_name,
                    dst,
                    self.resolve_val(args[2], values, params, false)
                );
            }
            return None;
        };
        if start < 1 || end < start {
            if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
                eprintln!(
                    "RR_DEBUG_PARTIAL_SLICE skip=range_invalid fn={} dst={} start={} end={}",
                    self.current_fn_name, dst, start, end
                );
            }
            return None;
        }
        let Some(known_end) = self.known_full_end_bound_for_var(dst, values) else {
            if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
                eprintln!(
                    "RR_DEBUG_PARTIAL_SLICE skip=unknown_known_end fn={} dst={}",
                    self.current_fn_name, dst
                );
            }
            return None;
        };
        if end > known_end {
            if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
                eprintln!(
                    "RR_DEBUG_PARTIAL_SLICE skip=end_oob fn={} dst={} end={} known_end={}",
                    self.current_fn_name, dst, end, known_end
                );
            }
            return None;
        }
        if !self.rep_int_matches_slice_len(args[3], start, end, values) {
            if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
                eprintln!(
                    "RR_DEBUG_PARTIAL_SLICE skip=len_mismatch fn={} dst={} start={} end={} rhs={}",
                    self.current_fn_name,
                    dst,
                    start,
                    end,
                    self.resolve_bound_temp_expr(
                        args[3],
                        values,
                        params,
                        &mut FxHashSet::default()
                    )
                );
            }
            return None;
        }
        let rhs = self.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default());
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE hit fn={} stmt={} [{}:{}] rhs={}",
                self.current_fn_name, dst, start, end, rhs
            );
        }
        Some(format!("{dst}[{start}:{end}] <- {rhs}"))
    }

    fn try_render_safe_idx_cube_row_slice_assign(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
            return None;
        };
        if *callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let base_var = self.resolve_named_mutable_base_var(args[0], values, params)?;
        if base_var != dst {
            return None;
        }
        let row_size_expr = self.idx_cube_row_size_expr(args[1], args[2], values, params)?;
        if !self.value_matches_known_length_expr(args[3], row_size_expr.as_str(), values, params) {
            return None;
        }
        let start_expr = self.resolve_preferred_live_expr_alias(args[1], values, params);
        let end_expr = self.resolve_preferred_live_expr_alias(args[2], values, params);
        let rhs = self.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default());
        Some(format!("{dst}[{start_expr}:{end_expr}] <- {rhs}"))
    }

    fn resolve_preferred_live_expr_alias(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind) {
            if let Some(alias) = self.resolve_readonly_arg_alias_name(var, values) {
                return alias;
            }
            if !var.starts_with('.') {
                return var.clone();
            }
        }
        if let Some(bound) = self.resolve_bound_value(val_id)
            && !bound.starts_with('.')
        {
            return bound;
        }
        let rendered =
            self.resolve_bound_temp_expr(val_id, values, params, &mut FxHashSet::default());
        if Self::is_plain_symbol_expr(rendered.as_str()) {
            return rendered;
        }
        self.find_live_plain_symbol_for_exact_expr(rendered.as_str(), values, params)
            .unwrap_or(rendered)
    }

    fn find_live_plain_symbol_for_exact_expr(
        &self,
        expr: &str,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let mut candidate: Option<String> = None;
        for (var, (bound_val_id, version)) in &self.var_value_bindings {
            if var.starts_with('.') || self.current_var_version(var) != *version {
                continue;
            }
            let bound_expr = self.rewrite_live_readonly_arg_aliases(
                self.resolve_val(*bound_val_id, values, params, true),
                values,
            );
            if bound_expr != expr {
                continue;
            }
            if candidate.is_some() {
                return None;
            }
            candidate = Some(var.clone());
        }
        candidate
    }

    fn idx_cube_row_size_expr(
        &self,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        if let Some(ValueKind::Load { var }) = values.get(start).map(|v| &v.kind)
            && let Some(bound) = self.resolve_bound_value_id(var)
            && bound != start
        {
            return self.idx_cube_row_size_expr(bound, end, values, params);
        }
        if let Some(ValueKind::Load { var }) = values.get(end).map(|v| &v.kind)
            && let Some(bound) = self.resolve_bound_value_id(var)
            && bound != end
        {
            return self.idx_cube_row_size_expr(start, bound, values, params);
        }
        let ValueKind::Call {
            callee: start_callee,
            args: start_args,
            names: start_names,
        } = &values.get(start)?.kind
        else {
            return None;
        };
        let ValueKind::Call {
            callee: end_callee,
            args: end_args,
            names: end_names,
        } = &values.get(end)?.kind
        else {
            return None;
        };
        if start_callee != "rr_idx_cube_vec_i"
            || end_callee != "rr_idx_cube_vec_i"
            || start_args.len() != 4
            || end_args.len() != 4
            || start_names.iter().any(Option::is_some)
            || end_names.iter().any(Option::is_some)
        {
            return None;
        }
        let start_face = self.resolve_val(start_args[0], values, params, false);
        let end_face = self.resolve_val(end_args[0], values, params, false);
        let start_x = self.resolve_val(start_args[1], values, params, false);
        let end_x = self.resolve_val(end_args[1], values, params, false);
        let start_size = self.resolve_val(start_args[3], values, params, false);
        let end_size = self.resolve_val(end_args[3], values, params, false);
        if start_face != end_face || start_x != end_x || start_size != end_size {
            return None;
        }
        if !self.value_is_known_one(start_args[2], values) {
            return None;
        }
        let end_y = self.resolve_val(end_args[2], values, params, false);
        if end_y != start_size {
            return None;
        }
        Some(start_size)
    }

    fn value_matches_known_length_expr(
        &self,
        val_id: usize,
        target_end_expr: &str,
        values: &[Value],
        params: &[String],
    ) -> bool {
        if self
            .resolve_known_full_end_expr(val_id, values, params)
            .as_deref()
            == Some(target_end_expr)
        {
            return true;
        }
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Load { var }) => {
                self.resolve_bound_value_id(var).is_some_and(|bound| {
                    self.value_matches_known_length_expr(bound, target_end_expr, values, params)
                }) || self.resolve_val(val_id, values, params, false) == target_end_expr
            }
            Some(ValueKind::Param { index }) => {
                self.resolve_param(*index, params) == target_end_expr
            }
            Some(ValueKind::Call { args, .. }) | Some(ValueKind::Intrinsic { args, .. }) => {
                args.iter().any(|arg| {
                    self.value_matches_known_length_expr(*arg, target_end_expr, values, params)
                }) || (args.iter().any(|arg| {
                    self.value_can_be_allocator_scalar_arg(*arg, values)
                        && self.resolve_val(*arg, values, params, false) == target_end_expr
                }) && args
                    .iter()
                    .any(|arg| !self.value_can_be_allocator_scalar_arg(*arg, values)))
            }
            _ => false,
        }
    }

    fn rep_int_matches_slice_len(
        &self,
        val_id: usize,
        start: i64,
        end: i64,
        values: &[Value],
    ) -> bool {
        let expected = end - start + 1;
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rep.int" && args.len() >= 2 =>
            {
                self.const_index_int_value(args[1], values) == Some(expected)
            }
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .is_some_and(|bound| self.rep_int_matches_slice_len(bound, start, end, values)),
            _ => false,
        }
    }

    fn try_resolve_whole_range_call_map_rhs(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
            return None;
        };
        if *callee != "rr_call_map_slice_auto" || args.len() < 7 {
            return None;
        }
        let dest_var = self.resolve_named_mutable_base_var(args[0], values, params)?;
        if dest_var != dst || !self.value_is_known_one(args[1], values) {
            return None;
        }
        if !self.value_is_full_dest_end(args[0], args[2], values, params, &mut FxHashSet::default())
            && !self.whole_dest_end_matches_known_var(dst, args[2], values, params)
        {
            return None;
        }
        let callee_name = self.const_string_value(args[3], values)?;
        let vector_slots = self.resolve_val(args[5], values, params, false);
        let helper_cost = self.resolve_val(args[4], values, params, false);
        let rendered_args: Vec<String> = args[6..]
            .iter()
            .map(|arg| {
                self.normalize_whole_range_vector_expr(
                    self.resolve_bound_temp_expr(*arg, values, params, &mut FxHashSet::default()),
                    args[1],
                    args[2],
                    values,
                    params,
                )
            })
            .collect();
        if self.direct_call_map_slots_supported(
            callee_name.as_str(),
            rendered_args.len(),
            args[5],
            values,
        ) && let Some(expr) =
            self.direct_whole_range_call_map_expr(callee_name.as_str(), &rendered_args)
        {
            return Some(expr);
        }
        Some(self.render_call_map_whole_auto_expr(
            dst,
            callee_name.as_str(),
            helper_cost.as_str(),
            vector_slots.as_str(),
            &rendered_args,
        ))
    }

    fn try_resolve_whole_auto_call_map_rhs(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
            return None;
        };
        if *callee != "rr_call_map_whole_auto" || args.len() < 5 {
            return None;
        }
        let dest_var = self.resolve_named_mutable_base_var(args[0], values, params)?;
        if dest_var != dst {
            return None;
        }
        let callee_name = self.const_string_value(args[1], values)?;
        let helper_cost = self.resolve_val(args[2], values, params, false);
        let vector_slots = self.resolve_val(args[3], values, params, false);
        let rendered_args: Vec<String> = args[4..]
            .iter()
            .map(|arg| {
                self.resolve_bound_temp_expr(*arg, values, params, &mut FxHashSet::default())
            })
            .collect();
        if self.direct_call_map_slots_supported(
            callee_name.as_str(),
            rendered_args.len(),
            args[3],
            values,
        ) && args[4..].iter().all(|arg| {
            !self.value_requires_runtime_auto_profit_guard(*arg, values, &mut FxHashSet::default())
        }) && let Some(expr) =
            self.direct_whole_range_call_map_expr(callee_name.as_str(), &rendered_args)
        {
            return Some(expr);
        }
        Some(self.render_call_map_whole_auto_expr(
            dst,
            callee_name.as_str(),
            helper_cost.as_str(),
            vector_slots.as_str(),
            &rendered_args,
        ))
    }

    fn try_resolve_mutated_whole_range_copy_alias(
        &self,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
            return None;
        };
        if *callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let base_var = self.resolve_named_mutable_base_var(args[0], values, params)?;
        if !self.value_is_known_one(args[1], values) {
            return None;
        }
        if !self.value_is_full_dest_end(args[0], args[2], values, params, &mut FxHashSet::default())
            && !self.whole_dest_end_matches_known_var(base_var.as_str(), args[2], values, params)
        {
            return None;
        }
        let rhs = self.normalize_whole_range_vector_expr(
            self.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default()),
            args[1],
            args[2],
            values,
            params,
        );
        if !Self::is_plain_symbol_expr(rhs.as_str()) || rhs == base_var {
            return None;
        }
        self.resolve_mutated_descendant_var(src)
            .filter(|var| var != &base_var && var != &rhs)
    }

    fn resolve_bound_temp_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> String {
        if !seen.insert(val_id) {
            return self.resolve_val(val_id, values, params, false);
        }
        if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind)
            && let Some(alias) = self.resolve_readonly_arg_alias_name(var, values)
        {
            return alias;
        }
        if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind)
            && var.starts_with('.')
            && let Some(bound) = self.resolve_bound_value_id(var)
            && bound != val_id
        {
            return self.resolve_bound_temp_expr(bound, values, params, seen);
        }
        if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind)
            && let Some(stripped) = var.strip_prefix(".arg_")
            && self.current_var_version(var) <= 1
            && !stripped.is_empty()
        {
            return stripped.to_string();
        }
        if let Some(bound) = self.resolve_bound_value(val_id)
            && !bound.starts_with('.')
        {
            return bound;
        }
        self.rewrite_live_readonly_arg_aliases(
            self.resolve_val(val_id, values, params, true),
            values,
        )
    }

    fn resolve_named_mutable_base_var(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        if let Some(var) =
            Self::named_mutable_base_expr(val_id, values, &self.value_bindings, &self.var_versions)
        {
            return Some(var);
        }
        let rendered = self.resolve_mutable_base(val_id, values, params);
        Self::is_plain_symbol_expr(rendered.as_str()).then_some(rendered)
    }

    fn resolve_mutated_descendant_var(&self, val_id: usize) -> Option<String> {
        let mut candidate: Option<String> = None;
        for (var, (bound_val_id, version)) in &self.var_value_bindings {
            if *bound_val_id != val_id {
                continue;
            }
            if self.current_var_version(var) <= *version {
                continue;
            }
            if candidate.is_some() {
                return None;
            }
            candidate = Some(var.clone());
        }
        candidate
    }

    fn is_plain_symbol_expr(expr: &str) -> bool {
        !expr.is_empty()
            && expr
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
    }

    fn direct_call_map_slots_supported(
        &self,
        callee_name: &str,
        arg_count: usize,
        vector_slots_val: usize,
        values: &[Value],
    ) -> bool {
        let Some(slots) = self.const_int_vector_values(vector_slots_val, values) else {
            return false;
        };
        match (callee_name, arg_count) {
            ("abs" | "log" | "sqrt", 1) => slots == [1],
            ("pmax" | "pmin", 2) => {
                !slots.is_empty()
                    && slots.len() <= 2
                    && slots.iter().all(|slot| matches!(*slot, 1 | 2))
                    && slots.windows(2).all(|w| w[0] < w[1])
            }
            _ => false,
        }
    }

    fn const_int_vector_values(&self, val_id: usize, values: &[Value]) -> Option<Vec<i64>> {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Call { callee, args, .. }) if callee == "c" => args
                .iter()
                .map(|arg| self.const_int_value(*arg, values))
                .collect(),
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.const_int_vector_values(bound, values)),
            _ => None,
        }
    }

    fn const_int_value(&self, val_id: usize, values: &[Value]) -> Option<i64> {
        self.const_int_value_impl(val_id, values, &mut FxHashSet::default())
    }

    fn const_int_value_impl(
        &self,
        val_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> Option<i64> {
        if !seen.insert(val_id) {
            return None;
        }
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Const(Lit::Int(v))) => Some(*v),
            Some(ValueKind::Const(Lit::Float(v)))
                if v.is_finite()
                    && (*v - v.trunc()).abs() < f64::EPSILON
                    && *v >= i64::MIN as f64
                    && *v <= i64::MAX as f64 =>
            {
                Some(*v as i64)
            }
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.const_int_value_impl(bound, values, seen)),
            Some(ValueKind::Unary {
                op: UnaryOp::Neg,
                rhs,
            }) => self.const_int_value_impl(*rhs, values, seen).map(|v| -v),
            Some(ValueKind::Binary { op, lhs, rhs }) => {
                let lhs = self.const_int_value_impl(*lhs, values, seen)?;
                let rhs = self.const_int_value_impl(*rhs, values, seen)?;
                match op {
                    BinOp::Add => Some(lhs.saturating_add(rhs)),
                    BinOp::Sub => Some(lhs.saturating_sub(rhs)),
                    BinOp::Mul => Some(lhs.saturating_mul(rhs)),
                    BinOp::Div if rhs != 0 && lhs % rhs == 0 => Some(lhs / rhs),
                    BinOp::Mod if rhs != 0 => Some(lhs % rhs),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn const_index_int_value(&self, val_id: usize, values: &[Value]) -> Option<i64> {
        self.const_int_value(val_id, values)
    }

    fn value_requires_runtime_auto_profit_guard(
        &self,
        val_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(val_id) {
            return false;
        }
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Const(_))
            | Some(ValueKind::Param { .. })
            | Some(ValueKind::RSymbol { .. }) => false,
            Some(ValueKind::Phi { args }) => args
                .iter()
                .any(|(arg, _)| self.value_requires_runtime_auto_profit_guard(*arg, values, seen)),
            Some(ValueKind::Len { base })
            | Some(ValueKind::Indices { base })
            | Some(ValueKind::Unary { rhs: base, .. }) => {
                self.value_requires_runtime_auto_profit_guard(*base, values, seen)
            }
            Some(ValueKind::Range { start, end }) => {
                self.value_requires_runtime_auto_profit_guard(*start, values, seen)
                    || self.value_requires_runtime_auto_profit_guard(*end, values, seen)
            }
            Some(ValueKind::Binary { lhs, rhs, .. }) => {
                self.value_requires_runtime_auto_profit_guard(*lhs, values, seen)
                    || self.value_requires_runtime_auto_profit_guard(*rhs, values, seen)
            }
            Some(ValueKind::Call { callee, args, .. }) => {
                callee.starts_with("rr_")
                    || args.iter().any(|arg| {
                        self.value_requires_runtime_auto_profit_guard(*arg, values, seen)
                    })
            }
            Some(ValueKind::Intrinsic { args, .. }) => args
                .iter()
                .any(|arg| self.value_requires_runtime_auto_profit_guard(*arg, values, seen)),
            Some(ValueKind::Index1D { .. })
            | Some(ValueKind::Index2D { .. })
            | Some(ValueKind::Index3D { .. }) => true,
            Some(ValueKind::Load { var }) => {
                self.resolve_bound_value_id(var).is_some_and(|bound| {
                    self.value_requires_runtime_auto_profit_guard(bound, values, seen)
                })
            }
            None => false,
        }
    }

    fn direct_whole_range_call_map_expr(
        &self,
        callee_name: &str,
        rendered_args: &[String],
    ) -> Option<String> {
        let rendered_args: Vec<String> = rendered_args
            .iter()
            .map(|arg| self.wrap_backend_builtin_expr(arg))
            .collect();
        match (
            callee_name,
            rendered_args.as_slice(),
            self.direct_builtin_vector_math,
        ) {
            ("pmax", [lhs, rhs], true) => Some(format!("pmax({lhs}, {rhs})")),
            ("pmin", [lhs, rhs], true) => Some(format!("pmin({lhs}, {rhs})")),
            ("abs", [arg], true) => Some(format!("abs({arg})")),
            ("log", [arg], true) => Some(format!("log({arg})")),
            ("sqrt", [arg], true) => Some(format!("sqrt({arg})")),
            ("pmax", [lhs, rhs], false) => Some(format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})")),
            ("pmin", [lhs, rhs], false) => Some(format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})")),
            ("abs", [arg], false) => Some(format!("rr_intrinsic_vec_abs_f64({arg})")),
            ("log", [arg], false) => Some(format!("rr_intrinsic_vec_log_f64({arg})")),
            ("sqrt", [arg], false) => Some(format!("rr_intrinsic_vec_sqrt_f64({arg})")),
            _ => None,
        }
    }

    fn render_call_map_whole_auto_expr(
        &self,
        dest: &str,
        callee_name: &str,
        helper_cost: &str,
        vector_slots: &str,
        rendered_args: &[String],
    ) -> String {
        let mut args = Vec::with_capacity(4 + rendered_args.len());
        args.push(dest.to_string());
        args.push(format!("\"{}\"", callee_name));
        args.push(helper_cost.to_string());
        args.push(vector_slots.to_string());
        args.extend(rendered_args.iter().cloned());
        format!("rr_call_map_whole_auto({})", args.join(", "))
    }

    fn const_string_value(&self, val_id: usize, values: &[Value]) -> Option<String> {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Const(Lit::Str(s))) => Some(s.clone()),
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.const_string_value(bound, values)),
            _ => None,
        }
    }

    fn normalize_whole_range_vector_expr(
        &self,
        expr: String,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let mut normalized =
            self.rewrite_known_full_range_index_reads(&expr, start, end, values, params);
        if normalized.contains("rr_ifelse_strict(") && !normalized.contains("rr_index1_read_vec(") {
            normalized = normalized.replace("rr_ifelse_strict(", "ifelse(");
        }
        normalized =
            self.rewrite_known_one_based_full_range_alias_reads(&normalized, values, params);
        normalized
    }

    fn wrap_backend_builtin_expr(&self, expr: &str) -> String {
        if self.direct_builtin_vector_math {
            return expr.trim().to_string();
        }
        let trimmed = expr.trim();
        if let Some(inner) = trimmed
            .strip_prefix("abs(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return format!("rr_intrinsic_vec_abs_f64({inner})");
        }
        if let Some(inner) = trimmed
            .strip_prefix("log(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return format!("rr_intrinsic_vec_log_f64({inner})");
        }
        if let Some(inner) = trimmed
            .strip_prefix("sqrt(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return format!("rr_intrinsic_vec_sqrt_f64({inner})");
        }
        trimmed.to_string()
    }

    fn rewrite_known_one_based_full_range_alias_reads(
        &self,
        expr: &str,
        values: &[Value],
        params: &[String],
    ) -> String {
        let pattern = format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>[^\)]*)\)",
            IDENT_PATTERN
        );
        let Some(re) = compile_regex(pattern) else {
            return expr.to_string();
        };
        re.replace_all(expr, |caps: &Captures<'_>| {
            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
            let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
            let Some(end_expr) = self.known_full_end_expr_for_var(base) else {
                return caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
            };
            if Self::expr_is_one_based_full_range_for_end(idx_expr, end_expr) {
                return base.to_string();
            }
            let Some(alias_name) = Self::extract_one_based_alias_name(idx_expr) else {
                return caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
            };
            let is_full = self
                .resolve_temp_bound_value_id(alias_name.as_str())
                .is_some_and(|bound| {
                    self.value_is_one_based_full_range_alias(
                        bound,
                        end_expr,
                        values,
                        params,
                        &mut FxHashSet::default(),
                    )
                });
            if is_full {
                base.to_string()
            } else {
                caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
            }
        })
        .to_string()
    }

    fn expr_is_one_based_full_range_for_end(idx_expr: &str, end_expr: &str) -> bool {
        let idx = idx_expr
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        let end = end_expr
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        ["1L", "1", "1.0", "1.0L"].iter().any(|start| {
            idx == format!("{start}:{end}") || idx == format!("rr_index_vec_floor({start}:{end})")
        })
    }

    fn extract_one_based_alias_name(idx_expr: &str) -> Option<String> {
        let trimmed = idx_expr.trim();
        if let Some(re) = compile_regex(format!(r"^{}$", IDENT_PATTERN))
            && re.is_match(trimmed)
        {
            return Some(trimmed.to_string());
        }
        if let Some(inner) = trimmed
            .strip_prefix("rr_index_vec_floor(")
            .and_then(|s| s.strip_suffix(')'))
            && let Some(re) = compile_regex(format!(r"^{}$", IDENT_PATTERN))
            && re.is_match(inner.trim())
        {
            return Some(inner.trim().to_string());
        }
        None
    }

    fn value_is_full_dest_end(
        &self,
        base: usize,
        end: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(base) {
            return false;
        }
        let end_rendered = self.resolve_val(end, values, params, false);
        let end_canonical = self.resolve_known_full_end_expr(end, values, params);
        let ok = match values.get(base).map(|v| &v.kind) {
            Some(ValueKind::Call { callee, args, .. })
                if self.call_is_known_fresh_allocation(callee)
                    && self
                        .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                        .is_some() =>
            {
                let len_idx = self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .unwrap_or(0);
                let len_rendered = self.resolve_val(args[len_idx], values, params, false);
                len_rendered == end_rendered
                    || self
                        .resolve_known_full_end_expr(args[len_idx], values, params)
                        .zip(end_canonical.as_ref())
                        .is_some_and(|(lhs, rhs)| lhs == *rhs)
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_assign_slice" && !args.is_empty() =>
            {
                self.value_is_full_dest_end(args[0], end, values, params, seen)
            }
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .is_some_and(|bound| self.value_is_full_dest_end(bound, end, values, params, seen)),
            Some(ValueKind::Len { base: len_base }) => {
                self.resolve_val(*len_base, values, params, false) == end_rendered
                    || self
                        .resolve_known_full_end_expr(*len_base, values, params)
                        .zip(end_canonical.as_ref())
                        .is_some_and(|(lhs, rhs)| lhs == *rhs)
            }
            _ => false,
        };
        seen.remove(&base);
        ok
    }

    fn rewrite_known_full_range_index_reads(
        &self,
        expr: &str,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let mut out = expr.to_string();

        let start_exprs = self.full_range_start_spellings(start, values, params);
        let end_expr = regex::escape(self.resolve_val(end, values, params, false).trim());
        for start_expr in start_exprs {
            let escaped_start = regex::escape(start_expr.trim());
            for pattern in [
                format!(
                    r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*:\s*{}\)",
                    IDENT_PATTERN, escaped_start, end_expr
                ),
                format!(
                    r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*rr_index_vec_floor\(\s*{}\s*:\s*{}\s*\)\)",
                    IDENT_PATTERN, escaped_start, end_expr
                ),
            ] {
                if let Some(re) = compile_regex(pattern) {
                    out = re
                        .replace_all(&out, |caps: &Captures<'_>| {
                            caps.name("base")
                                .map(|m| m.as_str())
                                .unwrap_or("")
                                .to_string()
                        })
                        .to_string();
                }
            }
        }

        for (var, (val_id, version)) in &self.var_value_bindings {
            let temp_stale_ok =
                var.starts_with(".__rr_cse_") || var.starts_with(".tachyon_exprmap");
            if !var.starts_with('.')
                || (self.current_var_version(var) != *version && !temp_stale_ok)
            {
                continue;
            }
            if !self.value_is_full_range_alias(*val_id, start, end, values, params) {
                continue;
            }
            let pattern = format!(
                r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*\)",
                IDENT_PATTERN,
                regex::escape(var),
            );
            if let Some(re) = compile_regex(pattern) {
                out = re
                    .replace_all(&out, |caps: &Captures<'_>| {
                        caps.name("base")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string()
                    })
                    .to_string();
            }
        }

        out
    }

    fn full_range_start_spellings(
        &self,
        start: usize,
        values: &[Value],
        params: &[String],
    ) -> Vec<String> {
        let mut out = Vec::new();
        let rendered = self.resolve_val(start, values, params, false);
        out.push(rendered);
        for one in ["1L", "1", "1.0"] {
            if !out.iter().any(|s| s == one) && self.value_is_known_one(start, values) {
                out.push(one.to_string());
            }
        }
        out
    }

    fn value_is_known_one(&self, val_id: usize, values: &[Value]) -> bool {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Const(Lit::Int(1))) => true,
            Some(ValueKind::Const(Lit::Float(f))) if (*f - 1.0).abs() <= f64::EPSILON => true,
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .is_some_and(|bound| self.value_is_known_one(bound, values)),
            _ => false,
        }
    }

    fn value_is_full_range_alias(
        &self,
        val_id: usize,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> bool {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Range { start: s, end: e }) => {
                self.value_is_known_one(*s, values)
                    && self.resolve_val(*e, values, params, false)
                        == self.resolve_val(end, values, params, false)
                    && self.value_is_known_one(start, values)
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_index_vec_floor" && args.len() == 1 =>
            {
                self.value_is_full_range_alias(args[0], start, end, values, params)
            }
            Some(ValueKind::Load { var }) => {
                self.resolve_bound_value_id(var).is_some_and(|bound| {
                    self.value_is_full_range_alias(bound, start, end, values, params)
                })
            }
            _ => false,
        }
    }

    fn value_is_one_based_full_range_alias(
        &self,
        val_id: usize,
        end_expr: &str,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(val_id) {
            return false;
        }
        let ok = match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Range { start, end }) => {
                self.value_is_known_one(*start, values)
                    && self.resolve_val(*end, values, params, false) == end_expr
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_index_vec_floor" && args.len() == 1 =>
            {
                self.value_is_one_based_full_range_alias(args[0], end_expr, values, params, seen)
            }
            Some(ValueKind::Load { var }) => {
                self.resolve_temp_bound_value_id(var).is_some_and(|bound| {
                    self.value_is_one_based_full_range_alias(bound, end_expr, values, params, seen)
                })
            }
            _ => false,
        };
        seen.remove(&val_id);
        ok
    }

    fn resolve_intrinsic_expr(
        &self,
        op: IntrinsicOp,
        args: &[usize],
        values: &[Value],
        params: &[String],
    ) -> String {
        let has_matrix_arg = args
            .iter()
            .any(|arg| values[*arg].value_ty.shape == ShapeTy::Matrix);
        if self.direct_builtin_vector_math && !has_matrix_arg {
            let resolved: Vec<String> = args
                .iter()
                .map(|arg| self.resolve_val(*arg, values, params, false))
                .collect();
            return match op {
                IntrinsicOp::VecAddF64 => format!("({} + {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSubF64 => format!("({} - {})", resolved[0], resolved[1]),
                IntrinsicOp::VecMulF64 => format!("({} * {})", resolved[0], resolved[1]),
                IntrinsicOp::VecDivF64 => format!("({} / {})", resolved[0], resolved[1]),
                IntrinsicOp::VecAbsF64 => format!("abs({})", resolved[0]),
                IntrinsicOp::VecLogF64 => format!("log({})", resolved[0]),
                IntrinsicOp::VecSqrtF64 => format!("sqrt({})", resolved[0]),
                IntrinsicOp::VecPmaxF64 => format!("pmax({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecPminF64 => format!("pmin({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSumF64 => format!("sum({})", resolved[0]),
                IntrinsicOp::VecMeanF64 => format!("mean({})", resolved[0]),
            };
        }
        if has_matrix_arg {
            let resolved: Vec<String> = args
                .iter()
                .map(|arg| self.resolve_val(*arg, values, params, false))
                .collect();
            return match op {
                IntrinsicOp::VecAddF64 => format!("({} + {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSubF64 => format!("({} - {})", resolved[0], resolved[1]),
                IntrinsicOp::VecMulF64 => format!("({} * {})", resolved[0], resolved[1]),
                IntrinsicOp::VecDivF64 => format!("({} / {})", resolved[0], resolved[1]),
                IntrinsicOp::VecAbsF64 => format!("abs({})", resolved[0]),
                IntrinsicOp::VecLogF64 => format!("log({})", resolved[0]),
                IntrinsicOp::VecSqrtF64 => format!("sqrt({})", resolved[0]),
                IntrinsicOp::VecPmaxF64 => format!("pmax({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecPminF64 => format!("pmin({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSumF64 => format!("sum({})", resolved[0]),
                IntrinsicOp::VecMeanF64 => format!("mean({})", resolved[0]),
            };
        }
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
        let b = self.resolve_read_base(base, values, params);
        if let Some(end_expr) = self.known_full_end_expr_for_value(base, values, params)
            && self.value_is_one_based_full_range_alias(
                idx,
                end_expr.as_str(),
                values,
                params,
                &mut FxHashSet::default(),
            )
        {
            return b;
        }
        let i = self.resolve_val(idx, values, params, false);
        if (is_safe && is_na_safe) || self.can_elide_index_expr(idx, values, params) {
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
        let b = self.resolve_read_base(base, values, params);
        let rr = self.resolve_val(r, values, params, false);
        let cc = self.resolve_val(c, values, params, false);
        let r_idx = if self.can_elide_index_expr(r, values, params) {
            rr
        } else {
            format!("rr_index1_write({}, \"row\")", rr)
        };
        let c_idx = if self.can_elide_index_expr(c, values, params) {
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
        let b = self.resolve_read_base(base, values, params);
        let i_val = self.resolve_val(i, values, params, false);
        let j_val = self.resolve_val(j, values, params, false);
        let k_val = self.resolve_val(k, values, params, false);
        let i_idx = if self.can_elide_index_expr(i, values, params) {
            i_val
        } else {
            format!("rr_index1_write({}, \"dim1\")", i_val)
        };
        let j_idx = if self.can_elide_index_expr(j, values, params) {
            j_val
        } else {
            format!("rr_index1_write({}, \"dim2\")", j_val)
        };
        let k_idx = if self.can_elide_index_expr(k, values, params) {
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
            UnaryOp::Formula => "~",
        }
    }

    fn resolve_cond(&self, cond: usize, values: &[Value], params: &[String]) -> String {
        let c = self.resolve_val(cond, values, params, false);
        let typed_bool_scalar = matches!(values[cond].value_term, TypeTerm::Logical)
            && values[cond].value_ty.shape == ShapeTy::Scalar;
        if values[cond].value_ty.is_logical_scalar_non_na()
            || typed_bool_scalar
            || self.comparison_is_scalar_non_na(cond, values)
        {
            c
        } else {
            format!("rr_truthy1({}, \"condition\")", c)
        }
    }

    fn comparison_is_scalar_non_na(&self, cond: usize, values: &[Value]) -> bool {
        let ValueKind::Binary { op, lhs, rhs } = values[cond].kind else {
            return false;
        };
        if !matches!(
            op,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        ) {
            return false;
        }
        self.value_is_scalar_non_na(lhs, values) && self.value_is_scalar_non_na(rhs, values)
    }

    fn value_is_scalar_non_na(&self, value_id: usize, values: &[Value]) -> bool {
        let mut seen = FxHashSet::default();
        self.value_is_scalar_non_na_impl(value_id, values, &mut seen)
    }

    fn value_is_scalar_non_na_impl(
        &self,
        value_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(value_id) {
            return false;
        }
        let value = &values[value_id];
        let scalar_shape = value.value_ty.shape == ShapeTy::Scalar
            || value.facts.has(Facts::INT_SCALAR)
            || value.facts.has(Facts::BOOL_SCALAR);
        let non_na =
            value.value_ty.na == crate::typeck::NaTy::Never || value.facts.has(Facts::NON_NA);
        if scalar_shape && non_na {
            return true;
        }
        match &value.kind {
            ValueKind::Const(_) => true,
            ValueKind::Load { var } => self
                .resolve_bound_value_id(var)
                .filter(|bound_id| *bound_id != value_id)
                .is_some_and(|bound_id| self.value_is_scalar_non_na_impl(bound_id, values, seen)),
            ValueKind::Unary { rhs, .. } => self.value_is_scalar_non_na_impl(*rhs, values, seen),
            ValueKind::Binary { op, lhs, rhs } => match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul => {
                    self.value_is_scalar_non_na_impl(*lhs, values, seen)
                        && self.value_is_scalar_non_na_impl(*rhs, values, seen)
                }
                BinOp::Div | BinOp::Mod => {
                    self.value_is_scalar_non_na_impl(*lhs, values, seen)
                        && self.value_is_scalar_non_na_impl(*rhs, values, seen)
                        && self.value_is_proven_non_zero(*rhs, values)
                }
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => {
                    self.value_is_scalar_non_na_impl(*lhs, values, seen)
                        && self.value_is_scalar_non_na_impl(*rhs, values, seen)
                }
                _ => false,
            },
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                names.iter().all(|name| name.is_none())
                    && args.len() == 1
                    && matches!(callee.as_str(), "floor" | "ceiling" | "trunc" | "abs")
                    && self.value_is_scalar_non_na_impl(args[0], values, seen)
            }
            _ => false,
        }
    }

    fn value_is_proven_non_zero(&self, value_id: usize, values: &[Value]) -> bool {
        let mut seen = FxHashSet::default();
        self.value_is_proven_non_zero_impl(value_id, values, &mut seen)
    }

    fn value_is_proven_non_zero_impl(
        &self,
        value_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(value_id) {
            return false;
        }
        match &values[value_id].kind {
            ValueKind::Const(Lit::Int(v)) => *v != 0,
            ValueKind::Const(Lit::Float(v)) => *v != 0.0,
            ValueKind::Load { var } => self
                .resolve_bound_value_id(var)
                .filter(|bound_id| *bound_id != value_id)
                .is_some_and(|bound_id| self.value_is_proven_non_zero_impl(bound_id, values, seen)),
            ValueKind::Unary {
                op: UnaryOp::Neg,
                rhs,
            } => self.value_is_proven_non_zero_impl(*rhs, values, seen),
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs,
                rhs,
            } => {
                self.value_is_proven_non_zero_impl(*lhs, values, seen)
                    && self.value_is_proven_non_zero_impl(*rhs, values, seen)
            }
            _ => false,
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
        if v.facts.has(Facts::INT_SCALAR | Facts::NON_NA) && v.facts.interval.min >= 1 {
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
            ValueKind::Call { callee, args, .. }
                if (callee == "rr_wrap_index_vec_i" && (args.len() == 4 || args.len() == 5))
                    || (callee == "rr_idx_cube_vec_i" && (args.len() == 4 || args.len() == 5)) =>
            {
                true
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } if callee == "rr_idx_cube_vec_i"
                && (args.len() == 4 || args.len() == 5)
                && names.iter().take(4).all(std::option::Option::is_none) =>
            {
                true
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } if callee == "rr_wrap_index_vec_i"
                && (args.len() == 4 || args.len() == 5)
                && names.iter().take(4).all(std::option::Option::is_none) =>
            {
                true
            }
            _ => false,
        }
    }

    fn emit_lit(&self, lit: &Lit) -> String {
        match lit {
            Lit::Int(i) => format!("{}L", i),
            Lit::Float(f) => self.emit_float_lit(*f),
            Lit::Str(s) => format!("\"{}\"", s),
            Lit::Bool(true) => "TRUE".to_string(),
            Lit::Bool(false) => "FALSE".to_string(),
            Lit::Null => "NULL".to_string(),
            Lit::Na => "NA".to_string(),
        }
    }

    fn emit_lit_with_value(&self, lit: &Lit, value: &Value) -> String {
        match lit {
            Lit::Float(f)
                if value.value_ty.prim == PrimTy::Double
                    || matches!(value.value_term, TypeTerm::Double) =>
            {
                self.emit_float_lit(*f)
            }
            _ => self.emit_lit(lit),
        }
    }

    fn emit_float_lit(&self, value: f64) -> String {
        let mut rendered = value.to_string();
        if value.is_finite() && !rendered.contains(['.', 'e', 'E']) {
            rendered.push_str(".0");
        }
        rendered
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

    fn rewrite_safe_scalar_loop_index_helpers(output: &mut String) {
        let Some(assign_re) =
            compile_regex(format!(r"^(?P<lhs>{}) <- (?P<rhs>.+)$", IDENT_PATTERN))
        else {
            return;
        };
        let Some(guard_re) = compile_regex(format!(
            r"^if \(!\((?P<var>{}) (?P<op><|<=) (?P<bound>{})\)\) break$",
            IDENT_PATTERN, IDENT_PATTERN
        )) else {
            return;
        };
        let Some(read_re) = compile_regex(format!(
            r#"rr_index1_read\((?P<base>{}),\s*(?P<idx>\([^)]*\)|{})\s*,\s*(?:"index"|'index')\)"#,
            IDENT_PATTERN, IDENT_PATTERN
        )) else {
            return;
        };
        let Some(write_re) = compile_regex(format!(
            r#"rr_index1_write\((?P<idx>{}),\s*(?:"index"|'index')\)"#,
            IDENT_PATTERN
        )) else {
            return;
        };
        let mut lines: Vec<String> = output.lines().map(str::to_string).collect();
        let mut i = 0usize;
        while i + 3 < lines.len() {
            let init_line = lines[i].trim().to_string();
            let Some(init_caps) = assign_re.captures(&init_line) else {
                i += 1;
                continue;
            };
            let idx_var = init_caps
                .name("lhs")
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let init_rhs = init_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let Some(start_value) = init_rhs
                .trim_end_matches('L')
                .trim_end_matches('l')
                .parse::<i64>()
                .ok()
            else {
                i += 1;
                continue;
            };
            if start_value < 1 || lines[i + 1].trim() != "repeat {" {
                i += 1;
                continue;
            }
            let Some(guard_caps) = guard_re.captures(lines[i + 2].trim()) else {
                i += 1;
                continue;
            };
            if guard_caps
                .name("var")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                != idx_var
            {
                i += 1;
                continue;
            }
            let allow_plus_one = guard_caps
                .name("op")
                .map(|m| m.as_str())
                .is_some_and(|op| op == "<");
            let mut cursor = i + 3;
            while cursor < lines.len() {
                let trimmed = lines[cursor].trim();
                if trimmed == "}" {
                    break;
                }
                let rewritten = read_re
                    .replace_all(&lines[cursor], |caps: &Captures<'_>| {
                        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                        let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                        let compact = idx_expr
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .collect::<String>();
                        if compact == idx_var {
                            return format!("{base}[{idx_var}]");
                        }
                        let minus_one = format!("({idx_var}-1)");
                        if compact == minus_one && start_value >= 2 {
                            return format!("{base}[({idx_var} - 1)]");
                        }
                        let plus_one = format!("({idx_var}+1)");
                        if compact == plus_one && allow_plus_one {
                            return format!("{base}[({idx_var} + 1)]");
                        }
                        caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                    })
                    .to_string();
                let rewritten = write_re
                    .replace_all(&rewritten, |caps: &Captures<'_>| {
                        let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                        if idx_expr == idx_var {
                            idx_var.to_string()
                        } else {
                            caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                        }
                    })
                    .to_string();
                lines[cursor] = rewritten;
                cursor += 1;
            }
            i = cursor.saturating_add(1);
        }
        *output = lines.join("\n");
    }

    fn emit_mark(&mut self, span: Span, label: Option<&str>) {
        if span.start_line == 0 {
            return;
        }
        self.write_indent();
        let _ = label;
        self.write(&format!(
            "rr_mark({}L, {}L);",
            span.start_line, span.start_col
        ));
        self.newline();
    }
}

#[cfg(test)]
mod tests {
    use super::{ActiveScalarLoopIndex, RBackend, ScalarLoopCmp};
    use crate::mir::def::{
        BinOp, EscapeStatus, FnIR, Instr, Lit, Terminator, UnaryOp, Value, ValueKind,
    };
    use crate::mir::flow::Facts;
    use crate::mir::structurizer::StructuredBlock;
    use crate::typeck::{NaTy, PrimTy, ShapeTy, TypeState, TypeTerm};
    use crate::utils::Span;
    use rustc_hash::{FxHashMap, FxHashSet};

    fn backend_with_sym17_fresh() -> RBackend {
        RBackend::with_fresh_result_calls(FxHashSet::from_iter([String::from("Sym_17")]))
    }

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
    fn prune_dead_cse_temps_removes_unused_tachyon_callmap_temp() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  .tachyon_callmap_arg0_0 <- abs((x + y))",
            "  score <- pmax(abs((x + y)), 0.05)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(!output.contains(".tachyon_callmap_arg0_0 <-"));
        assert!(output.contains("# rr-cse-pruned"));
    }

    #[test]
    fn prune_dead_cse_temps_removes_unused_loop_seed_before_whole_assign() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  i_9 <- 1L",
            "  clean <- ifelse((score > 0.4), sqrt((score + 0.1)), ((score * 0.55) + 0.03))",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(!output.contains("i_9 <- 1L"));
        assert!(output.contains("# rr-cse-pruned"));
    }

    #[test]
    fn prune_dead_cse_temps_keeps_loop_seed_used_by_following_slice_assign() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  i <- 1L",
            "  out <- rr_assign_slice(out, i, length(xs), xs)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(output.contains("i <- 1L"));
        assert!(!output.contains("# rr-cse-pruned"));
    }

    #[test]
    fn prune_dead_cse_temps_removes_straight_line_dead_init_before_overwrite() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  y <- Sym_17(n, 0, 2)",
            "  tmp <- 0",
            "  y <- (a + b)",
            "  return(y)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(!output.contains("y <- Sym_17(n, 0, 2)"));
        assert!(output.contains("# rr-cse-pruned"));
        assert!(output.contains("y <- (a + b)"));
    }

    #[test]
    fn prune_dead_cse_temps_keeps_init_when_overwrite_is_not_straight_line() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  x <- rep.int(0, n)",
            "  if ((flag == 1)) {",
            "    x <- vals",
            "  } else {",
            "  }",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(output.contains("x <- rep.int(0, n)"));
    }

    #[test]
    fn prune_dead_cse_temps_keeps_init_when_overwrite_reads_previous_value() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  x <- rep.int(0, n)",
            "  x <- (x + p)",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(output.contains("x <- rep.int(0, n)"));
    }

    #[test]
    fn prune_dead_cse_temps_removes_globally_unused_scalar_init() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  ii <- 0",
            "  x <- y",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(!output.contains("ii <- 0"));
        assert!(output.contains("# rr-cse-pruned"));
    }

    #[test]
    fn prune_dead_cse_temps_keeps_scalar_init_that_is_later_used() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  ii <- 0",
            "  x <- (ii + 1)",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(output.contains("ii <- 0"));
    }

    #[test]
    fn prune_dead_cse_temps_compacts_adjacent_pruned_markers() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  a <- 0",
            "  b <- 0",
            "  c <- 0",
            "  x <- 1",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert_eq!(output.matches("# rr-cse-pruned").count(), 1);
    }

    #[test]
    fn prune_dead_cse_temps_does_not_treat_other_function_uses_as_live() {
        let mut output = [
            "Sym_a <- function() ",
            "{",
            "  ii <- 0",
            "  x <- 1",
            "  return(x)",
            "}",
            "",
            "Sym_b <- function() ",
            "{",
            "  ii <- 0",
            "  ii <- 1",
            "  return(ii)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(
            !output.contains("Sym_a <- function() \n{\n  ii <- 0"),
            "dead init in first function should be pruned even if the same symbol is used in a later function"
        );
        assert!(
            output.contains("Sym_b <- function() \n{\n  ii <- 0")
                || output.contains("Sym_b <- function() \n{\n  # rr-cse-pruned\n  ii <- 1"),
            "second function should remain structurally intact"
        );
    }

    #[test]
    fn prune_dead_cse_temps_removes_dead_pre_loop_init_overwritten_in_loop() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  i <- 1",
            "  x <- 0",
            "  repeat {",
            "    if (!(i <= n)) break",
            "    x <- vals[i]",
            "    y <- x",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(y)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(!output.contains("  x <- 0\n"));
        assert!(output.contains("# rr-cse-pruned"));
    }

    #[test]
    fn prune_dead_cse_temps_keeps_pre_loop_init_used_after_loop() {
        let mut output = [
            "Sym <- function() ",
            "{",
            "  i <- 1",
            "  x <- 0",
            "  repeat {",
            "    if (!(i <= n)) break",
            "    x <- vals[i]",
            "    break",
            "  }",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");
        RBackend::prune_dead_cse_temps(&mut output);
        assert!(output.contains("  x <- 0\n"));
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
    fn stale_cse_temp_still_rewrites_full_range_alias_reads() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Range { start: 1, end: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Int, true),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .known_full_end_exprs
            .insert("x".to_string(), "8L".to_string());
        backend
            .known_full_end_exprs
            .insert("p".to_string(), "8L".to_string());
        backend.note_var_write(".__rr_cse_218");
        backend.bind_var_to_value(".__rr_cse_218", 2);
        backend.note_var_write(".__rr_cse_218");

        let expr = "(rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))";
        let rewritten = backend.rewrite_known_one_based_full_range_alias_reads(expr, &values, &[]);

        assert_eq!(rewritten, "(x + (alpha * p))");
    }

    #[test]
    fn stale_cse_temp_allows_direct_full_range_read_elision() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Range { start: 1, end: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Int, true),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Load {
                    var: "x".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, true),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Load {
                    var: ".__rr_cse_218".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some(".__rr_cse_218".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Int, true),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .known_full_end_exprs
            .insert("x".to_string(), "8L".to_string());
        backend.note_var_write(".__rr_cse_218");
        backend.bind_var_to_value(".__rr_cse_218", 2);
        backend.note_var_write(".__rr_cse_218");

        let rendered = backend.resolve_call_expr(
            &values[0],
            "rr_index1_read_vec",
            &[3, 4],
            &[None, None],
            &values,
            &[],
        );

        assert_eq!(rendered, "x");
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

    #[test]
    fn resolve_val_prefers_current_var_after_indexed_store_mutates_origin() {
        let mut backend = RBackend::new();
        let seq = Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("p".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        };
        let n = Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(10)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        };
        let values = vec![seq, n];

        backend.note_var_write("p");
        backend.bind_value_to_var(0, "p");
        backend.bind_var_to_value("p", 0);
        backend.note_var_write("p");

        let rendered = backend.resolve_val(0, &values, &[], false);
        assert_eq!(rendered, "p");
    }

    #[test]
    fn stale_fresh_alloc_is_rendered_as_current_var_in_call_args() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![2, 1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("r".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![0, 4, 2, 5],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("r".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Load {
                    var: "b".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "Sym_117".to_string(),
                    args: vec![0, 0, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("rs_old".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.note_var_write("r");
        backend.bind_value_to_var(0, "r");
        backend.bind_var_to_value("r", 0);
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "r".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("self-update assignment should emit");

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "rs_old".to_string(),
                    src: 6,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("dot assignment should emit");

        assert!(backend.output.contains("rs_old <- Sym_117(r, r, 8"));
    }

    #[test]
    fn stale_self_copy_assignment_is_skipped() {
        let mut backend = RBackend::new();
        let values = vec![Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "Sym_17".to_string(),
                args: vec![],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("adj_rr".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        }];

        backend.note_var_write("adj_rr");
        backend.bind_value_to_var(0, "adj_rr");
        backend.bind_var_to_value("adj_rr", 0);
        backend.note_var_write("adj_rr");

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "adj_rr".to_string(),
                    src: 0,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("assign emission should succeed");

        assert!(
            backend.output.trim().is_empty()
                || backend
                    .output
                    .lines()
                    .any(|line| line.trim() == "adj_rr <- Sym_17()")
        );
        assert!(
            !backend
                .output
                .lines()
                .any(|line| line.trim() == "adj_rr <- adj_rr")
        );
    }

    #[test]
    fn stale_fresh_aggregate_without_live_binding_falls_back_to_origin_var() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "seq_len".to_string(),
                    args: vec![1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("p".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(10)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.note_var_write("p");
        backend.bind_value_to_var(0, "p");
        backend.bind_var_to_value("p", 0);
        backend.note_var_write("p");
        backend.invalidate_var_binding("p");

        let rendered = backend.resolve_val(0, &values, &[], false);
        assert_eq!(rendered, "p");
    }

    #[test]
    fn same_kind_assignment_after_rhs_change_is_not_skipped() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "a".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("a".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "b".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.last_assigned_value_ids.insert("x".to_string(), 0);
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "x".to_string(),
                    src: 1,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("assign emission should succeed");

        assert!(
            backend.output.lines().any(|line| line.trim() == "x <- b"),
            "same-kind loads should still emit when the RHS source changed: {}",
            backend.output
        );
    }

    #[test]
    fn configured_user_fresh_call_is_treated_as_fresh_aggregate() {
        let mut backend = RBackend::with_fresh_result_calls(FxHashSet::from_iter([String::from(
            "Sym_custom_alloc",
        )]));
        let values = vec![Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "Sym_custom_alloc".to_string(),
                args: vec![1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("buf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        }];

        backend.note_var_write("buf");

        assert_eq!(
            backend.resolve_stale_origin_var(0, &values[0], &values),
            Some("buf".to_string())
        );
    }

    #[test]
    fn configured_user_fresh_call_counts_as_full_dest_end() {
        let backend = RBackend::with_fresh_result_calls(FxHashSet::from_iter([String::from(
            "Sym_custom_alloc",
        )]));
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "Sym_custom_alloc".to_string(),
                    args: vec![1, 2, 3],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("buf".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(10)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        assert!(backend.value_is_full_dest_end(0, 1, &values, &[], &mut FxHashSet::default()));
    }

    #[test]
    fn whole_range_call_map_slice_is_emitted_as_direct_vector_call() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![2, 1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Str("pmax".to_string())),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Const(Lit::Int(25)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "c".to_string(),
                    args: vec![3],
                    names: vec![None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Load {
                    var: "x".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Const(Lit::Float(0.05)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Call {
                    callee: "rr_call_map_slice_auto".to_string(),
                    args: vec![0, 3, 1, 4, 5, 6, 7, 8],
                    names: vec![None; 8],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "score");
        backend.bind_var_to_value("score", 0);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "score".to_string(),
                    src: 9,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("whole-range call-map emission should succeed");

        assert!(backend.output.contains("score <- pmax(x, 0.05)"));
        assert!(!backend.output.contains("rr_call_map_slice_auto("));
    }

    #[test]
    fn loop_stable_known_full_end_allows_whole_range_call_map_fold() {
        let mut backend = RBackend::new();
        backend
            .known_full_end_exprs
            .insert("score".to_string(), "n".to_string());

        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "score".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Load {
                    var: "i".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("i".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Str("pmax".to_string())),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Const(Lit::Int(25)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "c".to_string(),
                    args: vec![1],
                    names: vec![None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Load {
                    var: "x".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Const(Lit::Float(0.05)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Call {
                    callee: "rr_call_map_slice_auto".to_string(),
                    args: vec![0, 2, 3, 4, 5, 6, 7, 8],
                    names: vec![None; 8],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(1, "i");
        backend.bind_var_to_value("i", 1);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "score".to_string(),
                    src: 9,
                    span: Span::dummy(),
                },
                &values,
                &["n".to_string()],
            )
            .expect("loop-stable whole-range call-map emission should succeed");

        assert!(backend.output.contains("score <- pmax(x, 0.05)"));
        assert!(!backend.output.contains("rr_call_map_slice_auto("));
    }

    #[test]
    fn whole_range_rr_index1_read_vec_call_elides_to_base_expr() {
        let mut backend = RBackend::new();
        backend
            .known_full_end_exprs
            .insert("x".to_string(), "n".to_string());

        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "x".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Range { start: 2, end: 1 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Int, true),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![3],
                    names: vec![None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Int, true),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_call_expr(
            &values[0],
            "rr_index1_read_vec",
            &[0, 4],
            &[None, None],
            &values,
            &["n".to_string()],
        );
        assert_eq!(rendered, "x");
    }

    #[test]
    fn cube_index_scalar_read_elides_index_wrapper() {
        let backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "u".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("u".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(3)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Call {
                    callee: "rr_idx_cube_vec_i".to_string(),
                    args: vec![1, 2, 3, 1],
                    names: vec![None, None, None, None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Index1D {
                    base: 0,
                    idx: 4,
                    is_safe: false,
                    is_na_safe: false,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_val(5, &values, &[], false);
        assert_eq!(rendered, "u[rr_idx_cube_vec_i(1L, 2L, 3L, 1L)]");
    }

    #[test]
    fn wrap_index_scalar_read_elides_index_wrapper() {
        let backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "B".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("B".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(32)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Call {
                    callee: "rr_wrap_index_vec_i".to_string(),
                    args: vec![1, 1, 1, 1],
                    names: vec![None, None, None, None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Index1D {
                    base: 0,
                    idx: 2,
                    is_safe: false,
                    is_na_safe: false,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_val(3, &values, &[], false);
        assert_eq!(rendered, "B[rr_wrap_index_vec_i(32L, 32L, 32L, 32L)]");
    }

    #[test]
    fn direct_rr_index1_read_call_elides_when_index_is_safe() {
        let backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "clean".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("clean".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(250000)),
                span: Span::dummy(),
                facts: Facts::new(
                    Facts::INT_SCALAR | Facts::NON_NA | Facts::ONE_BASED,
                    crate::mir::flow::Interval::point(250000),
                ),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_call_expr(
            &values[0],
            "rr_index1_read",
            &[0, 1],
            &[None, None],
            &values,
            &[],
        );
        assert_eq!(rendered, "clean[250000L]");
    }

    #[test]
    fn direct_rr_index1_read_call_elides_when_index_var_is_bound_to_safe_value() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "clean".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("clean".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "n".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(250000)),
                span: Span::dummy(),
                facts: Facts::new(
                    Facts::INT_SCALAR | Facts::NON_NA | Facts::ONE_BASED,
                    crate::mir::flow::Interval::point(250000),
                ),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];
        backend.bind_var_to_value("n", 2);

        let rendered = backend.resolve_call_expr(
            &values[0],
            "rr_index1_read",
            &[0, 1],
            &[None, None],
            &values,
            &[],
        );
        assert_eq!(rendered, "clean[n]");
    }

    #[test]
    fn index1d_expr_elides_when_index_var_is_bound_to_safe_value() {
        let mut backend = RBackend::new();
        backend
            .known_full_end_exprs
            .insert("clean".to_string(), "n".to_string());
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "clean".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("clean".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "n".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(250000)),
                span: Span::dummy(),
                facts: Facts::new(
                    Facts::INT_SCALAR | Facts::NON_NA | Facts::ONE_BASED,
                    crate::mir::flow::Interval::point(250000),
                ),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Index1D {
                    base: 0,
                    idx: 1,
                    is_safe: false,
                    is_na_safe: false,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
        ];
        backend.bind_var_to_value("n", 2);

        let rendered = backend.resolve_val(3, &values, &[], false);
        assert_eq!(rendered, "clean[n]");
    }

    #[test]
    fn active_scalar_loop_index_load_does_not_fold_to_seed_constant() {
        let mut backend = RBackend::new();
        backend
            .active_scalar_loop_indices
            .push(ActiveScalarLoopIndex {
                var: "i".to_string(),
                start_min: 1,
                cmp: ScalarLoopCmp::Le,
            });
        backend.bind_value_to_var(0, "i");
        backend.bind_var_to_value("i", 1);

        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "i".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("i".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, false),
                value_term: TypeTerm::Int,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, false),
                value_term: TypeTerm::Int,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_val(0, &values, &[], false);
        assert_eq!(rendered, "i");
    }

    #[test]
    fn index1d_expr_elides_when_loop_offset_is_proven_safe() {
        let mut backend = RBackend::new();
        backend
            .active_scalar_loop_indices
            .push(ActiveScalarLoopIndex {
                var: "i".to_string(),
                start_min: 2,
                cmp: ScalarLoopCmp::Lt,
            });
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "a".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("a".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "i".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("i".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Binary {
                    op: BinOp::Sub,
                    lhs: 1,
                    rhs: 2,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Index1D {
                    base: 0,
                    idx: 3,
                    is_safe: false,
                    is_na_safe: false,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_val(4, &values, &[], false);
        assert_eq!(rendered, "a[(i - 1L)]");
    }

    #[test]
    fn constant_safe_partial_self_assign_renders_direct_slice_write() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(192)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(88)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("i".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(104)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Binary {
                    op: BinOp::Sub,
                    lhs: 4,
                    rhs: 3,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 6,
                    rhs: 7,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![5, 8],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 10,
                kind: ValueKind::Load {
                    var: "b".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 11,
                kind: ValueKind::Load {
                    var: "i".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("i".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 12,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![10, 11, 4, 9],
                    names: vec![None; 4],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "b");
        backend.bind_var_to_value("b", 0);
        backend.bind_value_to_var(3, "i");
        backend.bind_var_to_value("i", 3);

        let rendered = backend
            .try_render_constant_safe_partial_self_assign("b", 12, &values, &[])
            .expect("constant partial fill should render as direct slice write");
        assert_eq!(rendered, "b[88:104] <- rep.int(1L, ((104L - i) + 1L))");
    }

    #[test]
    fn constant_safe_partial_self_assign_recovers_alias_base_var() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![1, 2],
                    names: vec![None; 2],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(192)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(88)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(104)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Load {
                    var: "i".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("i".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Binary {
                    op: BinOp::Sub,
                    lhs: 4,
                    rhs: 5,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 6,
                    rhs: 7,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, true),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![7, 8],
                    names: vec![None; 2],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Int, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 10,
                kind: ValueKind::Load {
                    var: ".tmp_b".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 11,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![10, 5, 4, 9],
                    names: vec![None; 4],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::vector(PrimTy::Double, false),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "b");
        backend.bind_var_to_value("b", 0);
        backend.bind_value_to_var(10, "b");
        backend.bind_value_to_var(3, "i");
        backend.bind_var_to_value("i", 3);

        let rendered = backend
            .try_render_constant_safe_partial_self_assign("b", 11, &values, &[])
            .expect("alias base should still render as direct slice write");
        assert_eq!(rendered, "b[88:104] <- rep.int(1L, ((104L - i) + 1L))");
    }

    #[test]
    fn whole_auto_builtin_call_map_is_emitted_as_direct_vector_call() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![2, 1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Str("pmax".to_string())),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(25)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Call {
                    callee: "c".to_string(),
                    args: vec![6],
                    names: vec![None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Load {
                    var: "x".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Const(Lit::Float(0.05)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Call {
                    callee: "rr_call_map_whole_auto".to_string(),
                    args: vec![0, 3, 4, 5, 7, 8],
                    names: vec![None; 6],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "score");
        backend.bind_var_to_value("score", 0);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "score".to_string(),
                    src: 9,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("whole-auto builtin call-map emission should succeed");

        assert!(backend.output.contains("score <- pmax(x, 0.05)"));
        assert!(!backend.output.contains("rr_call_map_whole_auto("));
    }

    #[test]
    fn whole_auto_pmax_zip_call_map_is_emitted_as_direct_vector_call() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![2, 1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Str("pmax".to_string())),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(44)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Call {
                    callee: "c".to_string(),
                    args: vec![6, 7],
                    names: vec![None, None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Load {
                    var: "x".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Load {
                    var: "z".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("z".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 10,
                kind: ValueKind::Call {
                    callee: "rr_call_map_whole_auto".to_string(),
                    args: vec![0, 3, 4, 5, 8, 9],
                    names: vec![None; 6],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "score");
        backend.bind_var_to_value("score", 0);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "score".to_string(),
                    src: 10,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("whole-auto pmax zip call-map emission should succeed");

        assert!(backend.output.contains("score <- pmax(x, z)"));
        assert!(!backend.output.contains("rr_call_map_whole_auto("));
    }

    #[test]
    fn whole_range_pmax_zip_call_map_slice_is_emitted_as_direct_vector_call() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![2, 1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Str("pmax".to_string())),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Const(Lit::Int(44)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "c".to_string(),
                    args: vec![3, 7],
                    names: vec![None, None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Load {
                    var: "x".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Load {
                    var: "z".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("z".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 10,
                kind: ValueKind::Call {
                    callee: "rr_call_map_slice_auto".to_string(),
                    args: vec![0, 3, 1, 4, 5, 6, 8, 9],
                    names: vec![None; 8],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("score".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "score");
        backend.bind_var_to_value("score", 0);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "score".to_string(),
                    src: 10,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("whole-range pmax zip call-map slice emission should succeed");

        assert!(backend.output.contains("score <- pmax(x, z)"));
        assert!(!backend.output.contains("rr_call_map_slice_auto("));
    }

    #[test]
    fn helper_heavy_whole_auto_builtin_call_map_stays_runtime_guarded() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("out".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(6)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Str("abs".to_string())),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(44)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Call {
                    callee: "c".to_string(),
                    args: vec![6],
                    names: vec![None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 7,
                kind: ValueKind::Load {
                    var: "src".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("src".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 8,
                kind: ValueKind::Load {
                    var: "idx".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("idx".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 9,
                kind: ValueKind::Call {
                    callee: "rr_gather".to_string(),
                    args: vec![7, 8],
                    names: vec![None, None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 10,
                kind: ValueKind::Call {
                    callee: "rr_call_map_whole_auto".to_string(),
                    args: vec![0, 3, 4, 5, 9],
                    names: vec![None; 5],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("out".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "out");
        backend.bind_var_to_value("out", 0);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "out".to_string(),
                    src: 10,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("helper-heavy whole-auto builtin call-map emission should succeed");

        assert!(backend.output.contains("out <- rr_call_map_whole_auto("));
        assert!(!backend.output.contains("out <- abs("));
    }

    #[test]
    fn whole_dest_end_known_var_matches_len_alias_expr() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("xs".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Len { base: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![2, 1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("out".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Len { base: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "n".to_string(),
                    src: 1,
                    span: Span::dummy(),
                },
                &values,
                &["xs".to_string()],
            )
            .expect("len alias assignment should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "out".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &["xs".to_string()],
            )
            .expect("fresh allocation assignment should emit");

        assert!(backend.whole_dest_end_matches_known_var("out", 4, &values, &["xs".to_string()]));
    }

    #[test]
    fn whole_range_sym17_allocator_like_assign_resolves_direct_rhs() {
        let mut backend = backend_with_sym17_fresh();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("size".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![0, 1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("y".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Param { index: 1 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("src".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![3, 4, 0, 5],
                    names: vec![None; 4],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("y".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "y".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &["size".to_string(), "src".to_string()],
            )
            .expect("fresh allocator assignment should emit");

        let rendered = backend
            .try_resolve_whole_range_self_assign_rhs(
                "y",
                6,
                &values,
                &["size".to_string(), "src".to_string()],
            )
            .expect("whole-range Sym_17 replay should resolve directly");
        assert_eq!(rendered, "src");
    }

    #[test]
    fn singleton_size_boundary_assign_collapses_rep_int_wrapper_to_scalar_rhs() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("size".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(6)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![1, 0],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("nf".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(5)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![3, 4],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![2, 0, 0, 5],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("nf".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "nf".to_string(),
                    src: 2,
                    span: Span::dummy(),
                },
                &values,
                &["size".to_string()],
            )
            .expect("base alloc should emit");

        let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(6, &values, &["size".to_string()]);
        assert_eq!(rendered, "replace(nf, size, 5L)");
    }

    #[test]
    fn callsite_seq_len_summary_allows_replace_at_size_boundary() {
        let mut summaries = FxHashMap::default();
        summaries.insert(
            "Sym_72".to_string(),
            FxHashMap::from_iter([(2usize, 3usize)]),
        );
        let mut backend = RBackend::with_analysis_options(FxHashSet::default(), summaries, false);
        backend.current_fn_name = "Sym_72".to_string();
        backend.current_seq_len_param_end_slots = FxHashMap::from_iter([(2usize, 3usize)]);

        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("f".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Param { index: 2 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("ys".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Len { base: 1 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("width".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![0, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("nf".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Param { index: 3 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("size".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Const(Lit::Int(5)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![3, 4, 4, 5],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("nf".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        let params = [
            "f".to_string(),
            "x".to_string(),
            "ys".to_string(),
            "size".to_string(),
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "nf".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &params,
            )
            .expect("base alloc should emit");

        let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(6, &values, &params);
        assert_eq!(rendered, "replace(nf, size, 5L)");
    }

    #[test]
    fn remember_known_full_end_expr_handles_self_referential_assign_slice_cycle() {
        let mut backend = RBackend::new();
        backend.note_var_write("temp");
        backend
            .known_full_end_exprs
            .insert("temp".to_string(), "n".to_string());

        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "temp".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("temp".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Len { base: 1 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("inlined_n".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![3, 0],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("inlined_out".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("i".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![4, 5, 2, 1],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("inlined_out".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(6, "temp");
        backend.bind_var_to_value("temp", 6);

        backend.remember_known_full_end_expr("temp", 6, &values, &["n".to_string()]);

        assert_eq!(
            backend.known_full_end_exprs.get("temp").map(String::as_str),
            Some("n")
        );
    }

    #[test]
    fn scalar_adjusted_end_expr_is_not_treated_as_full_end() {
        let backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
                origin_var: Some("n".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, false),
                value_term: TypeTerm::Int,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(-1)),
                span: Span::dummy(),
                facts: Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, false),
                value_term: TypeTerm::Int,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 1,
                },
                span: Span::dummy(),
                facts: Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Int, false),
                value_term: TypeTerm::Int,
                escape: EscapeStatus::Unknown,
            },
        ];

        assert_eq!(
            backend.known_full_end_expr_for_value(2, &values, &["n".to_string()]),
            None
        );
    }

    #[test]
    fn whole_dest_end_known_var_matches_param_alias_expr() {
        let mut backend = backend_with_sym17_fresh();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("size".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![0, 1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("x".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Param { index: 0 },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("size".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Load {
                    var: ".arg_size".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some(".arg_size".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: ".arg_size".to_string(),
                    src: 4,
                    span: Span::dummy(),
                },
                &values,
                &["size".to_string()],
            )
            .expect("param alias assignment should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "x".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &["size".to_string()],
            )
            .expect("fresh allocator assignment should emit");

        assert!(backend.whole_dest_end_matches_known_var("x", 5, &values, &["size".to_string()]));
    }

    #[test]
    fn whole_range_copy_wrapper_finds_mutated_descendant_alias() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("temp".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("inlined_9_n".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("inlined_9_out".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("inlined_9_i".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Load {
                    var: "temp".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("temp".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![3, 4, 2, 5],
                    names: vec![None; 4],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("temp".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "temp".to_string(),
                    src: 0,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("temp init should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "inlined_9_out".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("copy wrapper base init should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "inlined_9_out".to_string(),
                    src: 6,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("whole-range copy replay should lower to direct alias");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "next_temp".to_string(),
                    src: 6,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("next_temp copy alias should emit");

        backend.note_var_write("next_temp");

        let alias = backend
            .try_resolve_mutated_whole_range_copy_alias(6, &values, &[])
            .expect("mutated descendant alias should be recoverable");
        assert_eq!(alias, "next_temp");
    }

    #[test]
    fn stale_fresh_aggregate_call_arg_is_not_hoisted_to_cse_temp() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "rep.int".to_string(),
                    args: vec![1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("r".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(8)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![0, 4, 2, 5],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("r".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Load {
                    var: "b".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Call {
                    callee: "Sym_117".to_string(),
                    args: vec![0, 0, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("rs_old".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "r".to_string(),
                    src: 0,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("seed assign should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "r".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("slice update should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "rs_old".to_string(),
                    src: 6,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("dot assign should emit");

        assert!(backend.output.contains("rs_old <- Sym_117(r, r, 8"));
        assert!(!backend.output.contains(".__rr_cse_"));
    }

    #[test]
    fn scalar_stage_assignment_reuses_live_origin_var() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "sun".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("sun".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 0,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("next_cloud".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 0,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("next_cloud".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "next_cloud".to_string(),
                    src: 1,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("stage assign should emit");
        assert_eq!(
            backend.resolve_bound_value(1).as_deref(),
            Some("next_cloud")
        );
        assert_eq!(
            backend.resolve_stale_origin_var(2, &values[2], &values),
            Some("next_cloud".to_string())
        );
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "cloud".to_string(),
                    src: 2,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("copy from staged scalar should emit");

        assert!(backend.output.contains("next_cloud <- (sun + sun)"));
        assert!(
            backend
                .output
                .lines()
                .any(|line| line.trim() == "cloud <- next_cloud")
        );
        assert!(
            !backend
                .output
                .lines()
                .any(|line| line.trim() == "cloud <- (sun + sun)")
        );
    }

    #[test]
    fn scalar_stage_assignment_reuses_same_value_id_bound_var() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "sun".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("sun".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 0,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("next_sun".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "next_sun".to_string(),
                    src: 1,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("stage assign should emit");
        assert_eq!(backend.resolve_bound_value(1).as_deref(), Some("next_sun"));
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "sun".to_string(),
                    src: 1,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("copy from same staged scalar should emit");

        assert!(backend.output.contains("next_sun <- (sun + sun)"));
        assert!(
            backend
                .output
                .lines()
                .any(|line| line.trim() == "sun <- next_sun")
        );
        assert!(
            !backend
                .output
                .lines()
                .any(|line| line.trim() == "sun <- (sun + sun)")
        );
    }

    #[test]
    fn same_origin_assignment_uses_expr_instead_of_stale_bound_var() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("s".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Call {
                    callee: "sum".to_string(),
                    args: vec![2],
                    names: vec![None],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("s".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Load {
                    var: "xs".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("xs".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "s".to_string(),
                    src: 0,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("seed assign should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "s".to_string(),
                    src: 1,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("updated assign should emit");

        assert!(backend.output.lines().any(|line| line.trim() == "s <- 0L"));
        assert!(
            backend
                .output
                .lines()
                .any(|line| line.trim() == "s <- sum(xs)")
        );
        assert!(!backend.output.lines().any(|line| line.trim() == "s <- s"));
    }

    #[test]
    fn binary_expr_prefers_shared_origin_var_over_literal_clone() {
        let backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Const(Lit::Float(40.0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("N".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "N".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("N".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Binary {
                    op: BinOp::Mul,
                    lhs: 0,
                    rhs: 1,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("grid_sq".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_val(2, &values, &[], false);
        assert_eq!(rendered, "(N * N)");
    }

    #[test]
    fn binary_expr_prefers_live_scalar_origin_var_over_literal_clone() {
        let mut backend = RBackend::new();
        backend.var_versions.insert("N".to_string(), 1);
        backend.var_value_bindings.insert("N".to_string(), (0, 1));

        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Const(Lit::Float(40.0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("N".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "rem".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("rem".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Binary {
                    op: BinOp::Div,
                    lhs: 1,
                    rhs: 0,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("tmp_div".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Binary {
                    op: BinOp::Mod,
                    lhs: 1,
                    rhs: 0,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("tmp_mod".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered_div = backend.resolve_val(2, &values, &[], false);
        let rendered_mod = backend.resolve_val(3, &values, &[], false);
        assert_eq!(rendered_div, "(rem / N)");
        assert_eq!(rendered_mod, "(rem %% N)");
    }

    #[test]
    fn binary_expr_does_not_replace_literal_with_nonlive_origin_var_name() {
        let backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "ff".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("ff".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("ff".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Binary {
                    op: BinOp::Lt,
                    lhs: 0,
                    rhs: 1,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        let rendered = backend.resolve_val(2, &values, &[], false);
        assert_eq!(rendered, "(ff < 1L)");
    }

    #[test]
    fn const_seed_assignment_does_not_alias_to_mutable_bound_var() {
        let mut backend = RBackend::new();
        let values = vec![Value {
            id: 0,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        }];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "acc".to_string(),
                    src: 0,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("seed assign should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "i".to_string(),
                    src: 0,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("second seed assign should emit");

        assert!(backend.output.contains("acc <- 1L"));
        assert!(backend.output.contains("i <- 1L"));
        assert!(!backend.output.contains("i <- acc"));
    }

    #[test]
    fn redundant_self_replay_assignment_is_skipped() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "y".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("y".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "dy".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("dy".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 1,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("y".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 1,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("y".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.note_var_write("y");
        backend.bind_value_to_var(2, "y");
        backend.bind_var_to_value("y", 2);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "y".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("assign emission should succeed");

        assert!(backend.output.trim().is_empty());
    }

    #[test]
    fn divergent_branch_assignments_invalidate_pre_branch_binding() {
        let mut backend = RBackend::new();
        backend.var_versions.insert("s".to_string(), 1);
        backend.var_value_bindings.insert("s".to_string(), (10, 1));

        let mut then_versions = FxHashMap::default();
        then_versions.insert("s".to_string(), 2);
        let mut then_bindings = FxHashMap::default();
        then_bindings.insert("s".to_string(), (11, 2));

        let mut else_versions = FxHashMap::default();
        else_versions.insert("s".to_string(), 2);
        let mut else_bindings = FxHashMap::default();
        else_bindings.insert("s".to_string(), (12, 2));

        backend.join_branch_var_value_bindings(
            &then_versions,
            &then_bindings,
            &else_versions,
            &else_bindings,
        );

        assert_eq!(backend.var_versions.get("s").copied(), Some(2));
        assert!(!backend.var_value_bindings.contains_key("s"));
    }

    #[test]
    fn loop_merge_copy_of_current_acc_value_is_skipped_after_branch_join() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "j".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("j".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 1,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("j".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "Sym_1".to_string(),
                    args: vec![],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("s".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Load {
                    var: "acc".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("acc".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 4,
                    rhs: 3,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("acc".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 6,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 4,
                    rhs: 3,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("acc".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.note_var_write("acc");
        backend.bind_value_to_var(5, "acc");
        backend.bind_var_to_value("acc", 5);
        backend.note_var_write("s");
        backend.bind_value_to_var(3, "s");
        backend.bind_var_to_value("s", 3);

        let then_versions = backend.var_versions.clone();
        let then_bindings = backend.var_value_bindings.clone();
        let else_versions = backend.var_versions.clone();
        let else_bindings = backend.var_value_bindings.clone();
        backend.join_branch_var_value_bindings(
            &then_versions,
            &then_bindings,
            &else_versions,
            &else_bindings,
        );

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "j".to_string(),
                    src: 2,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("j update should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "acc".to_string(),
                    src: 6,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("acc merge copy should emit");

        let lines: Vec<_> = backend
            .output
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        assert!(lines.contains(&"j <- (j + 1L)".to_string()));
        assert!(!lines.contains(&"acc <- (acc + Sym_1())".to_string()));
        assert!(!lines.contains(&"acc <- (acc + s)".to_string()));
    }

    #[test]
    fn reassigning_current_bound_value_is_skipped() {
        let mut backend = RBackend::new();
        let values = vec![Value {
            id: 0,
            kind: ValueKind::Load {
                var: "y".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        }];

        backend.note_var_write("y");
        backend.bind_value_to_var(0, "y");
        backend.bind_var_to_value("y", 0);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "y".to_string(),
                    src: 0,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("assign emission should succeed");

        assert!(backend.output.trim().is_empty());
    }

    #[test]
    fn reassigning_same_expr_to_current_bound_var_is_skipped_even_without_origin_var() {
        let mut backend = RBackend::new();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![1, 2, 3],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("coriolis".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(10)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(3)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![1, 2, 3],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.note_var_write("coriolis");
        backend.bind_value_to_var(0, "coriolis");
        backend.bind_var_to_value("coriolis", 0);

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "coriolis".to_string(),
                    src: 4,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("assign emission should succeed");

        assert!(backend.output.trim().is_empty());
    }

    #[test]
    fn stale_fresh_clone_selection_is_deterministic_across_binding_insertion_order() {
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![3, 4, 5],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("beta".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![3, 4, 5],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("alpha".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![3, 4, 5],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Const(Lit::Int(10)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Const(Lit::Int(3)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
        ];

        let mut backend_a = backend_with_sym17_fresh();
        backend_a.var_versions.insert("beta".to_string(), 1);
        backend_a.var_versions.insert("alpha".to_string(), 1);
        backend_a.value_bindings.insert(0, ("beta".to_string(), 0));
        backend_a.value_bindings.insert(1, ("alpha".to_string(), 0));

        let mut backend_b = backend_with_sym17_fresh();
        backend_b.var_versions.insert("beta".to_string(), 1);
        backend_b.var_versions.insert("alpha".to_string(), 1);
        backend_b.value_bindings.insert(1, ("alpha".to_string(), 0));
        backend_b.value_bindings.insert(0, ("beta".to_string(), 0));

        assert_eq!(
            backend_a.resolve_stale_fresh_clone_var(2, &values[2], &values),
            Some("alpha".to_string())
        );
        assert_eq!(
            backend_b.resolve_stale_fresh_clone_var(2, &values[2], &values),
            Some("alpha".to_string())
        );
    }

    #[test]
    fn loop_local_reseed_is_not_skipped_when_var_is_mutated_in_loop() {
        let mut fn_ir = FnIR::new("loop_reset".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let two = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: one,
            span: Span::dummy(),
        });
        fn_ir.blocks[entry].term = Terminator::Unreachable;
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: entry,
        };
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: one,
            span: Span::dummy(),
        });
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: two,
            span: Span::dummy(),
        });
        fn_ir.blocks[body].term = Terminator::Unreachable;

        let structured = StructuredBlock::Sequence(vec![
            StructuredBlock::BasicBlock(entry),
            StructuredBlock::Loop {
                header,
                cond,
                continue_on_true: true,
                body: Box::new(StructuredBlock::BasicBlock(body)),
            },
        ]);

        let mut backend = RBackend::new();
        backend.current_fn_name = "loop_reset".to_string();
        backend
            .emit_structured(&structured, &fn_ir)
            .expect("structured loop emission should succeed");

        assert_eq!(backend.output.matches("i <- 1").count(), 2);
        assert!(backend.output.contains("i <- 2"));
    }

    #[test]
    fn init_plus_scalar_conditional_loop_is_emitted_as_vector_ifelse() {
        let mut fn_ir = FnIR::new("loop_ifelse".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        let incr_bb = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let n = fn_ir.add_value(
            ValueKind::Const(Lit::Int(8)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let i_load = fn_ir.add_value(
            ValueKind::Load {
                var: "i_9".to_string(),
            },
            Span::dummy(),
            Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
            Some("i_9".to_string()),
        );
        let loop_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: i_load,
                rhs: n,
            },
            Span::dummy(),
            Facts::new(Facts::BOOL_SCALAR, crate::mir::flow::Interval::BOTTOM),
            None,
        );
        let clean_seed = fn_ir.add_value(
            ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![zero, n],
                names: vec![],
            },
            Span::dummy(),
            Facts::empty(),
            Some("clean".to_string()),
        );
        let score_load = fn_ir.add_value(
            ValueKind::Load {
                var: "score".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("score".to_string()),
        );
        let clean_load = fn_ir.add_value(
            ValueKind::Load {
                var: "clean".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("clean".to_string()),
        );
        let score_at_i = fn_ir.add_value(
            ValueKind::Index1D {
                base: score_load,
                idx: i_load,
                is_safe: true,
                is_na_safe: true,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let threshold = fn_ir.add_value(
            ValueKind::Const(Lit::Float(0.4)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let branch_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Gt,
                lhs: score_at_i,
                rhs: threshold,
            },
            Span::dummy(),
            Facts::new(Facts::BOOL_SCALAR, crate::mir::flow::Interval::BOTTOM),
            None,
        );
        let plus_const = fn_ir.add_value(
            ValueKind::Const(Lit::Float(0.1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let then_add = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: score_at_i,
                rhs: plus_const,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let then_sqrt = fn_ir.add_value(
            ValueKind::Call {
                callee: "sqrt".to_string(),
                args: vec![then_add],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let mul_const = fn_ir.add_value(
            ValueKind::Const(Lit::Float(0.55)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let else_mul = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs: score_at_i,
                rhs: mul_const,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let add_const = fn_ir.add_value(
            ValueKind::Const(Lit::Float(0.03)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let else_add = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: else_mul,
                rhs: add_const,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let inc = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: i_load,
                rhs: one,
            },
            Span::dummy(),
            Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "i_9".to_string(),
            src: one,
            span: Span::dummy(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond: loop_cond,
            then_bb,
            else_bb: entry,
        };
        fn_ir.blocks[then_bb].instrs.push(Instr::StoreIndex1D {
            base: clean_load,
            idx: i_load,
            val: then_sqrt,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::dummy(),
        });
        fn_ir.blocks[then_bb].term = Terminator::Goto(incr_bb);
        fn_ir.blocks[else_bb].instrs.push(Instr::StoreIndex1D {
            base: clean_load,
            idx: i_load,
            val: else_add,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::dummy(),
        });
        fn_ir.blocks[else_bb].term = Terminator::Goto(incr_bb);
        fn_ir.blocks[incr_bb].instrs.push(Instr::Assign {
            dst: "i_9".to_string(),
            src: inc,
            span: Span::dummy(),
        });
        fn_ir.blocks[incr_bb].term = Terminator::Goto(header);

        let structured = StructuredBlock::Sequence(vec![
            StructuredBlock::BasicBlock(entry),
            StructuredBlock::Loop {
                header,
                cond: loop_cond,
                continue_on_true: true,
                body: Box::new(StructuredBlock::Sequence(vec![
                    StructuredBlock::If {
                        cond: branch_cond,
                        then_body: Box::new(StructuredBlock::BasicBlock(then_bb)),
                        else_body: Some(Box::new(StructuredBlock::BasicBlock(else_bb))),
                    },
                    StructuredBlock::BasicBlock(incr_bb),
                    StructuredBlock::Next,
                ])),
            },
        ]);

        let mut backend = RBackend::new();
        backend.current_fn_name = "loop_ifelse".to_string();
        backend.bind_value_to_var(clean_seed, "clean");
        backend.bind_var_to_value("clean", clean_seed);
        let StructuredBlock::Sequence(items) = &structured else {
            panic!("expected sequence");
        };
        assert_eq!(
            backend.extract_full_range_loop_guard(loop_cond, "i_9", &fn_ir),
            Some(("i_9".to_string(), n))
        );
        assert_eq!(
            backend.extract_conditional_loop_shape(match &items[1] {
                StructuredBlock::Loop { body, .. } => body.as_ref(),
                _ => panic!("expected loop"),
            }),
            Some((branch_cond, then_bb, else_bb, incr_bb))
        );
        assert_eq!(
            backend.extract_conditional_loop_store(then_bb, "i_9", n, &fn_ir),
            Some(("clean".to_string(), then_sqrt))
        );
        assert_eq!(
            backend.extract_conditional_loop_store(else_bb, "i_9", n, &fn_ir),
            Some(("clean".to_string(), else_add))
        );
        assert!(backend.loop_increment_matches(incr_bb, "i_9", &fn_ir));
        assert_eq!(
            backend.try_emit_full_range_conditional_loop_sequence(items, &fn_ir),
            Some(2)
        );

        let mut backend = RBackend::new();
        backend.current_fn_name = "loop_ifelse".to_string();
        backend.bind_value_to_var(clean_seed, "clean");
        backend.bind_var_to_value("clean", clean_seed);
        backend
            .emit_structured(&structured, &fn_ir)
            .expect("structured scalar conditional loop emission should succeed");

        assert!(!backend.output.contains("repeat {"));
        assert!(!backend.output.contains("i_9 <- 1"));
        assert!(backend.output.contains(
            "clean <- ifelse(((score > 0.4)), sqrt((score + 0.1)), ((score * 0.55) + 0.03))"
        ));
    }

    #[test]
    fn stale_fresh_self_replay_after_full_update_is_skipped() {
        let mut backend = backend_with_sym17_fresh();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Const(Lit::Int(10)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![0, 1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("adj_ll".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Const(Lit::Int(1)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 5,
                kind: ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![3, 4, 0, 3],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("adj_ll".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "adj_ll".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("initial alloc should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "adj_ll".to_string(),
                    src: 5,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("full update should fold to an identity");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "adj_ll".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("stale replay should be skipped");

        let out = backend.output;
        assert_eq!(out.matches("adj_ll <- Sym_17(10L, 0L, 2L)").count(), 1);
        assert_eq!(
            out.matches("adj_ll <-").count(),
            1,
            "whole-range self update plus stale replay should both be skipped as identities: {out}"
        );
    }

    #[test]
    fn earlier_same_origin_fresh_value_is_skipped_after_newer_binding() {
        let mut backend = backend_with_sym17_fresh();
        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Const(Lit::Int(10)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Const(Lit::Int(0)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Const(Lit::Int(2)),
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Any,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Call {
                    callee: "Sym_17".to_string(),
                    args: vec![0, 1, 2],
                    names: vec![],
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("r".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Load {
                    var: "b".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("b".to_string()),
                phi_block: None,
                value_ty: TypeState::unknown(),
                value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
                escape: EscapeStatus::Unknown,
            },
        ];

        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "r".to_string(),
                    src: 4,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("newer binding should emit");
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "r".to_string(),
                    src: 3,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("stale earlier fresh value should be skipped");

        let out = backend.output;
        assert!(out.contains("r <- b"));
        assert!(!out.contains("r <- Sym_17(10L, 0L, 2L)"));
    }

    #[test]
    fn loop_carried_scalar_self_update_is_emitted_as_assignment() {
        let mut backend = RBackend::new();
        backend
            .active_loop_mutated_vars
            .push(FxHashSet::from_iter(["vy".to_string(), "y".to_string()]));

        let values = vec![
            Value {
                id: 0,
                kind: ValueKind::Load {
                    var: "vy".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("vy".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 1,
                kind: ValueKind::Load {
                    var: "g".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("g".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 2,
                kind: ValueKind::Load {
                    var: "dt".to_string(),
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("dt".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 3,
                kind: ValueKind::Binary {
                    op: BinOp::Mul,
                    lhs: 1,
                    rhs: 2,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: None,
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
            Value {
                id: 4,
                kind: ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: 0,
                    rhs: 3,
                },
                span: Span::dummy(),
                facts: Facts::empty(),
                origin_var: Some("vy".to_string()),
                phi_block: None,
                value_ty: TypeState::scalar(PrimTy::Double, false),
                value_term: TypeTerm::Double,
                escape: EscapeStatus::Unknown,
            },
        ];

        backend.bind_value_to_var(0, "vy");
        backend.bind_var_to_value("vy", 0);
        backend
            .emit_instr(
                &Instr::Assign {
                    dst: "vy".to_string(),
                    src: 4,
                    span: Span::dummy(),
                },
                &values,
                &[],
            )
            .expect("loop-carried scalar self update should emit");

        assert!(
            backend.output.contains("vy <- (vy + (g * dt))"),
            "{}",
            backend.output
        );
    }

    #[test]
    fn float_literals_keep_trailing_decimal_when_integral() {
        let backend = RBackend::new();
        let value = Value {
            id: 0,
            kind: ValueKind::Const(Lit::Float(5.0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        };

        assert_eq!(backend.emit_lit_with_value(&Lit::Float(5.0), &value), "5.0");
    }

    #[test]
    fn unary_neg_constant_float_is_folded_in_emission() {
        let backend = RBackend::new();
        let values = vec![Value {
            id: 0,
            kind: ValueKind::Const(Lit::Float(9.81)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        }];

        assert_eq!(
            backend.resolve_unary_expr(UnaryOp::Neg, 0, &values, &[]),
            "-9.81"
        );
    }

    #[test]
    fn marks_emit_integer_suffixes() {
        let mut backend = RBackend::new();
        backend.emit_mark(
            Span {
                start_line: 9,
                start_col: 5,
                end_line: 9,
                end_col: 5,
                ..Span::default()
            },
            None,
        );

        assert!(
            backend.output.contains("rr_mark(9L, 5L);"),
            "{}",
            backend.output
        );
    }
}
