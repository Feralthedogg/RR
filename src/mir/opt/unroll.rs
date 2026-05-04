use super::*;

#[path = "unroll/analysis.rs"]
mod analysis;
#[path = "unroll/policy.rs"]
mod policy;
#[path = "unroll/transform.rs"]
mod transform;

pub(crate) use self::policy::UnrollPolicy;

pub fn optimize(fn_ir: &mut FnIR, engine: &TachyonEngine, stats: &mut TachyonPulseStats) -> usize {
    let policy = UnrollPolicy::for_engine(engine);
    if !policy.enabled || fn_ir.requires_conservative_optimization() {
        return 0;
    }
    let loops = loop_analysis::LoopAnalyzer::new(fn_ir).find_loops();
    let mut applied = 0usize;
    for lp in loops {
        let Some(candidate) = analysis::analyze(fn_ir, &lp, policy) else {
            continue;
        };
        stats.unroll_candidates += 1;
        if transform::apply(fn_ir, &candidate) {
            stats.unroll_applied += 1;
            applied += 1;
        } else {
            stats.unroll_skipped += 1;
        }
    }
    applied
}

#[cfg(test)]
#[path = "unroll/tests.rs"]
mod tests;
