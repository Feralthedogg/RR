use super::*;

pub(crate) struct OptimizedAssemblyStoreRequest<'a> {
    pub(crate) cache: Option<&'a dyn EmitFunctionCache>,
    pub(crate) optimized_assembly_key: Option<&'a str>,
    pub(crate) final_output: &'a str,
    pub(crate) final_source_map: &'a [MapEntry],
    pub(crate) emitted_fragments: &'a [EmittedFnFragment],
    pub(crate) options: OptimizedAssemblyOptions,
    pub(crate) calls: OptimizedAssemblyCalls<'a>,
}

pub(crate) fn store_optimized_assembly_cache_state(
    request: OptimizedAssemblyStoreRequest<'_>,
) -> crate::error::RR<()> {
    let Some(cache) = request.cache else {
        return Ok(());
    };

    let fallback_key;
    let key = if let Some(key) = request.optimized_assembly_key {
        key
    } else {
        let (unoptimized_output, _) = assemble_emitted_fragments(request.emitted_fragments, false);
        fallback_key = crate::compiler::pipeline::optimized_assembly_cache_key(
            &unoptimized_output,
            request.calls.pure_user_calls,
            request.calls.fresh_user_calls,
            crate::compiler::pipeline::OutputCacheKeyOptions {
                opt_level: request.options.opt_level,
                direct_builtin_call_map: request.options.direct_builtin_call_map,
                preserve_all_defs: request.options.output_opts.preserve_all_defs,
                compile_mode: request.options.output_opts.compile_mode,
            },
        );
        fallback_key.as_str()
    };

    let (optimized_output, optimized_source_map) =
        assemble_emitted_fragments(request.emitted_fragments, true);
    cache.store_optimized_assembly_artifact(key, request.final_output, request.final_source_map)?;
    if optimized_output == request.final_output {
        cache.store_optimized_assembly_source_map(key, request.final_source_map)?;
        cache.store_optimized_assembly_safe(key)?;
        return Ok(());
    }

    let optimized_raw_output = apply_post_assembly_finalize_rewrites(apply_full_raw_rewrites(
        optimized_output.clone(),
        request.calls.pure_user_calls,
        request.options.output_opts,
    ));
    if optimized_raw_output == request.final_output {
        cache.store_optimized_assembly_source_map(key, request.final_source_map)?;
        cache.store_optimized_raw_assembly_safe(key)?;
        return Ok(());
    }

    let (optimized_peephole_output, _optimized_peephole_map) = apply_full_peephole_to_output(
        &optimized_output,
        &optimized_source_map,
        request.options.opt_level,
        request.options.direct_builtin_call_map,
        request.calls.pure_user_calls,
        request.calls.fresh_user_calls,
        request.options.output_opts,
    );
    if optimized_peephole_output == request.final_output {
        cache.store_optimized_assembly_source_map(key, request.final_source_map)?;
        cache.store_optimized_peephole_assembly_safe(key)?;
    }
    Ok(())
}
