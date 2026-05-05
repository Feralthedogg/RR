use super::const_eval::{
    as_integral, collect_reachable_blocks, collect_reachable_values, eval_const, is_zero_number,
    matrix_known_dims, validate_const_condition, validate_index_lit_for_read,
    validate_index_lit_for_write,
};
use super::runtime_proofs::{
    RuntimeSafetyNeeds, bid_for_value, division_by_zero_diagnostic,
    flow_interval_guarantees_below_one, flow_interval_guarantees_negative, format_interval,
    interval_guarantees_above_base_len, interval_guarantees_above_const,
    interval_guarantees_below_one, interval_guarantees_negative, interval_guarantees_zero,
    range_interval_to_fact_interval, runtime_safety_needs, seq_len_negative_diagnostic,
};
use super::*;
pub(crate) type RuntimeNaStates = Vec<crate::mir::analyze::na::NaState>;
pub(crate) type RuntimeRangeFacts = Vec<crate::mir::analyze::range::RangeFacts>;
pub(crate) type RuntimeDataflowFacts = FxHashMap<ValueId, crate::mir::flow::Facts>;

pub(crate) struct RuntimeValidationData {
    pub(crate) reachable_blocks: FxHashSet<BlockId>,
    pub(crate) reachable_values: FxHashSet<ValueId>,
    pub(crate) na_states: Option<RuntimeNaStates>,
    pub(crate) range_out: Option<RuntimeRangeFacts>,
    pub(crate) dataflow: Option<RuntimeDataflowFacts>,
}

impl RuntimeValidationData {
    pub(crate) fn new(fn_ir: &FnIR) -> Self {
        let reachable_blocks = collect_reachable_blocks(fn_ir);
        let reachable_values = collect_reachable_values(fn_ir, &reachable_blocks);
        let needs = runtime_safety_needs(fn_ir);
        let na_states = needs
            .needs_na
            .then(|| crate::mir::analyze::na::compute_na_states(fn_ir));
        let range_out = compute_runtime_range_out(fn_ir, &needs);
        let dataflow_targets = collect_runtime_dataflow_targets(fn_ir, &needs);
        let dataflow = needs
            .needs_dataflow
            .then(|| crate::mir::flow::DataflowSolver::analyze_values(fn_ir, &dataflow_targets));

        Self {
            reachable_blocks,
            reachable_values,
            na_states,
            range_out,
            dataflow,
        }
    }

    pub(crate) fn block_reachable(&self, bid: BlockId) -> bool {
        self.reachable_blocks.contains(&bid)
    }

    pub(crate) fn value_reachable(&self, vid: ValueId) -> bool {
        self.reachable_values.contains(&vid)
    }

    pub(crate) fn value_is_always_na(&self, vid: ValueId) -> bool {
        self.na_states
            .as_ref()
            .is_some_and(|states| matches!(states[vid], crate::mir::analyze::na::NaState::Always))
    }

    pub(crate) fn out_ranges_for_block(
        &self,
        bid: BlockId,
    ) -> Option<&crate::mir::analyze::range::RangeFacts> {
        self.range_out.as_ref().map(|facts| &facts[bid])
    }

    pub(crate) fn dataflow_interval(&self, value: ValueId) -> Option<crate::mir::flow::Interval> {
        self.dataflow
            .as_ref()
            .and_then(|facts| facts.get(&value).map(|f| f.interval))
    }
}

pub(crate) fn compute_runtime_range_out(
    fn_ir: &FnIR,
    needs: &RuntimeSafetyNeeds,
) -> Option<RuntimeRangeFacts> {
    needs.needs_range.then(|| {
        let mut out = crate::mir::analyze::range::analyze_ranges(fn_ir);
        for (bid, block_facts) in out.iter_mut().enumerate() {
            crate::mir::analyze::range::transfer_block(bid, fn_ir, block_facts);
        }
        out
    })
}

pub(crate) fn collect_runtime_dataflow_targets(
    fn_ir: &FnIR,
    needs: &RuntimeSafetyNeeds,
) -> Vec<ValueId> {
    if !needs.needs_dataflow {
        return Vec::new();
    }

    let mut targets = Vec::new();
    for value in &fn_ir.values {
        collect_value_dataflow_targets(value, &mut targets);
    }
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            collect_instr_dataflow_targets(instr, &mut targets);
        }
    }
    targets
}

