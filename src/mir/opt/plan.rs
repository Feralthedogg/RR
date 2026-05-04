use super::*;

impl TachyonEngine {
    pub(crate) fn load_hot_profile_counts() -> FxHashMap<String, usize> {
        let mut counts = FxHashMap::default();
        let Some(path) = Self::profile_use_path() else {
            return counts;
        };
        let Ok(content) = fs::read_to_string(path) else {
            return counts;
        };

        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (name, count_str) = if let Some((k, v)) = line.split_once('=') {
                (k.trim(), v.trim())
            } else if let Some((k, v)) = line.split_once(':') {
                (k.trim(), v.trim())
            } else {
                let mut parts = line.split_whitespace();
                let Some(k) = parts.next() else { continue };
                let Some(v) = parts.next() else { continue };
                (k, v)
            };
            if name.is_empty() {
                continue;
            }
            let Ok(parsed) = count_str.parse::<usize>() else {
                continue;
            };
            let entry = counts.entry(name.to_string()).or_insert(0);
            *entry = (*entry).saturating_add(parsed);
        }
        counts
    }

    pub(crate) fn fn_static_hotness(fn_ir: &FnIR) -> usize {
        let mut loops = 0usize;
        let mut branches = 0usize;
        let mut calls = 0usize;
        let mut stores = 0usize;
        for (bid, bb) in fn_ir.blocks.iter().enumerate() {
            match bb.term {
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    branches += 1;
                    if then_bb <= bid {
                        loops += 1;
                    }
                    if else_bb <= bid {
                        loops += 1;
                    }
                }
                Terminator::Goto(t) if t <= bid => {
                    loops += 1;
                }
                _ => {}
            }
            for ins in &bb.instrs {
                if matches!(
                    ins,
                    Instr::StoreIndex1D { .. }
                        | Instr::StoreIndex2D { .. }
                        | Instr::StoreIndex3D { .. }
                ) {
                    stores += 1;
                }
            }
        }
        for v in &fn_ir.values {
            if matches!(v.kind, ValueKind::Call { .. } | ValueKind::Intrinsic { .. }) {
                calls += 1;
            }
        }
        loops
            .saturating_mul(20)
            .saturating_add(branches.saturating_mul(8))
            .saturating_add(calls.saturating_mul(6))
            .saturating_add(stores.saturating_mul(4))
    }

    pub(crate) fn fn_ir_fingerprint(fn_ir: &FnIR) -> u64 {
        pub(crate) fn hash_instr(h: &mut DefaultHasher, instr: &Instr) {
            match instr {
                Instr::Assign { dst, src, .. } => {
                    1u8.hash(h);
                    dst.hash(h);
                    src.hash(h);
                }
                Instr::Eval { val, .. } => {
                    2u8.hash(h);
                    val.hash(h);
                }
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_safe,
                    is_na_safe,
                    is_vector,
                    ..
                } => {
                    3u8.hash(h);
                    base.hash(h);
                    idx.hash(h);
                    val.hash(h);
                    is_safe.hash(h);
                    is_na_safe.hash(h);
                    is_vector.hash(h);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    4u8.hash(h);
                    base.hash(h);
                    r.hash(h);
                    c.hash(h);
                    val.hash(h);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    5u8.hash(h);
                    base.hash(h);
                    i.hash(h);
                    j.hash(h);
                    k.hash(h);
                    val.hash(h);
                }
                Instr::UnsafeRBlock { code, .. } => {
                    6u8.hash(h);
                    code.hash(h);
                }
            }
        }

        let mut h = DefaultHasher::new();
        fn_ir.name.hash(&mut h);
        fn_ir.params.hash(&mut h);
        fn_ir.entry.hash(&mut h);
        fn_ir.body_head.hash(&mut h);
        fn_ir.blocks.len().hash(&mut h);
        fn_ir.values.len().hash(&mut h);
        for v in &fn_ir.values {
            v.kind.hash(&mut h);
            v.origin_var.hash(&mut h);
            v.phi_block.hash(&mut h);
        }
        for b in &fn_ir.blocks {
            b.term.hash(&mut h);
            b.instrs.len().hash(&mut h);
            for ins in &b.instrs {
                hash_instr(&mut h, ins);
            }
        }
        h.finish()
    }

    pub(crate) fn fn_ir_size(fn_ir: &FnIR) -> usize {
        let instrs: usize = fn_ir.blocks.iter().map(|b| b.instrs.len()).sum();
        fn_ir.values.len() + instrs
    }

    pub(crate) fn fn_opt_score(fn_ir: &FnIR) -> usize {
        let mut score = 0usize;
        for v in &fn_ir.values {
            score += match &v.kind {
                ValueKind::Binary { .. } => 3,
                ValueKind::Unary { .. } => 2,
                ValueKind::Call { .. } => 5,
                ValueKind::Intrinsic { .. } => 8,
                ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
                | ValueKind::Index3D { .. } => 4,
                ValueKind::Phi { .. } => 2,
                ValueKind::Len { .. } | ValueKind::Range { .. } => 2,
                _ => 1,
            };
        }
        for b in &fn_ir.blocks {
            if matches!(b.term, Terminator::If { .. }) {
                score += 8;
            }
            for ins in &b.instrs {
                score += match ins {
                    Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. } => 6,
                    Instr::Eval { .. } => 2,
                    Instr::Assign { .. } => 1,
                    Instr::UnsafeRBlock { .. } => 32,
                };
            }
        }
        score.saturating_add(Self::fn_ir_size(fn_ir) / 12)
    }

    pub(crate) fn fn_size_score(fn_ir: &FnIR) -> usize {
        let mut score = 0usize;
        for v in &fn_ir.values {
            score += match &v.kind {
                ValueKind::Phi { .. } => 5,
                ValueKind::Load { .. } => 2,
                ValueKind::Len { .. } | ValueKind::Range { .. } => 3,
                ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
                | ValueKind::Index3D { .. } => 4,
                ValueKind::Call { .. } | ValueKind::Intrinsic { .. } => 3,
                _ => 1,
            };
        }
        for block in &fn_ir.blocks {
            score += block.instrs.iter().fold(0usize, |acc, instr| {
                acc + match instr {
                    Instr::Assign { .. } => 4,
                    Instr::Eval { .. } => 2,
                    Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. } => 3,
                    Instr::UnsafeRBlock { .. } => 0,
                }
            });
            if matches!(block.term, Terminator::Goto(_) | Terminator::If { .. }) {
                score += 2;
            }
        }
        score
    }

    pub(crate) fn fn_risk_score(fn_ir: &FnIR) -> usize {
        let mut score = 0usize;
        if fn_ir.requires_conservative_optimization() {
            score += 1024;
        }
        for v in &fn_ir.values {
            if let ValueKind::Call { callee, .. } = &v.kind
                && !callee.starts_with("RR_")
            {
                score += 8;
            }
        }
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                if matches!(instr, Instr::UnsafeRBlock { .. }) {
                    score += 128;
                }
            }
        }
        score
    }

    pub(crate) fn fn_opportunity_score_v2(&self, fn_ir: &FnIR) -> usize {
        let base = Self::fn_opt_score(fn_ir);
        let size = Self::fn_size_score(fn_ir);
        let static_hot = Self::fn_static_hotness(fn_ir);
        let risk = Self::fn_risk_score(fn_ir);
        let mut loop_count = 0usize;
        let mut canonical_like = 0usize;
        let mut index_count = 0usize;
        for (bid, block) in fn_ir.blocks.iter().enumerate() {
            match block.term {
                Terminator::Goto(target) if target <= bid => {
                    loop_count += 1;
                    canonical_like += 1;
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } if (then_bb <= bid || else_bb <= bid) => {
                    loop_count += 1;
                }
                _ => {}
            }
        }
        for value in &fn_ir.values {
            if matches!(
                value.kind,
                ValueKind::Index1D { .. } | ValueKind::Index2D { .. } | ValueKind::Index3D { .. }
            ) {
                index_count += 1;
            }
        }

        let opt_level_bonus = if self.aggressive_opt_enabled() {
            static_hot
                .saturating_mul(2)
                .saturating_add(loop_count.saturating_mul(60))
                .saturating_add(canonical_like.saturating_mul(40))
                .saturating_add(index_count.saturating_mul(10))
        } else if self.size_opt_enabled() {
            size.saturating_mul(2)
                .saturating_add(base / 3)
                .saturating_sub(loop_count.saturating_mul(8))
        } else {
            static_hot
                .saturating_add(loop_count.saturating_mul(24))
                .saturating_add(index_count.saturating_mul(4))
        };

        base.saturating_add(opt_level_bonus)
            .saturating_sub(risk.min(base / 2 + 64))
    }

    pub(crate) fn adaptive_full_opt_limits(
        &self,
        all_fns: &FxHashMap<String, FnIR>,
        total_ir: usize,
        max_fn_ir: usize,
    ) -> (usize, usize) {
        let base_prog = self.configured_max_full_opt_ir();
        let base_fn = self.configured_max_full_opt_fn_ir();
        if self.size_opt_enabled() || !Self::adaptive_ir_budget_enabled() {
            return (base_prog, base_fn);
        }

        let fn_count = all_fns.len().max(1);
        let avg_ir = total_ir / fn_count;
        let mut branch_terms = 0usize;
        let mut call_like = 0usize;
        let mut mem_like = 0usize;
        let mut arith_like = 0usize;

        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            for blk in &fn_ir.blocks {
                if matches!(blk.term, Terminator::If { .. }) {
                    branch_terms += 1;
                }
                for ins in &blk.instrs {
                    if matches!(
                        ins,
                        Instr::StoreIndex1D { .. }
                            | Instr::StoreIndex2D { .. }
                            | Instr::StoreIndex3D { .. }
                    ) {
                        mem_like += 1;
                    }
                }
            }
            for v in &fn_ir.values {
                match &v.kind {
                    ValueKind::Binary { .. } | ValueKind::Unary { .. } => arith_like += 1,
                    ValueKind::Call { .. } | ValueKind::Intrinsic { .. } => call_like += 1,
                    ValueKind::Index1D { .. }
                    | ValueKind::Index2D { .. }
                    | ValueKind::Index3D { .. } => mem_like += 1,
                    _ => {}
                }
            }
        }

        let hot_ops = branch_terms
            .saturating_add(call_like)
            .saturating_add(mem_like)
            .saturating_add(arith_like);
        let hot_density_permille = hot_ops
            .saturating_mul(1000)
            .checked_div(total_ir)
            .unwrap_or(0);
        let fn_bonus = fn_count.saturating_mul(32).min(1800);
        let avg_bonus = avg_ir.saturating_mul(2).min(3200);
        let density_bonus = hot_density_permille.saturating_mul(3).min(1400);
        let max_skew_bonus = max_fn_ir.saturating_sub(avg_ir).min(1200);

        let program_upper = if self.aggressive_opt_enabled() {
            base_prog.max(24_000)
        } else {
            base_prog.max(12_000)
        };
        let fn_upper = if self.aggressive_opt_enabled() {
            base_fn.max(2_400)
        } else {
            base_fn.max(1_600)
        };

        let program_limit = base_prog
            .saturating_add(fn_bonus)
            .saturating_add(avg_bonus)
            .saturating_add(density_bonus)
            .saturating_add(max_skew_bonus / 4)
            .clamp(base_prog, program_upper);

        let fn_limit = base_fn
            .saturating_add(avg_ir.saturating_mul(2).min(500))
            .saturating_add(hot_density_permille.min(300))
            .clamp(base_fn, fn_upper);

        (program_limit, fn_limit)
    }

    pub(crate) fn fn_hot_weight(
        name: &str,
        fn_ir: &FnIR,
        profile_counts: &FxHashMap<String, usize>,
        max_profile_count: usize,
    ) -> usize {
        let static_hot = Self::fn_static_hotness(fn_ir).min(800);
        let static_weight = 1024usize.saturating_add(static_hot.saturating_mul(3));
        let profile_weight = match Self::profile_count_for(name, fn_ir, profile_counts) {
            Some(count) if max_profile_count > 0 => {
                1024usize.saturating_add(count.saturating_mul(3072) / max_profile_count)
            }
            _ => 1024usize,
        };
        static_weight
            .saturating_mul(profile_weight)
            .saturating_div(1024)
    }

    pub(crate) fn profile_count_for(
        name: &str,
        fn_ir: &FnIR,
        profile_counts: &FxHashMap<String, usize>,
    ) -> Option<usize> {
        profile_counts.get(name).copied().or_else(|| {
            fn_ir
                .user_name
                .as_deref()
                .and_then(|user_name| profile_counts.get(user_name).copied())
        })
    }

    pub(crate) fn build_opt_plan_with_profile(
        &self,
        all_fns: &FxHashMap<String, FnIR>,
        profile_counts: &FxHashMap<String, usize>,
    ) -> ProgramOptPlan {
        // Proof correspondence:
        // `ProgramOptPlanSoundness` fixes the reduced program-budget boundary
        // for this helper. The reduced model keeps the same three top-level
        // cases: under-budget all-safe selection, over-budget selective mode,
        // and fallback to the smallest eligible function when selective
        // selection would otherwise be empty.
        let total_ir: usize = all_fns.values().map(Self::fn_ir_size).sum();
        let max_fn_ir: usize = all_fns.values().map(Self::fn_ir_size).max().unwrap_or(0);
        let (program_limit, fn_limit) = self.adaptive_full_opt_limits(all_fns, total_ir, max_fn_ir);

        let mut selected = FxHashSet::default();
        let needs_budget = total_ir > program_limit || max_fn_ir > fn_limit;
        let ordered_names = Self::sorted_fn_names(all_fns);
        if !needs_budget {
            for name in &ordered_names {
                let Some(fn_ir) = all_fns.get(name) else {
                    continue;
                };
                if !fn_ir.requires_conservative_optimization() {
                    selected.insert(name.clone());
                }
            }
            return ProgramOptPlan {
                program_limit,
                fn_limit,
                total_ir,
                max_fn_ir,
                selective_mode: false,
                selected_functions: selected,
            };
        }

        let mut profiles = Vec::new();
        let soft_fn_limit = fn_limit.min(self.configured_heavy_pass_fn_ir().max(64));
        let max_profile_count = profile_counts.values().copied().max().unwrap_or(0);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            if fn_ir.requires_conservative_optimization() {
                continue;
            }
            let ir_size = Self::fn_ir_size(fn_ir);
            let score = Self::fn_opt_score(fn_ir);
            let opportunity_score = self.fn_opportunity_score_v2(fn_ir);
            let size_score = Self::fn_size_score(fn_ir);
            let risk_score = Self::fn_risk_score(fn_ir);
            let hot_weight = Self::fn_hot_weight(name, fn_ir, profile_counts, max_profile_count);
            let profile_count = Self::profile_count_for(name, fn_ir, profile_counts).unwrap_or(0);
            let weighted_score = opportunity_score
                .saturating_mul(hot_weight)
                .saturating_div(1024)
                .saturating_add(if self.size_opt_enabled() {
                    size_score
                } else {
                    0
                })
                .saturating_sub(risk_score.min(opportunity_score / 2 + 64));
            let density = weighted_score.saturating_mul(1024) / ir_size.max(1);
            profiles.push(FunctionBudgetProfile {
                name: name.clone(),
                ir_size,
                score,
                opportunity_score,
                size_score,
                risk_score,
                weighted_score,
                density,
                hot_weight,
                profile_count,
                within_fn_limit: ir_size <= soft_fn_limit,
            });
        }

        profiles.sort_by(|a, b| {
            b.within_fn_limit
                .cmp(&a.within_fn_limit)
                .then_with(|| b.density.cmp(&a.density))
                .then_with(|| b.hot_weight.cmp(&a.hot_weight))
                .then_with(|| b.profile_count.cmp(&a.profile_count))
                .then_with(|| b.weighted_score.cmp(&a.weighted_score))
                .then_with(|| b.opportunity_score.cmp(&a.opportunity_score))
                .then_with(|| b.size_score.cmp(&a.size_score))
                .then_with(|| a.risk_score.cmp(&b.risk_score))
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| a.ir_size.cmp(&b.ir_size))
                .then_with(|| a.name.cmp(&b.name))
        });

        let mut used_budget = 0usize;
        for p in &profiles {
            if !p.within_fn_limit {
                continue;
            }
            if used_budget.saturating_add(p.ir_size) > program_limit {
                continue;
            }
            used_budget = used_budget.saturating_add(p.ir_size);
            selected.insert(p.name.clone());
        }

        if selected.is_empty()
            && let Some(fallback) = profiles
                .iter()
                .filter(|p| p.ir_size <= soft_fn_limit.saturating_mul(2))
                .min_by_key(|p| p.ir_size)
                .or_else(|| profiles.iter().min_by_key(|p| p.ir_size))
        {
            selected.insert(fallback.name.clone());
        }

        ProgramOptPlan {
            program_limit,
            fn_limit,
            total_ir,
            max_fn_ir,
            selective_mode: true,
            selected_functions: selected,
        }
    }

    pub(crate) fn build_opt_plan(&self, all_fns: &FxHashMap<String, FnIR>) -> ProgramOptPlan {
        let profile_counts = Self::load_hot_profile_counts();
        self.build_opt_plan_with_profile(all_fns, &profile_counts)
    }

    pub(crate) fn sorted_fn_names(all_fns: &FxHashMap<String, FnIR>) -> Vec<String> {
        let mut names: Vec<String> = all_fns.keys().cloned().collect();
        names.sort();
        names
    }

    pub(crate) fn sorted_names(set: &FxHashSet<String>) -> Vec<String> {
        let mut names: Vec<String> = set.iter().cloned().collect();
        names.sort();
        names
    }
}
