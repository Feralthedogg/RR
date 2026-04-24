use crate::mir::flow::Facts;
use crate::mir::*;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

pub struct MirInliner {
    policy: InlinePolicy,
}
type InlineCall = (String, Vec<ValueId>, ValueId, Option<VarId>, Span);

#[derive(Default)]
struct InlineMap {
    v: FxHashMap<ValueId, ValueId>,
    b: FxHashMap<BlockId, BlockId>,
    vars: FxHashMap<VarId, VarId>,
    inline_tag: String,
}

impl InlineMap {
    fn map_var(&mut self, old: &VarId) -> VarId {
        if let Some(mapped) = self.vars.get(old) {
            return mapped.clone();
        }
        let new_name = format!("inlined_{}_{}", self.inline_tag, old);
        self.vars.insert(old.clone(), new_name.clone());
        new_name
    }
}

impl Default for MirInliner {
    fn default() -> Self {
        Self::new()
    }
}

impl MirInliner {
    pub fn new() -> Self {
        Self {
            policy: Self::standard_policy(),
        }
    }

    pub fn new_fast_dev() -> Self {
        Self {
            policy: Self::fast_dev_policy(),
        }
    }

    pub fn optimize(&self, all_fns: &mut FxHashMap<String, FnIR>) -> bool {
        self.optimize_with_hot_filter(all_fns, None)
    }

    pub fn optimize_with_hot_filter(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        hot_callers: Option<&FxHashSet<String>>,
    ) -> bool {
        if Self::inline_disabled() {
            return false;
        }
        let policy = self.policy;
        let mut growth = InlineGrowthBudget::new(all_fns, &policy);
        let mut global_changed = false;
        let mut fn_names: Vec<String> = all_fns.keys().cloned().collect();
        fn_names.sort();

        for name in fn_names {
            if let Some(hot_set) = hot_callers
                && !hot_set.contains(&name)
            {
                continue;
            }
            if let Some(mut fn_ir) = all_fns.remove(&name) {
                if fn_ir.unsupported_dynamic {
                    all_fns.insert(name, fn_ir);
                    continue;
                }
                let mut local_changed = true;
                let mut iterations = 0;
                let local_rounds = Self::env_usize("RR_INLINE_LOCAL_ROUNDS", 2);

                while local_changed && iterations < local_rounds {
                    local_changed =
                        self.inline_calls(&mut fn_ir, &name, all_fns, &policy, &mut growth);
                    if local_changed {
                        global_changed = true;
                    }
                    iterations += 1;
                }

                all_fns.insert(name, fn_ir);
            }
        }
        global_changed
    }

    fn inline_calls(
        &self,
        caller: &mut FnIR,
        caller_name: &str,
        all_fns: &FxHashMap<String, FnIR>,
        policy: &InlinePolicy,
        growth: &mut InlineGrowthBudget,
    ) -> bool {
        let mut changed = false;
        let caller_instr_cnt: usize = caller.blocks.iter().map(|b| b.instrs.len()).sum();
        if caller_instr_cnt > policy.max_caller_instrs {
            return false;
        }
        let caller_growth_limit = growth.caller_limit(caller_name);
        if Self::fn_ir_size(caller) > caller_growth_limit {
            return false;
        }

        if self.inline_value_calls(caller, all_fns, policy, growth, caller_growth_limit) {
            return true;
        }

        let mut candidate = None;

        'scan: for bid in 0..caller.blocks.len() {
            for (idx, instr) in caller.blocks[bid].instrs.iter().enumerate() {
                if let Some((callee_name, args, target_val, call_dst, call_span)) =
                    self.analyze_instr(caller, instr, all_fns)
                {
                    if callee_name == caller.name {
                        continue;
                    }

                    if let Some(callee) = all_fns.get(&callee_name) {
                        if callee.unsupported_dynamic {
                            continue;
                        }
                        if !self.should_inline(callee, caller, policy) {
                            continue;
                        }
                        let predicted_growth = Self::estimate_inline_growth(callee, policy);
                        if !growth.can_inline(
                            Self::fn_ir_size(caller),
                            caller_growth_limit,
                            predicted_growth,
                        ) {
                            continue;
                        }
                        if self.inline_callsite_cost(callee) > policy.max_callsite_cost {
                            continue;
                        }
                        candidate =
                            Some((bid, idx, callee_name, args, target_val, call_dst, call_span));
                        break 'scan;
                    }
                }
            }
        }

        if let Some((bid, idx, callee_name, args, target_val, call_dst, call_span)) = candidate
            && let Some(callee) = all_fns.get(&callee_name)
        {
            let before_size = Self::fn_ir_size(caller);
            self.perform_inline(
                caller, bid, idx, &args, target_val, call_dst, callee, call_span,
            );
            let after_size = Self::fn_ir_size(caller);
            growth.apply_resize(before_size, after_size);
            changed = true;
        }

