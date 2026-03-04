use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::time::Instant;

pub mod bce;
pub mod de_ssa;
pub mod fresh_alloc;
pub mod gvn;
pub mod inline;
pub mod intrinsics;
pub mod licm;
pub mod loop_analysis;
pub mod loop_opt;
pub mod parallel_copy;
pub mod sccp;
pub mod simplify;
pub mod tco;
pub mod type_specialize;
pub mod v_opt;

pub struct TachyonEngine;

#[derive(Debug, Default, Clone, Copy)]
pub struct TachyonPulseStats {
    pub vectorized: usize,
    pub reduced: usize,
    pub simplified_loops: usize,
    pub tco_hits: usize,
    pub sccp_hits: usize,
    pub intrinsics_hits: usize,
    pub gvn_hits: usize,
    pub licm_hits: usize,
    pub fresh_alloc_hits: usize,
    pub bce_hits: usize,
    pub simplify_hits: usize,
    pub dce_hits: usize,
    pub inline_rounds: usize,
    pub inline_cleanup_hits: usize,
    pub de_ssa_hits: usize,
    pub always_tier_functions: usize,
    pub optimized_functions: usize,
    pub skipped_functions: usize,
    pub full_opt_ir_limit: usize,
    pub full_opt_fn_limit: usize,
    pub total_program_ir: usize,
    pub max_function_ir: usize,
    pub selective_budget_mode: bool,
}

impl TachyonPulseStats {
    fn accumulate(&mut self, other: Self) {
        self.vectorized += other.vectorized;
        self.reduced += other.reduced;
        self.simplified_loops += other.simplified_loops;
        self.tco_hits += other.tco_hits;
        self.sccp_hits += other.sccp_hits;
        self.intrinsics_hits += other.intrinsics_hits;
        self.gvn_hits += other.gvn_hits;
        self.licm_hits += other.licm_hits;
        self.fresh_alloc_hits += other.fresh_alloc_hits;
        self.bce_hits += other.bce_hits;
        self.simplify_hits += other.simplify_hits;
        self.dce_hits += other.dce_hits;
        self.inline_rounds += other.inline_rounds;
        self.inline_cleanup_hits += other.inline_cleanup_hits;
        self.de_ssa_hits += other.de_ssa_hits;
        self.always_tier_functions += other.always_tier_functions;
        self.optimized_functions += other.optimized_functions;
        self.skipped_functions += other.skipped_functions;
    }
}

#[derive(Debug, Clone)]
struct FunctionBudgetProfile {
    name: String,
    ir_size: usize,
    score: usize,
    weighted_score: usize,
    density: usize,
    hot_weight: usize,
    within_fn_limit: bool,
}

#[derive(Debug, Clone)]
struct ProgramOptPlan {
    program_limit: usize,
    fn_limit: usize,
    total_ir: usize,
    max_fn_ir: usize,
    selective_mode: bool,
    selected_functions: FxHashSet<String>,
}

// Backward compatibility alias for older call sites.
pub type MirOptimizer = TachyonEngine;

impl TachyonEngine {
    pub fn new() -> Self {
        Self
    }

    fn verify_or_panic(fn_ir: &FnIR, stage: &str) {
        if let Err(e) = crate::mir::verify::verify_ir(fn_ir) {
            panic!(
                "MIR Verification Failed at {}: {}\nFunction: {}",
                stage, e, fn_ir.name
            );
        }
    }

