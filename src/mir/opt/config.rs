use super::types::{FunctionPhasePlan, PhaseOrderingMode};
use super::*;

impl TachyonEngine {
    pub(crate) fn env_bool(key: &str, default_v: bool) -> bool {
        match env::var(key) {
            Ok(v) => matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => default_v,
        }
    }

    pub(crate) fn env_usize(key: &str, default_v: usize) -> usize {
        env::var(key)
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(default_v)
    }

    pub(crate) fn verify_each_pass() -> bool {
        Self::env_bool("RR_VERIFY_EACH_PASS", false)
    }

    pub(crate) fn verify_dump_dir() -> Option<String> {
        env::var("RR_VERIFY_DUMP_DIR").ok().and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }

    pub(crate) fn sanitize_dump_component(raw: &str) -> String {
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

    pub(crate) fn dump_verify_failure(fn_ir: &FnIR, stage: &str, reason: &str) {
        let Some(root) = Self::verify_dump_dir() else {
            return;
        };
        Self::dump_verify_failure_to(std::path::Path::new(&root), fn_ir, stage, reason);
    }

    pub(crate) fn dump_verify_failure_to(
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

    pub(crate) fn mir_dump_dir() -> Option<String> {
        env::var("RR_MIR_DUMP_DIR").ok().and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }

    pub(crate) fn mir_dump_filter_allows(fn_ir: &FnIR) -> bool {
        let Some(filter) = env::var_os("RR_MIR_DUMP_FILTER") else {
            return true;
        };
        let filter = filter.to_string_lossy();
        filter.split(',').map(str::trim).any(|item| {
            item == "*"
                || item == fn_ir.name
                || fn_ir.user_name.as_deref().is_some_and(|name| item == name)
                || (!item.is_empty()
                    && (fn_ir.name.contains(item)
                        || fn_ir
                            .user_name
                            .as_deref()
                            .is_some_and(|name| name.contains(item))))
        })
    }

    pub(crate) fn mir_dump_stage_allows(stage: &str, pass: &str) -> bool {
        let Some(filter) = env::var_os("RR_MIR_DUMP_STAGE") else {
            return true;
        };
        let filter = filter.to_string_lossy();
        filter.split(',').map(str::trim).any(|item| {
            item == "*"
                || item.eq_ignore_ascii_case("all")
                || (!item.is_empty() && (stage.contains(item) || pass.contains(item)))
        })
    }

    pub(crate) fn mir_dump_when_allows(moment: &str) -> bool {
        let raw = env::var("RR_MIR_DUMP_WHEN").unwrap_or_else(|_| "both".to_string());
        raw.split(',').map(str::trim).any(|item| {
            item.eq_ignore_ascii_case("both")
                || item.eq_ignore_ascii_case("all")
                || item.eq_ignore_ascii_case(moment)
        })
    }

    pub(crate) fn dump_mir_snapshot(fn_ir: &FnIR, stage: &str, pass: &str, moment: &str) {
        let Some(root) = Self::mir_dump_dir() else {
            return;
        };
        if !Self::mir_dump_filter_allows(fn_ir)
            || !Self::mir_dump_stage_allows(stage, pass)
            || !Self::mir_dump_when_allows(moment)
        {
            return;
        }
        static MIR_DUMP_COUNTER: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let seq = MIR_DUMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::path::Path::new(&root);
        if fs::create_dir_all(root).is_err() {
            return;
        }
        let file_name = format!(
            "{seq:06}__{}__{}__{}__{}.mir.txt",
            Self::sanitize_dump_component(moment),
            Self::sanitize_dump_component(pass),
            Self::sanitize_dump_component(stage),
            Self::sanitize_dump_component(&fn_ir.name),
        );
        let payload = format!(
            "# mir dump\nmoment: {moment}\npass: {pass}\nstage: {stage}\nfunction: {}\n\n{:#?}\n",
            fn_ir.name, fn_ir
        );
        let _ = fs::write(root.join(file_name), payload);
    }

    pub(crate) fn dump_program_mir_snapshots(
        all_fns: &FxHashMap<String, FnIR>,
        stage: &str,
        pass: &str,
        moment: &str,
    ) {
        if Self::mir_dump_dir().is_none() {
            return;
        }
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            Self::dump_mir_snapshot(fn_ir, stage, pass, moment);
        }
    }

    pub(crate) fn maybe_verify(fn_ir: &FnIR, stage: &str) {
        if Self::verify_each_pass() {
            Self::verify_or_panic(fn_ir, stage);
        }
    }

    pub(crate) fn max_opt_iterations() -> usize {
        24
    }

    pub(crate) fn max_inline_rounds() -> usize {
        3
    }

    pub(crate) fn max_full_opt_ir() -> usize {
        2500
    }

    pub(crate) fn max_full_opt_fn_ir() -> usize {
        900
    }

    pub(crate) fn adaptive_ir_budget_enabled() -> bool {
        true
    }

    pub(crate) fn selective_budget_enabled() -> bool {
        true
    }

    pub(crate) fn heavy_pass_fn_ir() -> usize {
        650
    }

    pub(crate) fn always_bce_fn_ir() -> usize {
        Self::heavy_pass_fn_ir().max(64)
    }

    pub(crate) fn max_fn_opt_ms() -> u128 {
        250
    }

    pub(crate) fn always_tier_max_iters() -> usize {
        2
    }

    pub(crate) fn licm_enabled() -> bool {
        true
    }

    pub(crate) fn licm_allowed_for_fn(fn_ir: &FnIR) -> bool {
        if fn_ir.values.len() > 256 {
            return false;
        }
        if fn_ir.blocks.len() > 64 {
            return false;
        }
        let loop_count = loop_analysis::LoopAnalyzer::new(fn_ir).find_loops().len();
        loop_count > 0 && loop_count <= 4
    }

    pub(crate) fn gvn_enabled() -> bool {
        true
    }

    pub(crate) fn parse_phase_ordering_mode(raw: Option<&str>) -> PhaseOrderingMode {
        match raw
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            None | Some("off") => PhaseOrderingMode::Off,
            Some("balanced") => PhaseOrderingMode::Balanced,
            Some("auto") => PhaseOrderingMode::Auto,
            Some(_) => PhaseOrderingMode::Off,
        }
    }

