use super::*;

impl TachyonEngine {
    pub(super) fn env_bool(key: &str, default_v: bool) -> bool {
        match env::var(key) {
            Ok(v) => matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => default_v,
        }
    }

    pub(super) fn env_usize(key: &str, default_v: usize) -> usize {
        env::var(key)
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(default_v)
    }

    pub(super) fn verify_each_pass() -> bool {
        Self::env_bool("RR_VERIFY_EACH_PASS", false)
    }

    pub(super) fn verify_dump_dir() -> Option<String> {
        env::var("RR_VERIFY_DUMP_DIR").ok().and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }

    pub(super) fn sanitize_dump_component(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        for ch in raw.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                out.push(ch);
            } else {
                out.push('_');
            }
        }
        if out.is_empty() {
            "unnamed".to_string()
        } else {
            out
        }
    }

    pub(super) fn dump_verify_failure(fn_ir: &FnIR, stage: &str, reason: &str) {
        let Some(root) = Self::verify_dump_dir() else {
            return;
        };
        Self::dump_verify_failure_to(std::path::Path::new(&root), fn_ir, stage, reason);
    }

    pub(super) fn dump_verify_failure_to(
        root: &std::path::Path,
        fn_ir: &FnIR,
        stage: &str,
        reason: &str,
    ) {
        if fs::create_dir_all(root).is_err() {
            return;
        }
        let file_name = format!(
            "{}__{}.mir.txt",
            Self::sanitize_dump_component(stage),
            Self::sanitize_dump_component(&fn_ir.name)
        );
        let path = root.join(file_name);
        let payload = format!(
            "# verify failure\nstage: {stage}\nfunction: {}\nreason: {reason}\n\n{:#?}\n",
            fn_ir.name, fn_ir
        );
        let _ = fs::write(path, payload);
    }

    pub(super) fn maybe_verify(fn_ir: &FnIR, stage: &str) {
        if Self::verify_each_pass() {
            Self::verify_or_panic(fn_ir, stage);
        }
    }

    pub(super) fn max_opt_iterations() -> usize {
        Self::env_usize("RR_OPT_MAX_ITERS", 24)
    }

    pub(super) fn max_inline_rounds() -> usize {
        Self::env_usize("RR_INLINE_MAX_ROUNDS", 3)
    }

    pub(super) fn max_full_opt_ir() -> usize {
        Self::env_usize("RR_MAX_FULL_OPT_IR", 2500)
    }

    pub(super) fn max_full_opt_fn_ir() -> usize {
        Self::env_usize("RR_MAX_FULL_OPT_FN_IR", 900)
    }

    pub(super) fn adaptive_ir_budget_enabled() -> bool {
        Self::env_bool("RR_ADAPTIVE_IR_BUDGET", true)
    }

    pub(super) fn selective_budget_enabled() -> bool {
        Self::env_bool("RR_SELECTIVE_OPT_BUDGET", true) || Self::adaptive_ir_budget_enabled()
    }

    pub(super) fn heavy_pass_fn_ir() -> usize {
        Self::env_usize("RR_HEAVY_PASS_FN_IR", 650)
    }

    pub(super) fn always_bce_fn_ir() -> usize {
        let default_limit = Self::heavy_pass_fn_ir().max(64);
        Self::env_usize("RR_ALWAYS_BCE_FN_IR", default_limit)
    }

    pub(super) fn max_fn_opt_ms() -> u128 {
        Self::env_usize("RR_MAX_FN_OPT_MS", 250) as u128
    }

    pub(super) fn always_tier_max_iters() -> usize {
        Self::env_usize("RR_ALWAYS_TIER_ITERS", 2).clamp(1, 6)
    }

    pub(super) fn licm_enabled() -> bool {
        Self::env_bool("RR_ENABLE_LICM", true)
    }

    pub(super) fn licm_allowed_for_fn(fn_ir: &FnIR) -> bool {
        if fn_ir.values.len() > 256 {
            return false;
        }
        if fn_ir.blocks.len() > 64 {
            return false;
        }
        let loop_count = loop_analysis::LoopAnalyzer::new(fn_ir).find_loops().len();
        loop_count > 0 && loop_count <= 4
    }

    pub(super) fn gvn_enabled() -> bool {
        Self::env_bool("RR_ENABLE_GVN", true)
    }

    pub(super) fn profile_use_path() -> Option<String> {
        env::var("RR_PROFILE_USE").ok().and_then(|v| {
            let p = v.trim();
            if p.is_empty() {
                None
            } else {
                Some(p.to_string())
            }
        })
    }

    pub(super) fn wrap_trace_enabled() -> bool {
        Self::env_bool("RR_WRAP_TRACE", false)
    }

    pub(super) fn debug_wrap_candidates(all_fns: &FxHashMap<String, FnIR>) {
        if !Self::wrap_trace_enabled() {
            return;
        }
        let names = Self::sorted_fn_names(all_fns);
        for name in names {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if fn_ir.params.len() != 4 {
                continue;
            }
            let mut if_terms = 0usize;
            let mut store_count = 0usize;
            let mut eval_count = 0usize;
            let mut phi_count = 0usize;
            let mut call_names: FxHashSet<String> = FxHashSet::default();
            for bb in &fn_ir.blocks {
                if matches!(bb.term, Terminator::If { .. }) {
                    if_terms += 1;
                }
                for ins in &bb.instrs {
                    match ins {
                        Instr::Eval { .. } => eval_count += 1,
                        Instr::StoreIndex1D { .. }
                        | Instr::StoreIndex2D { .. }
                        | Instr::StoreIndex3D { .. } => store_count += 1,
                        Instr::Assign { .. } => {}
                    }
                }
            }
            for v in &fn_ir.values {
                match &v.kind {
                    ValueKind::Phi { .. } => phi_count += 1,
                    ValueKind::Call { callee, .. } => {
                        call_names.insert(callee.clone());
                    }
                    _ => {}
                }
            }
            eprintln!(
                "   [wrap-cand] {} params=4 blocks={} if={} stores={} eval={} phi={} calls={:?}",
                fn_ir.name,
                fn_ir.blocks.len(),
                if_terms,
                store_count,
                eval_count,
                phi_count,
                call_names
            );
        }
    }
}
