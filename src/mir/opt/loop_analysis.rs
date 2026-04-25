use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub header: BlockId,
    pub latch: BlockId,           // The block that jumps back to header
    pub exits: Vec<BlockId>,      // Blocks outside loop targeted by loop blocks
    pub body: FxHashSet<BlockId>, // All blocks in the loop

    pub is_seq_len: Option<ValueId>,   // If it's 1:N, stores N
    pub is_seq_along: Option<ValueId>, // If it's seq_along(X), stores X
    pub iv: Option<InductionVar>,
    pub limit: Option<ValueId>,
    pub limit_adjust: i64,
}

#[derive(Debug, Clone)]
pub struct InductionVar {
    pub phi_val: ValueId,
    pub init_val: ValueId,
    pub step: i64,      // +1, -1, etc.
    pub step_op: BinOp, // Add/Sub
}

pub struct LoopAnalyzer<'a> {
    fn_ir: &'a FnIR,
    preds: FxHashMap<BlockId, Vec<BlockId>>,
}

impl<'a> LoopAnalyzer<'a> {
    pub fn new(fn_ir: &'a FnIR) -> Self {
        let preds = build_pred_map(fn_ir);
        Self { fn_ir, preds }
    }

    pub fn find_loops(&self) -> Vec<LoopInfo> {
        // 1. Compute Dominators (Simplified for structured/reducible CFG)
        // For standard "natural loops", we look for back-edges A->B where B dominates A.
        // B is header, A is latch.

        let doms = self.compute_dominators();
        let mut loops = Vec::new();
        let mut seen: FxHashSet<(BlockId, Vec<BlockId>, Vec<BlockId>)> = FxHashSet::default();

        // 2. Find Back-edges
        for (src, targets) in self.get_cfg_edges() {
            for &dst in &targets {
                if self.dominates(&doms, dst, src) {
                    // Back-edge src -> dst
                    // dst is Header, src is Latch
                    if let Some(loop_info) = self.analyze_natural_loop(dst, src) {
                        let mut body_key: Vec<BlockId> = loop_info.body.iter().copied().collect();
                        body_key.sort_unstable();
                        let mut exits_key = loop_info.exits.clone();
                        exits_key.sort_unstable();
                        exits_key.dedup();
                        if seen.insert((loop_info.header, body_key, exits_key)) {
                            loops.push(loop_info);
                        }
                    }
                }
            }
        }

        loops
    }

    fn analyze_natural_loop(&self, header: BlockId, latch: BlockId) -> Option<LoopInfo> {
        // Collect body blocks (Reach backwards from latch to header)
        let mut body = FxHashSet::default();
        let mut stack = vec![latch];
        body.insert(header);
        body.insert(latch);

        while let Some(node) = stack.pop() {
            // Natural-loop backwalk must not traverse predecessors of the header.
            // Including header predecessors pulls preheader/outer blocks into the loop body
            // and breaks IV seed/step inference.
            if node == header {
                continue;
            }
            if let Some(node_preds) = self.preds.get(&node) {
                for &pred in node_preds {
                    if !body.contains(&pred) {
                        body.insert(pred);
                        stack.push(pred);
                    }
                }
            }
        }

        // Find exits (successors of body blocks NOT in body)
        let mut exits = Vec::new();
        for &block in &body {
            let succs = self.get_block_successors(block);
            for succ in succs {
                if !body.contains(&succ) {
                    exits.push(succ);
                }
            }
        }
        exits.sort_unstable();
        exits.dedup();

        // Analyze IV
        let (iv, limit) = self.find_induction_variable(header, &body);

        // Detect if it's 1:N (Canonical seq_len loop)
        let mut is_seq_len = None;
        let mut is_seq_along = None;
        if let Some(iv_val) = &iv {
            let init_is_1 = self.const_integral_value(iv_val.init_val) == Some(1);

            if init_is_1 && iv_val.step == 1 && iv_val.step_op == BinOp::Add {
                // Treat 1..N and 1..<N as canonical ascending vector ranges.
                if let Terminator::If { cond, .. } = &self.fn_ir.blocks[header].term
                    && let Some((op, lhs, rhs)) = self.resolve_condition_compare(*cond)
                    && (((op == BinOp::Le || op == BinOp::Lt)
                        && self.is_phi_equivalent(lhs, iv_val.phi_val))
                        || ((op == BinOp::Ge || op == BinOp::Gt)
                            && self.is_phi_equivalent(rhs, iv_val.phi_val)))
                {
                    let cmp_limit = if op == BinOp::Le || op == BinOp::Lt {
                        rhs
                    } else {
                        lhs
                    };
                    is_seq_len = Some(cmp_limit);

                    // Check if limit is length(X), including load aliases assigned from length(X).
                    is_seq_along = self.resolve_len_base(cmp_limit);
                }
            }
        }

        let limit_adjust = iv
            .as_ref()
            .map(|iv_val| self.compute_limit_adjust(header, iv_val))
            .unwrap_or(0);

        Some(LoopInfo {
            header,
            latch,
            body,
            exits,
            iv,
            limit,
            limit_adjust,
            is_seq_len,
            is_seq_along,
        })
    }