pub(crate) fn collect_value_dataflow_targets(value: &Value, targets: &mut Vec<ValueId>) {
    match &value.kind {
        ValueKind::Binary {
            op: BinOp::Div | BinOp::Mod,
            rhs,
            ..
        } => targets.push(*rhs),
        ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
            targets.push(args[0]);
        }
        ValueKind::Index1D { idx, .. } => targets.push(*idx),
        ValueKind::Index2D { r, c, .. } => {
            targets.push(*r);
            targets.push(*c);
        }
        ValueKind::Index3D { i, j, k, .. } => {
            targets.push(*i);
            targets.push(*j);
            targets.push(*k);
        }
        _ => {}
    }
}

pub(crate) fn collect_instr_dataflow_targets(instr: &Instr, targets: &mut Vec<ValueId>) {
    match *instr {
        Instr::StoreIndex1D { idx, .. } => targets.push(idx),
        Instr::StoreIndex2D { r, c, .. } => {
            targets.push(r);
            targets.push(c);
        }
        Instr::StoreIndex3D { i, j, k, .. } => {
            targets.push(i);
            targets.push(j);
            targets.push(k);
        }
        Instr::Assign { .. } | Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => {}
    }
}

pub(crate) fn validate_function_runtime(fn_ir: &FnIR) -> Vec<RRException> {
    RuntimeValidator::new(fn_ir).validate()
}

pub(crate) struct RuntimeValidator<'a> {
    pub(crate) fn_ir: &'a FnIR,
    pub(crate) memo: FxHashMap<ValueId, Option<Lit>>,
    pub(crate) runtime: RuntimeValidationData,
    pub(crate) errors: Vec<RRException>,
}

impl<'a> RuntimeValidator<'a> {
    pub(crate) fn new(fn_ir: &'a FnIR) -> Self {
        Self {
            fn_ir,
            memo: FxHashMap::default(),
            runtime: RuntimeValidationData::new(fn_ir),
            errors: Vec::new(),
        }
    }

    pub(crate) fn validate(mut self) -> Vec<RRException> {
        self.validate_blocks();
        self.validate_values();
        self.errors
    }

    pub(crate) fn validate_blocks(&mut self) {
        for (bid, block) in self.fn_ir.blocks.iter().enumerate() {
            if self.runtime.block_reachable(bid) {
                self.validate_block_runtime(bid, block);
            }
        }
    }

    pub(crate) fn validate_block_runtime(&mut self, bid: BlockId, block: &crate::mir::def::Block) {
        self.validate_branch_condition(block);
        for instr in &block.instrs {
            self.validate_runtime_instr(bid, instr);
        }
    }

    pub(crate) fn validate_branch_condition(&mut self, block: &crate::mir::def::Block) {
        let Terminator::If { cond, .. } = block.term else {
            return;
        };
        if !self.runtime.value_reachable(cond) {
            return;
        }
        if let Some(lit) = self.eval_const_value(cond)
            && let Err(error) = validate_const_condition(lit, self.fn_ir.values[cond].span)
        {
            self.errors.push(error);
        } else if self.runtime.value_is_always_na(cond) {
            self.errors.push(self.condition_always_na_error(cond));
        }
    }

    pub(crate) fn validate_runtime_instr(&mut self, bid: BlockId, instr: &Instr) {
        match *instr {
            Instr::StoreIndex1D {
                base, idx, span, ..
            } => self.validate_store_index1d(bid, base, idx, span),
            Instr::StoreIndex2D {
                base, r, c, span, ..
            } => self.validate_store_index2d(bid, base, r, c, span),
            Instr::StoreIndex3D { i, j, k, span, .. } => {
                self.validate_store_index3d(bid, [i, j, k], span);
            }
            Instr::Assign { .. } | Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => {}
        }
    }

    pub(crate) fn validate_store_index1d(
        &mut self,
        bid: BlockId,
        base: ValueId,
        idx: ValueId,
        span: crate::utils::Span,
    ) {
        if self.push_invalid_write_index_lit(idx, span) {
            return;
        }
        if self.runtime.value_is_always_na(idx) {
            self.errors
                .push(self.assignment_index_always_na_error(idx, span));
            return;
        }
        if let Some(error) = self.assignment_index_below_one_range_error(bid, idx, span) {
            self.errors.push(error);
            return;
        }
        if flow_interval_guarantees_below_one(self.runtime.dataflow_interval(idx)) {
            self.errors
                .push(self.assignment_index_below_one_flow_error(idx, span));
            return;
        }
        if let Some(error) = self.assignment_index_above_base_len_error(bid, base, idx, span) {
            self.errors.push(error);
        }
    }

