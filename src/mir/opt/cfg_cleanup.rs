use super::TachyonEngine;
use super::*;
use rustc_hash::FxHashSet;

impl TachyonEngine {
    pub(crate) fn simplify_cfg(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        let mut reachable = FxHashSet::default();
        let mut queue = vec![fn_ir.entry];
        reachable.insert(fn_ir.entry);

        let mut head = 0;
        while head < queue.len() {
            let bid = queue[head];
            head += 1;

            if let Some(blk) = fn_ir.blocks.get(bid) {
                match &blk.term {
                    Terminator::Goto(target) if reachable.insert(*target) => {
                        queue.push(*target);
                    }
                    Terminator::If {
                        then_bb, else_bb, ..
                    } => {
                        if reachable.insert(*then_bb) {
                            queue.push(*then_bb);
                        }
                        if reachable.insert(*else_bb) {
                            queue.push(*else_bb);
                        }
                    }
                    _ => {}
                }
            }
        }

        for bid in 0..fn_ir.blocks.len() {
            if !reachable.contains(&bid) {
                let blk = &mut fn_ir.blocks[bid];
                if !blk.instrs.is_empty() || !matches!(blk.term, Terminator::Unreachable) {
                    blk.instrs.clear();
                    blk.term = Terminator::Unreachable;
                    changed = true;
                }
            }
        }

        changed
    }