    pub(crate) fn phase_ordering_mode() -> PhaseOrderingMode {
        let raw = env::var("RR_PHASE_ORDERING").ok();
        Self::parse_phase_ordering_mode(raw.as_deref())
    }

    pub(crate) fn phase_ordering_mode_with_default(
        default_mode: PhaseOrderingMode,
    ) -> PhaseOrderingMode {
        match env::var("RR_PHASE_ORDERING") {
            Ok(raw) => Self::parse_phase_ordering_mode(Some(raw.as_str())),
            Err(_) => default_mode,
        }
    }

    pub(crate) fn phase_ordering_trace_enabled() -> bool {
        Self::env_bool("RR_PHASE_ORDERING_TRACE", false)
    }

    pub(crate) const fn phase_ordering_opt_level_default(
        opt_level: crate::compiler::OptLevel,
    ) -> PhaseOrderingMode {
        match opt_level {
            crate::compiler::OptLevel::O0 => PhaseOrderingMode::Off,
            crate::compiler::OptLevel::O1 => PhaseOrderingMode::Balanced,
            crate::compiler::OptLevel::O2 => PhaseOrderingMode::Auto,
            crate::compiler::OptLevel::O3 => PhaseOrderingMode::Auto,
            crate::compiler::OptLevel::Oz => PhaseOrderingMode::Balanced,
        }
    }

    pub(crate) fn phase_ordering_default_mode_for_opt_level(
        opt_level: crate::compiler::OptLevel,
    ) -> PhaseOrderingMode {
        Self::phase_ordering_opt_level_default(opt_level)
    }

    pub(crate) fn phase_ordering_mode_for_opt_level(
        opt_level: crate::compiler::OptLevel,
    ) -> PhaseOrderingMode {
        Self::phase_ordering_mode_with_default(Self::phase_ordering_default_mode_for_opt_level(
            opt_level,
        ))
    }

    pub(crate) fn resolved_phase_ordering_mode(&self) -> PhaseOrderingMode {
        Self::phase_ordering_mode_with_default(self.phase_ordering_default_mode)
    }

    pub(crate) fn build_legacy_function_phase_plan(&self, function: &str) -> FunctionPhasePlan {
        FunctionPhasePlan::legacy(
            function.to_string(),
            self.resolved_phase_ordering_mode(),
            Self::phase_ordering_trace_enabled(),
        )
    }

    pub(crate) fn profile_use_path() -> Option<String> {
        env::var("RR_PROFILE_USE")
            .ok()
            .or_else(|| env::var("RR_PROFILE_USE_PATH").ok())
            .and_then(|v| {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
    }

    pub(crate) fn wrap_trace_enabled() -> bool {
        Self::env_bool("RR_WRAP_TRACE", false)
    }

    pub(crate) fn debug_wrap_candidates(all_fns: &FxHashMap<String, FnIR>) {
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
                        | Instr::StoreIndex3D { .. }
                        | Instr::UnsafeRBlock { .. } => store_count += 1,
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
