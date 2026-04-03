mod data_misc;
mod env_io;

use crate::typeck::lattice::TypeState;

pub(crate) fn infer_base_extra_package_call(
    callee: &str,
    arg_tys: &[TypeState],
) -> Option<TypeState> {
    env_io::infer_base_extra_package_call_env_io(callee, arg_tys)
        .or_else(|| data_misc::infer_base_extra_package_call_data_misc(callee, arg_tys))
}
