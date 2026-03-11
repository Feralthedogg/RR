use super::*;

impl TachyonEngine {
    pub(super) fn load_hot_profile_counts() -> FxHashMap<String, usize> {
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

    pub(super) fn fn_static_hotness(fn_ir: &FnIR) -> usize {
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
                Terminator::Goto(t) => {
                    if t <= bid {
                        loops += 1;
                    }
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

    pub(super) fn fn_ir_fingerprint(fn_ir: &FnIR) -> u64 {
        fn hash_instr(h: &mut DefaultHasher, instr: &Instr) {
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

    pub(super) fn fn_ir_size(fn_ir: &FnIR) -> usize {
        let instrs: usize = fn_ir.blocks.iter().map(|b| b.instrs.len()).sum();
        fn_ir.values.len() + instrs
    }

    pub(super) fn fn_opt_score(fn_ir: &FnIR) -> usize {
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
                };
            }
        }
        score.saturating_add(Self::fn_ir_size(fn_ir) / 12)
    }

    pub(super) fn adaptive_full_opt_limits(
        all_fns: &FxHashMap<String, FnIR>,
        total_ir: usize,
        max_fn_ir: usize,
    ) -> (usize, usize) {
        let base_prog = Self::max_full_opt_ir();
        let base_fn = Self::max_full_opt_fn_ir();
        if !Self::adaptive_ir_budget_enabled() {
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
        let hot_density_permille = if total_ir == 0 {
            0
        } else {
            hot_ops.saturating_mul(1000) / total_ir
        };
        let fn_bonus = fn_count.saturating_mul(32).min(1800);
        let avg_bonus = avg_ir.saturating_mul(2).min(3200);
        let density_bonus = hot_density_permille.saturating_mul(3).min(1400);
        let max_skew_bonus = max_fn_ir.saturating_sub(avg_ir).min(1200);

        let program_upper = base_prog.max(12_000);
        let fn_upper = base_fn.max(1_600);

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

    pub(super) fn fn_hot_weight(
        name: &str,
        fn_ir: &FnIR,
        profile_counts: &FxHashMap<String, usize>,
        max_profile_count: usize,
    ) -> usize {
        let static_hot = Self::fn_static_hotness(fn_ir).min(800);
        let static_weight = 1024usize.saturating_add(static_hot.saturating_mul(3));
        let profile_weight = match profile_counts.get(name).copied() {
            Some(count) if max_profile_count > 0 => {
                1024usize.saturating_add(count.saturating_mul(3072) / max_profile_count)
            }
            _ => 1024usize,
        };
        static_weight
            .saturating_mul(profile_weight)
            .saturating_div(1024)
    }

    pub(super) fn build_opt_plan_with_profile(
        all_fns: &FxHashMap<String, FnIR>,
        profile_counts: &FxHashMap<String, usize>,
    ) -> ProgramOptPlan {
        let total_ir: usize = all_fns.values().map(Self::fn_ir_size).sum();
        let max_fn_ir: usize = all_fns.values().map(Self::fn_ir_size).max().unwrap_or(0);
        let (program_limit, fn_limit) =
            Self::adaptive_full_opt_limits(all_fns, total_ir, max_fn_ir);

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
        let soft_fn_limit = fn_limit.min(Self::heavy_pass_fn_ir().max(64));
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
            let hot_weight = Self::fn_hot_weight(name, fn_ir, profile_counts, max_profile_count);
            let weighted_score = score.saturating_mul(hot_weight).saturating_div(1024);
            let density = weighted_score.saturating_mul(1024) / ir_size.max(1);
            profiles.push(FunctionBudgetProfile {
                name: name.clone(),
                ir_size,
                score,
                weighted_score,
                density,
                hot_weight,
                within_fn_limit: ir_size <= soft_fn_limit,
            });
        }

        profiles.sort_by(|a, b| {
            b.within_fn_limit
                .cmp(&a.within_fn_limit)
                .then_with(|| b.density.cmp(&a.density))
                .then_with(|| b.hot_weight.cmp(&a.hot_weight))
                .then_with(|| b.weighted_score.cmp(&a.weighted_score))
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

    pub(super) fn build_opt_plan(all_fns: &FxHashMap<String, FnIR>) -> ProgramOptPlan {
        let profile_counts = Self::load_hot_profile_counts();
        Self::build_opt_plan_with_profile(all_fns, &profile_counts)
    }

    pub(super) fn sorted_fn_names(all_fns: &FxHashMap<String, FnIR>) -> Vec<String> {
        let mut names: Vec<String> = all_fns.keys().cloned().collect();
        names.sort();
        names
    }

    pub(super) fn sorted_names(set: &FxHashSet<String>) -> Vec<String> {
        let mut names: Vec<String> = set.iter().cloned().collect();
        names.sort();
        names
    }
}
