use super::*;
pub(crate) struct IndexSafetyCollector<'a> {
    pub(crate) bid: BlockId,
    pub(crate) facts: &'a mut crate::mir::analyze::range::RangeFacts,
    pub(crate) fn_ir: &'a FnIR,
    pub(crate) canonical_ivs: &'a [CanonicalIvRule<'a>],
    pub(crate) one_based_ivs: &'a [OneBasedIvRule<'a>],
    pub(crate) na_states: &'a [NaState],
    pub(crate) safe_values: &'a mut FxHashSet<ValueId>,
    pub(crate) non_na_values: &'a mut FxHashSet<ValueId>,
    pub(crate) one_based_values: &'a mut FxHashSet<ValueId>,
    pub(crate) seen: &'a mut FxHashSet<ValueId>,
    pub(crate) node_visits: &'a mut usize,
    pub(crate) visit_limit: usize,
}

impl IndexSafetyCollector<'_> {
    pub(crate) fn collect(&mut self, vid: ValueId) {
        if *self.node_visits >= self.visit_limit {
            return;
        }
        *self.node_visits += 1;

        if !self.seen.insert(vid) {
            return;
        }

        match self.fn_ir.values[vid].kind.clone() {
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => self.collect_index1d(vid, base, idx, is_safe, is_na_safe),
            ValueKind::RecordLit { fields } => {
                self.collect_many(fields.into_iter().map(|(_, value)| value));
            }
            ValueKind::FieldGet { base, .. } => self.collect(base),
            ValueKind::FieldSet { base, value, .. } => self.collect_many([base, value]),
            ValueKind::Binary { lhs, rhs, .. } => self.collect_many([lhs, rhs]),
            ValueKind::Unary { rhs, .. } => self.collect(rhs),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                self.collect_many(args);
            }
            ValueKind::Phi { args } => {
                self.collect_many(args.into_iter().map(|(arg, _)| arg));
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => self.collect(base),
            ValueKind::Range { start, end } => self.collect_many([start, end]),
            ValueKind::Index2D { base, r, c } => self.collect_many([base, r, c]),
            ValueKind::Index3D { base, i, j, k } => self.collect_many([base, i, j, k]),
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
        }
    }

    pub(crate) fn collect_many(&mut self, values: impl IntoIterator<Item = ValueId>) {
        for value in values {
            self.collect(value);
        }
    }

    pub(crate) fn collect_index1d(
        &mut self,
        vid: ValueId,
        base: ValueId,
        idx: ValueId,
        is_safe: bool,
        is_na_safe: bool,
    ) {
        ensure_value_range(idx, &self.fn_ir.values, self.facts);
        let iv_proven = iv_in_bounds_for_base(self.bid, idx, base, self.canonical_ivs, self.fn_ir);
        let idx_intv = self.facts.get(idx);
        if iv_non_na_in_block(
            self.bid,
            idx,
            self.canonical_ivs,
            self.one_based_ivs,
            self.fn_ir,
        ) {
            self.one_based_values.insert(idx);
        }
        if !is_safe && (interval_proves_in_bounds(self.fn_ir, &idx_intv, base) || iv_proven) {
            self.safe_values.insert(vid);
        }
        let na_proven = matches!(self.na_states[idx], NaState::Never)
            || iv_non_na_in_block(
                self.bid,
                idx,
                self.canonical_ivs,
                self.one_based_ivs,
                self.fn_ir,
            );
        if !is_na_safe && (na_proven || interval_proves_in_bounds(self.fn_ir, &idx_intv, base)) {
            self.non_na_values.insert(vid);
        }

        self.collect_many([base, idx]);
    }
}

pub(crate) fn interval_proves_in_bounds(fn_ir: &FnIR, intv: &RangeInterval, base: ValueId) -> bool {
    let lo_safe = match &intv.lo {
        SymbolicBound::Const(n) => *n >= 1,
        SymbolicBound::LenOf(_, off) => *off >= 1,
        _ => false,
    };
    let hi_safe = match &intv.hi {
        SymbolicBound::LenOf(b, off) => *off <= 0 && same_base_for_len(fn_ir, *b, base),
        SymbolicBound::Const(_) => false,
        _ => false,
    };
    lo_safe && hi_safe
}

pub(crate) fn same_base_for_len(fn_ir: &FnIR, len_base: ValueId, index_base: ValueId) -> bool {
    if len_base == index_base {
        return true;
    }
    let len_ty = fn_ir.values[len_base].value_ty;
    let idx_ty = fn_ir.values[index_base].value_ty;
    if len_ty.len_sym.is_some() && len_ty.len_sym == idx_ty.len_sym {
        return true;
    }
    let a = value_base_name(fn_ir, len_base);
    let b = value_base_name(fn_ir, index_base);
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

pub(crate) fn value_base_name(fn_ir: &FnIR, vid: ValueId) -> Option<&str> {
    if let Some(name) = fn_ir.values[vid].origin_var.as_deref() {
        return Some(name);
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Load { var } => Some(var.as_str()),
        _ => None,
    }
}
