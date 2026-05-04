use std::env;

pub(crate) mod compile;
pub(crate) mod io_errors;
pub(crate) mod legacy;
pub(crate) mod module;
pub(crate) mod package;
pub(crate) mod panic;
pub(crate) mod project;
pub(crate) mod registry;
pub(crate) mod usage;
pub(crate) mod watch_hash;

use self::compile::{cmd_build, cmd_run, cmd_watch};
use self::legacy::cmd_legacy;
use self::module::cmd_mod;
use self::package::{cmd_install, cmd_outdated, cmd_publish, cmd_remove, cmd_update};
use self::panic::{install_broken_pipe_panic_hook, panic_payload_is_broken_pipe};
use self::project::{cmd_init, cmd_new};
use self::registry::{cmd_registry, cmd_search};
use self::usage::{print_usage, print_version};

pub(crate) fn run() {
    install_broken_pipe_panic_hook();
    let result = std::panic::catch_unwind(run_cli);
    match result {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(payload) => {
            if panic_payload_is_broken_pipe(payload.as_ref()) {
                std::process::exit(0);
            }
            std::panic::resume_unwind(payload);
        }
    }
}

pub(crate) fn run_cli() -> i32 {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return 0;
    }

    if args.len() >= 3 && matches!(args[2].as_str(), "--help" | "-h" | "help") {
        match args[1].as_str() {
            "build" | "run" | "watch" => {
                print_usage();
                return 0;
            }
            _ => {}
        }
    }

    match args[1].as_str() {
        "--version" | "-V" | "version" => {
            print_version();
            0
        }
        "--help" | "-h" | "help" => {
            print_usage();
            0
        }
        "new" => cmd_new(&args[2..]),
        "init" => cmd_init(&args[2..]),
        "install" => cmd_install(&args[2..]),
        "remove" => cmd_remove(&args[2..]),
        "outdated" => cmd_outdated(&args[2..]),
        "update" => cmd_update(&args[2..]),
        "publish" => cmd_publish(&args[2..]),
        "search" => cmd_search(&args[2..]),
        "registry" => cmd_registry(&args[2..]),
        "mod" => cmd_mod(&args[2..]),
        "build" => cmd_build(&args[2..]),
        "run" => cmd_run(&args[2..]),
        "watch" => cmd_watch(&args[2..]),
        _ => cmd_legacy(&args[1..]),
    }
}
