mod paths;
mod scaffold;
mod templates;

pub(crate) use self::paths::{default_build_output_dir, default_watch_output_file};
pub(crate) use self::scaffold::{cmd_init, cmd_new};
