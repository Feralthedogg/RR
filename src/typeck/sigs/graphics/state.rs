#[path = "state/ggplot2.rs"]
mod ggplot2;
#[path = "state/graphics_pkg.rs"]
mod graphics_pkg;
#[path = "state/grdevices.rs"]
mod grdevices;

use crate::typeck::lattice::TypeState;

pub(crate) fn infer_graphics_package_call(
    callee: &str,
    arg_tys: &[TypeState],
) -> Option<TypeState> {
    graphics_pkg::infer_graphics_pkg_state(callee, arg_tys)
        .or_else(|| grdevices::infer_grdevices_state(callee, arg_tys))
        .or_else(|| ggplot2::infer_ggplot2_state(callee, arg_tys))
}
