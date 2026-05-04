use super::*;
pub(crate) struct InlineSite<'a> {
    pub(crate) call_block: BlockId,
    pub(crate) instr_idx: usize,
    pub(crate) call_args: &'a [ValueId],
    pub(crate) call_val_target: ValueId,
    pub(crate) call_dst: Option<VarId>,
    pub(crate) call_span: Span,
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

    pub fn new_aggressive() -> Self {
        Self {
            policy: Self::aggressive_policy(),
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

    pub(crate) fn inline_calls(
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
                caller,
                callee,
                InlineSite {
                    call_block: bid,
                    instr_idx: idx,
                    call_args: &args,
                    call_val_target: target_val,
                    call_dst,
                    call_span,
                },
            );
            let after_size = Self::fn_ir_size(caller);
            growth.apply_resize(before_size, after_size);
            changed = true;
        }

        changed
    }

    pub(crate) fn analyze_instr(
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

    pub(crate) fn should_inline(
        &self,
        target: &FnIR,
        caller: &FnIR,
        policy: &InlinePolicy,
    ) -> bool {
        if target.unsupported_dynamic {
            return false;
        }
        if target.name.starts_with("Sym_top_") {
            return false;
        }
        if target.name.starts_with("__rr_outline_") {
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
        let kernel_cost = self.inline_kernel_cost(target);
        if kernel_cost > policy.max_kernel_cost {
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
                Terminator::Goto(t) if t <= bid => {
                    loop_edges += 1;
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

    pub(crate) fn inline_disabled() -> bool {
        false
    }

    pub(crate) fn env_usize(key: &str, default_v: usize) -> usize {
        std::env::var(key)
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .unwrap_or(default_v)
    }

    pub(crate) fn env_bool(key: &str, default_v: bool) -> bool {
        std::env::var(key)
            .ok()
            .map(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(default_v)
    }

    pub(crate) fn standard_policy() -> InlinePolicy {
        InlinePolicy {
            max_blocks: Self::env_usize("RR_INLINE_MAX_BLOCKS", 24),
            max_instrs: Self::env_usize("RR_INLINE_MAX_INSTRS", 160),
            max_cost: Self::env_usize("RR_INLINE_MAX_COST", 220),
            max_callsite_cost: Self::env_usize("RR_INLINE_MAX_CALLSITE_COST", 240),
            max_kernel_cost: Self::env_usize("RR_INLINE_MAX_KERNEL_COST", 170),
            max_caller_instrs: Self::env_usize("RR_INLINE_MAX_CALLER_INSTRS", 480),
            max_total_instrs: Self::env_usize("RR_INLINE_MAX_TOTAL_INSTRS", 900),
            max_unit_growth_pct: Self::env_usize("RR_INLINE_MAX_UNIT_GROWTH_PCT", 25),
            max_fn_growth_pct: Self::env_usize("RR_INLINE_MAX_FN_GROWTH_PCT", 35),
            min_growth_abs: 0,
            allow_loops: Self::env_bool("RR_INLINE_ALLOW_LOOPS", false),
        }
    }

    pub(crate) fn fast_dev_policy() -> InlinePolicy {
        InlinePolicy {
            max_blocks: 8,
            max_instrs: 48,
            max_cost: 128,
            max_callsite_cost: 144,
            max_kernel_cost: 96,
            max_caller_instrs: 192,
            max_total_instrs: 320,
            max_unit_growth_pct: 8,
            max_fn_growth_pct: 12,
            min_growth_abs: 24,
            allow_loops: false,
        }
    }

    pub(crate) fn aggressive_policy() -> InlinePolicy {
        InlinePolicy {
            max_blocks: Self::env_usize("RR_INLINE_O3_MAX_BLOCKS", 40),
            max_instrs: Self::env_usize("RR_INLINE_O3_MAX_INSTRS", 260),
            max_cost: Self::env_usize("RR_INLINE_O3_MAX_COST", 420),
            max_callsite_cost: Self::env_usize("RR_INLINE_O3_MAX_CALLSITE_COST", 480),
            max_kernel_cost: Self::env_usize("RR_INLINE_O3_MAX_KERNEL_COST", 260),
            max_caller_instrs: Self::env_usize("RR_INLINE_O3_MAX_CALLER_INSTRS", 900),
            max_total_instrs: Self::env_usize("RR_INLINE_O3_MAX_TOTAL_INSTRS", 1600),
            max_unit_growth_pct: Self::env_usize("RR_INLINE_O3_MAX_UNIT_GROWTH_PCT", 45),
            max_fn_growth_pct: Self::env_usize("RR_INLINE_O3_MAX_FN_GROWTH_PCT", 70),
            min_growth_abs: 48,
            allow_loops: Self::env_bool("RR_INLINE_O3_ALLOW_LOOPS", false),
        }
    }

    pub(crate) fn perform_inline(&self, caller: &mut FnIR, callee: &FnIR, site: InlineSite<'_>) {
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
                if index < site.call_args.len() {
                    if mutated_params.contains(&index) {
                        let param_name = callee.params[index].clone();
                        let mapped_var = map.map_var(&param_name);
                        let load_id = caller.add_value(
                            ValueKind::Load {
                                var: mapped_var.clone(),
                            },
                            site.call_span,
                            Facts::empty(),
                            None,
                        );
                        map.v.insert(cvid, load_id);
                        mutated_param_inits.insert(mapped_var, site.call_args[index]);
                    } else {
                        map.v.insert(cvid, site.call_args[index]);
                    }
                } else {
                    let dummy = caller.add_value(
                        ValueKind::Const(crate::syntax::ast::Lit::Null),
                        site.call_span,
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
            copy_cloned_value_metadata(caller, new_vid, val);

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
        let post_split: Vec<Instr> = caller.blocks[site.call_block]
            .instrs
            .drain((site.instr_idx + 1)..)
            .collect();
        caller.blocks[continuation_bb].instrs = post_split;
        caller.blocks[continuation_bb].term = caller.blocks[site.call_block].term.clone();
        let old_term = caller.blocks[continuation_bb].term.clone();

        caller.blocks[site.call_block]
            .instrs
            .truncate(site.instr_idx);

        let callee_entry = map.b[&callee.entry];
        caller.blocks[site.call_block].term = Terminator::Goto(callee_entry);

        let old_succs = term_successors(&old_term);
        if !old_succs.is_empty() {
            for val in &mut caller.values {
                if let ValueKind::Phi { args } = &mut val.kind
                    && let Some(phi_bb) = val.phi_block
                    && old_succs.contains(&phi_bb)
                {
                    for (_, pred_bb) in args.iter_mut() {
                        if *pred_bb == site.call_block {
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
                        site.call_span,
                        Facts::empty(),
                        None,
                    );
                    returns.push((null_val, nbid));
                }
                caller.blocks[nbid].term = Terminator::Goto(continuation_bb);
            }
        }

        let res_id: ValueId = if returns.is_empty() {
            caller.values[site.call_val_target].kind =
                ValueKind::Const(crate::syntax::ast::Lit::Null);
            site.call_val_target
        } else if returns.len() == 1 {
            let (single_ret, _) = returns[0];
            self.replace_uses(caller, site.call_val_target, single_ret);
            single_ret
        } else {
            caller.blocks[continuation_bb].instrs.insert(
                0,
                Instr::Eval {
                    val: site.call_val_target,
                    span: site.call_span,
                },
            );
            let phi_args = returns;
            caller.values[site.call_val_target].kind = ValueKind::Phi { args: phi_args };
            caller.values[site.call_val_target].phi_block = Some(continuation_bb);
            site.call_val_target
        };

        if let Some(dst) = site.call_dst {
            caller.blocks[continuation_bb].instrs.insert(
                0,
                Instr::Assign {
                    dst,
                    src: res_id,
                    span: site.call_span,
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

    pub(crate) fn resolve_callee_name<'a>(&self, callee: &'a str) -> &'a str {
        callee.strip_suffix("_fresh").unwrap_or(callee)
    }

    pub(crate) fn fn_ir_size(fn_ir: &FnIR) -> usize {
        let instrs: usize = fn_ir.blocks.iter().map(|b| b.instrs.len()).sum();
        fn_ir.values.len().saturating_add(instrs)
    }

    pub(crate) fn inline_callsite_cost(&self, target: &FnIR) -> usize {
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
                Terminator::Goto(t) if t <= bid => {
                    loop_edges += 1;
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
            .saturating_add(self.inline_kernel_cost(target))
    }

    pub(crate) fn inline_kernel_cost(&self, target: &FnIR) -> usize {
        let mut cost = target.blocks.len().saturating_mul(2);
        for value in &target.values {
            cost = cost.saturating_add(match value.kind {
                ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => 1,
                ValueKind::Unary { .. } => 2,
                ValueKind::Binary { .. } => 4,
                ValueKind::Len { .. } | ValueKind::Indices { .. } | ValueKind::Range { .. } => 4,
                ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
                | ValueKind::Index3D { .. } => 8,
                ValueKind::RecordLit { .. }
                | ValueKind::FieldGet { .. }
                | ValueKind::FieldSet { .. } => 6,
                ValueKind::Intrinsic { .. } => 12,
                ValueKind::Call { .. } => 24,
                ValueKind::Phi { .. } => 12,
                ValueKind::RSymbol { .. } => 2,
            });
        }
        for (bid, block) in target.blocks.iter().enumerate() {
            cost = cost.saturating_add(match block.term {
                Terminator::Return(_) => 1,
                Terminator::Goto(target_bid) => {
                    if target_bid <= bid {
                        40
                    } else {
                        4
                    }
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    let back_edges = usize::from(then_bb <= bid) + usize::from(else_bb <= bid);
                    12usize.saturating_add(back_edges.saturating_mul(40))
                }
                Terminator::Unreachable => 1,
            });
            for instr in &block.instrs {
                cost = cost.saturating_add(match instr {
                    Instr::Assign { .. } => 2,
                    Instr::Eval { .. } => 3,
                    Instr::StoreIndex1D { .. } => 10,
                    Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => 14,
                    Instr::UnsafeRBlock { .. } => 128,
                });
            }
        }
        cost
    }

    pub(crate) fn estimate_inline_growth(callee: &FnIR, policy: &InlinePolicy) -> usize {
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

    pub(crate) fn inline_value_calls(
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

    pub(crate) fn can_inline_expr(&self, callee: &FnIR) -> Option<ValueId> {
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

    pub(crate) fn reachable_blocks(&self, callee: &FnIR) -> FxHashSet<BlockId> {
        let mut reachable = FxHashSet::default();
        let mut queue = VecDeque::new();
        queue.push_back(callee.entry);
        reachable.insert(callee.entry);
        while let Some(bid) = queue.pop_front() {
            let blk = &callee.blocks[bid];
            match blk.term {
                Terminator::Goto(t) if reachable.insert(t) => {
                    queue.push_back(t);
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

    pub(crate) fn inline_call_value(
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
            pub(crate) fn clone_rec(
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
                let new_id = match &val.kind {
                    ValueKind::Param { index } => {
                        if *index < args.len() {
                            let mapped = args[*index];
                            map.insert(vid, mapped);
                            return Some(mapped);
                        }
                        return None;
                    }
                    ValueKind::Const(lit) => {
                        caller.add_value(ValueKind::Const(lit.clone()), val.span, val.facts, None)
                    }
                    ValueKind::Binary { op, lhs, rhs } => {
                        let l = clone_rec(*lhs, caller, callee, map, args)?;
                        let r = clone_rec(*rhs, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::Binary {
                                op: *op,
                                lhs: l,
                                rhs: r,
                            },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::Unary { op, rhs } => {
                        let r = clone_rec(*rhs, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::Unary { op: *op, rhs: r },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::Len { base } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        caller.add_value(ValueKind::Len { base: b }, val.span, val.facts, None)
                    }
                    ValueKind::Indices { base } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        caller.add_value(ValueKind::Indices { base: b }, val.span, val.facts, None)
                    }
                    ValueKind::Range { start, end } => {
                        let s = clone_rec(*start, caller, callee, map, args)?;
                        let e = clone_rec(*end, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::Range { start: s, end: e },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::Index1D {
                        base,
                        idx,
                        is_safe,
                        is_na_safe,
                    } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let i = clone_rec(*idx, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::Index1D {
                                base: b,
                                idx: i,
                                is_safe: *is_safe,
                                is_na_safe: *is_na_safe,
                            },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::Index2D { base, r, c } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let rv = clone_rec(*r, caller, callee, map, args)?;
                        let cv = clone_rec(*c, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::Index2D {
                                base: b,
                                r: rv,
                                c: cv,
                            },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::Index3D { base, i, j, k } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let iv = clone_rec(*i, caller, callee, map, args)?;
                        let jv = clone_rec(*j, caller, callee, map, args)?;
                        let kv = clone_rec(*k, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::Index3D {
                                base: b,
                                i: iv,
                                j: jv,
                                k: kv,
                            },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::RecordLit { fields } => {
                        let mut new_fields = Vec::with_capacity(fields.len());
                        for (name, field_val) in fields {
                            let mapped = clone_rec(*field_val, caller, callee, map, args)?;
                            new_fields.push((name.clone(), mapped));
                        }
                        caller.add_value(
                            ValueKind::RecordLit { fields: new_fields },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::FieldGet { base, field } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::FieldGet {
                                base: b,
                                field: field.clone(),
                            },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::FieldSet { base, field, value } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let v = clone_rec(*value, caller, callee, map, args)?;
                        caller.add_value(
                            ValueKind::FieldSet {
                                base: b,
                                field: field.clone(),
                                value: v,
                            },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::Intrinsic {
                        op,
                        args: intrinsic_args,
                    } => {
                        let mut new_args = Vec::with_capacity(intrinsic_args.len());
                        for arg in intrinsic_args {
                            new_args.push(clone_rec(*arg, caller, callee, map, args)?);
                        }
                        caller.add_value(
                            ValueKind::Intrinsic {
                                op: *op,
                                args: new_args,
                            },
                            val.span,
                            val.facts,
                            None,
                        )
                    }
                    ValueKind::RSymbol { name } => caller.add_value(
                        ValueKind::RSymbol { name: name.clone() },
                        val.span,
                        val.facts,
                        None,
                    ),
                    _ => return None,
                };
                copy_cloned_value_metadata(caller, new_id, val);
                map.insert(vid, new_id);
                Some(new_id)
            }
            clone_rec(vid, caller, callee, map, args)
        };

        let replacement = clone_value(ret_val, caller, &mut map, args)?;
        if replacement == call_val_id {
            return Some(replacement);
        }
        Some(replacement)
    }
}
