pub mod pipeline;

pub use pipeline::{
    CliLog, OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile, compile_with_config,
    compile_with_configs,
};
