use super::{report_dir_create_failure, report_file_write_failure, report_path_read_failure};

#[path = "main_project_paths.rs"]
mod main_project_paths;
#[path = "main_project_scaffold.rs"]
mod main_project_scaffold;
#[path = "main_project_templates.rs"]
mod main_project_templates;

pub(crate) use self::main_project_paths::{default_build_output_dir, default_watch_output_file};
pub(crate) use self::main_project_scaffold::{cmd_init, cmd_new};