    pub(crate) fn validate_store_index2d(
        &mut self,
        bid: BlockId,
        base: ValueId,
        row: ValueId,
        col: ValueId,
        span: crate::utils::Span,
    ) {
        self.push_invalid_write_index_lit(row, span);
        self.push_invalid_write_index_lit(col, span);
        self.validate_matrix_assignment_index(bid, base, row, span, "row");
        self.validate_matrix_assignment_index(bid, base, col, span, "column");
    }

    pub(crate) fn validate_matrix_assignment_index(
        &mut self,
        bid: BlockId,
        base: ValueId,
        idx: ValueId,
        span: crate::utils::Span,
        axis: &str,
    ) {
        if self.runtime.value_is_always_na(idx) {
            self.errors
                .push(self.matrix_assignment_index_always_na_error(idx, span));
        } else if let Some(error) = self.matrix_assignment_below_one_range_error(bid, idx, span) {
            self.errors.push(error);
        } else if flow_interval_guarantees_below_one(self.runtime.dataflow_interval(idx)) {
            self.errors
                .push(self.matrix_assignment_below_one_flow_error(idx, span));
        }

        if let Some(error) = self.matrix_assignment_extent_error(bid, base, idx, span, axis) {
            self.errors.push(error);
        }
    }

    pub(crate) fn validate_store_index3d(
        &mut self,
        bid: BlockId,
        indices: [ValueId; 3],
        span: crate::utils::Span,
    ) {
        for idx in indices {
            self.push_invalid_write_index_lit(idx, span);
            if let Some(error) = self.array3d_assignment_below_one_error(bid, idx, span) {
                self.errors.push(error);
            }
        }
    }

    pub(crate) fn validate_values(&mut self) {
        for (vid, value) in self.fn_ir.values.iter().enumerate() {
            if self.runtime.value_reachable(vid) {
                self.validate_value_runtime(vid, value);
            }
        }
    }

    pub(crate) fn validate_value_runtime(&mut self, vid: ValueId, value: &Value) {
        match &value.kind {
            ValueKind::Binary {
                op: BinOp::Div | BinOp::Mod,
                rhs,
                ..
            } => self.validate_division_rhs(vid, value, *rhs),
            ValueKind::Index1D { base, idx, .. } => {
                self.validate_index1d_read(vid, value, *base, *idx);
            }
            ValueKind::Index2D { base, r, c } => {
                self.validate_index2d_read(vid, value, *base, *r, *c);
            }
            ValueKind::Index3D { i, j, k, .. } => {
                self.validate_index3d_read(vid, value, [*i, *j, *k]);
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                self.validate_seq_len_arg(vid, value, args[0]);
            }
            _ => {}
        }
    }

    pub(crate) fn validate_division_rhs(&mut self, vid: ValueId, value: &Value, rhs: ValueId) {
        if let Some(lit) = self.eval_const_value(rhs)
            && is_zero_number(&lit)
        {
            self.errors.push(division_by_zero_diagnostic(
                value.span,
                self.fn_ir.values[rhs].span,
                "division by zero is guaranteed at compile-time",
            ));
        } else if interval_guarantees_zero(self.runtime.dataflow_interval(rhs))
            || self.runtime.range_out.as_ref().is_some_and(|facts| {
                interval_guarantees_zero(range_interval_to_fact_interval(
                    facts,
                    bid_for_value(self.fn_ir, vid),
                    rhs,
                ))
            })
        {
            self.errors.push(division_by_zero_diagnostic(
                value.span,
                self.fn_ir.values[rhs].span,
                "division by zero is guaranteed by range/dataflow analysis",
            ));
        }
    }

    pub(crate) fn validate_index1d_read(
        &mut self,
        vid: ValueId,
        value: &Value,
        base: ValueId,
        idx: ValueId,
    ) {
        if self.push_invalid_read_index_lit(idx, value.span) {
            return;
        }
        let Some(facts) = self.runtime.range_out.as_ref() else {
            return;
        };

        let bid = bid_for_value(self.fn_ir, vid);
        let idx_range = facts[bid].get(idx);
        if interval_guarantees_below_one(&idx_range) {
            self.errors
                .push(self.index_read_below_one_range_error(idx, value.span, &idx_range));
        } else if flow_interval_guarantees_below_one(self.runtime.dataflow_interval(idx)) {
            self.errors
                .push(self.index_read_below_one_flow_error(idx, value.span));
        } else if interval_guarantees_above_base_len(&idx_range, base) {
            self.errors
                .push(self.index_read_above_base_len_error(idx, value.span, &idx_range));
        }
    }

