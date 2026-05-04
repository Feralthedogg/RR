use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo};
use crate::mir::{BinOp, FnIR, Instr, Lit, Terminator, ValueId, ValueKind};

pub struct MirLoopOptimizer;

impl Default for MirLoopOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MirLoopOptimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, fn_ir: &mut FnIR) -> bool {
        self.optimize_with_count(fn_ir) > 0
    }

    pub fn optimize_with_count(&self, fn_ir: &mut FnIR) -> usize {
        let loops = LoopAnalyzer::new(fn_ir).find_loops();
        self.optimize_with_loop_info(fn_ir, &loops)
    }

    pub fn optimize_with_loop_info(&self, fn_ir: &mut FnIR, loops: &[LoopInfo]) -> usize {
        let mut count = 0usize;
        for lp in loops {
            if self.canonicalize_loop(fn_ir, lp) {
                count += 1;
            }
            if self.vectorize_loop(fn_ir, lp) {
                count += 1;
            }
        }
        count
    }

    fn canonicalize_loop(&self, fn_ir: &mut FnIR, lp: &LoopInfo) -> bool {
        let Some(iv) = lp.iv.as_ref() else {
            return false;
        };
        if !self.iv_progression_is_integral(fn_ir, lp) {
            return false;
        }

        let mut body_blocks: Vec<usize> = lp.body.iter().copied().collect();
        body_blocks.sort_unstable();
        let loop_values = self.collect_loop_reachable_values(fn_ir, &body_blocks);

        let mut changed = false;
        for vid in loop_values {
            let replacement = match fn_ir.values[vid].kind.clone() {
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } => {
                    let is_floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
                    let has_single_positional_arg = args.len() == 1
                        && names.len() <= 1
                        && names
                            .first()
                            .and_then(std::option::Option::as_ref)
                            .is_none();
                    if !(is_floor_like
                        && has_single_positional_arg
                        && Self::is_iv_equivalent(fn_ir, args[0], iv.phi_val))
                    {
                        None
                    } else {
                        self.floor_identity_replacement(fn_ir, args[0], iv.phi_val)
                    }
                }
                _ => None,
            };
            if let Some(new_kind) = replacement
                && fn_ir.values[vid].kind != new_kind
            {
                fn_ir.values[vid].kind = new_kind;
                changed = true;
            }
        }
        changed |= self.mark_seq_along_index_safety(fn_ir, lp, iv.phi_val, &body_blocks);
        changed
    }

    fn vectorize_loop(&self, fn_ir: &mut FnIR, lp: &LoopInfo) -> bool {
        // This legacy fast path is only valid for exact 1..N full-range loops.
        if !self.is_exact_full_range_loop(fn_ir, lp) {
            return false;
        }

        // Require a single body block plus header.
        if lp.body.len() != 2 {
            return false;
        } // Header + Body block

        // Find the body block (not the header)
        let Some(body_bb) = lp.body.iter().find(|&&b| b != lp.header).copied() else {
            return false;
        };

        // Verify body block instructions
        // We look for: x[i] <- x[i] op y[i]
        // where i is the IV.
        let iv = match lp.iv.as_ref() {
            Some(iv) => iv.phi_val,
            None => return false,
        };

        let mut vectorized_instrs = Vec::new();
        let mut is_pure_vectorizable = true;

        // For simplicity: look at all StoreIndex1D in the body block
        let instrs = fn_ir.blocks[body_bb].instrs.clone();
        for instr in &instrs {
            match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    span,
                    ..
                } => {
                    if *idx == iv {
                        // Check if val is a Binary(Add, Index1D(A, iv), Index1D(B, iv))
                        let attempt = self.try_vectorize_value(fn_ir, *val, iv);
                        if let Some(transformed) = attempt {
                            vectorized_instrs.push(Instr::StoreIndex1D {
                                base: *base,
                                idx: iv,
                                val: transformed,
                                is_safe: true,
                                is_na_safe: false,
                                is_vector: true,
                                span: *span,
                            });
                        } else {
                            is_pure_vectorizable = false;
                        }
                    } else {
                        is_pure_vectorizable = false;
                    }
                }
                _ => is_pure_vectorizable = false,
            }
        }

        if !is_pure_vectorizable || vectorized_instrs.is_empty() {
            return false;
        }

        // 3. Construct new body block (vectorized)
        let new_body_bb = fn_ir.add_block();
        fn_ir.blocks[new_body_bb].instrs = vectorized_instrs;

        // 4. Update CFG
        // Header -> NewBody
        fn_ir.blocks[lp.header].term = Terminator::Goto(new_body_bb);

        // NewBody -> Exit
        let exit_bb = if !lp.exits.is_empty() {
            lp.exits[0]
        } else {
            return false;
        };
        fn_ir.blocks[new_body_bb].term = Terminator::Goto(exit_bb);

        // Ensure predecessors are updated?
        // simplify_cfg will handle reachability of old body.

        true
    }

    fn is_exact_full_range_loop(&self, fn_ir: &FnIR, lp: &LoopInfo) -> bool {
        let Some(iv) = lp.iv.as_ref() else {
            return false;
        };
        if lp.is_seq_len.is_none()
            || lp.limit_adjust != 0
            || Self::const_integral_value(fn_ir, iv.init_val) != Some(1)
            || iv.step != 1
            || iv.step_op != BinOp::Add
        {
            return false;
        }
        let Some(limit) = lp.is_seq_len else {
            return false;
        };
        !matches!(
            fn_ir.values[limit].kind,
            ValueKind::Binary { .. }
                | ValueKind::Range { .. }
                | ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
        )
    }

    fn try_vectorize_value(
        &self,
        fn_ir: &mut FnIR,
        val_id: ValueId,
        iv_id: ValueId,
    ) -> Option<ValueId> {
        let val_kind = fn_ir.values[val_id].kind.clone();
        let span = fn_ir.values[val_id].span;
        let facts = fn_ir.values[val_id].facts;

        match val_kind {
            ValueKind::Binary { op, lhs, rhs } => {
                let v_lhs = self.try_vectorize_value(fn_ir, lhs, iv_id)?;
                let v_rhs = self.try_vectorize_value(fn_ir, rhs, iv_id)?;

                let new_id = fn_ir.add_value(
                    ValueKind::Binary {
                        op,
                        lhs: v_lhs,
                        rhs: v_rhs,
                    },
                    span,
                    facts,
                    None,
                );
                fn_ir.values[new_id].value_ty = fn_ir.values[val_id].value_ty;
                fn_ir.values[new_id].value_term = fn_ir.values[val_id].value_term.clone();
                Some(new_id)
            }
            ValueKind::Index1D { base, idx, .. } => {
                if idx == iv_id {
                    // Vectorization: Return the base array directly
                    // This transforms a[i] -> a
                    Some(base)
                } else {
                    None
                }
            }
            ValueKind::Const(_) => Some(val_id),
            _ => None,
        }
    }

    fn iv_progression_is_integral(&self, fn_ir: &FnIR, lp: &LoopInfo) -> bool {
        let Some(iv) = lp.iv.as_ref() else {
            return false;
        };
        if !matches!(iv.step_op, BinOp::Add | BinOp::Sub) {
            return false;
        }
        Self::const_integral_value(fn_ir, iv.init_val).is_some()
    }

    fn floor_identity_replacement(
        &self,
        fn_ir: &FnIR,
        arg: ValueId,
        iv_phi: ValueId,
    ) -> Option<ValueKind> {
        match &fn_ir.values[arg].kind {
            ValueKind::Load { var } => Some(ValueKind::Load { var: var.clone() }),
            ValueKind::Param { index } => Some(ValueKind::Param { index: *index }),
            ValueKind::Const(lit) if Self::lit_is_integral(lit) => {
                Some(ValueKind::Const(lit.clone()))
            }
            ValueKind::Phi { .. } => fn_ir.values[iv_phi]
                .origin_var
                .clone()
                .map(|var| ValueKind::Load { var }),
            _ => None,
        }
    }

    fn collect_loop_reachable_values(&self, fn_ir: &FnIR, body_blocks: &[usize]) -> Vec<ValueId> {
        let mut seeds = Vec::new();
        for &bb in body_blocks {
            for instr in &fn_ir.blocks[bb].instrs {
                Self::push_instr_values(instr, &mut seeds);
            }
            Self::push_term_values(&fn_ir.blocks[bb].term, &mut seeds);
        }

        let mut seen = vec![false; fn_ir.values.len()];
        let mut stack = seeds;
        while let Some(vid) = stack.pop() {
            if vid >= fn_ir.values.len() || seen[vid] {
                continue;
            }
            seen[vid] = true;
            Self::push_value_operands(&fn_ir.values[vid].kind, &mut stack);
        }

        let mut loop_values = Vec::new();
        for (vid, in_loop) in seen.into_iter().enumerate() {
            if in_loop {
                loop_values.push(vid);
            }
        }
        loop_values
    }

    fn push_instr_values(instr: &Instr, out: &mut Vec<ValueId>) {
        match instr {
            Instr::Assign { src, .. } => out.push(*src),
            Instr::Eval { val, .. } => out.push(*val),
            Instr::StoreIndex1D { base, idx, val, .. } => {
                out.push(*base);
                out.push(*idx);
                out.push(*val);
            }
            Instr::StoreIndex2D {
                base, r, c, val, ..
            } => {
                out.push(*base);
                out.push(*r);
                out.push(*c);
                out.push(*val);
            }
            Instr::StoreIndex3D {
                base, i, j, k, val, ..
            } => {
                out.push(*base);
                out.push(*i);
                out.push(*j);
                out.push(*k);
                out.push(*val);
            }
            Instr::UnsafeRBlock { .. } => {}
        }
    }

    fn push_term_values(term: &Terminator, out: &mut Vec<ValueId>) {
        match term {
            Terminator::If { cond, .. } => out.push(*cond),
            Terminator::Return(Some(v)) => out.push(*v),
            _ => {}
        }
    }

    fn push_value_operands(kind: &ValueKind, out: &mut Vec<ValueId>) {
        match kind {
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
            ValueKind::Phi { args } => {
                for (v, _) in args {
                    out.push(*v);
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => out.push(*base),
            ValueKind::Range { start, end } => {
                out.push(*start);
                out.push(*end);
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                out.push(*lhs);
                out.push(*rhs);
            }
            ValueKind::Unary { rhs, .. } => out.push(*rhs),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    out.push(*arg);
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    out.push(*value);
                }
            }
            ValueKind::FieldGet { base, .. } => out.push(*base),
            ValueKind::FieldSet { base, value, .. } => {
                out.push(*base);
                out.push(*value);
            }
            ValueKind::Index1D { base, idx, .. } => {
                out.push(*base);
                out.push(*idx);
            }
            ValueKind::Index2D { base, r, c } => {
                out.push(*base);
                out.push(*r);
                out.push(*c);
            }
            ValueKind::Index3D { base, i, j, k } => {
                out.push(*base);
                out.push(*i);
                out.push(*j);
                out.push(*k);
            }
        }
    }

    fn is_iv_equivalent(fn_ir: &FnIR, candidate: ValueId, iv_phi: ValueId) -> bool {
        let mut seen = vec![false; fn_ir.values.len()];
        Self::is_iv_equivalent_rec(fn_ir, candidate, iv_phi, &mut seen)
    }

    fn is_iv_equivalent_rec(
        fn_ir: &FnIR,
        candidate: ValueId,
        iv_phi: ValueId,
        seen: &mut [bool],
    ) -> bool {
        if candidate >= fn_ir.values.len() {
            return false;
        }
        if candidate == iv_phi || Self::canonical_value(fn_ir, candidate) == iv_phi {
            return true;
        }
        if seen[candidate] {
            return false;
        }
        seen[candidate] = true;
        match &fn_ir.values[candidate].kind {
            ValueKind::Load { var } => {
                fn_ir.values[iv_phi].origin_var.as_deref() == Some(var.as_str())
            }
            ValueKind::Phi { args } if args.is_empty() => {
                match (
                    fn_ir.values[candidate].origin_var.as_deref(),
                    fn_ir.values[iv_phi].origin_var.as_deref(),
                ) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                }
            }
            ValueKind::Phi { args } => args
                .iter()
                .all(|(v, _)| Self::is_iv_equivalent_rec(fn_ir, *v, iv_phi, seen)),
            _ => false,
        }
    }

    fn mark_seq_along_index_safety(
        &self,
        fn_ir: &mut FnIR,
        lp: &LoopInfo,
        iv_phi: ValueId,
        body_blocks: &[usize],
    ) -> bool {
        let Some(loop_base) = lp.is_seq_along else {
            return false;
        };

        let mut changed = false;
        let mut store_updates = Vec::new();
        for &bb in body_blocks {
            for (ins_idx, instr) in fn_ir.blocks[bb].instrs.iter().enumerate() {
                let Instr::StoreIndex1D { base, idx, .. } = instr else {
                    continue;
                };
                if Self::is_base_equivalent(fn_ir, *base, loop_base)
                    && Self::is_iv_equivalent(fn_ir, *idx, iv_phi)
                {
                    store_updates.push((bb, ins_idx));
                }
            }
        }

        for (bb, ins_idx) in store_updates {
            if let Instr::StoreIndex1D {
                is_safe,
                is_na_safe,
                ..
            } = &mut fn_ir.blocks[bb].instrs[ins_idx]
            {
                if !*is_safe {
                    *is_safe = true;
                    changed = true;
                }
                if !*is_na_safe {
                    *is_na_safe = true;
                    changed = true;
                }
            }
        }

        for vid in 0..fn_ir.values.len() {
            let (base, idx, is_safe, is_na_safe) = match &fn_ir.values[vid].kind {
                ValueKind::Index1D {
                    base,
                    idx,
                    is_safe,
                    is_na_safe,
                } => (*base, *idx, *is_safe, *is_na_safe),
                _ => continue,
            };
            if !Self::is_base_equivalent(fn_ir, base, loop_base)
                || !Self::is_iv_equivalent(fn_ir, idx, iv_phi)
            {
                continue;
            }
            if let ValueKind::Index1D {
                is_safe: ref mut safe_mut,
                is_na_safe: ref mut na_mut,
                ..
            } = fn_ir.values[vid].kind
            {
                if !is_safe {
                    *safe_mut = true;
                    changed = true;
                }
                if !is_na_safe {
                    *na_mut = true;
                    changed = true;
                }
            }
        }

        changed
    }

    fn canonical_value(fn_ir: &FnIR, mut vid: ValueId) -> ValueId {
        let mut seen = vec![false; fn_ir.values.len()];
        while vid < fn_ir.values.len() && !seen[vid] {
            seen[vid] = true;
            match &fn_ir.values[vid].kind {
                ValueKind::Phi { args } if !args.is_empty() => {
                    let first = args[0].0;
                    if args.iter().all(|(v, _)| *v == first) {
                        vid = first;
                        continue;
                    }
                    let mut seed = None;
                    let mut mismatch = false;
                    for (v, _) in args {
                        if *v == vid {
                            continue;
                        }
                        match seed {
                            None => seed = Some(*v),
                            Some(prev) if prev == *v => {}
                            Some(_) => {
                                mismatch = true;
                                break;
                            }
                        }
                    }
                    if !mismatch && let Some(unique) = seed {
                        vid = unique;
                        continue;
                    }
                }
                _ => {}
            }
            break;
        }
        vid
    }

    fn is_base_equivalent(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
        if a == b || Self::canonical_value(fn_ir, a) == Self::canonical_value(fn_ir, b) {
            return true;
        }
        let a_ty = fn_ir.values[a].value_ty;
        let b_ty = fn_ir.values[b].value_ty;
        if a_ty.len_sym.is_some() && a_ty.len_sym == b_ty.len_sym {
            return true;
        }
        match (
            Self::value_base_name(fn_ir, a),
            Self::value_base_name(fn_ir, b),
        ) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    fn value_base_name(fn_ir: &FnIR, vid: ValueId) -> Option<&str> {
        if let Some(name) = fn_ir.values[vid].origin_var.as_deref() {
            return Some(name);
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Load { var } => Some(var.as_str()),
            _ => None,
        }
    }

    fn const_integral_value(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
        match &fn_ir.values[vid].kind {
            ValueKind::Const(Lit::Int(n)) => Some(*n),
            ValueKind::Const(Lit::Float(f))
                if f.is_finite()
                    && (*f - f.trunc()).abs() < f64::EPSILON
                    && *f >= i64::MIN as f64
                    && *f <= i64::MAX as f64 =>
            {
                Some(*f as i64)
            }
            _ => None,
        }
    }

    fn lit_is_integral(lit: &Lit) -> bool {
        match lit {
            Lit::Int(_) => true,
            Lit::Float(f) => {
                f.is_finite()
                    && (*f - f.trunc()).abs() < f64::EPSILON
                    && *f >= i64::MIN as f64
                    && *f <= i64::MAX as f64
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MirLoopOptimizer;
    use crate::mir::{Facts, FnIR, Instr, Lit, Terminator, ValueId, ValueKind};
    use crate::syntax::ast::BinOp;
    use crate::utils::Span;

    fn build_floor_loop(init: Lit) -> (FnIR, ValueId) {
        let mut fn_ir = FnIR::new("floor_loop".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let init_val = fn_ir.add_value(ValueKind::Const(init), Span::dummy(), Facts::empty(), None);
        let step = fn_ir.add_value(
            ValueKind::Const(Lit::Float(1.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let limit = fn_ir.add_value(
            ValueKind::Const(Lit::Float(4.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let phi_i = fn_ir.add_value(
            ValueKind::Phi { args: vec![] },
            Span::dummy(),
            Facts::empty(),
            Some("i".to_string()),
        );
        fn_ir.values[phi_i].phi_block = Some(header);
        let load_i = fn_ir.add_value(
            ValueKind::Load {
                var: "i".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let floor_i = fn_ir.add_value(
            ValueKind::Call {
                callee: "floor".to_string(),
                args: vec![load_i],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let next = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_i,
                rhs: step,
            },
            Span::dummy(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: phi_i,
                rhs: limit,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
            args.push((init_val, entry));
            args.push((next, body));
        }

        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "ii".to_string(),
            src: floor_i,
            span: Span::dummy(),
        });
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next,
            span: Span::dummy(),
        });
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(None);
        (fn_ir, floor_i)
    }

    #[test]
    fn loop_canonicalize_rewrites_floor_for_integral_float_iv() {
        let (mut fn_ir, floor_i) = build_floor_loop(Lit::Float(1.0));
        let changed = MirLoopOptimizer::new().optimize_with_count(&mut fn_ir);
        assert!(changed > 0, "expected canonicalization change");
        match &fn_ir.values[floor_i].kind {
            ValueKind::Load { var } => assert_eq!(var, "i"),
            other => panic!("expected floor to fold into load(i), got {:?}", other),
        }
    }

    #[test]
    fn loop_canonicalize_keeps_floor_for_non_integral_iv_seed() {
        let (mut fn_ir, floor_i) = build_floor_loop(Lit::Float(0.5));
        let changed = MirLoopOptimizer::new().optimize_with_count(&mut fn_ir);
        assert_eq!(changed, 0, "non-integral seed should not be canonicalized");
        match &fn_ir.values[floor_i].kind {
            ValueKind::Call { callee, .. } => assert_eq!(callee, "floor"),
            other => panic!("floor should remain call, got {:?}", other),
        }
    }
}
