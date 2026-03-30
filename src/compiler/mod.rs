pub mod incremental;
pub mod pipeline;
mod r_peephole;
pub mod scheduler;

pub use incremental::{
    IncrementalCompileOutput, IncrementalOptions, IncrementalSession, IncrementalStats,
    compile_with_configs_incremental, compile_with_configs_incremental_with_output_options,
    compile_with_configs_incremental_with_output_options_and_compiler_parallel,
    module_tree_fingerprint, module_tree_snapshot,
};
pub use pipeline::{
    CliLog, CompileOutputOptions, OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile,
    compile_with_config, compile_with_configs, compile_with_configs_with_options,
    compile_with_configs_with_options_and_compiler_parallel, default_parallel_config,
    default_type_config,
};
pub use scheduler::{
    CompilerParallelConfig, CompilerParallelMode, CompilerScheduler,
    default_compiler_parallel_config,
};
