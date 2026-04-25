use RR::compiler::{
    CompileOutputOptions, CompileProfile, CompilerParallelConfig, IncrementalCompileOutput,
    IncrementalOptions, IncrementalSession, IncrementalStats, OptLevel, ParallelConfig,
    compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile as compile_incremental_with_profile,
    compile_with_configs_with_options_and_compiler_parallel_and_profile as compile_regular_with_profile,
};
use RR::error::RRException;
use RR::typeck::TypeConfig;

use super::CommonOpts;

pub(super) struct CliCompileRequest<'a> {
    pub(super) entry_path: &'a str,
    pub(super) input: &'a str,
    pub(super) opt_level: OptLevel,
    pub(super) type_cfg: TypeConfig,
    pub(super) parallel_cfg: ParallelConfig,
    pub(super) compiler_parallel_cfg: CompilerParallelConfig,
    pub(super) incremental: IncrementalOptions,
    pub(super) output_opts: CompileOutputOptions,
    pub(super) session: Option<&'a mut IncrementalSession>,
    pub(super) profile: Option<&'a mut CompileProfile>,
    pub(super) cold_compile: bool,
}

pub(super) fn compile_output_options(
    opts: &CommonOpts,
    inject_runtime: bool,
) -> CompileOutputOptions {
    CompileOutputOptions {
        inject_runtime,
        preserve_all_defs: opts.preserve_all_defs,
        strict_let: opts.strict_let,
        warn_implicit_decl: opts.warn_implicit_decl,
        compile_mode: opts.compile_mode,
    }
}

pub(super) fn compile_cli_source(
    mut req: CliCompileRequest<'_>,
) -> Result<IncrementalCompileOutput, RRException> {
    super::with_compile_cache_override(req.cold_compile, || {
        if req.incremental.enabled {
            compile_incremental_with_profile(
                req.entry_path,
                req.input,
                req.opt_level,
                req.type_cfg,
                req.parallel_cfg,
                req.compiler_parallel_cfg,
                req.incremental,
                req.output_opts,
                req.session.as_mut().map(|session| &mut **session),
                req.profile.as_mut().map(|profile| &mut **profile),
            )
        } else {
            compile_regular_with_profile(
                req.entry_path,
                req.input,
                req.opt_level,
                req.type_cfg,
                req.parallel_cfg,
                req.compiler_parallel_cfg,
                req.output_opts,
                req.profile.as_mut().map(|profile| &mut **profile),
            )
            .map(|(r_code, source_map)| IncrementalCompileOutput {
                r_code,
                source_map,
                stats: IncrementalStats::default(),
            })
        }
    })
}
