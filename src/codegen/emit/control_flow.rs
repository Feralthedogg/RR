use crate::codegen::backend::state::{RBackend, TypedParallelWrapperPlan};
use crate::mir::def::{BinOp, FnIR, Instr, IntrinsicOp, Terminator, ValueKind};
use crate::typeck::{ShapeTy, TypeTerm};
use rustc_hash::{FxHashMap, FxHashSet};

impl RBackend {
    pub(crate) fn emit_typed_parallel_wrapper(
        &mut self,
        fn_ir: &FnIR,
        plan: &TypedParallelWrapperPlan,
    ) {
        self.write_stmt("# rr-typed-parallel-wrapper");
        self.write(fn_ir.name.as_str());
        self.write(" <- function(");
        for (idx, param) in fn_ir.params.iter().enumerate() {
            if idx > 0 {
                self.write(", ");
            }
            self.write(param);
            if let Some(Some(default_expr)) = fn_ir.param_default_r_exprs.get(idx) {
                self.write(" = ");
                self.write(default_expr);
            }
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

    pub(crate) fn typed_parallel_wrapper_plan(fn_ir: &FnIR) -> Option<TypedParallelWrapperPlan> {
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
            ValueKind::RecordLit { fields } => fields.iter().all(|(_, value)| {
                Self::is_typed_parallel_safe_value(fn_ir, *value, bindings, seen)
            }),
            ValueKind::FieldGet { base, .. } => {
                Self::is_typed_parallel_safe_value(fn_ir, *base, bindings, seen)
            }
            ValueKind::FieldSet { base, value, .. } => {
                Self::is_typed_parallel_safe_value(fn_ir, *base, bindings, seen)
                    && Self::is_typed_parallel_safe_value(fn_ir, *value, bindings, seen)
            }
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
}
