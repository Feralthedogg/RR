use super::*;

pub(in crate::mir::opt::v_opt) fn build_loop_index_vector(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
) -> Option<ValueId> {
    let iv = lp.iv.as_ref()?;
    let end = adjusted_loop_limit(fn_ir, lp.limit?, lp.limit_adjust);
    Some(fn_ir.add_value(
        ValueKind::Range {
            start: iv.init_val,
            end,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    ))
}

pub(in crate::mir::opt::v_opt) fn add_int_offset(
    fn_ir: &mut FnIR,
    base: ValueId,
    offset: i64,
) -> ValueId {
    if offset == 0 {
        return base;
    }
    if let ValueKind::Const(Lit::Int(n)) = fn_ir.values[base].kind {
        return fn_ir.add_value(
            ValueKind::Const(Lit::Int(n + offset)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
    }
    let k = fn_ir.add_value(
        ValueKind::Const(Lit::Int(offset)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: base,
            rhs: k,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

/// Apply the loop-analysis limit adjustment so downstream vector codegen can
/// use the exact inclusive range implied by the original scalar loop guard.
pub(in crate::mir::opt::v_opt) fn adjusted_loop_limit(
    fn_ir: &mut FnIR,
    limit: ValueId,
    adjust: i64,
) -> ValueId {
    if adjust == 0 {
        return limit;
    }
    add_int_offset(fn_ir, limit, adjust)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::mir::opt::v_opt) struct MaterializedExprKey {
    pub(crate) kind: ValueKind,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::mir::opt::v_opt) struct VectorMaterializePolicy {
    pub(in crate::mir::opt::v_opt) allow_any_base: bool,
    pub(in crate::mir::opt::v_opt) require_safe_index: bool,
}

pub(in crate::mir::opt::v_opt) const RELAXED_VECTOR_MATERIALIZE_POLICY: VectorMaterializePolicy =
    VectorMaterializePolicy {
        allow_any_base: true,
        require_safe_index: false,
    };

pub(in crate::mir::opt::v_opt) const SAFE_INDEX_VECTOR_MATERIALIZE_POLICY: VectorMaterializePolicy =
    VectorMaterializePolicy {
        allow_any_base: true,
        require_safe_index: true,
    };

pub(in crate::mir::opt::v_opt) type MaterializeRecurseFn =
    for<'a> fn(&mut FnIR, ValueId, &mut VectorMaterializeCtx<'a>) -> Option<ValueId>;

pub(in crate::mir::opt::v_opt) struct VectorMaterializeCtx<'a> {
    pub(in crate::mir::opt::v_opt) iv_phi: ValueId,
    pub(in crate::mir::opt::v_opt) idx_vec: ValueId,
    pub(in crate::mir::opt::v_opt) lp: &'a LoopInfo,
    pub(in crate::mir::opt::v_opt) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(in crate::mir::opt::v_opt) interner: &'a mut FxHashMap<MaterializedExprKey, ValueId>,
    pub(in crate::mir::opt::v_opt) visiting: &'a mut FxHashSet<ValueId>,
    pub(in crate::mir::opt::v_opt) policy: VectorMaterializePolicy,
    pub(in crate::mir::opt::v_opt) recurse: MaterializeRecurseFn,
}

impl VectorMaterializeCtx<'_> {
    pub(in crate::mir::opt::v_opt) fn recurse(
        &mut self,
        fn_ir: &mut FnIR,
        root: ValueId,
    ) -> Option<ValueId> {
        let recurse = self.recurse;
        recurse(fn_ir, root, self)
    }

    pub(in crate::mir::opt::v_opt) fn recurse_with_policy(
        &mut self,
        fn_ir: &mut FnIR,
        root: ValueId,
        policy: VectorMaterializePolicy,
    ) -> Option<ValueId> {
        let saved = self.policy;
        self.policy = policy;
        let out = self.recurse(fn_ir, root);
        self.policy = saved;
        out
    }
}

pub(in crate::mir::opt::v_opt) fn intern_materialized_value(
    fn_ir: &mut FnIR,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    kind: ValueKind,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
) -> ValueId {
    let key = MaterializedExprKey { kind: kind.clone() };
    if let Some(existing) = interner.get(&key) {
        // Reuse structurally identical expressions, but keep analysis metadata
        // conservative across reuse sites.
        let merged = fn_ir.values[*existing].facts.join(&facts);
        fn_ir.values[*existing].facts = merged;
        return *existing;
    }
    let id = fn_ir.add_value(kind, span, facts, None);
    interner.insert(key, id);
    id
}

pub(in crate::mir::opt::v_opt) fn recurse_materialized_load_source(
    fn_ir: &mut FnIR,
    root: ValueId,
    src: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    if canonical_value(fn_ir, src) == root {
        return Some(root);
    }
    ctx.recurse(fn_ir, src)
}

pub(in crate::mir::opt::v_opt) fn select_origin_phi_load_source(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    use_bb: Option<BlockId>,
    visiting: &FxHashSet<ValueId>,
) -> Option<(ValueId, &'static str)> {
    if let Some(use_bb) = use_bb
        && let Some(src) = unique_origin_phi_value_in_loop(fn_ir, lp, var)
            .filter(|src| {
                let src = canonical_value(fn_ir, *src);
                !visiting.contains(&src)
                    && fn_ir.values[src]
                        .phi_block
                        .is_some_and(|phi_bb| phi_bb < use_bb)
            })
            .or_else(|| {
                nearest_origin_phi_value_in_loop(fn_ir, lp, var, use_bb)
                    .filter(|src| !visiting.contains(&canonical_value(fn_ir, *src)))
            })
    {
        return Some((src, "prior-origin-phi"));
    }

    unique_origin_phi_value_in_loop(fn_ir, lp, var)
        .filter(|src| !visiting.contains(&canonical_value(fn_ir, *src)))
        .map(|src| (src, "fallback-origin-phi"))
}

pub(in crate::mir::opt::v_opt) fn materialize_passthrough_origin_phi_for_load(
    fn_ir: &mut FnIR,
    var: &str,
    use_bb: BlockId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<(ValueId, ValueId)> {
    let nearest_phi =
        nearest_visiting_origin_phi_value_in_loop(fn_ir, ctx.lp, var, use_bb, ctx.visiting)?;
    let phi_src = materialize_passthrough_origin_phi_state(fn_ir, nearest_phi, var, ctx)?;
    Some((nearest_phi, phi_src))
}
