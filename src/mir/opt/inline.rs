use crate::mir::flow::Facts;
use crate::mir::*;
use crate::utils::Span;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;

pub struct MirInliner;

#[derive(Default)]
struct InlineMap {
    v: HashMap<ValueId, ValueId>,
    b: HashMap<BlockId, BlockId>,
    vars: HashMap<VarId, VarId>,
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

impl MirInliner {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, all_fns: &mut HashMap<String, FnIR>) -> bool {
        if Self::inline_disabled() {
            return false;
        }
        let mut global_changed = false;
        let fn_names: Vec<String> = all_fns.keys().cloned().collect();

        for name in fn_names {
            if let Some(mut fn_ir) = all_fns.remove(&name) {
                if fn_ir.unsupported_dynamic {
                    all_fns.insert(name, fn_ir);
                    continue;
                }
                let mut local_changed = true;
                let mut iterations = 0;
                let local_rounds = Self::env_usize("RR_INLINE_LOCAL_ROUNDS", 2);

                while local_changed && iterations < local_rounds {
                    local_changed = self.inline_calls(&mut fn_ir, all_fns);
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

    fn inline_calls(&self, caller: &mut FnIR, all_fns: &HashMap<String, FnIR>) -> bool {
        let mut changed = false;
        let policy = Self::policy();
        let caller_instr_cnt: usize = caller.blocks.iter().map(|b| b.instrs.len()).sum();
        if caller_instr_cnt > policy.max_caller_instrs {
            return false;
        }

        if self.inline_value_calls(caller, all_fns) {
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
                        if self.should_inline(callee, caller) {
                            candidate = Some((
                                bid,
                                idx,
                                callee_name,
                                args,
                                target_val,
                                call_dst,
                                call_span,
                            ));
                            break 'scan;
                        }
                    }
                }
            }
        }

        if let Some((bid, idx, callee_name, args, target_val, call_dst, call_span)) = candidate {
            let callee = all_fns.get(&callee_name).unwrap();
            self.perform_inline(
                caller, bid, idx, &args, target_val, call_dst, callee, call_span,
            );
            changed = true;
        }

        changed
    }

