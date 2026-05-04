use super::*;

#[derive(Clone, Copy)]
pub(crate) struct InlinePolicy {
    pub(crate) max_blocks: usize,
    pub(crate) max_instrs: usize,
    pub(crate) max_cost: usize,
    pub(crate) max_callsite_cost: usize,
    pub(crate) max_kernel_cost: usize,
    pub(crate) max_caller_instrs: usize,
    pub(crate) max_total_instrs: usize,
    pub(crate) max_unit_growth_pct: usize,
    pub(crate) max_fn_growth_pct: usize,
    pub(crate) min_growth_abs: usize,
    pub(crate) allow_loops: bool,
}

pub(crate) struct InlineGrowthBudget {
    pub(crate) total_ir: usize,
    pub(crate) max_total_ir: usize,
    pub(crate) fn_limits: FxHashMap<String, usize>,
}

impl InlineGrowthBudget {
    pub(crate) fn growth_cap(base: usize, pct: usize, min_bonus: usize) -> usize {
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

    pub(crate) fn new(all_fns: &FxHashMap<String, FnIR>, policy: &InlinePolicy) -> Self {
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

    pub(crate) fn caller_limit(&self, caller: &str) -> usize {
        self.fn_limits
            .get(caller)
            .copied()
            .unwrap_or(usize::MAX.saturating_div(2))
    }

    pub(crate) fn can_inline(
        &self,
        caller_ir: usize,
        caller_limit: usize,
        predicted_growth: usize,
    ) -> bool {
        let next_caller = caller_ir.saturating_add(predicted_growth);
        if next_caller > caller_limit {
            return false;
        }
        let next_total = self.total_ir.saturating_add(predicted_growth);
        next_total <= self.max_total_ir
    }

    pub(crate) fn apply_resize(&mut self, before: usize, after: usize) {
        if after >= before {
            self.total_ir = self.total_ir.saturating_add(after - before);
        } else {
            self.total_ir = self.total_ir.saturating_sub(before - after);
        }
    }
}

pub(crate) fn term_successors(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Goto(b) => vec![*b],
        Terminator::If {
            then_bb, else_bb, ..
        } => vec![*then_bb, *else_bb],
        _ => vec![],
    }
}

pub(crate) fn new_bid_offset(_fn_ir: &FnIR, bid: BlockId) -> String {
    format!("{}", bid)
}

#[cfg(test)]
#[path = "../inline/tests.rs"]
pub(crate) mod tests;