    fn verify_or_reject(fn_ir: &mut FnIR, stage: &str) -> bool {
        match crate::mir::verify::verify_ir(fn_ir) {
            Ok(()) => true,
            Err(e) => {
                fn_ir.unsupported_dynamic = true;
                let reason = format!("invalid MIR at {}: {}", stage, e);
                if !fn_ir.fallback_reasons.iter().any(|r| r == &reason) {
                    fn_ir.fallback_reasons.push(reason);
                }
                false
            }
        }
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

    fn env_usize(key: &str, default_v: usize) -> usize {
        env::var(key)
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(default_v)
    }

    fn verify_each_pass() -> bool {
        Self::env_bool("RR_VERIFY_EACH_PASS", false)
    }

    fn maybe_verify(fn_ir: &FnIR, stage: &str) {
        if Self::verify_each_pass() {
            Self::verify_or_panic(fn_ir, stage);
        }
    }

    fn max_opt_iterations() -> usize {
        Self::env_usize("RR_OPT_MAX_ITERS", 24)
    }

    fn max_inline_rounds() -> usize {
        Self::env_usize("RR_INLINE_MAX_ROUNDS", 3)
    }

    fn max_full_opt_ir() -> usize {
        Self::env_usize("RR_MAX_FULL_OPT_IR", 2500)
    }

    fn max_full_opt_fn_ir() -> usize {
        Self::env_usize("RR_MAX_FULL_OPT_FN_IR", 900)
    }

    fn adaptive_ir_budget_enabled() -> bool {
        Self::env_bool("RR_ADAPTIVE_IR_BUDGET", false)
    }

    fn selective_budget_enabled() -> bool {
        Self::env_bool("RR_SELECTIVE_OPT_BUDGET", false) || Self::adaptive_ir_budget_enabled()
    }

    fn heavy_pass_fn_ir() -> usize {
        Self::env_usize("RR_HEAVY_PASS_FN_IR", 650)
    }

    fn max_fn_opt_ms() -> u128 {
        Self::env_usize("RR_MAX_FN_OPT_MS", 250) as u128
    }

    fn always_tier_max_iters() -> usize {
        Self::env_usize("RR_ALWAYS_TIER_ITERS", 2).clamp(1, 6)
    }

    fn licm_enabled() -> bool {
        Self::env_bool("RR_ENABLE_LICM", false)
    }

    fn gvn_enabled() -> bool {
        Self::env_bool("RR_ENABLE_GVN", false)
    }

    fn profile_use_path() -> Option<String> {
        env::var("RR_PROFILE_USE").ok().and_then(|v| {
            let p = v.trim();
            if p.is_empty() {
                None
            } else {
                Some(p.to_string())
            }
        })
    }

    fn load_hot_profile_counts() -> FxHashMap<String, usize> {
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

    fn fn_static_hotness(fn_ir: &FnIR) -> usize {
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
                if matches!(ins, Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. }) {
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

    fn fn_ir_fingerprint(fn_ir: &FnIR) -> u64 {
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

    fn fn_ir_size(fn_ir: &FnIR) -> usize {
        let instrs: usize = fn_ir.blocks.iter().map(|b| b.instrs.len()).sum();
        fn_ir.values.len() + instrs
    }

    fn fn_opt_score(fn_ir: &FnIR) -> usize {
        let mut score = 0usize;
        for v in &fn_ir.values {
            score += match &v.kind {
                ValueKind::Binary { .. } => 3,
                ValueKind::Unary { .. } => 2,
                ValueKind::Call { .. } => 5,
                ValueKind::Intrinsic { .. } => 8,
                ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => 4,
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
                    Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => 6,
                    Instr::Eval { .. } => 2,
                    Instr::Assign { .. } => 1,
                };
            }
        }
        // Mild size-bias so tiny helper functions don't always dominate ranking.
        score.saturating_add(Self::fn_ir_size(fn_ir) / 12)
    }

    fn adaptive_full_opt_limits(
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

        for fn_ir in all_fns.values() {
            for blk in &fn_ir.blocks {
                if matches!(blk.term, Terminator::If { .. }) {
                    branch_terms += 1;
                }
                for ins in &blk.instrs {
                    if matches!(ins, Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. }) {
                        mem_like += 1;
                    }
                }
            }
            for v in &fn_ir.values {
                match &v.kind {
                    ValueKind::Binary { .. } | ValueKind::Unary { .. } => arith_like += 1,
                    ValueKind::Call { .. } | ValueKind::Intrinsic { .. } => call_like += 1,
                    ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => mem_like += 1,
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

    fn fn_hot_weight(
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

    fn build_opt_plan_with_profile(
        all_fns: &FxHashMap<String, FnIR>,
        profile_counts: &FxHashMap<String, usize>,
    ) -> ProgramOptPlan {
        let total_ir: usize = all_fns.values().map(Self::fn_ir_size).sum();
        let max_fn_ir: usize = all_fns.values().map(Self::fn_ir_size).max().unwrap_or(0);
        let (program_limit, fn_limit) =
            Self::adaptive_full_opt_limits(all_fns, total_ir, max_fn_ir);

        let mut selected = FxHashSet::default();
        let needs_budget = total_ir > program_limit || max_fn_ir > fn_limit;
        if !needs_budget {
            for (name, fn_ir) in all_fns {
                if !fn_ir.unsupported_dynamic {
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
        for (name, fn_ir) in all_fns {
            if fn_ir.unsupported_dynamic {
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

        if selected.is_empty() {
            if let Some(fallback) = profiles
                .iter()
                .filter(|p| p.ir_size <= soft_fn_limit.saturating_mul(2))
                .min_by_key(|p| p.ir_size)
                .or_else(|| profiles.iter().min_by_key(|p| p.ir_size))
            {
                selected.insert(fallback.name.clone());
            }
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

    fn build_opt_plan(all_fns: &FxHashMap<String, FnIR>) -> ProgramOptPlan {
        let profile_counts = Self::load_hot_profile_counts();
        Self::build_opt_plan_with_profile(all_fns, &profile_counts)
    }

    // Required lowering-to-codegen stabilization passes.
    // This must run even in O0, because codegen cannot emit Phi.
    pub fn stabilize_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        for (_, fn_ir) in all_fns.iter_mut() {
            if !Self::verify_or_reject(fn_ir, "PrepareForCodegen/Start") {
                continue;
            }
            let _ = de_ssa::run(fn_ir);
            // Keep this lightweight but convergent to avoid dead temp noise after De-SSA.
            // Hybrid fallback functions skip cleanup to preserve dynamic semantics.
            if !fn_ir.unsupported_dynamic {
                let mut changed = true;
                let mut guard = 0;
                while changed && guard < 8 {
                    guard += 1;
                    changed = false;
                    changed |= self.simplify_cfg(fn_ir);
                    changed |= self.dce(fn_ir);
                }
            }
            let _ = Self::verify_or_reject(fn_ir, "PrepareForCodegen/End");
        }
    }

    fn run_always_tier_with_stats(&self, fn_ir: &mut FnIR) -> TachyonPulseStats {
        let mut stats = TachyonPulseStats::default();
        if fn_ir.unsupported_dynamic {
            return stats;
        }
        if !Self::verify_or_reject(fn_ir, "AlwaysTier/Start") {
            return stats;
        }

        stats.always_tier_functions = 1;
        let mut changed = true;
        let mut iter = 0usize;
        let max_iters = Self::always_tier_max_iters();
        let mut seen = FxHashSet::default();
        seen.insert(Self::fn_ir_fingerprint(fn_ir));
        let run_light_sccp = Self::fn_ir_size(fn_ir) <= Self::heavy_pass_fn_ir().saturating_mul(2);

        while changed && iter < max_iters {
            iter += 1;
            changed = false;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);

            let sc_changed = self.simplify_cfg(fn_ir);
            if sc_changed {
                stats.simplify_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/SimplifyCFG");

            if run_light_sccp {
                let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
                if sccp_changed {
                    stats.sccp_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/SCCP");

                let intr_changed = intrinsics::optimize(fn_ir);
                if intr_changed {
                    stats.intrinsics_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/Intrinsics");
            }

            let dce_changed = self.dce(fn_ir);
            if dce_changed {
                stats.dce_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/DCE");

            let after_hash = Self::fn_ir_fingerprint(fn_ir);
            if after_hash == before_hash {
                break;
            }
            if !seen.insert(after_hash) {
                break;
            }
        }

        let _ = Self::verify_or_reject(fn_ir, "AlwaysTier/End");
        stats
    }

    pub fn run_program(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        let _ = self.run_program_with_stats(all_fns);
    }

    pub fn run_program_with_stats(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
    ) -> TachyonPulseStats {
        /*
        // 1. Clean
        simplify::SimplifyCFG::new().optimize(fn_ir);

        loop {
             let mut changed = false;

             // 2. Sccp
             // changed |= sccp::MirSccp::new().optimize(fn_ir);

             // 3. LICM
             // changed |= licm::MirLicm::new().optimize(fn_ir);

             // 4. Clean again
             changed |= simplify::SimplifyCFG::new().optimize(fn_ir);

             if !changed { break; }
        }

        // TCO
        tco::optimize(fn_ir);

        // Final polish (DCE/cleanup)
        simplify::SimplifyCFG::new().optimize(fn_ir);
        */

        let mut stats = TachyonPulseStats::default();
        let plan = Self::build_opt_plan(all_fns);
        let selective_enabled = Self::selective_budget_enabled();
        let run_heavy_tier = !plan.selective_mode || selective_enabled;
        let run_full_inline_tier = run_heavy_tier;
        stats.total_program_ir = plan.total_ir;
        stats.max_function_ir = plan.max_fn_ir;
        stats.full_opt_ir_limit = plan.program_limit;
        stats.full_opt_fn_limit = plan.fn_limit;
        stats.selective_budget_mode = plan.selective_mode && selective_enabled;

        // Tier A (always): lightweight canonicalization for every safe function.
        for (_, fn_ir) in all_fns.iter_mut() {
            let s = self.run_always_tier_with_stats(fn_ir);
            stats.accumulate(s);
        }

        let heavy_targets_exist =
            run_heavy_tier && (!plan.selective_mode || !plan.selected_functions.is_empty());
        let callmap_user_whitelist = if heavy_targets_exist {
            Self::collect_callmap_user_whitelist(all_fns)
        } else {
            FxHashSet::default()
        };

        // Tier B (selective-heavy): optimize full pass pipeline only for selected functions.
        for (name, fn_ir) in all_fns.iter_mut() {
            if fn_ir.unsupported_dynamic {
                stats.skipped_functions += 1;
                let _ = Self::verify_or_reject(fn_ir, "SkipOpt/UnsupportedDynamic");
                continue;
            }
            let selected = !plan.selective_mode || plan.selected_functions.contains(name);
            if !run_heavy_tier || !selected {
                stats.skipped_functions += 1;
                let reason = if !run_heavy_tier {
                    "SkipOpt/HeavyTierDisabled"
                } else {
                    "SkipOpt/Budget"
                };
                let _ = Self::verify_or_reject(fn_ir, reason);
                continue;
            }
            stats.optimized_functions += 1;
            let s = self.run_function_with_stats(fn_ir, &callmap_user_whitelist);
            stats.accumulate(s);
        }

        // Tier C (full-program): bounded inter-procedural inlining.
        if run_full_inline_tier {
            let mut changed = true;
            let mut iter = 0;
            let inliner = inline::MirInliner::new();
            let hot_filter = if plan.selective_mode {
                Some(&plan.selected_functions)
            } else {
                None
            };
            while changed && iter < Self::max_inline_rounds() {
                changed = false;
                iter += 1;
                // Inlining needs access to the whole map
                let local_changed = inliner.optimize_with_hot_filter(all_fns, hot_filter);
                for (_, fn_ir) in all_fns.iter() {
                    Self::maybe_verify(fn_ir, "After Inlining");
                }
                if local_changed {
                    stats.inline_rounds += 1;
                    changed = true;
                    // Re-optimize each function if inlining happened
                    for (_, fn_ir) in all_fns.iter_mut() {
                        if fn_ir.unsupported_dynamic {
                            Self::maybe_verify(
                                fn_ir,
                                "After Inline Cleanup (Skipped: UnsupportedDynamic)",
                            );
                            continue;
                        }
                        // Run lightweight cleanup after inlining.
                        let inline_sc_changed = self.simplify_cfg(fn_ir);
                        let inline_dce_changed = self.dce(fn_ir);
                        if inline_sc_changed || inline_dce_changed {
                            stats.inline_cleanup_hits += 1;
                        }
                        if inline_sc_changed {
                            stats.simplify_hits += 1;
                        }
                        if inline_dce_changed {
                            stats.dce_hits += 1;
                        }
                        Self::maybe_verify(fn_ir, "After Inline Cleanup");
                    }
                }
            }
        }

        // 3. De-SSA (Phi elimination via parallel copy) before codegen.
        for (_, fn_ir) in all_fns.iter_mut() {
            let de_ssa_changed = de_ssa::run(fn_ir);
            if de_ssa_changed {
                stats.de_ssa_hits += 1;
            }
            // Cleanup after De-SSA to drop dead temps and unreachable blocks.
            if !fn_ir.unsupported_dynamic {
                let sc_changed = self.simplify_cfg(fn_ir);
                let dce_changed = self.dce(fn_ir);
                if sc_changed {
                    stats.simplify_hits += 1;
                }
                if dce_changed {
                    stats.dce_hits += 1;
                }
            }
            let _ = Self::verify_or_reject(fn_ir, "After De-SSA");
        }
        stats
    }

    pub fn run_function(&self, fn_ir: &mut FnIR) {
        let empty = FxHashSet::default();
        let _ = self.run_function_with_stats(fn_ir, &empty);
    }

    pub fn run_function_with_stats(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        let mut stats = TachyonPulseStats::default();
        let mut changed = true;
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        let mut iterations = 0;
        let mut seen_hashes = FxHashSet::default();
        let start_time = Instant::now();
        let fn_ir_size = Self::fn_ir_size(fn_ir);
        let max_iters = if fn_ir_size > 2200 {
            4
        } else if fn_ir_size > 1400 {
            8
        } else if fn_ir_size > 900 {
            12
        } else {
            Self::max_opt_iterations()
        };
        let heavy_pass_budgeted = fn_ir_size > Self::heavy_pass_fn_ir();

        // Initial Verify
        if !Self::verify_or_reject(fn_ir, "Start") {
            return stats;
        }
        seen_hashes.insert(Self::fn_ir_fingerprint(fn_ir));

        while changed && iterations < max_iters {
            if start_time.elapsed().as_millis() > Self::max_fn_opt_ms() {
                break;
            }
            changed = false;
            iterations += 1;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);

            // 1. Structural Transformations
            let mut pass_changed = false;
            let run_heavy_structural = !(heavy_pass_budgeted && iterations > 1);

            if run_heavy_structural {
                let type_spec_changed = type_specialize::optimize(fn_ir);
                Self::maybe_verify(fn_ir, "After TypeSpecialize");
                pass_changed |= type_spec_changed;

                // Vectorization
                let v_stats =
                    v_opt::optimize_with_stats_with_whitelist(fn_ir, callmap_user_whitelist);
                stats.vectorized += v_stats.vectorized;
                stats.reduced += v_stats.reduced;
                let v_changed = v_stats.changed();
                Self::maybe_verify(fn_ir, "After Vectorization");
                pass_changed |= v_changed;

                let type_spec_post_vec = type_specialize::optimize(fn_ir);
                Self::maybe_verify(fn_ir, "After TypeSpecialize(PostVec)");
                pass_changed |= type_spec_post_vec;

                // TCO
                let tco_changed = tco::optimize(fn_ir);
                if tco_changed {
                    stats.tco_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After TCO");
                pass_changed |= tco_changed;
            }

            if pass_changed {
                changed = true;
                // Intensive cleanup after structural changes
                let sc_changed = self.simplify_cfg(fn_ir);
                if sc_changed {
                    stats.simplify_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After Structural SimplifyCFG");
                let dce_changed = self.dce(fn_ir);
                if dce_changed {
                    stats.dce_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After Structural DCE");
                changed |= sc_changed || dce_changed;
            }

            // 2. Standard optimization passes
            let sc_changed = self.simplify_cfg(fn_ir);
            if sc_changed {
                stats.simplify_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After SimplifyCFG");
            changed |= sc_changed;

            let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
            if sccp_changed {
                stats.sccp_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After SCCP");
            changed |= sccp_changed;

            let intr_changed = intrinsics::optimize(fn_ir);
            if intr_changed {
                stats.intrinsics_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After Intrinsics");
            changed |= intr_changed;

            let gvn_changed = if Self::gvn_enabled() {
                let c = gvn::optimize(fn_ir);
                if c {
                    stats.gvn_hits += 1;
                }
                c
            } else {
                false
            };
            Self::maybe_verify(fn_ir, "After GVN");
            changed |= gvn_changed;

            let simplify_changed = simplify::optimize(fn_ir);
            if simplify_changed {
                stats.simplify_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After Simplify");
            changed |= simplify_changed;

            let dce_changed = self.dce(fn_ir);
            if dce_changed {
                stats.dce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After DCE");
            changed |= dce_changed;

            if !(heavy_pass_budgeted && iterations > 1) {
                let loop_changed_count = loop_opt.optimize_with_count(fn_ir);
                stats.simplified_loops += loop_changed_count;
                let loop_changed = loop_changed_count > 0;
                Self::maybe_verify(fn_ir, "After LoopOpt");
                changed |= loop_changed;

                let licm_changed = if Self::licm_enabled() {
                    let c = licm::MirLicm::new().optimize(fn_ir);
                    if c {
                        stats.licm_hits += 1;
                    }
                    c
                } else {
                    false
                };
                Self::maybe_verify(fn_ir, "After LICM");
                changed |= licm_changed;

                let fresh_changed = fresh_alloc::optimize(fn_ir);
                if fresh_changed {
                    stats.fresh_alloc_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After FreshAlloc");
                changed |= fresh_changed;

                let bce_changed = bce::optimize(fn_ir);
                if bce_changed {
                    stats.bce_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After BCE");
                changed |= bce_changed;
            }
            // check_elimination remains disabled.

            let after_hash = Self::fn_ir_fingerprint(fn_ir);
            if after_hash == before_hash {
                break;
            }
            if !seen_hashes.insert(after_hash) {
                // Degenerate oscillation guard.
                break;
            }
            changed |= after_hash != before_hash;
        }

        // Final polishing pass
        let mut polishing = true;
        let mut polish_guard = 0usize;
        let mut polish_seen: FxHashSet<u64> = FxHashSet::default();
        while polishing && polish_guard < 16 {
            if start_time.elapsed().as_millis() > Self::max_fn_opt_ms() {
                break;
            }
            polish_guard += 1;
            let before_polish = Self::fn_ir_fingerprint(fn_ir);
            polishing = self.simplify_cfg(fn_ir);
            if polishing {
                stats.simplify_hits += 1;
            }
            let dce_changed = self.dce(fn_ir);
            if dce_changed {
                stats.dce_hits += 1;
            }
            polishing |= dce_changed;
            let after_polish = Self::fn_ir_fingerprint(fn_ir);
            if after_polish == before_polish || !polish_seen.insert(after_polish) {
                break;
            }
        }
        let _ = Self::verify_or_reject(fn_ir, "End");
        stats
    }

    // Backward-compat wrappers.
    pub fn prepare_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.stabilize_for_codegen(all_fns);
    }

    pub fn optimize_all(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.run_program(all_fns);
    }

    pub fn optimize_function(&self, fn_ir: &mut FnIR) {
        self.run_function(fn_ir);
    }

    fn collect_callmap_user_whitelist(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
        let mut whitelist: FxHashSet<String> = FxHashSet::default();
        let mut changed = true;
        while changed {
            changed = false;
            for (name, fn_ir) in all_fns {
                if whitelist.contains(name) {
                    continue;
                }
                if Self::is_callmap_vector_safe_user_fn(name, fn_ir, &whitelist) {
                    whitelist.insert(name.clone());
                    changed = true;
                }
            }
        }
        whitelist
    }

    fn is_callmap_vector_safe_user_fn(
        name: &str,
        fn_ir: &FnIR,
        user_whitelist: &FxHashSet<String>,
    ) -> bool {
        if fn_ir.unsupported_dynamic {
            return false;
        }
        if name.starts_with("Sym_top_") {
            return false;
        }

        // Restrict to expression-like functions: no stores, no explicit eval, no branching.
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. } => return false,
                    Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => return false,
                }
            }
            match bb.term {
                Terminator::Goto(_) | Terminator::Return(_) | Terminator::Unreachable => {}
                Terminator::If { .. } => return false,
            }
        }

        // All returns must be value-returns and vector-safe expression trees.
        let mut saw_return = false;
        for bb in &fn_ir.blocks {
            if let Terminator::Return(ret) = bb.term {
                let Some(ret_vid) = ret else { return false };
                saw_return = true;
                if !Self::is_vector_safe_user_expr(
                    fn_ir,
                    ret_vid,
                    user_whitelist,
                    &mut FxHashSet::default(),
                ) {
                    return false;
                }
            }
        }
        saw_return
    }

    fn is_vector_safe_user_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        user_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let vid = Self::resolve_load_alias_value(fn_ir, vid);
        if !seen.insert(vid) {
            return true;
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => true,
            ValueKind::Unary { rhs, .. } => {
                Self::is_vector_safe_user_expr(fn_ir, *rhs, user_whitelist, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                Self::is_vector_safe_user_expr(fn_ir, *lhs, user_whitelist, seen)
                    && Self::is_vector_safe_user_expr(fn_ir, *rhs, user_whitelist, seen)
            }
            ValueKind::Call { callee, args, .. } => {
                (v_opt::is_builtin_vector_safe_call(callee, args.len())
                    || user_whitelist.contains(callee))
                    && args
                        .iter()
                        .all(|a| Self::is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| Self::is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen)),
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| Self::is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                Self::is_vector_safe_user_expr(fn_ir, *base, user_whitelist, seen)
            }
            ValueKind::Range { start, end } => {
                Self::is_vector_safe_user_expr(fn_ir, *start, user_whitelist, seen)
                    && Self::is_vector_safe_user_expr(fn_ir, *end, user_whitelist, seen)
            }
            ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => false,
        }
    }

    fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
        fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
            let mut src: Option<ValueId> = None;
            for bb in &fn_ir.blocks {
                for ins in &bb.instrs {
                    let Instr::Assign { dst, src: s, .. } = ins else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    match src {
                        None => src = Some(*s),
                        Some(prev) if prev == *s => {}
                        Some(_) => return None,
                    }
                }
            }
            src
        }

        let mut cur = vid;
        let mut seen = FxHashSet::default();
        while seen.insert(cur) {
            match &fn_ir.values[cur].kind {
                ValueKind::Load { var } => {
                    if let Some(src) = unique_assign_source(fn_ir, var) {
                        cur = src;
                        continue;
                    }
                }
                _ => {}
            }
            break;
        }
        cur
    }

    fn simplify_cfg(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        // 1. Identify reachable blocks
        let mut reachable = FxHashSet::default();
        let mut queue = vec![fn_ir.entry];
        reachable.insert(fn_ir.entry);

        let mut head = 0;
        while head < queue.len() {
            let bid = queue[head];
            head += 1;

            if let Some(blk) = fn_ir.blocks.get(bid) {
                match &blk.term {
                    Terminator::Goto(target) => {
                        if reachable.insert(*target) {
                            queue.push(*target);
                        }
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

        // 2. Clear out unreachable blocks
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

    fn dce(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        // 1. Mark used values
        let mut used = FxHashSet::default();

        // Final values used in terminators
        for blk in &fn_ir.blocks {
            match &blk.term {
                Terminator::If { cond, .. } => {
                    used.insert(*cond);
                }
                Terminator::Return(val) => {
                    if let Some(id) = val {
                        used.insert(*id);
                    }
                }
                _ => {}
            }
        }

        // Instructions with side effects are roots
        for blk in &fn_ir.blocks {
            for instr in &blk.instrs {
                if self.has_side_effect_instr(instr, &fn_ir.values) {
                    match instr {
                        Instr::Assign { src, .. } => {
                            used.insert(*src);
                        }
                        Instr::Eval { val, .. } => {
                            used.insert(*val);
                        }
                        Instr::StoreIndex1D { base, idx, val, .. } => {
                            used.insert(*base);
                            used.insert(*idx);
                            used.insert(*val);
                        }
                        Instr::StoreIndex2D {
                            base, r, c, val, ..
                        } => {
                            used.insert(*base);
                            used.insert(*r);
                            used.insert(*c);
                            used.insert(*val);
                        }
                    }
                }
            }
        }

        // 2. Propagate usage (transitive closure)
        let mut worklist: Vec<ValueId> = used.iter().cloned().collect();
        while let Some(vid) = worklist.pop() {
            let val = &fn_ir.values[vid];
            match &val.kind {
                ValueKind::Binary { lhs, rhs, .. } => {
                    if used.insert(*lhs) {
                        worklist.push(*lhs);
                    }
                    if used.insert(*rhs) {
                        worklist.push(*rhs);
                    }
                }
                ValueKind::Unary { rhs, .. } => {
                    if used.insert(*rhs) {
                        worklist.push(*rhs);
                    }
                }
                ValueKind::Call { args, .. } => {
                    for a in args {
                        if used.insert(*a) {
                            worklist.push(*a);
                        }
                    }
                }
                ValueKind::Phi { args } => {
                    for (a, _) in args {
                        if used.insert(*a) {
                            worklist.push(*a);
                        }
                    }
                }
                ValueKind::Index1D { base, idx, .. } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                    if used.insert(*idx) {
                        worklist.push(*idx);
                    }
                }
                ValueKind::Index2D { base, r, c } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                    if used.insert(*r) {
                        worklist.push(*r);
                    }
                    if used.insert(*c) {
                        worklist.push(*c);
                    }
                }
                ValueKind::Len { base } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                }
                ValueKind::Indices { base } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                }
                ValueKind::Range { start, end } => {
                    if used.insert(*start) {
                        worklist.push(*start);
                    }
                    if used.insert(*end) {
                        worklist.push(*end);
                    }
                }
                _ => {}
            }
        }

        // 3. Remove dead instructions
        for blk in &mut fn_ir.blocks {
            let old_len = blk.instrs.len();
            let values = &fn_ir.values; // Grab values before retain closure
            blk.instrs.retain(|instr| {
                if self.has_side_effect_instr(instr, values) {
                    return true;
                }

                match instr {
                    Instr::Assign { src, .. } => used.contains(src),
                    Instr::Eval { val, .. } => used.contains(val),
                    _ => true,
                }
            });
            if blk.instrs.len() != old_len {
                changed = true;
            }
        }

        changed
    }

    fn has_side_effect_instr(&self, instr: &Instr, values: &[Value]) -> bool {
        match instr {
            Instr::StoreIndex1D { .. } => true,
            Instr::StoreIndex2D { .. } => true,
            Instr::Assign { .. } => {
                // Assignments are kept conservative unless proven dead.
                true
            }
            Instr::Eval { val, .. } => self.has_side_effect_val(*val, values),
        }
    }

    fn has_side_effect_val(&self, val_id: ValueId, values: &[Value]) -> bool {
        let val = &values[val_id];
        match &val.kind {
            ValueKind::Call { callee, .. } => {
                // Whitelist known pure functions
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
                    "rr_named_list",
                ];
                if pure.contains(&callee.as_str()) {
                    return false;
                }
                true // Assume unknown calls have side effects
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                self.has_side_effect_val(*lhs, values) || self.has_side_effect_val(*rhs, values)
            }
            ValueKind::Unary { rhs, .. } => self.has_side_effect_val(*rhs, values),
            _ => false,
        }
    }

    fn check_elimination(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        // 1. Run Dataflow Analysis to get Interval Facts
        let facts = crate::mir::flow::DataflowSolver::analyze_function(fn_ir);

        // 2. Scan for Indexing operations
        // We need to iterate over values and instructions.

        // OPTIMIZATION: Index1D (Value)
        for val_idx in 0..fn_ir.values.len() {
            let mut is_proven_safe = false;
            {
                let val = &fn_ir.values[val_idx];
                if let ValueKind::Index1D {
                    base, idx, is_safe, ..
                } = &val.kind
                {
                    if !*is_safe {
                        if self.is_safe_access(fn_ir, *base, *idx, &facts) {
                            is_proven_safe = true;
                        }
                    }
                }
            }
            if is_proven_safe {
                if let ValueKind::Index1D {
                    ref mut is_safe, ..
                } = fn_ir.values[val_idx].kind
                {
                    *is_safe = true;
                    changed = true;
                }
            }
        }

        // OPTIMIZATION: StoreIndex1D (Instruction)
        for blk_idx in 0..fn_ir.blocks.len() {
            for instr_idx in 0..fn_ir.blocks[blk_idx].instrs.len() {
                let mut is_proven_safe = false;
                {
                    let instr = &fn_ir.blocks[blk_idx].instrs[instr_idx];
                    if let Instr::StoreIndex1D {
                        base, idx, is_safe, ..
                    } = instr
                    {
                        if !*is_safe {
                            if self.is_safe_access(fn_ir, *base, *idx, &facts) {
                                is_proven_safe = true;
                            }
                        }
                    }
                }
                if is_proven_safe {
                    if let Instr::StoreIndex1D {
                        ref mut is_safe, ..
                    } = fn_ir.blocks[blk_idx].instrs[instr_idx]
                    {
                        *is_safe = true;
                        changed = true;
                    }
                }
            }
        }

        changed
    }

    fn is_safe_access(
        &self,
        fn_ir: &FnIR,
        base_id: ValueId,
        idx_id: ValueId,
        facts: &FxHashMap<ValueId, crate::mir::flow::Facts>,
    ) -> bool {
        let f = facts.get(&idx_id).cloned().unwrap_or(Facts::empty());

        // Basic check: If it's ONE_BASED and fits in length.
        // Proving "fits in length" is hard without symbolic intervals.
        // Heuristic: If idx_id is from `Phi` of a loop whose limit is `len(base)`.

        // Case A: Index comes from `indices(base)`
        // `ValueKind::Indices { base: b }` where b == base_id?
        // Or if idx_id is a Phi whose inputs come from indices(base).

        // Case B: induction-variable pattern.
        if f.has(Facts::ONE_BASED) {
            if self.is_derived_from_len(fn_ir, idx_id, base_id, facts) {
                return true;
            }
        }

        false
    }

    fn is_derived_from_len(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        base_id: ValueId,
        facts: &FxHashMap<ValueId, crate::mir::flow::Facts>,
    ) -> bool {
        let val = &fn_ir.values[val_id];
        match &val.kind {
            ValueKind::Indices { base } => *base == base_id,
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                if let ValueKind::Const(Lit::Int(1)) = &fn_ir.values[*rhs].kind {
                    return self.is_loop_induction(fn_ir, *lhs, base_id);
                }
                false
            }
            ValueKind::Phi { args } => args
                .iter()
                .any(|(id, _)| self.is_derived_from_len(fn_ir, *id, base_id, facts)),
            _ => false,
        }
    }

    fn is_loop_induction(&self, fn_ir: &FnIR, val_id: ValueId, _base_id: ValueId) -> bool {
        let val = &fn_ir.values[val_id];
        if let ValueKind::Phi { args } = &val.kind {
            for (arg_id, _) in args {
                let arg_val = &fn_ir.values[*arg_id];
                if let ValueKind::Const(Lit::Int(0)) = &arg_val.kind {
                    // Heuristic: a phi starting at zero is treated as induction.
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;

    fn dummy_fn(name: &str, approx_size: usize) -> FnIR {
        let mut fn_ir = FnIR::new(name.to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let mut ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        // fn_ir_size = values + instrs; keep instrs=0 and control value count directly.
        let target_values = approx_size.max(1);
        while fn_ir.values.len() < target_values {
            ret = fn_ir.add_value(
                ValueKind::Const(Lit::Int(fn_ir.values.len() as i64)),
                Span::default(),
                Facts::empty(),
                None,
            );
        }
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir
    }

    fn fn_with_unreachable_block(name: &str) -> FnIR {
        let mut fn_ir = FnIR::new(name.to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let dead = fn_ir.add_block();
        let ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(7)),
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir.blocks[dead].term = Terminator::Return(Some(ret));
        fn_ir
    }

    #[test]
    fn opt_plan_selects_all_under_budget() {
        let mut all = FxHashMap::default();
        all.insert("a".to_string(), dummy_fn("a", 120));
        all.insert("b".to_string(), dummy_fn("b", 180));

        let plan = TachyonEngine::build_opt_plan(&all);
        assert!(!plan.selective_mode);
        assert_eq!(plan.selected_functions.len(), all.len());
    }

    #[test]
    fn opt_plan_selects_subset_over_budget() {
        let mut all = FxHashMap::default();
        for i in 0..5 {
            let name = format!("f{}", i);
            all.insert(name.clone(), dummy_fn(&name, 700));
        }

        let plan = TachyonEngine::build_opt_plan(&all);
        assert!(plan.selective_mode);
        assert!(!plan.selected_functions.is_empty());
        assert!(plan.selected_functions.len() < all.len());
    }

    #[test]
    fn opt_plan_prefers_profile_hot_function_under_budget() {
        let mut all = FxHashMap::default();
        all.insert("a".to_string(), dummy_fn("a", 620));
        all.insert("b".to_string(), dummy_fn("b", 620));
        all.insert("c".to_string(), dummy_fn("c", 620));
        all.insert("d".to_string(), dummy_fn("d", 620));
        all.insert("hot".to_string(), dummy_fn("hot", 620));

        let mut profile = FxHashMap::default();
        profile.insert("hot".to_string(), 1000usize);
        let plan = TachyonEngine::build_opt_plan_with_profile(&all, &profile);
        assert!(plan.selected_functions.contains("hot"));
    }

    #[test]
    fn always_tier_runs_light_cleanup() {
        let mut f = fn_with_unreachable_block("cleanup");
        let stats = TachyonEngine::new().run_always_tier_with_stats(&mut f);
        assert_eq!(stats.always_tier_functions, 1);
        assert!(crate::mir::verify::verify_ir(&f).is_ok());
    }
}
