use super::*;
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorExprPolicy {
    pub(crate) allow_any_base: bool,
    pub(crate) require_safe_index: bool,
}

pub(crate) const RELAXED_VECTOR_EXPR_POLICY: VectorExprPolicy = VectorExprPolicy {
    allow_any_base: true,
    require_safe_index: false,
};

pub(crate) fn is_vectorizable_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    policy: VectorExprPolicy,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        policy: VectorExprPolicy,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        if root == iv_phi {
            return true;
        }
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, lp, policy, seen)
                    && rec(fn_ir, *rhs, iv_phi, lp, policy, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, lp, policy, seen),
            ValueKind::Call { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, lp, policy, seen)),
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                if policy.require_safe_index && !(*is_safe && *is_na_safe) {
                    return false;
                }
                if !policy.allow_any_base && !is_loop_compatible_base(lp, fn_ir, *base) {
                    return false;
                }
                if is_iv_equivalent(fn_ir, *idx, iv_phi) {
                    return true;
                }
                !policy.require_safe_index
                    && expr_has_iv_dependency(fn_ir, *idx, iv_phi)
                    && rec(fn_ir, *idx, iv_phi, lp, policy, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                if !policy.allow_any_base && !is_loop_compatible_base(lp, fn_ir, *base) {
                    return false;
                }
                let Some(pattern) =
                    classify_3d_general_vector_access(fn_ir, *base, *i, *j, *k, iv_phi)
                else {
                    return false;
                };
                [pattern.i, pattern.j, pattern.k]
                    .into_iter()
                    .all(|operand| match operand {
                        VectorAccessOperand3D::Scalar(_) => true,
                        VectorAccessOperand3D::Vector(dep_idx) => {
                            is_iv_equivalent(fn_ir, dep_idx, iv_phi)
                                || rec(fn_ir, dep_idx, iv_phi, lp, policy, seen)
                        }
                    })
            }
            ValueKind::Phi { args } => {
                if fn_ir.values[root]
                    .phi_block
                    .is_some_and(|bb| !lp.body.contains(&bb))
                    && !expr_has_iv_dependency(fn_ir, root, iv_phi)
                {
                    true
                } else {
                    args.iter()
                        .all(|(a, _)| rec(fn_ir, *a, iv_phi, lp, policy, seen))
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, lp, policy, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, lp, policy, seen)
                    && rec(fn_ir, *end, iv_phi, lp, policy, seen)
            }
            _ => false,
        }
    }
    rec(fn_ir, root, iv_phi, lp, policy, &mut FxHashSet::default())
}