    fn compute_limit_adjust(&self, header: BlockId, iv: &InductionVar) -> i64 {
        let Terminator::If { cond, .. } = &self.fn_ir.blocks[header].term else {
            return 0;
        };
        let Some((op, lhs, rhs)) = self.resolve_condition_compare(*cond) else {
            return 0;
        };
        let lhs = self.normalize_floor_like_value(lhs);
        let rhs = self.normalize_floor_like_value(rhs);
        let normalized_op = if self.is_phi_equivalent(lhs, iv.phi_val) {
            op
        } else if self.is_phi_equivalent(rhs, iv.phi_val) {
            flip_compare(op)
        } else {
            return 0;
        };

        if iv.step > 0 {
            match normalized_op {
                BinOp::Lt => -1,
                BinOp::Le => 0,
                _ => 0,
            }
        } else if iv.step < 0 {
            match normalized_op {
                BinOp::Gt => 1,
                BinOp::Ge => 0,
                _ => 0,
            }
        } else {
            0
        }
    }

    fn find_induction_variable(
        &self,
        header: BlockId,
        body: &FxHashSet<BlockId>,
    ) -> (Option<InductionVar>, Option<ValueId>) {
        let mut candidates: Vec<InductionVar> = Vec::new();

        for (val_id, val) in self.fn_ir.values.iter().enumerate() {
            let ValueKind::Phi { args } = &val.kind else {
                continue;
            };
            if args.len() < 2 {
                continue;
            }

            let mut init_val: Option<ValueId> = None;
            let mut loop_next_vals: Vec<ValueId> = Vec::new();
            let mut invalid = false;
            for (arg_val, pred_bb) in args {
                if body.contains(pred_bb) {
                    loop_next_vals.push(*arg_val);
                } else if init_val.is_none() {
                    init_val = Some(*arg_val);
                } else {
                    // Multiple non-loop seeds are ambiguous.
                    invalid = true;
                    break;
                }
            }
            if invalid || loop_next_vals.is_empty() {
                continue;
            }
            let Some(init_val) = init_val else {
                continue;
            };

            let mut step_sig: Option<(i64, BinOp)> = None;
            let mut saw_progress = false;
            for next in loop_next_vals {
                // Some structured loop forms pass the IV through one latch and update it in
                // another latch. Treat pure pass-through as neutral and infer the step from
                // actual update edges.
                if self.is_phi_equivalent(next, val_id) {
                    continue;
                }
                let Some((step, step_op)) = self.analyze_step(next, val_id) else {
                    step_sig = None;
                    saw_progress = false;
                    break;
                };
                saw_progress = true;
                match step_sig {
                    None => step_sig = Some((step, step_op)),
                    Some((prev_step, prev_op)) if prev_step == step && prev_op == step_op => {}
                    Some(_) => {
                        step_sig = None;
                        saw_progress = false;
                        break;
                    }
                }
            }
            if !saw_progress {
                continue;
            }
            let Some((step, step_op)) = step_sig else {
                continue;
            };

            candidates.push(InductionVar {
                phi_val: val_id,
                init_val,
                step,
                step_op,
            });
        }

        if candidates.is_empty() {
            return self.find_counter_iv_from_condition(header, body);
        }

        if let Terminator::If { cond, .. } = &self.fn_ir.blocks[header].term
            && let Some((_, lhs, rhs)) = self.resolve_condition_compare(*cond)
        {
            for iv in &candidates {
                if self.is_phi_equivalent(lhs, iv.phi_val) {
                    return (Some(iv.clone()), Some(rhs));
                }
                if self.is_phi_equivalent(rhs, iv.phi_val) {
                    return (Some(iv.clone()), Some(lhs));
                }
            }
        }

        (Some(candidates.remove(0)), None)
    }