        changed
    }

    fn analyze_instr(
        &self,
        caller: &FnIR,
        instr: &Instr,
        all_fns: &FxHashMap<String, FnIR>,
    ) -> Option<InlineCall> {
        match instr {
            Instr::Assign { dst, src, span, .. } => {
                if let ValueKind::Call { callee, args, .. } = &caller.values[*src].kind {
                    let base_name = self.resolve_callee_name(callee);

                    if all_fns.contains_key(base_name) {
                        return Some((
                            base_name.to_string(),
                            args.clone(),
                            *src,
                            Some(dst.clone()),
                            *span,
                        ));
                    }
                }
            }
            Instr::Eval { val: src, span, .. } => {
                if let ValueKind::Call { callee, args, .. } = &caller.values[*src].kind {
                    let base_name = self.resolve_callee_name(callee);
                    if all_fns.contains_key(base_name) {
                        return Some((base_name.to_string(), args.clone(), *src, None, *span));
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn should_inline(&self, target: &FnIR, caller: &FnIR, policy: &InlinePolicy) -> bool {
        if target.unsupported_dynamic {
            return false;
        }
        if target.name.starts_with("Sym_top_") {
            return false;
        }
        let caller_instr_cnt: usize = caller.blocks.iter().map(|b| b.instrs.len()).sum();
        if caller_instr_cnt > policy.max_caller_instrs {
            return false;
        }
        let block_cnt = target.blocks.len();
        let instr_cnt: usize = target.blocks.iter().map(|b| b.instrs.len()).sum();
        if block_cnt > policy.max_blocks || instr_cnt > policy.max_instrs {
            return false;
        }
        if caller_instr_cnt.saturating_add(instr_cnt) > policy.max_total_instrs {
            return false;
        }
        if target
            .values
            .iter()
            .any(|value| matches!(value.kind, ValueKind::Call { .. }))
        {
            return false;
        }

        let mut loop_edges = 0usize;
        for (bid, bb) in target.blocks.iter().enumerate() {
            match bb.term {
                Terminator::Goto(t) => {
                    if t <= bid {
                        loop_edges += 1;
                    }
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    if then_bb <= bid {
                        loop_edges += 1;
                    }
                    if else_bb <= bid {
                        loop_edges += 1;
                    }
                }
                _ => {}
            }
        }
        if !policy.allow_loops && loop_edges > 0 {
            return false;
        }

        let cost = self.inline_callsite_cost(target);
        cost <= policy.max_cost && cost <= policy.max_callsite_cost
    }

    fn inline_disabled() -> bool {
        false
    }

    fn env_usize(key: &str, default_v: usize) -> usize {
        let _ = key;
        default_v
    }

    fn env_bool(key: &str, default_v: bool) -> bool {
        let _ = key;
        default_v
    }

    fn standard_policy() -> InlinePolicy {
        InlinePolicy {
            max_blocks: Self::env_usize("RR_INLINE_MAX_BLOCKS", 24),
            max_instrs: Self::env_usize("RR_INLINE_MAX_INSTRS", 160),
            max_cost: Self::env_usize("RR_INLINE_MAX_COST", 220),
            max_callsite_cost: Self::env_usize("RR_INLINE_MAX_CALLSITE_COST", 240),
            max_caller_instrs: Self::env_usize("RR_INLINE_MAX_CALLER_INSTRS", 480),
            max_total_instrs: Self::env_usize("RR_INLINE_MAX_TOTAL_INSTRS", 900),
            max_unit_growth_pct: Self::env_usize("RR_INLINE_MAX_UNIT_GROWTH_PCT", 25),
            max_fn_growth_pct: Self::env_usize("RR_INLINE_MAX_FN_GROWTH_PCT", 35),
            min_growth_abs: 0,
            allow_loops: Self::env_bool("RR_INLINE_ALLOW_LOOPS", false),
        }
    }

    fn fast_dev_policy() -> InlinePolicy {
        InlinePolicy {
            max_blocks: 8,
            max_instrs: 48,
            max_cost: 72,
            max_callsite_cost: 80,
            max_caller_instrs: 192,
            max_total_instrs: 320,
            max_unit_growth_pct: 8,
            max_fn_growth_pct: 12,
            min_growth_abs: 24,
            allow_loops: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn perform_inline(
        &self,
        caller: &mut FnIR,
        call_block: BlockId,
        instr_idx: usize,
        call_args: &[ValueId],
        call_val_target: ValueId,
        call_dst: Option<VarId>,
        callee: &FnIR,
        call_span: Span,
    ) {
        let mut map = InlineMap::default();
        let mut mutated_param_inits: FxHashMap<VarId, ValueId> = FxHashMap::default();

        let mut param_val_ids: FxHashMap<usize, ValueId> = FxHashMap::default();
        for (vid, val) in callee.values.iter().enumerate() {
            if let ValueKind::Param { index } = val.kind {
                param_val_ids.insert(index, vid);
            }
        }
        let mut mutated_params: FxHashSet<usize> = FxHashSet::default();
        for blk in &callee.blocks {
            for instr in &blk.instrs {
                if let Instr::Assign { dst, src, .. } = instr
                    && let Some(idx) = callee.params.iter().position(|p| p == dst)
                    && let Some(&param_vid) = param_val_ids.get(&idx)
                    && *src != param_vid
                {
                    mutated_params.insert(idx);
                }
            }
        }

        for (cbid, _) in callee.blocks.iter().enumerate() {
            let new_bid = caller.add_block();
            map.b.insert(cbid, new_bid);
        }
        if let Some(&entry_bid) = map.b.get(&callee.entry) {
            map.inline_tag = entry_bid.to_string();
        }

        for (cvid, val) in callee.values.iter().enumerate() {
            if let ValueKind::Param { index } = val.kind {
                if index < call_args.len() {
                    if mutated_params.contains(&index) {
                        let param_name = callee.params[index].clone();
                        let mapped_var = map.map_var(&param_name);
                        let load_id = caller.add_value(
                            ValueKind::Load {
                                var: mapped_var.clone(),
                            },
                            call_span,
                            Facts::empty(),
                            None,
                        );
                        map.v.insert(cvid, load_id);
                        mutated_param_inits.insert(mapped_var, call_args[index]);
                    } else {
                        map.v.insert(cvid, call_args[index]);
                    }
                } else {
                    let dummy = caller.add_value(
                        ValueKind::Const(crate::syntax::ast::Lit::Null),
                        call_span,
                        Facts::empty(),
                        None,
                    );
                    map.v.insert(cvid, dummy);
                }
                continue;
            }

            let new_vid = caller.add_value(
                ValueKind::Const(crate::syntax::ast::Lit::Null),
                val.span,
                val.facts,
                None,
            );

            if let Some(name) = &val.origin_var {
                let new_name = map.map_var(name);
                caller.values[new_vid].origin_var = Some(new_name);
            }
            if let Some(old_bb) = val.phi_block
                && let Some(&new_bb) = map.b.get(&old_bb)
            {
                caller.values[new_vid].phi_block = Some(new_bb);
            }

            map.v.insert(cvid, new_vid);
        }

        for (cvid, val) in callee.values.iter().enumerate() {
            if let ValueKind::Param { .. } = val.kind {
                continue;
            }

            let new_vid = map.v[&cvid];
            let mut new_kind = val.kind.clone();
            self.remap_value_kind(&mut new_kind, &mut map);

            caller.values[new_vid].kind = new_kind;
        }

        for (cbid, cblk) in callee.blocks.iter().enumerate() {
            let nbid = map.b[&cbid];

            let mut new_instrs = Vec::new();
            for instr in &cblk.instrs {
                let mut new_instr = instr.clone();
                self.remap_instr(&mut new_instr, &mut map);
                new_instrs.push(new_instr);
            }

            let mut new_term = cblk.term.clone();
            self.remap_term(&mut new_term, &map);

            caller.blocks[nbid].instrs = new_instrs;
            caller.blocks[nbid].term = new_term;
        }

        let continuation_bb = caller.add_block();
        let post_split: Vec<Instr> = caller.blocks[call_block]
            .instrs
            .drain((instr_idx + 1)..)
            .collect();
        caller.blocks[continuation_bb].instrs = post_split;
        caller.blocks[continuation_bb].term = caller.blocks[call_block].term.clone();
        let old_term = caller.blocks[continuation_bb].term.clone();

        caller.blocks[call_block].instrs.truncate(instr_idx);

        let callee_entry = map.b[&callee.entry];
        caller.blocks[call_block].term = Terminator::Goto(callee_entry);

        let old_succs = term_successors(&old_term);
        if !old_succs.is_empty() {
            for val in &mut caller.values {
                if let ValueKind::Phi { args } = &mut val.kind
                    && let Some(phi_bb) = val.phi_block
                    && old_succs.contains(&phi_bb)
                {
                    for (_, pred_bb) in args.iter_mut() {
                        if *pred_bb == call_block {
                            *pred_bb = continuation_bb;
                        }
                    }
                }
            }
        }

        let mut returns = Vec::new();

        let inlined_blocks: Vec<BlockId> = map.b.values().cloned().collect();

        for &nbid in &inlined_blocks {
            if let Terminator::Return(ret_opt) = &caller.blocks[nbid].term {
                if let Some(ret_val) = ret_opt {
                    returns.push((*ret_val, nbid));
                } else {
                    let null_val = caller.add_value(
                        ValueKind::Const(crate::syntax::ast::Lit::Null),
                        call_span,
                        Facts::empty(),
                        None,
                    );
                    returns.push((null_val, nbid));
                }
                caller.blocks[nbid].term = Terminator::Goto(continuation_bb);
            }
        }

        let res_id: ValueId = if returns.is_empty() {
            caller.values[call_val_target].kind = ValueKind::Const(crate::syntax::ast::Lit::Null);
            call_val_target
        } else if returns.len() == 1 {
            let (single_ret, _) = returns[0];
            self.replace_uses(caller, call_val_target, single_ret);
            single_ret
        } else {
            caller.blocks[continuation_bb].instrs.insert(
                0,
                Instr::Eval {
                    val: call_val_target,
                    span: call_span,
                },
            );
            let phi_args = returns;
            caller.values[call_val_target].kind = ValueKind::Phi { args: phi_args };
            caller.values[call_val_target].phi_block = Some(continuation_bb);
            call_val_target
        };

        if let Some(dst) = call_dst {
            caller.blocks[continuation_bb].instrs.insert(
                0,
                Instr::Assign {
                    dst,
                    src: res_id,
                    span: call_span,
                },
            );
        }

        if !mutated_param_inits.is_empty()
            && let Some(&entry_bid) = map.b.get(&callee.entry)
        {
            for instr in &mut caller.blocks[entry_bid].instrs {
                if let Instr::Assign { dst, src, .. } = instr
                    && let Some(&arg_val) = mutated_param_inits.get(dst)
                {
                    *src = arg_val;
                }
            }
        }
    }

    fn resolve_callee_name<'a>(&self, callee: &'a str) -> &'a str {
        callee.strip_suffix("_fresh").unwrap_or(callee)
    }

    fn fn_ir_size(fn_ir: &FnIR) -> usize {
        let instrs: usize = fn_ir.blocks.iter().map(|b| b.instrs.len()).sum();
        fn_ir.values.len().saturating_add(instrs)
    }

    fn inline_callsite_cost(&self, target: &FnIR) -> usize {
        let block_cnt = target.blocks.len();
        let instr_cnt: usize = target.blocks.iter().map(|b| b.instrs.len()).sum();
        let mut loop_edges = 0usize;
        let mut call_count = 0usize;
        for (bid, bb) in target.blocks.iter().enumerate() {
            for ins in &bb.instrs {
                if let Instr::Assign { src, .. } | Instr::Eval { val: src, .. } = ins
                    && matches!(target.values[*src].kind, ValueKind::Call { .. })
                {
                    call_count += 1;
                }
            }
            match bb.term {
                Terminator::Goto(t) => {
                    if t <= bid {
                        loop_edges += 1;
                    }
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    if then_bb <= bid {
                        loop_edges += 1;
                    }
                    if else_bb <= bid {
                        loop_edges += 1;
                    }
                }
                _ => {}
            }
        }
        instr_cnt
            .saturating_add(block_cnt.saturating_mul(2))
            .saturating_add(loop_edges.saturating_mul(40))
            .saturating_add(call_count.saturating_mul(8))
    }

    fn estimate_inline_growth(callee: &FnIR, policy: &InlinePolicy) -> usize {
        if policy.min_growth_abs == 0 {
            // Keep the standard profile on the historical conservative estimate.
            let callee_ir = Self::fn_ir_size(callee);
            return callee_ir
                .saturating_add(callee.values.len())
                .saturating_add(callee.blocks.len().saturating_mul(4))
                .saturating_add(16);
        }

        // Fast-dev uses a tighter upper-bound for tiny helpers so growth
        // budgeting does not completely starve otherwise-safe local inlining.
        Self::fn_ir_size(callee)
            .saturating_add(callee.blocks.len())
            .saturating_add(4)
    }

    fn inline_value_calls(
        &self,
        caller: &mut FnIR,
        all_fns: &FxHashMap<String, FnIR>,
        policy: &InlinePolicy,
        growth: &mut InlineGrowthBudget,
        caller_growth_limit: usize,
    ) -> bool {
        for val_id in 0..caller.values.len() {
            let (callee_name, args) = match &caller.values[val_id].kind {
                ValueKind::Call { callee, args, .. } => {
                    (self.resolve_callee_name(callee).to_string(), args.clone())
                }
                _ => continue,
            };

            let callee = match all_fns.get(&callee_name) {
                Some(f) => f,
                None => continue,
            };

            if !self.should_inline(callee, caller, policy) {
                continue;
            }
            if self.inline_callsite_cost(callee) > policy.max_callsite_cost {
                continue;
            }
            let predicted_growth = Self::estimate_inline_growth(callee, policy);
            if !growth.can_inline(
                Self::fn_ir_size(caller),
                caller_growth_limit,
                predicted_growth,
            ) {
                continue;
            }

            let ret_val = match self.can_inline_expr(callee) {
                Some(v) => v,
                None => continue,
            };

            let before_size = Self::fn_ir_size(caller);
            if let Some(replacement) =
                self.inline_call_value(caller, val_id, callee, ret_val, &args)
            {
                self.replace_uses(caller, val_id, replacement);
                caller.values[val_id].kind = ValueKind::Const(crate::syntax::ast::Lit::Null);
                let after_size = Self::fn_ir_size(caller);
                growth.apply_resize(before_size, after_size);
                return true;
            }
        }
        false
    }

    fn can_inline_expr(&self, callee: &FnIR) -> Option<ValueId> {
        let reachable = self.reachable_blocks(callee);
        if reachable.is_empty() || reachable.len() > 2 {
            return None;
        }

        let mut ret: Option<ValueId> = None;
        for bid in &reachable {
            let blk = &callee.blocks[*bid];
            if matches!(blk.term, Terminator::If { .. }) {
                return None;
            }
            for instr in &blk.instrs {
                match instr {
                    Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::Eval { .. } => return None,
                    _ => {}
                }
            }
            match blk.term {
                Terminator::Return(Some(v)) => {
                    if ret.is_some() && ret != Some(v) {
                        return None;
                    }
                    ret = Some(v);
                }
                Terminator::Return(None) => return None,
                _ => {}
            }
        }
        ret
    }

    fn reachable_blocks(&self, callee: &FnIR) -> FxHashSet<BlockId> {
        let mut reachable = FxHashSet::default();
        let mut queue = VecDeque::new();
        queue.push_back(callee.entry);
        reachable.insert(callee.entry);
        while let Some(bid) = queue.pop_front() {
            let blk = &callee.blocks[bid];
            match blk.term {
                Terminator::Goto(t) => {
                    if reachable.insert(t) {
                        queue.push_back(t);
                    }
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    if reachable.insert(then_bb) {
                        queue.push_back(then_bb);
                    }
                    if reachable.insert(else_bb) {
                        queue.push_back(else_bb);
                    }
                }
                _ => {}
            }
        }
        reachable
    }

    fn inline_call_value(
        &self,
        caller: &mut FnIR,
        call_val_id: ValueId,
        callee: &FnIR,
        ret_val: ValueId,
        args: &[ValueId],
    ) -> Option<ValueId> {
        let mut map: FxHashMap<ValueId, ValueId> = FxHashMap::default();

        let clone_value = |vid: ValueId,
                           caller: &mut FnIR,
                           map: &mut FxHashMap<ValueId, ValueId>,
                           args: &[ValueId]|
         -> Option<ValueId> {
            fn clone_rec(
                vid: ValueId,
                caller: &mut FnIR,
                callee: &FnIR,
                map: &mut FxHashMap<ValueId, ValueId>,
                args: &[ValueId],
            ) -> Option<ValueId> {
                if let Some(&mapped) = map.get(&vid) {
                    return Some(mapped);
                }
                let val = &callee.values[vid];
                match &val.kind {
                    ValueKind::Param { index } => {
                        if *index < args.len() {
                            let mapped = args[*index];
                            map.insert(vid, mapped);
                            return Some(mapped);
                        }
                        None
                    }
                    ValueKind::Const(lit) => {
                        let new_id = caller.add_value(
                            ValueKind::Const(lit.clone()),
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Binary { op, lhs, rhs } => {
                        let l = clone_rec(*lhs, caller, callee, map, args)?;
                        let r = clone_rec(*rhs, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Binary {
                                op: *op,
                                lhs: l,
                                rhs: r,
                            },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Unary { op, rhs } => {
                        let r = clone_rec(*rhs, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Unary { op: *op, rhs: r },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Len { base } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let new_id =
                            caller.add_value(ValueKind::Len { base: b }, val.span, val.facts, None);
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Indices { base } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Indices { base: b },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Range { start, end } => {
                        let s = clone_rec(*start, caller, callee, map, args)?;
                        let e = clone_rec(*end, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Range { start: s, end: e },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Index1D {
                        base,
                        idx,
                        is_safe,
                        is_na_safe,
                    } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let i = clone_rec(*idx, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Index1D {
                                base: b,
                                idx: i,
                                is_safe: *is_safe,
                                is_na_safe: *is_na_safe,
                            },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Index2D { base, r, c } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let rv = clone_rec(*r, caller, callee, map, args)?;
                        let cv = clone_rec(*c, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Index2D {
                                base: b,
                                r: rv,
                                c: cv,
                            },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Index3D { base, i, j, k } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let iv = clone_rec(*i, caller, callee, map, args)?;
                        let jv = clone_rec(*j, caller, callee, map, args)?;
                        let kv = clone_rec(*k, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Index3D {
                                base: b,
                                i: iv,
                                j: jv,
                                k: kv,
                            },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::RecordLit { fields } => {
                        let mut new_fields = Vec::with_capacity(fields.len());
                        for (name, field_val) in fields {
                            let mapped = clone_rec(*field_val, caller, callee, map, args)?;
                            new_fields.push((name.clone(), mapped));
                        }
                        let new_id = caller.add_value(
                            ValueKind::RecordLit { fields: new_fields },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::FieldGet { base, field } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::FieldGet {
                                base: b,
                                field: field.clone(),
                            },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::FieldSet { base, field, value } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let v = clone_rec(*value, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::FieldSet {
                                base: b,
                                field: field.clone(),
                                value: v,
                            },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::Intrinsic {
                        op,
                        args: intrinsic_args,
                    } => {
                        let mut new_args = Vec::with_capacity(intrinsic_args.len());
                        for arg in intrinsic_args {
                            new_args.push(clone_rec(*arg, caller, callee, map, args)?);
                        }
                        let new_id = caller.add_value(
                            ValueKind::Intrinsic {
                                op: *op,
                                args: new_args,
                            },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    ValueKind::RSymbol { name } => {
                        let new_id = caller.add_value(
                            ValueKind::RSymbol { name: name.clone() },
                            val.span,
                            val.facts,
                            None,
                        );
                        map.insert(vid, new_id);
                        Some(new_id)
                    }
                    _ => None,
                }
            }
            clone_rec(vid, caller, callee, map, args)
        };

        let replacement = clone_value(ret_val, caller, &mut map, args)?;
        if replacement == call_val_id {
            return Some(replacement);
        }
        Some(replacement)
    }

    fn remap_value_kind(&self, kind: &mut ValueKind, map: &mut InlineMap) {
        match kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                if let Some(&n) = map.v.get(lhs) {
                    *lhs = n;
                }
                if let Some(&n) = map.v.get(rhs) {
                    *rhs = n;
                }
            }
            ValueKind::Unary { rhs, .. } => {
                if let Some(&n) = map.v.get(rhs) {
                    *rhs = n;
                }
            }
            ValueKind::Call { args, .. } => {
                for a in args {
                    if let Some(&n) = map.v.get(a) {
                        *a = n;
                    }
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for a in args {
                    if let Some(&n) = map.v.get(a) {
                        *a = n;
                    }
                }
            }
            ValueKind::Phi { args } => {
                for (v, b) in args {
                    if let Some(&n) = map.v.get(v) {
                        *v = n;
                    }
                    if let Some(&n) = map.b.get(b) {
                        *b = n;
                    }
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(idx) {
                    *idx = n;
                }
            }
            ValueKind::Index2D { base, r, c } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(r) {
                    *r = n;
                }
                if let Some(&n) = map.v.get(c) {
                    *c = n;
                }
            }
            ValueKind::Index3D { base, i, j, k } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(i) {
                    *i = n;
                }
                if let Some(&n) = map.v.get(j) {
                    *j = n;
                }
                if let Some(&n) = map.v.get(k) {
                    *k = n;
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
            }
            ValueKind::Range { start, end } => {
                if let Some(&n) = map.v.get(start) {
                    *start = n;
                }
                if let Some(&n) = map.v.get(end) {
                    *end = n;
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    if let Some(&n) = map.v.get(value) {
                        *value = n;
                    }
                }
            }
            ValueKind::FieldGet { base, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
            }
            ValueKind::FieldSet { base, value, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(value) {
                    *value = n;
                }
            }
            ValueKind::Load { var } => {
                let mapped = map.map_var(var);
                *var = mapped;
            }
            _ => {}
        }
    }

    fn remap_instr(&self, instr: &mut Instr, map: &mut InlineMap) {
        match instr {
            Instr::Assign { dst, src, .. } => {
                if let Some(&n) = map.v.get(src) {
                    *src = n;
                }
                let mapped = map.map_var(dst);
                *dst = mapped;
            }
            Instr::Eval { val, .. } => {
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
            Instr::StoreIndex1D { base, idx, val, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(idx) {
                    *idx = n;
                }
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
            Instr::StoreIndex2D {
                base, r, c, val, ..
            } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(r) {
                    *r = n;
                }
                if let Some(&n) = map.v.get(c) {
                    *c = n;
                }
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
            Instr::StoreIndex3D {
                base, i, j, k, val, ..
            } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(i) {
                    *i = n;
                }
                if let Some(&n) = map.v.get(j) {
                    *j = n;
                }
                if let Some(&n) = map.v.get(k) {
                    *k = n;
                }
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
        }
    }

    fn remap_term(&self, term: &mut Terminator, map: &InlineMap) {
        match term {
            Terminator::Goto(b) => {
                if let Some(&n) = map.b.get(b) {
                    *b = n;
                }
            }
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                if let Some(&n) = map.v.get(cond) {
                    *cond = n;
                }
                if let Some(&n) = map.b.get(then_bb) {
                    *then_bb = n;
                }
                if let Some(&n) = map.b.get(else_bb) {
                    *else_bb = n;
                }
            }
            Terminator::Return(Some(v)) => {
                if let Some(&n) = map.v.get(v) {
                    *v = n;
                }
            }
            _ => {}
        }
    }

    fn replace_uses(&self, fn_ir: &mut FnIR, old: ValueId, new: ValueId) {
        for val in &mut fn_ir.values {
            match &mut val.kind {
                ValueKind::Binary { lhs, rhs, .. } => {
                    if *lhs == old {
                        *lhs = new;
                    }
                    if *rhs == old {
                        *rhs = new;
                    }
                }
                ValueKind::Unary { rhs, .. } => {
                    if *rhs == old {
                        *rhs = new;
                    }
                }
                ValueKind::Call { args, .. } => {
                    for a in args {
                        if *a == old {
                            *a = new;
                        }
                    }
                }
                ValueKind::Intrinsic { args, .. } => {
                    for a in args {
                        if *a == old {
                            *a = new;
                        }
                    }
                }
                ValueKind::Phi { args } => {
                    for (v, _) in args {
                        if *v == old {
                            *v = new;
                        }
                    }
                }
                ValueKind::Index1D { base, idx, .. } => {
                    if *base == old {
                        *base = new;
                    }
                    if *idx == old {
                        *idx = new;
                    }
                }
                ValueKind::Index2D { base, r, c } => {
                    if *base == old {
                        *base = new;
                    }
                    if *r == old {
                        *r = new;
                    }
                    if *c == old {
                        *c = new;
                    }
                }
                ValueKind::Index3D { base, i, j, k } => {
                    if *base == old {
                        *base = new;
                    }
                    if *i == old {
                        *i = new;
                    }
                    if *j == old {
                        *j = new;
                    }
                    if *k == old {
                        *k = new;
                    }
                }
                ValueKind::Len { base } | ValueKind::Indices { base } => {
                    if *base == old {
                        *base = new;
                    }
                }
                ValueKind::Range { start, end } => {
                    if *start == old {
                        *start = new;
                    }
                    if *end == old {
                        *end = new;
                    }
                }
                ValueKind::RecordLit { fields } => {
                    for (_, value) in fields {
                        if *value == old {
                            *value = new;
                        }
                    }
                }
                ValueKind::FieldGet { base, .. } => {
                    if *base == old {
                        *base = new;
                    }
                }
                ValueKind::FieldSet { base, value, .. } => {
                    if *base == old {
                        *base = new;
                    }
                    if *value == old {
                        *value = new;
                    }
                }
                _ => {}
            }
        }

        for blk in &mut fn_ir.blocks {
            for instr in &mut blk.instrs {
                match instr {
                    Instr::Assign { src, .. } => {
                        if *src == old {
                            *src = new;
                        }
                    }
                    Instr::Eval { val, .. } => {
                        if *val == old {
                            *val = new;
                        }
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        if *base == old {
                            *base = new;
                        }
                        if *idx == old {
                            *idx = new;
                        }
                        if *val == old {
                            *val = new;
                        }
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        if *base == old {
                            *base = new;
                        }
                        if *r == old {
                            *r = new;
                        }
                        if *c == old {
                            *c = new;
                        }
                        if *val == old {
                            *val = new;
                        }
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        if *base == old {
                            *base = new;
                        }
                        if *i == old {
                            *i = new;
                        }
                        if *j == old {
                            *j = new;
                        }
                        if *k == old {
                            *k = new;
                        }
                        if *val == old {
                            *val = new;
                        }
                    }
                }
            }
            match &mut blk.term {
                Terminator::If { cond, .. } => {
                    if *cond == old {
                        *cond = new;
                    }
                }
                Terminator::Return(Some(v)) => {
                    if *v == old {
                        *v = new;
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Copy)]
struct InlinePolicy {
    max_blocks: usize,
    max_instrs: usize,
    max_cost: usize,
    max_callsite_cost: usize,
    max_caller_instrs: usize,
    max_total_instrs: usize,
    max_unit_growth_pct: usize,
    max_fn_growth_pct: usize,
    min_growth_abs: usize,
    allow_loops: bool,
}

struct InlineGrowthBudget {
    total_ir: usize,
    max_total_ir: usize,
    fn_limits: FxHashMap<String, usize>,
}

impl InlineGrowthBudget {
    fn growth_cap(base: usize, pct: usize, min_bonus: usize) -> usize {
        if pct == 0 {
            return base;
        }
        let capped_pct = pct.min(1000);
        let bonus = base
            .saturating_mul(capped_pct)
            .saturating_add(99)
            .saturating_div(100);
        base.saturating_add(bonus.max(min_bonus)).max(base)
    }

    fn new(all_fns: &FxHashMap<String, FnIR>, policy: &InlinePolicy) -> Self {
        let mut fn_limits = FxHashMap::default();
        let mut total_ir = 0usize;
        for (name, fn_ir) in all_fns {
            let ir = MirInliner::fn_ir_size(fn_ir);
            total_ir = total_ir.saturating_add(ir);
            let limit = Self::growth_cap(ir, policy.max_fn_growth_pct, policy.min_growth_abs);
            fn_limits.insert(name.clone(), limit);
        }
        let max_total_ir =
            Self::growth_cap(total_ir, policy.max_unit_growth_pct, policy.min_growth_abs);
        Self {
            total_ir,
            max_total_ir,
            fn_limits,
        }
    }

    fn caller_limit(&self, caller: &str) -> usize {
        self.fn_limits
            .get(caller)
            .copied()
            .unwrap_or(usize::MAX.saturating_div(2))
    }

    fn can_inline(&self, caller_ir: usize, caller_limit: usize, predicted_growth: usize) -> bool {
        let next_caller = caller_ir.saturating_add(predicted_growth);
        if next_caller > caller_limit {
            return false;
        }
        let next_total = self.total_ir.saturating_add(predicted_growth);
        next_total <= self.max_total_ir
    }

    fn apply_resize(&mut self, before: usize, after: usize) {
        if after >= before {
            self.total_ir = self.total_ir.saturating_add(after - before);
        } else {
            self.total_ir = self.total_ir.saturating_sub(before - after);
        }
    }
}

fn term_successors(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Goto(b) => vec![*b],
        Terminator::If {
            then_bb, else_bb, ..
        } => vec![*then_bb, *else_bb],
        _ => vec![],
    }
}

fn new_bid_offset(_fn_ir: &FnIR, bid: BlockId) -> String {
    format!("{}", bid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::flow::Facts;
    use crate::syntax::ast::Lit;
    use crate::utils::Span;

    fn tiny_fn(name: &str) -> FnIR {
        let mut f = FnIR::new(name.to_string(), vec![]);
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;
        let c = f.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        f.blocks[entry].term = Terminator::Return(Some(c));
        f
    }

    #[test]
    fn inline_growth_cap_is_saturating() {
        let cap = InlineGrowthBudget::growth_cap(100, 25, 0);
        assert!(cap >= 125);
        let huge = InlineGrowthBudget::growth_cap(usize::MAX - 16, 1000, 0);
        assert!(huge >= usize::MAX - 16);
    }

    #[test]
    fn inline_growth_budget_blocks_when_no_growth_allowed() {
        let mut all = FxHashMap::default();
        all.insert("caller".to_string(), tiny_fn("caller"));
        let policy = InlinePolicy {
            max_blocks: 24,
            max_instrs: 160,
            max_cost: 220,
            max_callsite_cost: 240,
            max_caller_instrs: 480,
            max_total_instrs: 900,
            max_unit_growth_pct: 0,
            max_fn_growth_pct: 0,
            min_growth_abs: 0,
            allow_loops: false,
        };
        let budget = InlineGrowthBudget::new(&all, &policy);
        let caller = all.get("caller").unwrap();
        let caller_ir = MirInliner::fn_ir_size(caller);
        let caller_limit = budget.caller_limit("caller");
        assert!(!budget.can_inline(caller_ir, caller_limit, 1));
    }

    #[test]
    fn inline_value_calls_rejects_store_index3d_side_effect_helpers() {
        let mut callee = FnIR::new("helper3d".to_string(), vec!["arr".to_string()]);
        let centry = callee.add_block();
        callee.entry = centry;
        callee.body_head = centry;
        let arr = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let one = callee.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = callee.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        callee.blocks[centry].instrs.push(Instr::StoreIndex3D {
            base: arr,
            i: one,
            j: one,
            k: one,
            val: zero,
            span: Span::default(),
        });
        callee.blocks[centry].term = Terminator::Return(Some(zero));

        let mut caller = FnIR::new("caller".to_string(), vec!["arr".to_string()]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let carg = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "helper3d".to_string(),
                args: vec![carg],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(call));

        let mut all = FxHashMap::default();
        all.insert("helper3d".to_string(), callee);
        all.insert("caller".to_string(), caller);

        let changed = MirInliner::new().optimize(&mut all);
        assert!(
            !changed,
            "StoreIndex3D helpers must not inline as pure expressions"
        );
        let caller = all.get("caller").expect("caller should remain present");
        assert!(matches!(
            caller.blocks[entry].term,
            Terminator::Return(Some(v)) if matches!(caller.values[v].kind, ValueKind::Call { .. })
        ));
    }

    #[test]
    fn inline_rejects_helpers_with_nested_calls() {
        let mut callee = FnIR::new("call_wrapper".to_string(), vec!["x".to_string()]);
        let centry = callee.add_block();
        callee.entry = centry;
        callee.body_head = centry;
        let x = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let nested = callee.add_value(
            ValueKind::Call {
                callee: "print".to_string(),
                args: vec![x],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        callee.blocks[centry].instrs.push(Instr::Eval {
            val: nested,
            span: Span::default(),
        });
        callee.blocks[centry].term = Terminator::Return(Some(x));

        let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let arg = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arg".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "call_wrapper".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].instrs.push(Instr::Assign {
            dst: "out".to_string(),
            src: call,
            span: Span::default(),
        });
        caller.blocks[entry].term = Terminator::Return(Some(call));

        let mut all = FxHashMap::default();
        all.insert("call_wrapper".to_string(), callee);
        all.insert("caller".to_string(), caller);

        let changed = MirInliner::new().optimize(&mut all);
        assert!(
            !changed,
            "helpers with nested calls should not be chosen for full-program inlining"
        );
        let caller = all.get("caller").expect("caller should remain present");
        let Instr::Assign { src, .. } = &caller.blocks[entry].instrs[0] else {
            panic!("expected original call assignment to remain");
        };
        assert!(
            matches!(caller.values[*src].kind, ValueKind::Call { .. }),
            "nested-call helper must remain as a call"
        );
    }

    #[test]
    fn perform_inline_remaps_record_field_value_ids() {
        let mut callee = FnIR::new("field_helper".to_string(), vec!["x".to_string()]);
        let centry = callee.add_block();
        callee.entry = centry;
        callee.body_head = centry;
        let param_x = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let rec = callee.add_value(
            ValueKind::RecordLit {
                fields: vec![("v".to_string(), param_x)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let ret = callee.add_value(
            ValueKind::FieldGet {
                base: rec,
                field: "v".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        callee.blocks[centry].instrs.push(Instr::Assign {
            dst: "out".to_string(),
            src: ret,
            span: Span::default(),
        });
        callee.blocks[centry].instrs.push(Instr::Eval {
            val: ret,
            span: Span::default(),
        });
        callee.blocks[centry].term = Terminator::Return(Some(ret));

        let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let c1 = caller.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let arg = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arg".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "field_helper".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = caller.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: c1,
                rhs: call,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].instrs.push(Instr::Assign {
            dst: "y".to_string(),
            src: call,
            span: Span::default(),
        });
        caller.blocks[entry].term = Terminator::Return(Some(sum));

        let inliner = MirInliner::new();
        inliner.perform_inline(
            &mut caller,
            entry,
            0,
            &[arg],
            call,
            Some("y".to_string()),
            &callee,
            Span::default(),
        );

        let ret_id = caller
            .blocks
            .iter()
            .find_map(|blk| match blk.term {
                Terminator::Return(Some(v)) => Some(v),
                _ => None,
            })
            .expect("expected return after inline");
        let ValueKind::Binary { rhs, .. } = caller.values[ret_id].kind else {
            panic!("expected return sum to stay binary");
        };
        let ValueKind::FieldGet { base, .. } = caller.values[rhs].kind else {
            panic!("expected inlined field get");
        };
        let ValueKind::RecordLit { ref fields } = caller.values[base].kind else {
            panic!("expected inlined record literal");
        };
        assert_eq!(
            fields[0].1, arg,
            "record field should remap to caller arg, not stale callee id"
        );
    }

    #[test]
    fn inline_value_calls_supports_record_field_helpers() {
        let mut callee = FnIR::new("field_expr_helper".to_string(), vec!["x".to_string()]);
        let centry = callee.add_block();
        callee.entry = centry;
        callee.body_head = centry;
        let param_x = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let rec = callee.add_value(
            ValueKind::RecordLit {
                fields: vec![("v".to_string(), param_x)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let ret = callee.add_value(
            ValueKind::FieldGet {
                base: rec,
                field: "v".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        callee.blocks[centry].term = Terminator::Return(Some(ret));

        let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let arg = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arg".to_string()),
        );
        let one = caller.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "field_expr_helper".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = caller.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: call,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(sum));

        let inliner = MirInliner::new();
        let ret = inliner
            .can_inline_expr(&callee)
            .expect("pure field helper should be expression-inlineable");
        let replacement = inliner
            .inline_call_value(&mut caller, call, &callee, ret, &[arg])
            .expect("field helper should clone into caller value graph");
        inliner.replace_uses(&mut caller, call, replacement);

        let ret_id = match caller.blocks[entry].term {
            Terminator::Return(Some(v)) => v,
            _ => panic!("expected return"),
        };
        let ValueKind::Binary { lhs, .. } = caller.values[ret_id].kind else {
            panic!("expected return sum");
        };
        let ValueKind::FieldGet { base, .. } = caller.values[lhs].kind else {
            panic!("expected inlined field get");
        };
        let ValueKind::RecordLit { ref fields } = caller.values[base].kind else {
            panic!("expected inlined record literal");
        };
        assert_eq!(
            fields[0].1, arg,
            "inlined field helper should remap record payload to caller arg"
        );
    }

    #[test]
    fn inline_value_calls_supports_intrinsic_helpers() {
        let mut callee = FnIR::new("intrinsic_expr_helper".to_string(), vec!["x".to_string()]);
        let centry = callee.add_block();
        callee.entry = centry;
        callee.body_head = centry;
        let param_x = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let ret = callee.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![param_x],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        callee.blocks[centry].term = Terminator::Return(Some(ret));

        let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let arg = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arg".to_string()),
        );
        let one = caller.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "intrinsic_expr_helper".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = caller.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: call,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(sum));

        let inliner = MirInliner::new();
        let ret = inliner
            .can_inline_expr(&callee)
            .expect("pure intrinsic helper should be expression-inlineable");
        let replacement = inliner
            .inline_call_value(&mut caller, call, &callee, ret, &[arg])
            .expect("intrinsic helper should clone into caller value graph");
        inliner.replace_uses(&mut caller, call, replacement);

        let ret_id = match caller.blocks[entry].term {
            Terminator::Return(Some(v)) => v,
            _ => panic!("expected return"),
        };
        let ValueKind::Binary { lhs, .. } = caller.values[ret_id].kind else {
            panic!("expected return sum");
        };
        let ValueKind::Intrinsic { ref args, .. } = caller.values[lhs].kind else {
            panic!("expected inlined intrinsic");
        };
        assert_eq!(
            args.as_slice(),
            &[arg],
            "inlined intrinsic helper should remap args to caller"
        );
    }

    #[test]
    fn inline_value_calls_supports_fieldset_helpers() {
        let mut callee = FnIR::new("fieldset_expr_helper".to_string(), vec!["x".to_string()]);
        let centry = callee.add_block();
        callee.entry = centry;
        callee.body_head = centry;
        let param_x = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let zero = callee.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let ret = callee.add_value(
            ValueKind::FieldSet {
                base: param_x,
                field: "v".to_string(),
                value: zero,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        callee.blocks[centry].term = Terminator::Return(Some(ret));

        let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let arg = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arg".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "fieldset_expr_helper".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(call));

        let inliner = MirInliner::new();
        let ret = inliner
            .can_inline_expr(&callee)
            .expect("pure fieldset helper should be expression-inlineable");
        let replacement = inliner
            .inline_call_value(&mut caller, call, &callee, ret, &[arg])
            .expect("fieldset helper should clone into caller value graph");
        inliner.replace_uses(&mut caller, call, replacement);

        let ret_id = match caller.blocks[entry].term {
            Terminator::Return(Some(v)) => v,
            _ => panic!("expected return"),
        };
        let ValueKind::FieldSet {
            base,
            ref field,
            value,
        } = caller.values[ret_id].kind
        else {
            panic!("expected inlined fieldset");
        };
        assert_eq!(base, arg, "fieldset base should remap to caller arg");
        assert_eq!(field, "v");
        assert!(matches!(
            caller.values[value].kind,
            ValueKind::Const(Lit::Int(0))
        ));
    }

    #[test]
    fn inline_value_calls_supports_index3d_helpers() {
        let mut callee = FnIR::new("index3d_expr_helper".to_string(), vec!["arr".to_string()]);
        let centry = callee.add_block();
        callee.entry = centry;
        callee.body_head = centry;
        let arr = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let one = callee.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let ret = callee.add_value(
            ValueKind::Index3D {
                base: arr,
                i: one,
                j: one,
                k: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        callee.blocks[centry].term = Terminator::Return(Some(ret));

        let mut caller = FnIR::new("caller".to_string(), vec!["arr".to_string()]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let arg = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "index3d_expr_helper".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(call));

        let inliner = MirInliner::new();
        let ret = inliner
            .can_inline_expr(&callee)
            .expect("pure index3d helper should be expression-inlineable");
        let replacement = inliner
            .inline_call_value(&mut caller, call, &callee, ret, &[arg])
            .expect("index3d helper should clone into caller value graph");
        inliner.replace_uses(&mut caller, call, replacement);

        let ret_id = match caller.blocks[entry].term {
            Terminator::Return(Some(v)) => v,
            _ => panic!("expected return"),
        };
        let ValueKind::Index3D { base, i, j, k } = caller.values[ret_id].kind else {
            panic!("expected inlined index3d");
        };
        assert_eq!(base, arg, "index3d base should remap to caller arg");
        assert_eq!(i, j);
        assert_eq!(j, k);
        assert!(matches!(
            caller.values[i].kind,
            ValueKind::Const(Lit::Int(1))
        ));
    }
}
