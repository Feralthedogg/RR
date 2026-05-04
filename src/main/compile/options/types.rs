use rr::compiler::{
    CompileMode, CompilerParallelConfig, IncrementalOptions, OptLevel, ParallelConfig, TypeConfig,
    default_compiler_parallel_config, default_parallel_config, default_type_config,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandMode {
    Legacy,
    Run,
    Build,
    Watch,
}

impl CommandMode {
    pub(crate) fn default_target(self) -> &'static str {
        match self {
            Self::Legacy => "",
            Self::Run | Self::Build | Self::Watch => ".",
        }
    }

    pub(crate) fn default_output_path(self) -> Option<String> {
        None
    }

    pub(crate) fn takes_output_arg(self, arg: &str) -> bool {
        match self {
            Self::Legacy => arg == "-o",
            Self::Build => arg == "--out-dir" || arg == "-o",
            Self::Run => false,
            Self::Watch => arg == "-o",
        }
    }

    pub(crate) fn allow_keep_r(self) -> bool {
        matches!(self, Self::Legacy | Self::Run)
    }

    pub(crate) fn allow_no_runtime(self) -> bool {
        matches!(self, Self::Legacy)
    }

    pub(crate) fn allow_legacy_mir(self) -> bool {
        matches!(self, Self::Legacy)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CommonOpts {
    pub(crate) target: String,
    pub(crate) output_path: Option<String>,
    pub(crate) keep_r: bool,
    pub(crate) no_runtime: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) opt_level: OptLevel,
    pub(crate) type_cfg: TypeConfig,
    pub(crate) parallel_cfg: ParallelConfig,
    pub(crate) compiler_parallel_cfg: CompilerParallelConfig,
    pub(crate) strict_let: bool,
    pub(crate) warn_implicit_decl: bool,
    pub(crate) incremental: IncrementalOptions,
    pub(crate) cold_compile: bool,
    pub(crate) profile_compile: bool,
    pub(crate) profile_compile_out: Option<String>,
    pub(crate) profile_use: Option<String>,
    pub(crate) compile_mode: CompileMode,
    pub(crate) compile_mode_explicit: bool,
    pub(crate) watch_poll_ms: u64,
    pub(crate) watch_once: bool,
}

impl CommonOpts {
    pub(crate) fn new(mode: CommandMode) -> Self {
        Self {
            target: mode.default_target().to_string(),
            output_path: mode.default_output_path(),
            keep_r: false,
            no_runtime: false,
            preserve_all_defs: false,
            opt_level: OptLevel::O1,
            type_cfg: default_type_config(),
            parallel_cfg: default_parallel_config(),
            compiler_parallel_cfg: default_compiler_parallel_config(),
            strict_let: true,
            warn_implicit_decl: false,
            incremental: IncrementalOptions::auto(),
            cold_compile: false,
            profile_compile: false,
            profile_compile_out: None,
            profile_use: None,
            compile_mode: match mode {
                CommandMode::Legacy => CompileMode::Standard,
                CommandMode::Run | CommandMode::Build | CommandMode::Watch => CompileMode::FastDev,
            },
            compile_mode_explicit: false,
            watch_poll_ms: 500,
            watch_once: false,
        }
    }
}
