mod entry_policy;
pub(crate) mod incremental;
pub(crate) mod peephole;
pub(crate) mod pipeline;
mod r_peephole;
pub(crate) mod scheduler;

pub use crate::codegen::mir_emit::MapEntry;
pub use crate::typeck::{NativeBackend, TypeConfig, TypeMode};
pub use entry_policy::{prepare_project_entry_source, prepare_single_file_build_source};

#[doc(hidden)]
pub mod internal {
    pub mod codegen {
        pub use crate::codegen::*;
    }

    pub mod hir {
        pub use crate::hir::*;
    }

    pub mod mir {
        pub use crate::mir::*;
    }

    pub mod syntax {
        pub use crate::syntax::*;
    }

    pub mod typeck {
        pub use crate::typeck::*;
    }
}

pub use incremental::{
    IncrementalCompileOutput, IncrementalCompileRequest, IncrementalOptions, IncrementalSession,
    IncrementalStats, compile_incremental_request, compile_with_configs_incremental,
    module_tree_fingerprint, module_tree_snapshot,
};
pub use pipeline::{
    CliLog, CompileMode, CompileOutputOptions, CompileProfile, CompileWithProfileRequest, OptLevel,
    ParallelBackend, ParallelConfig, ParallelMode, compile, compile_with_config,
    compile_with_configs, compile_with_configs_with_options,
    compile_with_configs_with_options_and_compiler_parallel, compile_with_profile_request,
    default_parallel_config, default_type_config, json_escape,
};
pub use scheduler::{
    CompilerParallelConfig, CompilerParallelMode, CompilerParallelProfile, CompilerParallelStage,
    CompilerParallelStageProfile, CompilerScheduler, default_compiler_parallel_config,
};
