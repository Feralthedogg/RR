#[path = "main_compile_build.rs"]
mod main_compile_build;
#[path = "main_compile_run.rs"]
mod main_compile_run;
#[path = "main_compile_watch.rs"]
mod main_compile_watch;

pub(crate) use self::main_compile_build::cmd_build;
pub(crate) use self::main_compile_run::cmd_run;
pub(crate) use self::main_compile_watch::cmd_watch;
