use rr::compiler::{
    CompileOutputOptions, CompileProfile, CompileWithProfileRequest, CompilerParallelConfig,
    IncrementalCompileOutput, IncrementalCompileRequest, IncrementalOptions, IncrementalSession,
    IncrementalStats, OptLevel, ParallelConfig, TypeConfig, compile_incremental_request,
    compile_with_profile_request,
};
use rr::error::RRException;

use super::CommonOpts;

pub(crate) struct CliCompileRequest<'a> {
    pub(crate) entry_path: &'a str,
    pub(crate) input: &'a str,
    pub(crate) opt_level: OptLevel,
    pub(crate) type_cfg: TypeConfig,
    pub(crate) parallel_cfg: ParallelConfig,
    pub(crate) compiler_parallel_cfg: CompilerParallelConfig,
    pub(crate) incremental: IncrementalOptions,
    pub(crate) output_opts: CompileOutputOptions,
    pub(crate) session: Option<&'a mut IncrementalSession>,
    pub(crate) profile: Option<&'a mut CompileProfile>,
    pub(crate) cold_compile: bool,
    pub(crate) profile_use: Option<&'a str>,
}

pub(crate) fn compile_output_options(
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

pub(crate) fn compile_cli_source(
    mut req: CliCompileRequest<'_>,
) -> Result<IncrementalCompileOutput, RRException> {
    super::with_profile_use_override(req.profile_use, || {
        super::with_compile_cache_override(req.cold_compile, || {
            if req.incremental.enabled {
                compile_incremental_request(IncrementalCompileRequest {
                    entry_path: req.entry_path,
                    entry_input: req.input,
                    opt_level: req.opt_level,
                    type_cfg: req.type_cfg,
                    parallel_cfg: req.parallel_cfg,
                    compiler_parallel_cfg: req.compiler_parallel_cfg,
                    options: req.incremental,
                    output_options: req.output_opts,
                    session: req.session.as_deref_mut(),
                    profile: req.profile.as_deref_mut(),
                })
            } else {
                compile_with_profile_request(CompileWithProfileRequest {
                    entry_path: req.entry_path,
                    entry_input: req.input,
                    opt_level: req.opt_level,
                    type_cfg: req.type_cfg,
                    parallel_cfg: req.parallel_cfg,
                    compiler_parallel_cfg: req.compiler_parallel_cfg,
                    output_opts: req.output_opts,
                    profile: req.profile.as_deref_mut(),
                })
                .map(|(r_code, source_map)| IncrementalCompileOutput {
                    r_code,
                    source_map,
                    stats: IncrementalStats::default(),
                })
            }
        })
    })
}
