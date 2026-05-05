mod api;
mod build;
mod cache;
mod entry;
mod options;
mod profile;
mod run;
mod target;
mod watch;

pub(crate) use self::build::cmd_build;
pub(crate) use self::run::cmd_run;
pub(crate) use self::watch::cmd_watch;
pub(crate) use api::{CliCompileRequest, compile_cli_source, compile_output_options};
pub(crate) use cache::{with_compile_cache_override, with_profile_use_override};
pub(crate) use entry::{prepare_project_entry_source, prepare_single_file_build_source};
pub(crate) use options::{CommandMode, CommonOpts, parse_command_opts};
pub(crate) use profile::{write_compile_profile_artifact, write_compile_profile_collection};
pub(crate) use target::{
    file_name_is_main_rr, resolve_command_input, resolve_project_entry_in_dir,
};

pub(crate) use super::io_errors::{
    report_dir_create_failure, report_file_write_failure, report_path_read_failure,
};
pub(crate) use super::project::{default_build_output_dir, default_watch_output_file};
pub(crate) use super::watch_hash::{watch_output_hash, watch_output_matches_hash};
