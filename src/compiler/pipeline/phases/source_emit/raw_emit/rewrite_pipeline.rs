use super::raw_pass_manager::{
    run_fragment_raw_rewrite_passes, run_full_program_raw_rewrite_passes,
    run_post_assembly_finalize_passes,
};
use super::*;

pub(crate) fn apply_raw_rewrites_to_fragment(
    output: String,
    pure_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
) -> String {
    if contains_unsafe_r_escape(&output) {
        return output;
    }
    let _ = pure_user_calls;
    run_fragment_raw_rewrite_passes(output, output_opts)
}
pub(crate) fn optimize_emitted_fragment(
    code: &str,
    map: &[MapEntry],
    opt_level: OptLevel,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
) -> (String, Vec<MapEntry>) {
    let mut output = code.to_string();
    let skip_rewrites =
        contains_generated_poly_loop_controls(&output) || contains_unsafe_r_escape(&output);
    if !skip_rewrites {
        output = apply_raw_rewrites_to_fragment(output, pure_user_calls, output_opts);
    }
    let (optimized_output, line_map) = if skip_rewrites {
        let line_map = (1..=output.lines().count() as u32).collect::<Vec<_>>();
        (output, line_map)
    } else {
        let options = crate::compiler::peephole::PeepholeOptions::new(direct_builtin_call_map)
            .preserving_all_defs(output_opts.preserve_all_defs)
            .fast_dev(matches!(output_opts.compile_mode, CompileMode::FastDev))
            .opt_level(opt_level);
        crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_profile(
            &output,
            pure_user_calls,
            fresh_user_calls,
            options,
        )
        .0
    };
    let optimized_map = remap_source_map_lines(map.to_vec(), &line_map);
    (optimized_output, optimized_map)
}
pub(crate) fn apply_full_raw_rewrites(
    output: String,
    pure_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
) -> String {
    if contains_unsafe_r_escape(&output) {
        return output;
    }
    let _ = pure_user_calls;
    run_full_program_raw_rewrite_passes(output, output_opts)
}
pub(crate) fn apply_post_assembly_finalize_rewrites(output: String) -> String {
    run_post_assembly_finalize_passes(output)
}
pub(crate) fn apply_full_peephole_to_output(
    output: &str,
    map: &[MapEntry],
    opt_level: OptLevel,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
) -> (String, Vec<MapEntry>) {
    if contains_unsafe_r_escape(output) {
        return (output.to_string(), map.to_vec());
    }
    let options = crate::compiler::peephole::PeepholeOptions::new(direct_builtin_call_map)
        .preserving_all_defs(output_opts.preserve_all_defs)
        .fast_dev(matches!(output_opts.compile_mode, CompileMode::FastDev))
        .opt_level(opt_level);
    let ((optimized_output, line_map), _) =
        crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_profile(
            output,
            pure_user_calls,
            fresh_user_calls,
            options,
        );
    let optimized_map = remap_source_map_lines(map.to_vec(), &line_map);
    (optimized_output, optimized_map)
}