pub(crate) fn is_condition_vectorizable(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    struct ConditionVectorizable<'a> {
        pub(crate) fn_ir: &'a FnIR,
        pub(crate) iv_phi: ValueId,
        pub(crate) lp: &'a LoopInfo,
        pub(crate) user_call_whitelist: &'a FxHashSet<String>,
        pub(crate) seen_vals: FxHashSet<ValueId>,
        pub(crate) seen_vars: FxHashSet<String>,
    }

    impl ConditionVectorizable<'_> {
        pub(crate) fn load_var(&mut self, var: &str, depth: usize) -> bool {
            if proof_budget_exhausted(depth, &self.seen_vals, &self.seen_vars) {
                return false;
            }
            if !self.seen_vars.insert(var.to_string()) {
                return false;
            }
            let assigned_sources = self
                .fn_ir
                .blocks
                .iter()
                .flat_map(|bb| bb.instrs.iter())
                .filter_map(|ins| match ins {
                    Instr::Assign { dst, src, .. } if dst == var => Some(*src),
                    _ => None,
                })
                .collect::<Vec<_>>();

            for src in assigned_sources {
                if !self.rec(src, depth + 1) {
                    self.seen_vars.remove(var);
                    return false;
                }
            }
            self.seen_vars.remove(var);

            // Params and immutable captures can appear as bare loads with no local assignment.
            // Treat them as loop-invariant condition inputs.
            true
        }

        pub(crate) fn rec(&mut self, root: ValueId, depth: usize) -> bool {
            if proof_budget_exhausted(depth, &self.seen_vals, &self.seen_vars) {
                return false;
            }
            let root = canonical_value(self.fn_ir, root);
            if root == self.iv_phi || is_iv_equivalent(self.fn_ir, root, self.iv_phi) {
                return true;
            }
            if !self.seen_vals.insert(root) {
                return true;
            }
            match self.fn_ir.values[root].kind.clone() {
                ValueKind::Const(_) | ValueKind::Param { .. } => true,
                ValueKind::Binary { lhs, rhs, .. } => {
                    self.rec(lhs, depth + 1) && self.rec(rhs, depth + 1)
                }
                ValueKind::Unary { rhs, .. } => self.rec(rhs, depth + 1),
                // Data-dependent conditions are now allowed if the access is proven safe.
                ValueKind::Index1D {
                    base,
                    idx,
                    is_safe,
                    is_na_safe,
                } => self.index1d_vectorizable(base, idx, is_safe, is_na_safe),
                ValueKind::Index2D { .. } => false,
                ValueKind::Index3D { base, i, j, k } => {
                    self.index3d_vectorizable(base, i, j, k, depth)
                }
                ValueKind::Call { args, .. } => self.call_vectorizable(root, args, depth),
                ValueKind::Phi { args } => args.iter().all(|(arg, _)| self.rec(*arg, depth + 1)),
                ValueKind::Load { var } => self.load_var(&var, depth + 1),
                _ => false,
            }
        }

        pub(crate) fn index1d_vectorizable(
            &self,
            base: ValueId,
            idx: ValueId,
            is_safe: bool,
            is_na_safe: bool,
        ) -> bool {
            let iv_idx = is_iv_equivalent(self.fn_ir, idx, self.iv_phi);
            let iv_dependent_idx = iv_idx || expr_has_iv_dependency(self.fn_ir, idx, self.iv_phi);
            if !is_loop_compatible_base(self.lp, self.fn_ir, base) && !iv_dependent_idx {
                return false;
            }
            if is_safe && is_na_safe && iv_idx {
                return true;
            }
            iv_dependent_idx
        }

        pub(crate) fn index3d_vectorizable(
            &mut self,
            base: ValueId,
            i: ValueId,
            j: ValueId,
            k: ValueId,
            depth: usize,
        ) -> bool {
            is_loop_compatible_base(self.lp, self.fn_ir, base)
                && classify_3d_general_vector_access(self.fn_ir, base, i, j, k, self.iv_phi)
                    .is_some_and(|pattern| {
                        [pattern.i, pattern.j, pattern.k]
                            .into_iter()
                            .all(|operand| match operand {
                                VectorAccessOperand3D::Scalar(_) => true,
                                VectorAccessOperand3D::Vector(dep_idx) => {
                                    self.rec(dep_idx, depth + 1)
                                }
                            })
                    })
        }

        pub(crate) fn call_vectorizable(
            &mut self,
            root: ValueId,
            args: Vec<ValueId>,
            depth: usize,
        ) -> bool {
            let resolved = resolve_call_info(self.fn_ir, root);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.clone())
                .unwrap_or(args);
            let runtime_read = is_runtime_vector_read_call(callee, call_args.len());
            if runtime_read
                && let Some(base) = call_args.first().copied()
                && !is_loop_compatible_base(self.lp, self.fn_ir, base)
            {
                return false;
            }
            (is_vector_safe_call(callee, call_args.len(), self.user_call_whitelist) || runtime_read)
                && call_args.iter().all(|arg| self.rec(*arg, depth + 1))
        }
    }

    ConditionVectorizable {
        fn_ir,
        iv_phi,
        lp,
        user_call_whitelist,
        seen_vals: FxHashSet::default(),
        seen_vars: FxHashSet::default(),
    }
    .rec(root, 0)
}
