use super::*;
impl TachyonEngine {
    pub(crate) fn sroa_trace_enabled() -> bool {
        std::env::var_os("RR_SROA_TRACE").is_some()
    }

    pub(crate) fn sroa_trace_verbose() -> bool {
        std::env::var("RR_SROA_TRACE")
            .map(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "2" | "detail" | "debug" | "verbose"
                )
            })
            .unwrap_or(false)
    }

    pub(crate) fn debug_sroa_candidates(all_fns: &FxHashMap<String, FnIR>) {
        if !Self::sroa_trace_enabled() {
            return;
        }

        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            let analysis = analyze_function(fn_ir);
            let counts = analysis.counts();
            if counts.candidates == 0 {
                continue;
            }
            eprintln!(
                "   [sroa-cand] {} candidates={} record={} field-set={} phi={} alias={} scalar={} remat={} reject={}",
                name,
                counts.candidates,
                counts.record_lits,
                counts.field_sets,
                counts.phis,
                counts.load_aliases,
                counts.scalar_only,
                counts.needs_rematerialization,
                counts.rejected
            );
            if Self::sroa_trace_verbose() {
                for candidate in analysis.candidates {
                    eprintln!(
                        "      value={} source={:?} shape={:?} status={:?} uses={} rejects={:?}",
                        candidate.value,
                        candidate.source,
                        candidate.shape,
                        candidate.status,
                        candidate.uses.len(),
                        candidate.reject_reasons
                    );
                }
            }
        }
    }
}