    pub(crate) fn validate_index2d_read(
        &mut self,
        vid: ValueId,
        value: &Value,
        base: ValueId,
        row: ValueId,
        col: ValueId,
    ) {
        self.push_invalid_read_index_lit(row, value.span);
        self.push_invalid_read_index_lit(col, value.span);
        self.validate_matrix_read_index(vid, value, base, row, "row");
        self.validate_matrix_read_index(vid, value, base, col, "column");
    }

    pub(crate) fn validate_matrix_read_index(
        &mut self,
        vid: ValueId,
        value: &Value,
        base: ValueId,
        idx: ValueId,
        axis: &str,
    ) {
        let Some(facts) = self.runtime.range_out.as_ref() else {
            return;
        };
        let bid = bid_for_value(self.fn_ir, vid);
        let idx_range = facts[bid].get(idx);
        if interval_guarantees_below_one(&idx_range) {
            self.errors
                .push(self.matrix_read_below_one_range_error(idx, value.span, &idx_range));
        } else if flow_interval_guarantees_below_one(self.runtime.dataflow_interval(idx)) {
            self.errors
                .push(self.matrix_read_below_one_flow_error(idx, value.span));
        }

        if let Some(error) = self.matrix_read_extent_error(base, idx, value.span, axis, &idx_range)
        {
            self.errors.push(error);
        }
    }

    pub(crate) fn validate_index3d_read(
        &mut self,
        vid: ValueId,
        value: &Value,
        indices: [ValueId; 3],
    ) {
        for idx in indices {
            self.push_invalid_read_index_lit(idx, value.span);
            if let Some(error) = self.array3d_read_below_one_error(vid, idx, value.span) {
                self.errors.push(error);
            }
        }
    }

    pub(crate) fn validate_seq_len_arg(&mut self, vid: ValueId, value: &Value, arg: ValueId) {
        if let Some(lit) = self.eval_const_value(arg)
            && let Some(n) = as_integral(&lit)
            && n < 0
        {
            self.errors.push(seq_len_negative_diagnostic(
                value.span,
                self.fn_ir.values[arg].span,
            ));
        } else if self.runtime.range_out.as_ref().is_some_and(|facts| {
            interval_guarantees_negative(&facts[bid_for_value(self.fn_ir, vid)].get(arg))
        }) || self.runtime.dataflow.as_ref().is_some_and(|facts| {
            flow_interval_guarantees_negative(facts.get(&arg).map(|f| f.interval))
        }) {
            self.errors.push(seq_len_negative_diagnostic(
                value.span,
                self.fn_ir.values[arg].span,
            ));
        }
    }

    pub(crate) fn eval_const_value(&mut self, value: ValueId) -> Option<Lit> {
        eval_const(self.fn_ir, value, &mut self.memo, &mut FxHashSet::default())
    }

    pub(crate) fn matrix_dims(&mut self, base: ValueId) -> Option<(i64, i64)> {
        matrix_known_dims(self.fn_ir, base, &mut self.memo, &mut FxHashSet::default())
    }

    pub(crate) fn push_invalid_write_index_lit(
        &mut self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> bool {
        let Some(lit) = self.eval_const_value(idx) else {
            return false;
        };
        let Err(error) = validate_index_lit_for_write(lit, span) else {
            return false;
        };
        self.errors.push(error);
        true
    }

    pub(crate) fn push_invalid_read_index_lit(
        &mut self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> bool {
        let Some(lit) = self.eval_const_value(idx) else {
            return false;
        };
        let Err(error) = validate_index_lit_for_read(lit, span) else {
            return false;
        };
        self.errors.push(error);
        true
    }

    pub(crate) fn condition_always_na_error(&self, cond: ValueId) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "condition is guaranteed to evaluate to NA at runtime".to_string(),
        )
        .at(self.fn_ir.values[cond].span)
        .origin(
            self.fn_ir.values[cond].span,
            "condition value originates here and propagates NA on all paths",
        )
        .constraint(
            self.fn_ir.values[cond].span,
            "branch conditions must evaluate to TRUE or FALSE",
        )
        .use_site(
            self.fn_ir.values[cond].span,
            "used here as an if/while condition",
        )
        .fix("guard NA before branching, for example with is.na(...) checks")
        .build()
    }

