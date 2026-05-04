pub const R_RUNTIME: &str = concat!(
    include_str!("runtime_prelude/config.R"),
    "\n",
    include_str!("runtime_prelude/indexing.R"),
    "\n",
    include_str!("runtime_prelude/matrix_ops.R"),
    "\n",
    include_str!("runtime_prelude/array3_ops.R"),
    "\n",
    include_str!("runtime_prelude/records_closures.R"),
    "\n",
    include_str!("runtime_prelude/reductions.R"),
);
