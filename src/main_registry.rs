use RR::compiler::CliLog;

#[path = "main_registry_admin.rs"]
mod main_registry_admin;
#[path = "main_registry_args.rs"]
mod main_registry_args;
#[path = "main_registry_channel.rs"]
mod main_registry_channel;
#[path = "main_registry_policy.rs"]
mod main_registry_policy;
#[path = "main_registry_read.rs"]
mod main_registry_read;
#[path = "main_registry_release.rs"]
mod main_registry_release;
#[path = "main_registry_search.rs"]
mod main_registry_search;
#[path = "main_registry_usage.rs"]
mod main_registry_usage;

pub(crate) use self::main_registry_admin::*;
pub(crate) use self::main_registry_search::cmd_search;

use self::main_registry_args::{parse_registry_override_args, require_registry_root};
use self::main_registry_channel::{
    cmd_registry_channel_clear, cmd_registry_channel_set, cmd_registry_channel_show,
};
use self::main_registry_policy::{cmd_registry_policy_lint, cmd_registry_policy_rotate_key};
use self::main_registry_read::{
    cmd_registry_diff, cmd_registry_info, cmd_registry_list, cmd_registry_queue,
    cmd_registry_report, cmd_registry_verify,
};
use self::main_registry_release::{
    cmd_registry_approve, cmd_registry_deprecate, cmd_registry_promote, cmd_registry_unapprove,
    cmd_registry_undeprecate, cmd_registry_unyank, cmd_registry_yank,
};
use self::main_registry_usage::REGISTRY_USAGE;

pub(crate) fn cmd_registry(args: &[String]) -> i32 {
    let ui = CliLog::new();
    if matches!(args.first().map(String::as_str), Some("keygen")) {
        return cmd_registry_keygen(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("onboard")) {
        return cmd_registry_onboard(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("risk")) {
        return cmd_registry_risk(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("audit")) {
        return cmd_registry_audit(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("policy"))
        && matches!(args.get(1).map(String::as_str), Some("bootstrap"))
    {
        return cmd_registry_policy_bootstrap(&args[2..]);
    }
    if matches!(args.first().map(String::as_str), Some("policy"))
        && matches!(args.get(1).map(String::as_str), Some("show"))
    {
        return cmd_registry_policy_show(&args[2..]);
    }
    if matches!(args.first().map(String::as_str), Some("policy"))
        && matches!(args.get(1).map(String::as_str), Some("apply"))
    {
        return cmd_registry_policy_apply(&args[2..]);
    }

    let Ok((positional, registry)) = parse_registry_override_args(args, REGISTRY_USAGE, &ui) else {
        return 1;
    };
    let registry = registry.as_deref();

    match positional.as_slice() {
        [subcommand] if subcommand == "list" => cmd_registry_list(&ui, registry),
        [subcommand] if subcommand == "report" => cmd_registry_report(&ui, registry, None),
        [subcommand, module_path] if subcommand == "report" => {
            cmd_registry_report(&ui, registry, Some(module_path))
        }
        [subcommand, module_path, from_version, to_version] if subcommand == "diff" => {
            cmd_registry_diff(&ui, registry, module_path, from_version, to_version)
        }
        [group, subcommand, module_path] if group == "channel" && subcommand == "show" => {
            cmd_registry_channel_show(&ui, registry, module_path)
        }
        [group, subcommand, module_path, channel, version]
            if group == "channel" && subcommand == "set" =>
        {
            cmd_registry_channel_set(&ui, registry, module_path, channel, version)
        }
        [group, subcommand, module_path, channel]
            if group == "channel" && subcommand == "clear" =>
        {
            cmd_registry_channel_clear(&ui, registry, module_path, channel)
        }
        [subcommand] if subcommand == "queue" => cmd_registry_queue(&ui, registry),
        [group, subcommand] if group == "policy" && subcommand == "lint" => {
            cmd_registry_policy_lint(&ui, registry)
        }
        [group, subcommand, old_key, new_key]
            if group == "policy" && subcommand == "rotate-key" =>
        {
            cmd_registry_policy_rotate_key(&ui, registry, old_key, new_key)
        }
        [subcommand, module_path, version] if subcommand == "approve" => {
            cmd_registry_approve(&ui, registry, module_path, version)
        }
        [subcommand, module_path, version] if subcommand == "unapprove" => {
            cmd_registry_unapprove(&ui, registry, module_path, version)
        }
        [subcommand, module_path, version] if subcommand == "promote" => {
            cmd_registry_promote(&ui, registry, module_path, version)
        }
        [subcommand, module_path] if subcommand == "info" => {
            cmd_registry_info(&ui, registry, module_path)
        }
        [subcommand] if subcommand == "verify" => cmd_registry_verify(&ui, registry, None),
        [subcommand, module_path] if subcommand == "verify" => {
            cmd_registry_verify(&ui, registry, Some(module_path))
        }
        [subcommand, module_path, version] if subcommand == "yank" => {
            cmd_registry_yank(&ui, registry, module_path, version)
        }
        [subcommand, module_path, version] if subcommand == "unyank" => {
            cmd_registry_unyank(&ui, registry, module_path, version)
        }
        [subcommand, module_path, message @ ..] if subcommand == "deprecate" => {
            cmd_registry_deprecate(&ui, registry, module_path, message)
        }
        [subcommand, module_path] if subcommand == "undeprecate" => {
            cmd_registry_undeprecate(&ui, registry, module_path)
        }
        _ => {
            ui.error("RR registry expects a supported subcommand");
            ui.warn(REGISTRY_USAGE);
            1
        }
    }
}