    pub(crate) fn assignment_index_always_na_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "index is guaranteed to evaluate to NA in assignment".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            "index value originates here and is always NA",
        )
        .constraint(span, "assignment indices must be non-NA integer scalars")
        .use_site(span, "used here as an assignment index")
        .fix("validate or cast the index before assignment")
        .build()
    }

    pub(crate) fn matrix_assignment_index_always_na_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "matrix assignment index is guaranteed to evaluate to NA".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            "matrix index originates here and is always NA",
        )
        .constraint(
            span,
            "matrix assignment indices must be non-NA integer scalars",
        )
        .use_site(span, "used here as a matrix assignment index")
        .fix("validate or cast the matrix index before assignment")
        .build()
    }

    pub(crate) fn assignment_index_below_one_range_error(
        &self,
        bid: BlockId,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> Option<RRException> {
        let facts = self.runtime.out_ranges_for_block(bid)?;
        let idx_range = facts.get(idx);
        interval_guarantees_below_one(&idx_range).then(|| {
            DiagnosticBuilder::new(
                "RR.RuntimeError",
                RRCode::E2007,
                Stage::Mir,
                "assignment index is guaranteed out of bounds (must be >= 1)".to_string(),
            )
            .at(span)
            .origin(
                self.fn_ir.values[idx].span,
                format!("index range is proven as {}", format_interval(&idx_range)),
            )
            .constraint(span, "R assignment indexing is 1-based")
            .use_site(span, "used here as an assignment index")
            .fix("shift the index into the 1-based domain before writing")
            .build()
        })
    }

    pub(crate) fn assignment_index_below_one_flow_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            "assignment index is guaranteed out of bounds (must be >= 1)".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            "dataflow proves the index is always < 1",
        )
        .constraint(span, "R assignment indexing is 1-based")
        .use_site(span, "used here as an assignment index")
        .fix("shift the index into the 1-based domain before writing")
        .build()
    }

    pub(crate) fn assignment_index_above_base_len_error(
        &self,
        bid: BlockId,
        base: ValueId,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> Option<RRException> {
        let facts = self.runtime.out_ranges_for_block(bid)?;
        let idx_range = facts.get(idx);
        interval_guarantees_above_base_len(&idx_range, base).then(|| {
            DiagnosticBuilder::new(
                "RR.RuntimeError",
                RRCode::E2007,
                Stage::Mir,
                "assignment index is guaranteed out of bounds (> length(base))".to_string(),
            )
            .at(span)
            .origin(
                self.fn_ir.values[idx].span,
                format!("index range is proven as {}", format_interval(&idx_range)),
            )
            .constraint(span, "assignment index must be <= length(base)")
            .use_site(span, "used here as an assignment index")
            .fix("clamp or guard the index against length(base) before writing")
            .build()
        })
    }

    pub(crate) fn matrix_assignment_below_one_range_error(
        &self,
        bid: BlockId,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> Option<RRException> {
        let facts = self.runtime.out_ranges_for_block(bid)?;
        let idx_range = facts.get(idx);
        interval_guarantees_below_one(&idx_range).then(|| {
            DiagnosticBuilder::new(
                "RR.RuntimeError",
                RRCode::E2007,
                Stage::Mir,
                "matrix assignment index is guaranteed out of bounds (must be >= 1)".to_string(),
            )
            .at(span)
            .origin(
                self.fn_ir.values[idx].span,
                format!("index range is proven as {}", format_interval(&idx_range)),
            )
            .constraint(span, "matrix indexing is 1-based")
            .use_site(span, "used here as a matrix assignment index")
            .fix("shift the index into the 1-based domain before writing")
            .build()
        })
    }

    pub(crate) fn matrix_assignment_below_one_flow_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            "matrix assignment index is guaranteed out of bounds (must be >= 1)".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            "dataflow proves the matrix index is always < 1",
        )
        .constraint(span, "matrix indexing is 1-based")
        .use_site(span, "used here as a matrix assignment index")
        .fix("shift the index into the 1-based domain before writing")
        .build()
    }

    pub(crate) fn matrix_assignment_extent_error(
        &mut self,
        bid: BlockId,
        base: ValueId,
        idx: ValueId,
        span: crate::utils::Span,
        axis: &str,
    ) -> Option<RRException> {
        let (rows, cols) = self.matrix_dims(base)?;
        let limit = if axis == "row" { rows } else { cols };
        if let Some(lit) = self.eval_const_value(idx)
            && let Some(i) = as_integral(&lit)
            && i > limit
        {
            return Some(self.matrix_extent_const_error(idx, span, axis, i, limit, true));
        }

        let facts = self.runtime.out_ranges_for_block(bid)?;
        let idx_range = facts.get(idx);
        interval_guarantees_above_const(&idx_range, limit)
            .then(|| self.matrix_extent_range_error(idx, span, axis, limit, &idx_range, true))
    }

    pub(crate) fn array3d_assignment_below_one_error(
        &self,
        bid: BlockId,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> Option<RRException> {
        let facts = self.runtime.out_ranges_for_block(bid)?;
        let idx_range = facts.get(idx);
        if interval_guarantees_below_one(&idx_range) {
            Some(self.array3d_below_one_range_error(idx, span, &idx_range, true))
        } else if flow_interval_guarantees_below_one(self.runtime.dataflow_interval(idx)) {
            Some(self.array3d_below_one_flow_error(idx, span, true))
        } else {
            None
        }
    }

    pub(crate) fn index_read_below_one_range_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
        idx_range: &crate::mir::analyze::range::RangeInterval,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            "index is guaranteed out of bounds (must be >= 1)".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            format!("index range is proven as {}", format_interval(idx_range)),
        )
        .constraint(span, "R indexing is 1-based at runtime")
        .use_site(span, "used here in an index read")
        .fix("shift the index into the 1-based domain before reading")
        .build()
    }

    pub(crate) fn index_read_below_one_flow_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            "index is guaranteed out of bounds (must be >= 1)".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            "dataflow proves the index is always < 1",
        )
        .constraint(span, "R indexing is 1-based at runtime")
        .use_site(span, "used here in an index read")
        .fix("shift the index into the 1-based domain before reading")
        .build()
    }

    pub(crate) fn index_read_above_base_len_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
        idx_range: &crate::mir::analyze::range::RangeInterval,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            "index is guaranteed out of bounds (> length(base))".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            format!("index range is proven as {}", format_interval(idx_range)),
        )
        .constraint(span, "index must be <= length(base)")
        .use_site(span, "used here in an index read")
        .fix("clamp or guard the index against length(base) before reading")
        .build()
    }

    pub(crate) fn matrix_read_below_one_range_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
        idx_range: &crate::mir::analyze::range::RangeInterval,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            "matrix index is guaranteed out of bounds (must be >= 1)".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            format!("index range is proven as {}", format_interval(idx_range)),
        )
        .constraint(span, "R matrix indexing is 1-based at runtime")
        .use_site(span, "used here in a matrix index read")
        .fix("shift the row/column index into the 1-based domain before reading")
        .build()
    }

    pub(crate) fn matrix_read_below_one_flow_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> RRException {
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            "matrix index is guaranteed out of bounds (must be >= 1)".to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            "dataflow proves the matrix index is always < 1",
        )
        .constraint(span, "R matrix indexing is 1-based at runtime")
        .use_site(span, "used here in a matrix index read")
        .fix("shift the row/column index into the 1-based domain before reading")
        .build()
    }

    pub(crate) fn matrix_read_extent_error(
        &mut self,
        base: ValueId,
        idx: ValueId,
        span: crate::utils::Span,
        axis: &str,
        idx_range: &crate::mir::analyze::range::RangeInterval,
    ) -> Option<RRException> {
        let (rows, cols) = self.matrix_dims(base)?;
        let limit = if axis == "row" { rows } else { cols };
        if let Some(lit) = self.eval_const_value(idx)
            && let Some(i) = as_integral(&lit)
            && i > limit
        {
            return Some(self.matrix_extent_const_error(idx, span, axis, i, limit, false));
        }
        interval_guarantees_above_const(idx_range, limit)
            .then(|| self.matrix_extent_range_error(idx, span, axis, limit, idx_range, false))
    }

    pub(crate) fn matrix_extent_const_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
        axis: &str,
        index: i64,
        limit: i64,
        is_assignment: bool,
    ) -> RRException {
        let use_site = if is_assignment {
            "used here as a matrix assignment index"
        } else {
            "used here in a matrix index read"
        };
        let fix = if is_assignment {
            format!("clamp or guard the {axis} index against the matrix extent before writing")
        } else {
            format!("clamp or guard the {axis} index against the matrix extent before reading")
        };
        let subject = if is_assignment {
            format!("matrix {axis} assignment index")
        } else {
            format!("matrix {axis} index")
        };
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            format!("{subject} is guaranteed out of bounds (>{axis}s={limit})"),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            format!("{axis} index is proven constant at {index}"),
        )
        .constraint(span, format!("matrix {axis} index must be <= {limit}"))
        .use_site(span, use_site)
        .fix(fix)
        .build()
    }

    pub(crate) fn matrix_extent_range_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
        axis: &str,
        limit: i64,
        idx_range: &crate::mir::analyze::range::RangeInterval,
        is_assignment: bool,
    ) -> RRException {
        let use_site = if is_assignment {
            "used here as a matrix assignment index"
        } else {
            "used here in a matrix index read"
        };
        let fix = if is_assignment {
            format!("clamp or guard the {axis} index against the matrix extent before writing")
        } else {
            format!("clamp or guard the {axis} index against the matrix extent before reading")
        };
        let subject = if is_assignment {
            format!("matrix {axis} assignment index")
        } else {
            format!("matrix {axis} index")
        };
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            format!("{subject} is guaranteed out of bounds (>{axis}s={limit})"),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            format!(
                "{axis} index range is proven as {}",
                format_interval(idx_range)
            ),
        )
        .constraint(span, format!("matrix {axis} index must be <= {limit}"))
        .use_site(span, use_site)
        .fix(fix)
        .build()
    }

    pub(crate) fn array3d_read_below_one_error(
        &self,
        vid: ValueId,
        idx: ValueId,
        span: crate::utils::Span,
    ) -> Option<RRException> {
        let facts = self.runtime.range_out.as_ref()?;
        let bid = bid_for_value(self.fn_ir, vid);
        let idx_range = facts[bid].get(idx);
        if interval_guarantees_below_one(&idx_range) {
            Some(self.array3d_below_one_range_error(idx, span, &idx_range, false))
        } else if flow_interval_guarantees_below_one(self.runtime.dataflow_interval(idx)) {
            Some(self.array3d_below_one_flow_error(idx, span, false))
        } else {
            None
        }
    }

    pub(crate) fn array3d_below_one_range_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
        idx_range: &crate::mir::analyze::range::RangeInterval,
        is_assignment: bool,
    ) -> RRException {
        let (message, constraint, use_site, fix) = if is_assignment {
            (
                "3D assignment index is guaranteed out of bounds (must be >= 1)",
                "3D array indexing is 1-based",
                "used here as a 3D assignment index",
                "shift the 3D index into the 1-based domain before writing",
            )
        } else {
            (
                "3D index is guaranteed out of bounds (must be >= 1)",
                "3D array indexing is 1-based at runtime",
                "used here in a 3D index read",
                "shift the 3D index into the 1-based domain before reading",
            )
        };
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            message.to_string(),
        )
        .at(span)
        .origin(
            self.fn_ir.values[idx].span,
            format!("index range is proven as {}", format_interval(idx_range)),
        )
        .constraint(span, constraint)
        .use_site(span, use_site)
        .fix(fix)
        .build()
    }

    pub(crate) fn array3d_below_one_flow_error(
        &self,
        idx: ValueId,
        span: crate::utils::Span,
        is_assignment: bool,
    ) -> RRException {
        let (message, constraint, use_site, fix, origin) = if is_assignment {
            (
                "3D assignment index is guaranteed out of bounds (must be >= 1)",
                "3D array indexing is 1-based",
                "used here as a 3D assignment index",
                "shift the 3D index into the 1-based domain before writing",
                "dataflow proves the 3D index is always < 1",
            )
        } else {
            (
                "3D index is guaranteed out of bounds (must be >= 1)",
                "3D array indexing is 1-based at runtime",
                "used here in a 3D index read",
                "shift the 3D index into the 1-based domain before reading",
                "dataflow proves the 3D index is always < 1",
            )
        };
        DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            message.to_string(),
        )
        .at(span)
        .origin(self.fn_ir.values[idx].span, origin)
        .constraint(span, constraint)
        .use_site(span, use_site)
        .fix(fix)
        .build()
    }
}
