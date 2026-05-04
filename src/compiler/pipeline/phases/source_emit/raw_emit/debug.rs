use super::*;
use std::time::Instant;

pub(crate) fn contains_unsafe_r_escape(output: &str) -> bool {
    output.contains("# rr-unsafe-r-begin")
        || output.contains("# rr-unsafe-r-read-begin")
        || output.contains("# rr-opaque-interop: unsafe R block")
}
pub(crate) fn maybe_emit_raw_debug_output(
    assembled_output: &str,
    opt_level: OptLevel,
    pure_user_calls: &FxHashSet<String>,
    output_opts: CompileOutputOptions,
    cache: Option<&dyn EmitFunctionCache>,
) -> crate::error::RR<u128> {
    let Some(path) = std::env::var_os("RR_DEBUG_RAW_R_PATH") else {
        return Ok(0);
    };
    let started = Instant::now();
    let raw_output = if contains_generated_poly_loop_controls(assembled_output)
        || contains_unsafe_r_escape(assembled_output)
    {
        assembled_output.to_string()
    } else {
        let raw_rewrite_cache_key = cache.map(|_| {
            crate::compiler::pipeline::raw_rewrite_output_cache_key(
                assembled_output,
                opt_level,
                pure_user_calls,
                output_opts.preserve_all_defs,
                output_opts.compile_mode,
            )
        });
        if let (Some(cache), Some(cache_key)) = (cache, raw_rewrite_cache_key.as_deref()) {
            if let Some(cached_output) = cache.load_raw_rewrite(cache_key)? {
                cached_output
            } else {
                let rewritten = apply_full_raw_rewrites(
                    assembled_output.to_string(),
                    pure_user_calls,
                    output_opts,
                );
                cache.store_raw_rewrite(cache_key, &rewritten)?;
                rewritten
            }
        } else {
            apply_full_raw_rewrites(assembled_output.to_string(), pure_user_calls, output_opts)
        }
    };
    let _ = std::fs::write(path, &raw_output);
    Ok(started.elapsed().as_nanos())
}
