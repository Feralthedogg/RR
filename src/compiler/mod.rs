pub mod incremental;
pub mod pipeline;
mod r_peephole;

pub use incremental::{
    IncrementalCompileOutput, IncrementalOptions, IncrementalSession, IncrementalStats,
    compile_with_configs_incremental, compile_with_configs_incremental_with_output_options,
    module_tree_fingerprint, module_tree_snapshot,
};
pub use pipeline::{
    CliLog, CompileOutputOptions, OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile,
    compile_with_config, compile_with_configs, compile_with_configs_with_options,
    parallel_config_from_env, type_config_from_env,
};