    fn find_counter_iv_from_condition(
        &self,
        header: BlockId,
        body: &FxHashSet<BlockId>,
    ) -> (Option<InductionVar>, Option<ValueId>) {
        let Terminator::If { cond, .. } = &self.fn_ir.blocks[header].term else {
            return (None, None);
        };
        let Some((op, lhs, rhs)) = self.resolve_condition_compare(*cond) else {
            return (None, None);
        };
        if !matches!(op, BinOp::Le | BinOp::Lt | BinOp::Ge | BinOp::Gt) {
            return (None, None);
        }

        let lhs_counter = self.normalize_floor_like_value(lhs);
        let rhs_counter = self.normalize_floor_like_value(rhs);
        let (counter, bound) =
            if matches!(self.fn_ir.values[lhs_counter].kind, ValueKind::Load { .. }) {
                (lhs_counter, rhs)
            } else if matches!(self.fn_ir.values[rhs_counter].kind, ValueKind::Load { .. }) {
                (rhs_counter, lhs)
            } else {
                return (None, None);
            };
        let ValueKind::Load { var } = &self.fn_ir.values[counter].kind else {
            return (None, None);
        };
        let init_val = match self.find_seed_assignment_outside_loop(var, body) {
            Some(seed) => seed,
            None => return (None, None),
        };
        let (step, step_op) = match self.find_var_step_in_loop(var, counter, body) {
            Some(sig) => sig,
            None => return (None, None),
        };

        (
            Some(InductionVar {
                phi_val: counter,
                init_val,
                step,
                step_op,
            }),
            Some(bound),
        )
    }

    fn normalize_floor_like_value(&self, mut vid: ValueId) -> ValueId {
        loop {
            let ValueKind::Call {
                callee,
                args,
                names,
            } = &self.fn_ir.values[vid].kind
            else {
                return vid;
            };
            if !self.call_is_floor_like(vid, callee, args, names) {
                return vid;
            }
            vid = args[0];
        }
    }

    fn call_is_single_positional(&self, args: &[ValueId], names: &[Option<String>]) -> bool {
        args.len() == 1
            && names.len() <= 1
            && names
                .first()
                .and_then(std::option::Option::as_ref)
                .is_none()
    }

    fn call_is_floor_like(
        &self,
        vid: ValueId,
        callee: &str,
        args: &[ValueId],
        names: &[Option<String>],
    ) -> bool {
        if !self.call_is_single_positional(args, names) {
            return false;
        }
        self.fn_ir
            .call_semantics(vid)
            .and_then(|semantics| match semantics {
                CallSemantics::Builtin(kind) => Some(kind.is_floor_like()),
                _ => None,
            })
            .unwrap_or_else(|| {
                builtin_kind_for_name(callee.strip_prefix("base::").unwrap_or(callee))
                    .is_some_and(BuiltinKind::is_floor_like)
            })
    }