    pub(crate) fn dce(&self, fn_ir: &mut FnIR) -> bool {
        let reachable = Self::reachable_blocks(fn_ir);
        let mut live_in: Vec<FxHashSet<VarId>> = vec![FxHashSet::default(); fn_ir.blocks.len()];
        let mut live_out: Vec<FxHashSet<VarId>> = vec![FxHashSet::default(); fn_ir.blocks.len()];

        let mut dataflow_changed = true;
        while dataflow_changed {
            dataflow_changed = false;
            for bid in (0..fn_ir.blocks.len()).rev() {
                if !reachable.contains(&bid) {
                    continue;
                }

                let succ_live = Self::successor_live_vars(fn_ir, bid, &live_in);
                let block_live = self.compute_block_live_in(fn_ir, bid, &succ_live, &fn_ir.values);
                if live_out[bid] != succ_live {
                    live_out[bid] = succ_live.clone();
                    dataflow_changed = true;
                }
                if live_in[bid] != block_live {
                    live_in[bid] = block_live;
                    dataflow_changed = true;
                }
            }
        }

        let mut changed = false;
        for bid in 0..fn_ir.blocks.len() {
            if !reachable.contains(&bid) {
                continue;
            }
            let mut live = Self::successor_live_vars(fn_ir, bid, &live_in);
            self.collect_term_live_vars(&fn_ir.blocks[bid].term, &fn_ir.values, &mut live);

            let mut new_instrs_rev = Vec::with_capacity(fn_ir.blocks[bid].instrs.len());
            for instr in fn_ir.blocks[bid].instrs.iter().rev() {
                match instr {
                    Instr::Assign { dst, src, span } => {
                        let pinned = Self::is_pinned_live_var(dst);
                        let removable = Self::can_eliminate_assign_dst(dst);
                        if pinned || !removable || live.remove(dst) {
                            self.collect_value_live_vars(*src, &fn_ir.values, &mut live);
                            new_instrs_rev.push(instr.clone());
                        } else if self.has_side_effect_val(*src, &fn_ir.values) {
                            self.collect_value_live_vars(*src, &fn_ir.values, &mut live);
                            new_instrs_rev.push(Instr::Eval {
                                val: *src,
                                span: *span,
                            });
                            changed = true;
                        } else {
                            changed = true;
                        }
                    }
                    Instr::Eval { val, .. } => {
                        if self.has_side_effect_val(*val, &fn_ir.values) {
                            self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                            new_instrs_rev.push(instr.clone());
                        } else {
                            changed = true;
                        }
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        self.collect_value_live_vars(*base, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*idx, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                        new_instrs_rev.push(instr.clone());
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        self.collect_value_live_vars(*base, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*r, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*c, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                        new_instrs_rev.push(instr.clone());
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        self.collect_value_live_vars(*base, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*i, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*j, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*k, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                        new_instrs_rev.push(instr.clone());
                    }
                    Instr::UnsafeRBlock { code, .. } => {
                        Self::collect_unsafe_r_named_vars(code, &mut live);
                        new_instrs_rev.push(instr.clone());
                    }
                }
            }
            new_instrs_rev.reverse();
            if fn_ir.blocks[bid].instrs != new_instrs_rev {
                fn_ir.blocks[bid].instrs = new_instrs_rev;
                changed = true;
            }
        }

        changed
    }

    pub(crate) fn has_side_effect_instr(&self, instr: &Instr, values: &[Value]) -> bool {
        match instr {
            Instr::StoreIndex1D { .. } => true,
            Instr::StoreIndex2D { .. } => true,
            Instr::StoreIndex3D { .. } => true,
            Instr::Assign { src, .. } => self.has_side_effect_val(*src, values),
            Instr::Eval { val, .. } => self.has_side_effect_val(*val, values),
            Instr::UnsafeRBlock { .. } => true,
        }
    }

    pub(crate) fn reachable_blocks(fn_ir: &FnIR) -> FxHashSet<BlockId> {
        let mut reachable = FxHashSet::default();
        let mut queue = vec![fn_ir.entry];
        reachable.insert(fn_ir.entry);
        let mut head = 0;
        while head < queue.len() {
            let bid = queue[head];
            head += 1;
            for succ in Self::block_successors(fn_ir, bid) {
                if reachable.insert(succ) {
                    queue.push(succ);
                }
            }
        }
        reachable
    }

    pub(crate) fn block_successors(fn_ir: &FnIR, bid: BlockId) -> Vec<BlockId> {
        match &fn_ir.blocks[bid].term {
            Terminator::Goto(target) => vec![*target],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
        }
    }

    pub(crate) fn successor_live_vars(
        fn_ir: &FnIR,
        bid: BlockId,
        live_in: &[FxHashSet<VarId>],
    ) -> FxHashSet<VarId> {
        let mut live = FxHashSet::default();
        for succ in Self::block_successors(fn_ir, bid) {
            live.extend(live_in[succ].iter().cloned());
        }
        live
    }

    pub(crate) fn compute_block_live_in(
        &self,
        fn_ir: &FnIR,
        bid: BlockId,
        succ_live: &FxHashSet<VarId>,
        values: &[Value],
    ) -> FxHashSet<VarId> {
        let blk = &fn_ir.blocks[bid];
        let mut live = succ_live.clone();
        self.collect_term_live_vars(&blk.term, values, &mut live);
        for instr in blk.instrs.iter().rev() {
            match instr {
                Instr::Assign { dst, src, .. } => {
                    let removable = Self::can_eliminate_assign_dst(dst);
                    if Self::is_pinned_live_var(dst)
                        || !removable
                        || live.remove(dst)
                        || self.has_side_effect_val(*src, values)
                    {
                        self.collect_value_live_vars(*src, values, &mut live);
                    }
                }
                Instr::Eval { val, .. } => {
                    if self.has_side_effect_val(*val, values) {
                        self.collect_value_live_vars(*val, values, &mut live);
                    }
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    self.collect_value_live_vars(*base, values, &mut live);
                    self.collect_value_live_vars(*idx, values, &mut live);
                    self.collect_value_live_vars(*val, values, &mut live);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    self.collect_value_live_vars(*base, values, &mut live);
                    self.collect_value_live_vars(*r, values, &mut live);
                    self.collect_value_live_vars(*c, values, &mut live);
                    self.collect_value_live_vars(*val, values, &mut live);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    self.collect_value_live_vars(*base, values, &mut live);
                    self.collect_value_live_vars(*i, values, &mut live);
                    self.collect_value_live_vars(*j, values, &mut live);
                    self.collect_value_live_vars(*k, values, &mut live);
                    self.collect_value_live_vars(*val, values, &mut live);
                }
                Instr::UnsafeRBlock { code, .. } => {
                    Self::collect_unsafe_r_named_vars(code, &mut live);
                }
            }
        }
        live
    }

    pub(crate) fn collect_unsafe_r_named_vars(code: &str, live: &mut FxHashSet<VarId>) {
        let mut ident = String::new();
        let mut string_quote: Option<char> = None;
        let mut backtick_ident: Option<String> = None;
        let mut escaped = false;
        let mut in_comment = false;

        pub(crate) fn flush_ident(ident: &mut String, live: &mut FxHashSet<VarId>) {
            if !ident.is_empty() {
                live.insert(std::mem::take(ident));
            }
        }

        pub(crate) fn is_r_ident_continue(ch: char) -> bool {
            ch == '_' || ch == '.' || ch.is_ascii_alphanumeric()
        }

        for ch in code.chars() {
            if in_comment {
                if ch == '\n' {
                    in_comment = false;
                }
                continue;
            }
            if let Some(name) = backtick_ident.as_mut() {
                if escaped {
                    name.push(ch);
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '`' {
                    if let Some(name) = backtick_ident.take()
                        && !name.is_empty()
                    {
                        live.insert(name);
                    }
                } else {
                    name.push(ch);
                }
                continue;
            }
            if let Some(q) = string_quote {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == q {
                    string_quote = None;
                }
                continue;
            }

            match ch {
                '"' | '\'' => {
                    flush_ident(&mut ident, live);
                    string_quote = Some(ch);
                }
                '`' => {
                    flush_ident(&mut ident, live);
                    backtick_ident = Some(String::new());
                }
                '#' => {
                    flush_ident(&mut ident, live);
                    in_comment = true;
                }
                c if c == '_' || c == '.' || c.is_ascii_alphabetic() => ident.push(c),
                c if !ident.is_empty() && is_r_ident_continue(c) => ident.push(c),
                _ => flush_ident(&mut ident, live),
            }
        }
        flush_ident(&mut ident, live);
        if let Some(name) = backtick_ident
            && !name.is_empty()
        {
            live.insert(name);
        }
    }

    pub(crate) fn collect_term_live_vars(
        &self,
        term: &Terminator,
        values: &[Value],
        live: &mut FxHashSet<VarId>,
    ) {
        match term {
            Terminator::If { cond, .. } => self.collect_value_live_vars(*cond, values, live),
            Terminator::Return(Some(val)) => self.collect_value_live_vars(*val, values, live),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    pub(crate) fn collect_value_live_vars(
        &self,
        root: ValueId,
        values: &[Value],
        live: &mut FxHashSet<VarId>,
    ) {
        let mut stack = vec![root];
        let mut seen = FxHashSet::default();
        while let Some(vid) = stack.pop() {
            if !seen.insert(vid) {
                continue;
            }
            if let Some(origin_var) = &values[vid].origin_var {
                live.insert(origin_var.clone());
            }
            match &values[vid].kind {
                ValueKind::Load { var } => {
                    live.insert(var.clone());
                }
                ValueKind::Binary { lhs, rhs, .. } => {
                    stack.push(*lhs);
                    stack.push(*rhs);
                }
                ValueKind::Unary { rhs, .. } => stack.push(*rhs),
                ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                    stack.extend(args.iter().copied());
                }
                ValueKind::RecordLit { fields } => {
                    stack.extend(fields.iter().map(|(_, value)| *value));
                }
                ValueKind::FieldGet { base, .. } => stack.push(*base),
                ValueKind::FieldSet { base, value, .. } => {
                    stack.push(*base);
                    stack.push(*value);
                }
                ValueKind::Index1D { base, idx, .. } => {
                    stack.push(*base);
                    stack.push(*idx);
                }
                ValueKind::Index2D { base, r, c } => {
                    stack.push(*base);
                    stack.push(*r);
                    stack.push(*c);
                }
                ValueKind::Index3D { base, i, j, k } => {
                    stack.push(*base);
                    stack.push(*i);
                    stack.push(*j);
                    stack.push(*k);
                }
                ValueKind::Range { start, end } => {
                    stack.push(*start);
                    stack.push(*end);
                }
                ValueKind::Len { base } | ValueKind::Indices { base } => stack.push(*base),
                ValueKind::Phi { args } => {
                    stack.extend(args.iter().map(|(arg, _)| *arg));
                }
                ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => {}
            }
        }
    }

    pub(crate) fn is_pinned_live_var(var: &str) -> bool {
        var.starts_with(".arg_")
    }

    pub(crate) fn can_eliminate_assign_dst(var: &str) -> bool {
        var.starts_with(".tachyon_") || var.starts_with(".__rr_") || var.starts_with("inlined_")
    }

    pub(crate) fn has_side_effect_val(&self, val_id: ValueId, values: &[Value]) -> bool {
        let pure = [
            "length",
            "c",
            "seq_along",
            "list",
            "sum",
            "mean",
            "min",
            "max",
            "rr_field_get",
            "rr_field_exists",
            "rr_list_pattern_matchable",
            "rr_named_list",
        ];
        let mut stack = vec![val_id];
        let mut seen = FxHashSet::default();
        while let Some(current) = stack.pop() {
            if current >= values.len() {
                return true;
            }
            if !seen.insert(current) {
                continue;
            }
            let val = &values[current];
            if let ValueKind::Call { callee, .. } = &val.kind
                && !pure.contains(&callee.as_str())
            {
                return true;
            }
            stack.extend(value_dependencies(&val.kind));
        }
        false
    }
}

#[cfg(test)]
#[path = "cfg_cleanup/tests.rs"]
pub(crate) mod tests;
