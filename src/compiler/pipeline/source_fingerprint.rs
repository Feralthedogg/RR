pub(crate) fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
pub(crate) fn fn_emit_cache_salt() -> u64 {
    let build_hash = option_env!("RR_COMPILER_BUILD_HASH").unwrap_or("no-build-script");
    stable_hash_bytes(include_str!("../pipeline.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/mod.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/mir_emit.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/backend/mod.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/backend/state.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/backend/setup.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/mod.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/assign.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/bindings.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/branches.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/cse.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/cse_prune.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/index.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/instr.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/render.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/resolve.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite/poly_index.rs").as_bytes())
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/poly_index/scalar_loop_index.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/poly_index/generated_loop_steps.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite/literal_calls.rs").as_bytes())
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/literal_calls/call_parse.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/literal_calls/record_fields.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/literal_calls/named_list.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/literal_calls/field_get.rs").as_bytes(),
        )
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite/raw_text.rs").as_bytes())
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text_helpers.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text_helpers/assignments.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text_helpers/regexes.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text_helpers/expr_helpers.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text_helpers/symbol_rewrite.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text_helpers/function_spans.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text/sym_helpers.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text/tail_slice_helpers.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text/tail_slice_return.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/raw_text/symbol_count.rs").as_bytes(),
        )
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite/scalar_alias.rs").as_bytes())
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/single_use_index.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/branch_helpers.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!(
                "../../codegen/emit/rewrite/scalar_alias/branch_helpers/expr_classification.rs"
            )
            .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/branch_helpers/block_scan.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/branch_helpers/assign_query.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/branch_rebind.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/named_expr.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/named_expr/immediate_guard.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/named_expr/two_use.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/index_alias.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/scalar_alias/index_alias/small_multiuse.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!(
                "../../codegen/emit/rewrite/scalar_alias/index_alias/straight_line_reads.rs"
            )
            .as_bytes(),
        )
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite/loop_alias.rs").as_bytes())
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/loop_alias/index_alias.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/loop_alias/slice_bounds.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/loop_alias/particle_idx.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/loop_alias/guard_helpers.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/loop_alias/guard_literals.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/loop_alias/pure_call_alias.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/loop_alias/branch_hoist.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/duplicate_alias.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/duplicate_alias/duplicate_assignments.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/duplicate_alias/structural_cleanup.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/duplicate_alias/blank_cleanup.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/duplicate_alias/repeat_tail.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/duplicate_alias/temp_copy.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/duplicate_alias/dead_scalar.rs").as_bytes(),
        )
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite/temp_seed.rs").as_bytes())
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/temp_seed/temp_copy.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/temp_seed/seq_len_cleanup.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/temp_seed/loop_seed_literals.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/temp_seed/seq_len_full_overwrite.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/temp_seed/loop_counter_restore.rs").as_bytes(),
        )
        ^ stable_hash_bytes(include_str!("../../codegen/emit/rewrite/final_cleanup.rs").as_bytes())
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/final_cleanup/loop_counter_alias.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/final_cleanup/range_alias.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/final_cleanup/repeat_counter_restore.rs")
                .as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/final_cleanup/branch_vec_fill.rs").as_bytes(),
        )
        ^ stable_hash_bytes(
            include_str!("../../codegen/emit/rewrite/final_cleanup/raw_arg_alias.rs").as_bytes(),
        )
        ^ stable_hash_bytes(include_str!("../../codegen/emit/structured.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/structured_analysis.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../../codegen/emit/control_flow.rs").as_bytes())
        ^ stable_hash_bytes(build_hash.as_bytes())
}
