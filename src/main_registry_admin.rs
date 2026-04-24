#[path = "main_registry_admin_audit.rs"]
mod main_registry_admin_audit;
#[path = "main_registry_admin_keys.rs"]
mod main_registry_admin_keys;
#[path = "main_registry_admin_policy.rs"]
mod main_registry_admin_policy;
#[path = "main_registry_admin_risk.rs"]
mod main_registry_admin_risk;

pub(crate) use self::main_registry_admin_audit::cmd_registry_audit;
pub(crate) use self::main_registry_admin_keys::{cmd_registry_keygen, cmd_registry_onboard};
pub(crate) use self::main_registry_admin_policy::{
    cmd_registry_policy_apply, cmd_registry_policy_bootstrap, cmd_registry_policy_show,
};
pub(crate) use self::main_registry_admin_risk::cmd_registry_risk;
