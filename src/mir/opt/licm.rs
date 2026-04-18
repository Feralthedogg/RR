use crate::mir::analyze::{alias, effects};
use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::*;

pub struct MirLicm;

impl Default for MirLicm {
    fn default() -> Self {
        Self::new()
    }
}

impl MirLicm {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, fn_ir: &mut FnIR) -> bool {
        let analyzer = LoopAnalyzer::new(fn_ir);
        let loops = analyzer.find_loops();
        let mut changed = false;

        for loop_info in loops {
            changed |= self.hoist_invariants(fn_ir, &loop_info);
        }
        changed
    }

    fn hoist_invariants(
        &self,
        fn_ir: &mut FnIR,
        loop_info: &crate::mir::opt::loop_analysis::LoopInfo,
    ) -> bool {
        let mut changed = false;
        let loop_written_vars = self.collect_loop_written_vars(fn_ir, loop_info);

        // Build Value -> Block map
        let mut val_to_bb = std::collections::HashMap::new();
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                        val_to_bb.insert(*src, block.id);
                    }
                    Instr::StoreIndex1D { val: src, .. } => {
                        val_to_bb.insert(*src, block.id);
                    }
                    Instr::StoreIndex2D { val: src, .. } => {
                        val_to_bb.insert(*src, block.id);
                    }
                    Instr::StoreIndex3D { val: src, .. } => {
                        val_to_bb.insert(*src, block.id);
                    }
                }
            }
        }

        // Loop-local side effects / aliasing (conservative)
        let mut loop_mutated_aliases = std::collections::HashSet::new();
        let mut loop_has_unknown_mutation = false;
        for &bb in &loop_info.body {
            for instr in &fn_ir.blocks[bb].instrs {
                match instr {
                    Instr::StoreIndex1D { base, .. }
                    | Instr::StoreIndex2D { base, .. }
                    | Instr::StoreIndex3D { base, .. } => {
                        let cls = alias::alias_class_for_base(fn_ir, *base);
                        if matches!(cls, alias::AliasClass::Unknown) {
                            loop_has_unknown_mutation = true;
                        } else {
                            loop_mutated_aliases.insert(cls);
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut loop_has_impure_call = false;
        for (vid, bb) in &val_to_bb {
            if !loop_info.body.contains(bb) {
                continue;
            }
            if let ValueKind::Call { callee, .. } = &fn_ir.values[*vid].kind
                && !effects::call_is_pure(callee)
            {
                loop_has_impure_call = true;
                break;
            }
        }

        let loop_ctx = LoopEffectCtx {
            mutated_aliases: loop_mutated_aliases,
            has_unknown_mutation: loop_has_unknown_mutation,
            has_impure_call: loop_has_impure_call,
        };

        let invariants = self.compute_invariants(fn_ir, loop_info, &val_to_bb, &loop_ctx);
        let mut hoist_candidates = Vec::new();

        // Candidate Detection Loop
        // We look for usages in the loop body (Instrs).
        for &bb in &loop_info.body {
            for instr in &fn_ir.blocks[bb].instrs {
                match instr {
                    Instr::Assign { src, .. }
                    | Instr::Eval { val: src, .. }
                    | Instr::StoreIndex1D { val: src, .. }
                    | Instr::StoreIndex2D { val: src, .. }
                    | Instr::StoreIndex3D { val: src, .. } => {
                        // Already hoisted?
                        if let Some(name) = &fn_ir.values[*src].origin_var
                            && name.starts_with("licm_")
                        {
                            continue;
                        }

                        if self.is_hoistable(
                            fn_ir,
                            *src,
                            &invariants,
                            loop_info,
                            &val_to_bb,
                            &loop_ctx,
                            &loop_written_vars,
                        ) {
                            // Deduplicate
                            if !hoist_candidates.contains(src) {
                                hoist_candidates.push(*src);
                            }
                        }
                    }
                }
            }
        }

        if hoist_candidates.is_empty() {
            return false;
        }

        // 2. Find Pre-Header
        // Pre-header is a predecessor of Header that is NOT in Body.
        // Standard loops have well-defined pre-headers in our lowering (created before header).
        // Resolve it from the current CFG.
        let preds = crate::mir::opt::loop_analysis::build_pred_map(fn_ir);
        let header_preds = preds.get(&loop_info.header).cloned().unwrap_or_default();

        let pre_headers: Vec<BlockId> = header_preds
            .into_iter()
            .filter(|p| !loop_info.body.contains(p))
            .collect();

        if pre_headers.len() != 1 {
            // No unique pre-header (e.g. multiple entries or none). Abort.
            return false;
        }
        let pre_header = pre_headers[0];

        // 3. Hoist
        // We don't move the *definition* (Value stays in global vec).
        // We insert an `Eval` or `Assign` in the pre-header to force computation?
        // CodeGen emits values where they are used.
        // If we want to force computation ONCE, we must assign to a Variable.
        // `temp <- invariant_val`

        for val_id in hoist_candidates {
            // Create a temp variable
            let temp_name = format!("licm_{}", val_id);

            let val_id_to_hoist = self.clone_value(fn_ir, val_id);

            // Check if Pre-Header already has this assignment
            let exists = fn_ir.blocks[pre_header].instrs.iter().any(|instr| {
                if let Instr::Assign { dst, .. } = instr {
                    dst == &temp_name
                } else {
                    false
                }
            });

            if !exists {
                fn_ir.blocks[pre_header].instrs.push(Instr::Assign {
                    dst: temp_name.clone(),
                    src: val_id_to_hoist,
                    span: crate::utils::Span::default(),
                });
                changed = true;
            }

            // Bind only value kinds that are name-stable in codegen.
            if matches!(
                fn_ir.values[val_id].kind,
                ValueKind::Load { .. } | ValueKind::Param { .. } | ValueKind::Call { .. }
            ) {
                fn_ir.values[val_id].origin_var = Some(temp_name);
            }
        }

        changed
    }

    fn compute_invariants(
        &self,
        fn_ir: &FnIR,
        loop_info: &crate::mir::opt::loop_analysis::LoopInfo,
        val_to_bb: &std::collections::HashMap<ValueId, BlockId>,
        loop_ctx: &LoopEffectCtx,
    ) -> std::collections::HashSet<ValueId> {
        let mut invariants = std::collections::HashSet::new();

        // Initial setup: Constants/Params and values defined outside the loop are invariant
        for (id, val) in fn_ir.values.iter().enumerate() {
            let val_id = id;
            if matches!(val.kind, ValueKind::Const(_) | ValueKind::Param { .. }) {
                invariants.insert(val_id);
            } else if let Some(&bb) = val_to_bb.get(&val_id)
                && !loop_info.body.contains(&bb)
            {
                invariants.insert(val_id);
            }
        }

        // Fixed-point iteration
        let mut changed = true;
        while changed {
            changed = false;
            for (id, _val) in fn_ir.values.iter().enumerate() {
                let val_id = id;
                if invariants.contains(&val_id) {
                    continue;
                }

                let is_inv = self.is_value_invariant(fn_ir, val_id, &invariants, loop_ctx);

                if is_inv {
                    invariants.insert(val_id);
                    changed = true;
                }
            }
        }
        invariants
    }

    fn is_hoistable(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        invariants: &std::collections::HashSet<ValueId>,
        loop_info: &crate::mir::opt::loop_analysis::LoopInfo,
        val_to_bb: &std::collections::HashMap<ValueId, BlockId>,
        loop_ctx: &LoopEffectCtx,
        loop_written_vars: &std::collections::HashSet<String>,
    ) -> bool {
        // Can only hoist pure things defined INSIDE the loop.
        // If it's defined outside, it's already "hoisted" (invariant).

        if let Some(&bb) = val_to_bb.get(&val_id) {
            if !loop_info.body.contains(&bb) {
                return false; // Already outside
            }
        } else {
            return false; // Unknown definition site, safer not to touch
        }

        if !invariants.contains(&val_id) {
            return false;
        }
        if self.value_mentions_written_var(
            fn_ir,
            val_id,
            loop_written_vars,
            &mut std::collections::HashSet::new(),
        ) {
            return false;
        }
        if self.value_depends_on_loop_phi(
            fn_ir,
            val_id,
            loop_info,
            &mut std::collections::HashSet::new(),
        ) {
            return false;
        }

        // Must be speculatable to hoist
        self.is_value_invariant(fn_ir, val_id, invariants, loop_ctx)
            && !matches!(fn_ir.values[val_id].kind, ValueKind::Phi { .. })
    }

    fn collect_loop_written_vars(
        &self,
        fn_ir: &FnIR,
        loop_info: &crate::mir::opt::loop_analysis::LoopInfo,
    ) -> std::collections::HashSet<String> {
        let mut vars = std::collections::HashSet::new();
        for &bb in &loop_info.body {
            for instr in &fn_ir.blocks[bb].instrs {
                match instr {
                    Instr::Assign { dst, .. } => {
                        vars.insert(dst.clone());
                    }
                    Instr::StoreIndex1D { base, .. }
                    | Instr::StoreIndex2D { base, .. }
                    | Instr::StoreIndex3D { base, .. } => {
                        match &fn_ir.values[*base].kind {
                            ValueKind::Load { var } => {
                                vars.insert(var.clone());
                            }
                            _ => {
                                if let Some(origin_var) = &fn_ir.values[*base].origin_var {
                                    vars.insert(origin_var.clone());
                                }
                            }
                        }
                    }
                    Instr::Eval { .. } => {}
                }
            }
        }
        vars
    }

    fn value_mentions_written_var(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        loop_written_vars: &std::collections::HashSet<String>,
        seen: &mut std::collections::HashSet<ValueId>,
    ) -> bool {
        if !seen.insert(val_id) {
            return false;
        }
        match &fn_ir.values[val_id].kind {
            ValueKind::Load { var } => loop_written_vars.contains(var),
            ValueKind::Binary { lhs, rhs, .. } => {
                self.value_mentions_written_var(fn_ir, *lhs, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *rhs, loop_written_vars, seen)
            }
            ValueKind::Unary { rhs, .. }
            | ValueKind::Len { base: rhs }
            | ValueKind::Indices { base: rhs } => {
                self.value_mentions_written_var(fn_ir, *rhs, loop_written_vars, seen)
            }
            ValueKind::RecordLit { fields } => fields.iter().any(|(_, value)| {
                self.value_mentions_written_var(fn_ir, *value, loop_written_vars, seen)
            }),
            ValueKind::FieldGet { base, .. } => {
                self.value_mentions_written_var(fn_ir, *base, loop_written_vars, seen)
            }
            ValueKind::FieldSet { base, value, .. } => {
                self.value_mentions_written_var(fn_ir, *base, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *value, loop_written_vars, seen)
            }
            ValueKind::Range { start, end } => {
                self.value_mentions_written_var(fn_ir, *start, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *end, loop_written_vars, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                self.value_mentions_written_var(fn_ir, *base, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *idx, loop_written_vars, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                self.value_mentions_written_var(fn_ir, *base, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *r, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *c, loop_written_vars, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                self.value_mentions_written_var(fn_ir, *base, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *i, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *j, loop_written_vars, seen)
                    || self.value_mentions_written_var(fn_ir, *k, loop_written_vars, seen)
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| self.value_mentions_written_var(fn_ir, *arg, loop_written_vars, seen)),
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| {
                self.value_mentions_written_var(fn_ir, *arg, loop_written_vars, seen)
            }),
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    fn value_depends_on_loop_phi(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        loop_info: &crate::mir::opt::loop_analysis::LoopInfo,
        seen: &mut std::collections::HashSet<ValueId>,
    ) -> bool {
        if !seen.insert(val_id) {
            return false;
        }
        let value = &fn_ir.values[val_id];
        if matches!(value.kind, ValueKind::Phi { .. })
            && value
                .phi_block
                .is_some_and(|phi_bb| loop_info.body.contains(&phi_bb))
        {
            return true;
        }
        match &value.kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                self.value_depends_on_loop_phi(fn_ir, *lhs, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *rhs, loop_info, seen)
            }
            ValueKind::Unary { rhs, .. }
            | ValueKind::Len { base: rhs }
            | ValueKind::Indices { base: rhs } => {
                self.value_depends_on_loop_phi(fn_ir, *rhs, loop_info, seen)
            }
            ValueKind::RecordLit { fields } => fields.iter().any(|(_, value)| {
                self.value_depends_on_loop_phi(fn_ir, *value, loop_info, seen)
            }),
            ValueKind::FieldGet { base, .. } => {
                self.value_depends_on_loop_phi(fn_ir, *base, loop_info, seen)
            }
            ValueKind::FieldSet { base, value, .. } => {
                self.value_depends_on_loop_phi(fn_ir, *base, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *value, loop_info, seen)
            }
            ValueKind::Range { start, end } => {
                self.value_depends_on_loop_phi(fn_ir, *start, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *end, loop_info, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                self.value_depends_on_loop_phi(fn_ir, *base, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *idx, loop_info, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                self.value_depends_on_loop_phi(fn_ir, *base, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *r, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *c, loop_info, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                self.value_depends_on_loop_phi(fn_ir, *base, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *i, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *j, loop_info, seen)
                    || self.value_depends_on_loop_phi(fn_ir, *k, loop_info, seen)
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| self.value_depends_on_loop_phi(fn_ir, *arg, loop_info, seen)),
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| {
                self.value_depends_on_loop_phi(fn_ir, *arg, loop_info, seen)
            }),
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => false,
        }
    }

    fn is_value_invariant(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        invariants: &std::collections::HashSet<ValueId>,
        loop_ctx: &LoopEffectCtx,
    ) -> bool {
        let val = &fn_ir.values[val_id];
        match &val.kind {
            ValueKind::Const(_) | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                invariants.contains(lhs) && invariants.contains(rhs)
            }
            ValueKind::Unary { rhs, .. } => invariants.contains(rhs),
            ValueKind::RecordLit { fields } => {
                fields.iter().all(|(_, value)| invariants.contains(value))
            }
            ValueKind::FieldGet { base, .. } => {
                if loop_ctx.has_impure_call || loop_ctx.has_unknown_mutation {
                    return false;
                }
                let cls = alias::alias_class_for_base(fn_ir, *base);
                if matches!(cls, alias::AliasClass::Unknown) {
                    return false;
                }
                !loop_ctx.mutated_aliases.contains(&cls) && invariants.contains(base)
            }
            ValueKind::FieldSet { base, value, .. } => {
                if loop_ctx.has_impure_call || loop_ctx.has_unknown_mutation {
                    return false;
                }
                let cls = alias::alias_class_for_base(fn_ir, *base);
                if matches!(cls, alias::AliasClass::Unknown) {
                    return false;
                }
                !loop_ctx.mutated_aliases.contains(&cls)
                    && invariants.contains(base)
                    && invariants.contains(value)
            }
            ValueKind::Range { start, end } => {
                invariants.contains(start) && invariants.contains(end)
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                if loop_ctx.has_impure_call || loop_ctx.has_unknown_mutation {
                    return false;
                }
                let cls = alias::alias_class_for_base(fn_ir, *base);
                if matches!(cls, alias::AliasClass::Unknown) {
                    return false;
                }
                !loop_ctx.mutated_aliases.contains(&cls) && invariants.contains(base)
            }
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                if !*is_safe || !*is_na_safe {
                    return false;
                }
                if loop_ctx.has_impure_call || loop_ctx.has_unknown_mutation {
                    return false;
                }
                let cls = alias::alias_class_for_base(fn_ir, *base);
                if matches!(cls, alias::AliasClass::Unknown) {
                    return false;
                }
                if loop_ctx.mutated_aliases.contains(&cls) {
                    return false;
                }
                invariants.contains(base) && invariants.contains(idx)
            }
            ValueKind::Call { callee, args, .. } => {
                // Calls are treated as non-speculatable for LICM.
                // Even pure calls may throw/warn on some inputs, and hoisting them to
                // a pre-header can change behavior when loops execute zero iterations.
                let _ = callee;
                let _ = args;
                false
            }
            ValueKind::Phi { args } => {
                // In SSA, a Phi in a loop is invariant if all its non-self arguments are invariant
                args.iter()
                    .all(|(a, _)| *a == val_id || invariants.contains(a))
            }
            // Be conservative
            ValueKind::Load { .. }
            | ValueKind::RSymbol { .. }
            | ValueKind::Index2D { .. }
            | ValueKind::Index3D { .. }
            | ValueKind::Intrinsic { .. } => false,
        }
    }

    // Naive clone of value (shallow copy of kind, new ID)
    fn clone_value(&self, fn_ir: &mut FnIR, val_id: ValueId) -> ValueId {
        let v = &fn_ir.values[val_id];
        let new_kind = v.kind.clone();
        let span = v.span;
        fn_ir.add_value(new_kind, span, crate::mir::flow::Facts::empty(), None)
    }
}

#[derive(Debug)]
struct LoopEffectCtx {
    mutated_aliases: std::collections::HashSet<alias::AliasClass>,
    has_unknown_mutation: bool,
    has_impure_call: bool,
}