    fn analyze_instr(
        &self,
        caller: &FnIR,
        instr: &Instr,
        all_fns: &HashMap<String, FnIR>,
    ) -> Option<(String, Vec<ValueId>, ValueId, Option<VarId>, Span)> {
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

    fn should_inline(&self, target: &FnIR, caller: &FnIR) -> bool {
        if target.unsupported_dynamic {
            return false;
        }
        if target.name.starts_with("Sym_top_") {
            return false;
        }
        let policy = Self::policy();
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

        let mut loop_edges = 0usize;
        let mut call_count = 0usize;
        for (bid, bb) in target.blocks.iter().enumerate() {
            for ins in &bb.instrs {
                if let Instr::Assign { src, .. } | Instr::Eval { val: src, .. } = ins {
                    if matches!(target.values[*src].kind, ValueKind::Call { .. }) {
                        call_count += 1;
                    }
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
        if !policy.allow_loops && loop_edges > 0 {
            return false;
        }

        let cost = instr_cnt + (block_cnt * 2) + (loop_edges * 40) + (call_count * 8);
        cost <= policy.max_cost
    }

    fn inline_disabled() -> bool {
        match env::var("RR_DISABLE_INLINE") {
            Ok(v) => matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => false,
        }
    }

    fn env_usize(key: &str, default_v: usize) -> usize {
        env::var(key)
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(default_v)
    }

    fn env_bool(key: &str, default_v: bool) -> bool {
        match env::var(key) {
            Ok(v) => matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => default_v,
        }
    }

    fn policy() -> InlinePolicy {
        InlinePolicy {
            max_blocks: Self::env_usize("RR_INLINE_MAX_BLOCKS", 24),
            max_instrs: Self::env_usize("RR_INLINE_MAX_INSTRS", 160),
            max_cost: Self::env_usize("RR_INLINE_MAX_COST", 220),
            max_caller_instrs: Self::env_usize("RR_INLINE_MAX_CALLER_INSTRS", 480),
            max_total_instrs: Self::env_usize("RR_INLINE_MAX_TOTAL_INSTRS", 900),
            allow_loops: Self::env_bool("RR_INLINE_ALLOW_LOOPS", false),
        }
    }

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
        let mut mutated_param_inits: HashMap<VarId, ValueId> = HashMap::new();

        let mut param_val_ids: HashMap<usize, ValueId> = HashMap::new();
        for (vid, val) in callee.values.iter().enumerate() {
            if let ValueKind::Param { index } = val.kind {
                param_val_ids.insert(index, vid);
            }
        }
        let mut mutated_params: HashSet<usize> = HashSet::new();
        for blk in &callee.blocks {
            for instr in &blk.instrs {
                if let Instr::Assign { dst, src, .. } = instr {
                    if let Some(idx) = callee.params.iter().position(|p| p == dst) {
                        if let Some(&param_vid) = param_val_ids.get(&idx) {
                            if *src != param_vid {
                                mutated_params.insert(idx);
                            }
                        }
                    }
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
                val.facts.clone(),
                None,
            );

            if let Some(name) = &val.origin_var {
                let new_name = map.map_var(name);
                caller.values[new_vid].origin_var = Some(new_name);
            }
            if let Some(old_bb) = val.phi_block {
                if let Some(&new_bb) = map.b.get(&old_bb) {
                    caller.values[new_vid].phi_block = Some(new_bb);
                }
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
                if let ValueKind::Phi { args } = &mut val.kind {
                    if let Some(phi_bb) = val.phi_block {
                        if old_succs.contains(&phi_bb) {
                            for (_, pred_bb) in args.iter_mut() {
                                if *pred_bb == call_block {
                                    *pred_bb = continuation_bb;
                                }
                            }
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

        if !mutated_param_inits.is_empty() {
            if let Some(&entry_bid) = map.b.get(&callee.entry) {
                for instr in &mut caller.blocks[entry_bid].instrs {
                    if let Instr::Assign { dst, src, .. } = instr {
                        if let Some(&arg_val) = mutated_param_inits.get(dst) {
                            *src = arg_val;
                        }
                    }
                }
            }
        }
    }

    fn resolve_callee_name<'a>(&self, callee: &'a str) -> &'a str {
        if callee.ends_with("_fresh") {
            &callee[..callee.len() - 6]
        } else {
            callee
        }
    }

    fn inline_value_calls(&self, caller: &mut FnIR, all_fns: &HashMap<String, FnIR>) -> bool {
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

            let ret_val = match self.can_inline_expr(callee) {
                Some(v) => v,
                None => continue,
            };

            if let Some(replacement) =
                self.inline_call_value(caller, val_id, callee, ret_val, &args)
            {
                self.replace_uses(caller, val_id, replacement);
                caller.values[val_id].kind = ValueKind::Const(crate::syntax::ast::Lit::Null);
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

    fn reachable_blocks(&self, callee: &FnIR) -> HashSet<BlockId> {
        let mut reachable = HashSet::new();
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
        let mut map: HashMap<ValueId, ValueId> = HashMap::new();

        let clone_value = |vid: ValueId,
                           caller: &mut FnIR,
                           map: &mut HashMap<ValueId, ValueId>,
                           args: &[ValueId]|
         -> Option<ValueId> {
            fn clone_rec(
                vid: ValueId,
                caller: &mut FnIR,
                callee: &FnIR,
                map: &mut HashMap<ValueId, ValueId>,
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
                        return None;
                    }
                    ValueKind::Const(lit) => {
                        let new_id = caller.add_value(
                            ValueKind::Const(lit.clone()),
                            val.span,
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
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
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
                    }
                    ValueKind::Unary { op, rhs } => {
                        let r = clone_rec(*rhs, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Unary { op: *op, rhs: r },
                            val.span,
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
                    }
                    ValueKind::Len { base } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Len { base: b },
                            val.span,
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
                    }
                    ValueKind::Indices { base } => {
                        let b = clone_rec(*base, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Indices { base: b },
                            val.span,
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
                    }
                    ValueKind::Range { start, end } => {
                        let s = clone_rec(*start, caller, callee, map, args)?;
                        let e = clone_rec(*end, caller, callee, map, args)?;
                        let new_id = caller.add_value(
                            ValueKind::Range { start: s, end: e },
                            val.span,
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
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
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
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
                            val.facts.clone(),
                            None,
                        );
                        map.insert(vid, new_id);
                        return Some(new_id);
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

struct InlinePolicy {
    max_blocks: usize,
    max_instrs: usize,
    max_cost: usize,
    max_caller_instrs: usize,
    max_total_instrs: usize,
    allow_loops: bool,
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
