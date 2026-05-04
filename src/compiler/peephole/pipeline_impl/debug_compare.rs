use super::*;

pub(crate) fn compare_exact_block_ir(
    stage: &str,
    input: &[String],
    legacy: &[String],
    pure_user_calls: &FxHashSet<String>,
) {
    let Some(mode) = std::env::var_os("RR_COMPARE_EXACT_BLOCK_IR") else {
        return;
    };
    let mut ir = input.to_vec();
    match stage {
        "exact_pre" => {
            ir = rewrite_forward_exact_expr_reuse_ir(ir);
            ir = strip_redundant_identical_pure_rebinds_ir(ir, pure_user_calls);
        }
        "exact_reuse" => {
            ir = strip_dead_simple_eval_lines(ir);
            ir = strip_noop_self_assignments(ir);
            ir = strip_redundant_nested_temp_reassigns(ir);
            ir = rewrite_forward_exact_pure_call_reuse_ir(ir, pure_user_calls);
            ir = rewrite_forward_exact_expr_reuse_ir(ir);
            ir = hoist_repeated_vector_helper_calls_within_lines(ir);
            ir = rewrite_forward_exact_vector_helper_reuse(ir);
            ir = rewrite_forward_temp_aliases(ir);
            ir = strip_redundant_identical_pure_rebinds_ir(ir, pure_user_calls);
        }
        _ => return,
    }
    if ir == legacy {
        return;
    }
    let mismatch_idx = legacy
        .iter()
        .zip(ir.iter())
        .position(|(lhs, rhs)| lhs != rhs)
        .unwrap_or_else(|| legacy.len().min(ir.len()));
    let legacy_line = legacy
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    let ir_line = ir
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    eprintln!(
        "RR_COMPARE_EXACT_BLOCK_IR diff stage={stage} legacy_lines={} ir_lines={} first_mismatch={} legacy=`{}` ir=`{}`",
        legacy.len(),
        ir.len(),
        mismatch_idx + 1,
        legacy_line,
        ir_line
    );
    if mode == "verbose" {
        let start = mismatch_idx.saturating_sub(2);
        let end = (mismatch_idx + 3)
            .max(start)
            .min(legacy.len().max(ir.len()));
        for idx in start..end {
            let legacy_line = legacy.get(idx).map(|line| line.trim()).unwrap_or("<eof>");
            let ir_line = ir.get(idx).map(|line| line.trim()).unwrap_or("<eof>");
            eprintln!(
                "RR_COMPARE_EXACT_BLOCK_IR ctx line={} legacy=`{}` ir=`{}`",
                idx + 1,
                legacy_line,
                ir_line
            );
        }
    }
}

pub(crate) fn compare_exact_reuse_substep(
    step: &str,
    input: &[String],
    legacy: &[String],
    ir: &[String],
) {
    let Some(mode) = std::env::var_os("RR_COMPARE_EXACT_REUSE_STEPS") else {
        return;
    };
    if legacy == ir {
        return;
    }
    let mismatch_idx = legacy
        .iter()
        .zip(ir.iter())
        .position(|(lhs, rhs)| lhs != rhs)
        .unwrap_or_else(|| legacy.len().min(ir.len()));
    let legacy_line = legacy
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    let ir_line = ir
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    eprintln!(
        "RR_COMPARE_EXACT_REUSE_STEPS diff step={step} legacy_lines={} ir_lines={} first_mismatch={} legacy=`{}` ir=`{}`",
        legacy.len(),
        ir.len(),
        mismatch_idx + 1,
        legacy_line,
        ir_line
    );
    if mode == "verbose" {
        let start = mismatch_idx.saturating_sub(2);
        let end = (mismatch_idx + 3)
            .max(start)
            .min(legacy.len().max(ir.len()));
        for idx in start..end {
            let input_line = input.get(idx).map(|line| line.as_str()).unwrap_or("<eof>");
            let legacy_line = legacy.get(idx).map(|line| line.as_str()).unwrap_or("<eof>");
            let ir_line = ir.get(idx).map(|line| line.as_str()).unwrap_or("<eof>");
            eprintln!(
                "RR_COMPARE_EXACT_REUSE_STEPS ctx line={} input={:?} legacy={:?} ir={:?}",
                idx + 1,
                input_line,
                legacy_line,
                ir_line
            );
        }
    }
}

pub(crate) fn compare_exact_reuse_steps_enabled() -> bool {
    std::env::var_os("RR_COMPARE_EXACT_REUSE_STEPS").is_some()
}

pub(crate) fn compare_exact_block_enabled() -> bool {
    std::env::var_os("RR_COMPARE_EXACT_BLOCK_IR").is_some()
}
