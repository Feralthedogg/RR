#[path = "call_model_builtin_surface.rs"]
pub(crate) mod call_model_builtin_surface;
#[path = "call_model_package_surface.rs"]
pub(crate) mod call_model_package_surface;

pub(crate) use self::call_model_builtin_surface::{
    builtin_arity, is_dynamic_fallback_builtin, is_namespaced_r_call, is_tidy_data_mask_call,
    is_tidy_helper_call,
};
pub(crate) use self::call_model_package_surface::{
    is_runtime_helper, is_runtime_reserved_symbol, is_supported_package_call,
    is_supported_tidy_helper_call,
};
