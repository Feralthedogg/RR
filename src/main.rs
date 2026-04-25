use std::env;

#[path = "main_compile.rs"]
mod main_compile;
#[path = "main_compile_api.rs"]
mod main_compile_api;
#[path = "main_compile_cache.rs"]
mod main_compile_cache;
#[path = "main_compile_entry.rs"]
mod main_compile_entry;
#[path = "main_compile_options.rs"]
mod main_compile_options;
#[path = "main_compile_profile.rs"]
mod main_compile_profile;
#[path = "main_compile_target.rs"]
mod main_compile_target;
#[path = "main_io_errors.rs"]
mod main_io_errors;
#[path = "main_legacy.rs"]
mod main_legacy;
#[path = "main_mod.rs"]
mod main_mod;
#[path = "main_panic.rs"]
mod main_panic;
#[path = "main_pkg.rs"]
mod main_pkg;
#[path = "main_project.rs"]
mod main_project;
#[path = "main_registry.rs"]
mod main_registry;
#[path = "main_usage.rs"]
mod main_usage;
#[path = "main_watch_hash.rs"]
mod main_watch_hash;

use self::main_compile::*;
use self::main_compile_api::*;
use self::main_compile_cache::*;
use self::main_compile_entry::*;
use self::main_compile_options::*;
use self::main_compile_profile::*;
use self::main_compile_target::*;
use self::main_io_errors::*;
use self::main_legacy::*;
use self::main_mod::*;
use self::main_panic::*;
use self::main_pkg::*;
use self::main_project::*;
use self::main_registry::*;
use self::main_usage::*;
use self::main_watch_hash::*;

fn main() {
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

fn run_cli() -> i32 {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return 0;
    }

    match args[1].as_str() {
        "__rr_poly_isl_materialize" => {
            RR::mir::opt::poly::run_hidden_poly_cli(&args[2..]).unwrap_or(2)
        }
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
