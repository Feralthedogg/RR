#[path = "call_model_package_surface/base.rs"]
pub(crate) mod base;
#[path = "call_model_package_surface/compiler.rs"]
pub(crate) mod compiler;
#[path = "call_model_package_surface/dplyr.rs"]
pub(crate) mod dplyr;
#[path = "call_model_package_surface/ggplot2.rs"]
pub(crate) mod ggplot2;
#[path = "call_model_package_surface/graphics.rs"]
pub(crate) mod graphics;
#[path = "call_model_package_surface/grdevices.rs"]
pub(crate) mod grdevices;
#[path = "call_model_package_surface/grid.rs"]
pub(crate) mod grid;
#[path = "call_model_package_surface/methods.rs"]
pub(crate) mod methods;
#[path = "call_model_package_surface/parallel.rs"]
pub(crate) mod parallel;
#[path = "call_model_package_surface/readr.rs"]
pub(crate) mod readr;
#[path = "call_model_package_surface/runtime.rs"]
pub(crate) mod runtime;
#[path = "call_model_package_surface/splines.rs"]
pub(crate) mod splines;
#[path = "call_model_package_surface/stats.rs"]
pub(crate) mod stats;
#[path = "call_model_package_surface/stats4.rs"]
pub(crate) mod stats4;
#[path = "call_model_package_surface/tcltk.rs"]
pub(crate) mod tcltk;
#[path = "call_model_package_surface/tidyr.rs"]
pub(crate) mod tidyr;
#[path = "call_model_package_surface/tools.rs"]
pub(crate) mod tools;
#[path = "call_model_package_surface/utils.rs"]
pub(crate) mod utils;

pub(crate) fn is_supported_package_call(name: &str) -> bool {
    base::contains(name)
        || compiler::contains(name)
        || dplyr::contains(name)
        || ggplot2::contains(name)
        || graphics::contains(name)
        || grdevices::contains(name)
        || grid::contains(name)
        || methods::contains(name)
        || parallel::contains(name)
        || readr::contains(name)
        || splines::contains(name)
        || stats::contains(name)
        || stats4::contains(name)
        || tcltk::contains(name)
        || tidyr::contains(name)
        || tools::contains(name)
        || utils::contains(name)
        || name.starts_with("base::")
}

pub(crate) fn is_supported_tidy_helper_call(name: &str) -> bool {
    super::call_model_builtin_surface::is_tidy_helper_call(name)
}

pub(crate) fn is_runtime_helper(name: &str) -> bool {
    runtime::is_runtime_helper(name)
}

pub(crate) fn is_runtime_reserved_symbol(name: &str) -> bool {
    runtime::is_runtime_reserved_symbol(name)
}