    fn find_seed_assignment_outside_loop(
        &self,
        var: &str,
        body: &FxHashSet<BlockId>,
    ) -> Option<ValueId> {
        let mut seed = None;
        for (bid, block) in self.fn_ir.blocks.iter().enumerate() {
            if body.contains(&bid) {
                continue;
            }
            for ins in &block.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst == var {
                    seed = Some(*src);
                }
            }
        }
        seed
    }

    fn find_var_step_in_loop(
        &self,
        var: &str,
        iv_val: ValueId,
        body: &FxHashSet<BlockId>,
    ) -> Option<(i64, BinOp)> {
        let mut blocks: Vec<BlockId> = body.iter().copied().collect();
        blocks.sort_unstable();

        let mut saw_update = false;
        let mut step_sig: Option<(i64, BinOp)> = None;
        for bid in blocks {
            for ins in &self.fn_ir.blocks[bid].instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if self.is_phi_equivalent(*src, iv_val) {
                    continue;
                }
                let (step, op) = self.analyze_step(*src, iv_val)?;
                saw_update = true;
                match step_sig {
                    None => step_sig = Some((step, op)),
                    Some((prev_step, prev_op)) if prev_step == step && prev_op == op => {}
                    Some(_) => return None,
                }
            }
        }
        if !saw_update {
            return None;
        }
        step_sig
    }

    fn analyze_step(&self, val_id: ValueId, phi_id: ValueId) -> Option<(i64, BinOp)> {
        let mut seen_vals = FxHashSet::default();
        let mut seen_vars = FxHashSet::default();
        self.analyze_step_rec(val_id, phi_id, &mut seen_vals, &mut seen_vars)
    }

    fn analyze_step_rec(
        &self,
        val_id: ValueId,
        phi_id: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<(i64, BinOp)> {
        if !seen_vals.insert(val_id) {
            return None;
        }
        let val = &self.fn_ir.values[val_id];
        match &val.kind {
            ValueKind::Call {
                callee,
                args,
                names,
            } => self
                .call_is_floor_like(val_id, callee, args, names)
                .then(|| self.analyze_step_rec(args[0], phi_id, seen_vals, seen_vars))
                .flatten(),
            ValueKind::Binary { op, lhs, rhs } => {
                if self.value_depends_on_phi(*lhs, phi_id)
                    && let Some(n) = self.const_integral_value(*rhs)
                {
                    return Some((n, *op));
                }
                if *op == BinOp::Add
                    && self.value_depends_on_phi(*rhs, phi_id)
                    && let Some(n) = self.const_integral_value(*lhs)
                {
                    return Some((n, *op));
                }
                None
            }
            ValueKind::Load { var } => {
                self.analyze_step_from_var(var, phi_id, seen_vals, seen_vars)
            }
            ValueKind::Phi { args } => {
                let mut step_sig: Option<(i64, BinOp)> = None;
                for (arg, _) in args {
                    if self.is_phi_equivalent(*arg, phi_id) {
                        continue;
                    }
                    let step = self.analyze_step_rec(*arg, phi_id, seen_vals, seen_vars)?;
                    match step_sig {
                        None => step_sig = Some(step),
                        Some(prev) if prev == step => {}
                        Some(_) => return None,
                    }
                }
                step_sig
            }
            _ => None,
        }
    }

    fn analyze_step_from_var(
        &self,
        var: &str,
        phi_id: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<(i64, BinOp)> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }

        let mut found_assignment = false;
        let mut step_sig: Option<(i64, BinOp)> = None;
        for block in &self.fn_ir.blocks {
            for instr in &block.instrs {
                let Instr::Assign { dst, src, .. } = instr else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if !self.value_depends_on_phi(*src, phi_id) {
                    // Ignore non-recursive seeds/reinitializations (e.g., i <- 1).
                    continue;
                }
                found_assignment = true;
                if self.is_phi_equivalent(*src, phi_id) {
                    continue;
                }
                let step = self.analyze_step_rec(*src, phi_id, seen_vals, seen_vars)?;
                match step_sig {
                    None => step_sig = Some(step),
                    Some(prev) if prev == step => {}
                    Some(_) => return None,
                }
            }
        }

        seen_vars.remove(var);
        if !found_assignment {
            return None;
        }
        step_sig
    }

    fn value_depends_on_phi(&self, root: ValueId, phi_id: ValueId) -> bool {
        fn rec(
            analyzer: &LoopAnalyzer<'_>,
            root: ValueId,
            phi_id: ValueId,
            seen_vals: &mut FxHashSet<ValueId>,
        ) -> bool {
            if analyzer.is_phi_equivalent(root, phi_id) {
                return true;
            }
            if !seen_vals.insert(root) {
                return false;
            }
            match &analyzer.fn_ir.values[root].kind {
                ValueKind::Binary { lhs, rhs, .. } => {
                    rec(analyzer, *lhs, phi_id, seen_vals) || rec(analyzer, *rhs, phi_id, seen_vals)
                }
                ValueKind::Unary { rhs, .. } => rec(analyzer, *rhs, phi_id, seen_vals),
                ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                    args.iter().any(|a| rec(analyzer, *a, phi_id, seen_vals))
                }
                ValueKind::Phi { args } => args
                    .iter()
                    .any(|(a, _)| rec(analyzer, *a, phi_id, seen_vals)),
                ValueKind::Len { base } | ValueKind::Indices { base } => {
                    rec(analyzer, *base, phi_id, seen_vals)
                }
                ValueKind::Range { start, end } => {
                    rec(analyzer, *start, phi_id, seen_vals)
                        || rec(analyzer, *end, phi_id, seen_vals)
                }
                ValueKind::RecordLit { fields } => fields
                    .iter()
                    .any(|(_, value)| rec(analyzer, *value, phi_id, seen_vals)),
                ValueKind::FieldGet { base, .. } => rec(analyzer, *base, phi_id, seen_vals),
                ValueKind::FieldSet { base, value, .. } => {
                    rec(analyzer, *base, phi_id, seen_vals)
                        || rec(analyzer, *value, phi_id, seen_vals)
                }
                ValueKind::Index1D { base, idx, .. } => {
                    rec(analyzer, *base, phi_id, seen_vals)
                        || rec(analyzer, *idx, phi_id, seen_vals)
                }
                ValueKind::Index2D { base, r, c } => {
                    rec(analyzer, *base, phi_id, seen_vals)
                        || rec(analyzer, *r, phi_id, seen_vals)
                        || rec(analyzer, *c, phi_id, seen_vals)
                }
                ValueKind::Index3D { base, i, j, k } => {
                    rec(analyzer, *base, phi_id, seen_vals)
                        || rec(analyzer, *i, phi_id, seen_vals)
                        || rec(analyzer, *j, phi_id, seen_vals)
                        || rec(analyzer, *k, phi_id, seen_vals)
                }
                ValueKind::Const(_)
                | ValueKind::Param { .. }
                | ValueKind::Load { .. }
                | ValueKind::RSymbol { .. } => false,
            }
        }
        rec(self, root, phi_id, &mut FxHashSet::default())
    }

    fn is_phi_equivalent(&self, candidate: ValueId, phi_id: ValueId) -> bool {
        let mut seen = vec![false; self.fn_ir.values.len()];
        let mut seen_vars = FxHashSet::default();
        self.is_phi_equivalent_rec(candidate, phi_id, &mut seen, &mut seen_vars)
    }

    fn is_phi_equivalent_rec(
        &self,
        candidate: ValueId,
        phi_id: ValueId,
        seen: &mut [bool],
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        if candidate >= self.fn_ir.values.len() {
            return false;
        }
        if candidate == phi_id {
            return true;
        }
        if seen[candidate] {
            return false;
        }
        seen[candidate] = true;
        match &self.fn_ir.values[candidate].kind {
            ValueKind::Load { var } => {
                if self.fn_ir.values[phi_id].origin_var.as_deref() == Some(var.as_str()) {
                    return true;
                }
                self.load_var_is_phi_equivalent(var, phi_id, seen, seen_vars)
            }
            ValueKind::Phi { args } if args.is_empty() => {
                match (
                    self.fn_ir.values[candidate].origin_var.as_deref(),
                    self.fn_ir.values[phi_id].origin_var.as_deref(),
                ) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                }
            }
            ValueKind::Phi { args } => args
                .iter()
                .all(|(v, _)| self.is_phi_equivalent_rec(*v, phi_id, seen, seen_vars)),
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                self.call_is_floor_like(candidate, callee, args, names)
                    && self.is_phi_equivalent_rec(args[0], phi_id, seen, seen_vars)
            }
            _ => false,
        }
    }

    fn load_var_is_phi_equivalent(
        &self,
        var: &str,
        phi_id: ValueId,
        seen_vals: &mut [bool],
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        if !seen_vars.insert(var.to_string()) {
            return false;
        }

        let mut found = false;
        let mut all_match = true;
        for bb in &self.fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                found = true;
                if !self.is_phi_equivalent_rec(*src, phi_id, seen_vals, seen_vars) {
                    all_match = false;
                    break;
                }
            }
            if !all_match {
                break;
            }
        }

        seen_vars.remove(var);
        found && all_match
    }

    fn const_integral_value(&self, vid: ValueId) -> Option<i64> {
        fn rec(
            analyzer: &LoopAnalyzer<'_>,
            vid: ValueId,
            seen_vals: &mut FxHashSet<ValueId>,
            seen_vars: &mut FxHashSet<String>,
        ) -> Option<i64> {
            if !seen_vals.insert(vid) {
                return None;
            }
            match &analyzer.fn_ir.values[vid].kind {
                ValueKind::Const(Lit::Int(n)) => Some(*n),
                ValueKind::Const(Lit::Float(f))
                    if f.is_finite()
                        && (*f - f.trunc()).abs() < f64::EPSILON
                        && *f >= i64::MIN as f64
                        && *f <= i64::MAX as f64 =>
                {
                    Some(*f as i64)
                }
                ValueKind::Load { var } => {
                    if !seen_vars.insert(var.to_string()) {
                        return None;
                    }
                    let mut unique: Option<i64> = None;
                    for bb in &analyzer.fn_ir.blocks {
                        for ins in &bb.instrs {
                            let Instr::Assign { dst, src, .. } = ins else {
                                continue;
                            };
                            if dst != var {
                                continue;
                            }
                            let n = rec(analyzer, *src, seen_vals, seen_vars)?;
                            match unique {
                                None => unique = Some(n),
                                Some(prev) if prev == n => {}
                                Some(_) => return None,
                            }
                        }
                    }
                    seen_vars.remove(var);
                    unique
                }
                ValueKind::Phi { args } => {
                    let mut unique: Option<i64> = None;
                    for (arg, _) in args {
                        let n = rec(analyzer, *arg, seen_vals, seen_vars)?;
                        match unique {
                            None => unique = Some(n),
                            Some(prev) if prev == n => {}
                            Some(_) => return None,
                        }
                    }
                    unique
                }
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } => analyzer
                    .call_is_floor_like(vid, callee, args, names)
                    .then(|| rec(analyzer, args[0], seen_vals, seen_vars))
                    .flatten(),
                _ => None,
            }
        }
        rec(
            self,
            vid,
            &mut FxHashSet::<ValueId>::default(),
            &mut FxHashSet::<String>::default(),
        )
    }

    fn resolve_len_base(&self, vid: ValueId) -> Option<ValueId> {
        let mut seen_vals = FxHashSet::default();
        let mut seen_vars = FxHashSet::default();
        self.resolve_len_base_rec(vid, &mut seen_vals, &mut seen_vars)
    }

    fn resolve_condition_compare(&self, vid: ValueId) -> Option<(BinOp, ValueId, ValueId)> {
        let mut seen_vals = FxHashSet::default();
        let mut seen_vars = FxHashSet::default();
        self.resolve_condition_compare_rec(vid, &mut seen_vals, &mut seen_vars)
    }

    fn resolve_condition_compare_rec(
        &self,
        vid: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<(BinOp, ValueId, ValueId)> {
        if !seen_vals.insert(vid) {
            return None;
        }
        match &self.fn_ir.values[vid].kind {
            ValueKind::Binary { op, lhs, rhs }
                if matches!(op, BinOp::Le | BinOp::Lt | BinOp::Ge | BinOp::Gt) =>
            {
                Some((*op, *lhs, *rhs))
            }
            ValueKind::Call { callee, args, .. }
                if matches!(callee.as_str(), "rr_truthy1" | "rr_bool") && !args.is_empty() =>
            {
                self.resolve_condition_compare_rec(args[0], seen_vals, seen_vars)
            }
            ValueKind::Load { var } => {
                let src = self.resolve_unique_assignment(var, seen_vals, seen_vars)?;
                self.resolve_condition_compare_rec(src, seen_vals, seen_vars)
            }
            _ => None,
        }
    }

    fn resolve_unique_assignment(
        &self,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }

        let mut unique: Option<ValueId> = None;
        for bb in &self.fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if let Some(prev) = unique {
                    if prev != *src {
                        seen_vars.remove(var);
                        return None;
                    }
                } else {
                    unique = Some(*src);
                }
            }
        }
        seen_vars.remove(var);
        let src = unique?;
        if seen_vals.contains(&src) {
            return None;
        }
        Some(src)
    }

    fn resolve_len_base_rec(
        &self,
        vid: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        if !seen_vals.insert(vid) {
            return None;
        }
        match &self.fn_ir.values[vid].kind {
            ValueKind::Len { base } => Some(*base),
            ValueKind::Load { var } => self.resolve_len_base_from_var(var, seen_vals, seen_vars),
            ValueKind::Phi { args } if !args.is_empty() => {
                let mut unique = None;
                for (arg, _) in args {
                    let base = self.resolve_len_base_rec(*arg, seen_vals, seen_vars)?;
                    match unique {
                        None => unique = Some(base),
                        Some(prev) if prev == base => {}
                        Some(_) => return None,
                    }
                }
                unique
            }
            _ => None,
        }
    }

    fn resolve_len_base_from_var(
        &self,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }

        let mut unique = None;
        for block in &self.fn_ir.blocks {
            for instr in &block.instrs {
                let Instr::Assign { dst, src, .. } = instr else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                let base = self.resolve_len_base_rec(*src, seen_vals, seen_vars)?;
                match unique {
                    None => unique = Some(base),
                    Some(prev) if prev == base => {}
                    Some(_) => return None,
                }
            }
        }
        unique
    }

    // Helpers
    fn get_cfg_edges(&self) -> Vec<(BlockId, Vec<BlockId>)> {
        self.fn_ir
            .blocks
            .iter()
            .map(|b| {
                let succs = self.get_block_successors(b.id);
                (b.id, succs)
            })
            .collect()
    }

    fn get_block_successors(&self, bid: BlockId) -> Vec<BlockId> {
        match &self.fn_ir.blocks[bid].term {
            Terminator::Goto(t) => vec![*t],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            _ => vec![],
        }
    }

    fn compute_dominators(&self) -> FxHashMap<BlockId, FxHashSet<BlockId>> {
        // Naive Iterative Dominators
        // Dom(n) = {n} U (Inter(Dom(p)) for p in preds(n))
        // Init: Dom(entry) = {entry}, Dom(others) = All Blocks

        let all_blocks: FxHashSet<BlockId> = (0..self.fn_ir.blocks.len()).collect();
        let mut doms: FxHashMap<BlockId, FxHashSet<BlockId>> = FxHashMap::default();

        // Init
        doms.insert(
            self.fn_ir.entry,
            std::iter::once(self.fn_ir.entry).collect(),
        );
        for b in &all_blocks {
            if *b != self.fn_ir.entry {
                doms.insert(*b, all_blocks.clone());
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for bb in 0..self.fn_ir.blocks.len() {
                if bb == self.fn_ir.entry {
                    continue;
                }

                let preds = self.preds.get(&bb).cloned().unwrap_or_default();
                if preds.is_empty() {
                    continue;
                } // Unreachable

                // Intersect preds
                let mut new_dom: Option<FxHashSet<BlockId>> = None;
                for p in preds {
                    if let Some(p_dom) = doms.get(&p) {
                        match new_dom {
                            None => new_dom = Some(p_dom.clone()),
                            Some(ref mut set) => set.retain(|x| p_dom.contains(x)),
                        }
                    }
                }

                if let Some(mut set) = new_dom {
                    set.insert(bb);
                    if doms.get(&bb).is_some_and(|curr| set != *curr) {
                        doms.insert(bb, set);
                        changed = true;
                    }
                }
            }
        }
        doms
    }

    fn dominates(
        &self,
        doms: &FxHashMap<BlockId, FxHashSet<BlockId>>,
        master: BlockId,
        slave: BlockId,
    ) -> bool {
        if let Some(set) = doms.get(&slave) {
            set.contains(&master)
        } else {
            false
        }
    }
}

fn flip_compare(op: BinOp) -> BinOp {
    match op {
        BinOp::Le => BinOp::Ge,
        BinOp::Lt => BinOp::Gt,
        BinOp::Ge => BinOp::Le,
        BinOp::Gt => BinOp::Lt,
        other => other,
    }
}

pub fn build_pred_map(fn_ir: &FnIR) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut map = FxHashMap::default();
    for (src, blk) in fn_ir.blocks.iter().enumerate() {
        let targets = match &blk.term {
            Terminator::Goto(t) => vec![*t],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            _ => vec![],
        };
        for t in targets {
            map.entry(t).or_insert_with(Vec::new).push(src);
        }
    }
    map
}
