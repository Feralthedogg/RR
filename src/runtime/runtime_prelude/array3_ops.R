# --- RR runtime: 3D array helpers ---
rr_array3_dims <- function(base, ctx="array3") {
  if (!is.array(base) || length(dim(base)) != 3L) {
    rr_type_error(paste0(ctx, " base must be a rank-3 array"), "E1002", ctx)
  }
  as.integer(dim(base))
}

rr_array3_scalar_index <- function(v, what, ctx="array3") {
  if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", ctx)
  if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", ctx)
  if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", ctx)
  if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", ctx)
  as.integer(v)
}

rr_array3_axis_index_vec <- function(idx, limit, what, ctx="array3") {
  ii <- rr_index_vec_floor(idx, what)
  if (any(ii[!is.na(ii)] > limit)) {
    rr_bounds_error(
      paste0(what, " out of bounds for extent ", limit),
      "E2007",
      ctx
    )
  }
  ii
}

rr_array3_range_idx <- function(start, end, limit, what, ctx="array3") {
  s <- rr_array3_scalar_index(start, paste0(what, " start"), ctx)
  e <- rr_array3_scalar_index(end, paste0(what, " end"), ctx)
  if (e < s) return(integer(0))
  if (s < 1L || e > limit) {
    rr_bounds_error(
      paste0(what, " range out of bounds: [", s, ", ", e, "]"),
      "E2007",
      ctx
    )
  }
  seq.int(s, e)
}

rr_array3_normalize_values <- function(values, n, what, ctx="array3") {
  if (length(values) == 1L || length(values) == n) {
    return(values)
  }
  rr_value_error(
    paste0(what, " length mismatch (", length(values), " vs ", n, ")"),
    "E2001",
    ctx
  )
}

rr_array3_materialize_axis_arg <- function(src, axis, fixed_a, fixed_b, idx, label, ctx="array3") {
  if (is.array(src) && length(dim(src)) == 3L) {
    dims <- rr_array3_dims(src, ctx)
    fa <- rr_array3_scalar_index(fixed_a, paste0(label, " fixed_a"), ctx)
    fb <- rr_array3_scalar_index(fixed_b, paste0(label, " fixed_b"), ctx)
    ii <- rr_array3_axis_index_vec(idx, dims[[axis]], paste0(label, " idx"), ctx)
    if (axis == 1L) {
      if (fa < 1L || fa > dims[[2L]] || fb < 1L || fb > dims[[3L]]) {
        rr_bounds_error(paste0(label, " fixed coordinates out of bounds"), "E2007", ctx)
      }
      return(src[ii, fa, fb])
    }
    if (axis == 2L) {
      if (fa < 1L || fa > dims[[1L]] || fb < 1L || fb > dims[[3L]]) {
        rr_bounds_error(paste0(label, " fixed coordinates out of bounds"), "E2007", ctx)
      }
      return(src[fa, ii, fb])
    }
    if (fa < 1L || fa > dims[[1L]] || fb < 1L || fb > dims[[2L]]) {
      rr_bounds_error(paste0(label, " fixed coordinates out of bounds"), "E2007", ctx)
    }
    return(src[fa, fb, ii])
  }
  rr_array3_normalize_values(src, length(idx), label, ctx)
}

rr_array3_assign_axis <- function(dest, values, axis, fixed_a, fixed_b, idx, ctx="array3_assign") {
  dims <- rr_array3_dims(dest, ctx)
  fa <- rr_array3_scalar_index(fixed_a, paste0(ctx, " fixed_a"), ctx)
  fb <- rr_array3_scalar_index(fixed_b, paste0(ctx, " fixed_b"), ctx)
  ii <- rr_array3_axis_index_vec(idx, dims[[axis]], paste0(ctx, " idx"), ctx)
  vv <- rr_array3_normalize_values(values, length(ii), paste0(ctx, " values"), ctx)
  if (axis == 1L) {
    if (fa < 1L || fa > dims[[2L]] || fb < 1L || fb > dims[[3L]]) {
      rr_bounds_error(paste0(ctx, " fixed coordinates out of bounds"), "E2007", ctx)
    }
    dest[ii, fa, fb] <- vv
    return(dest)
  }
  if (axis == 2L) {
    if (fa < 1L || fa > dims[[1L]] || fb < 1L || fb > dims[[3L]]) {
      rr_bounds_error(paste0(ctx, " fixed coordinates out of bounds"), "E2007", ctx)
    }
    dest[fa, ii, fb] <- vv
    return(dest)
  }
  if (fa < 1L || fa > dims[[1L]] || fb < 1L || fb > dims[[2L]]) {
    rr_bounds_error(paste0(ctx, " fixed coordinates out of bounds"), "E2007", ctx)
  }
  dest[fa, fb, ii] <- vv
  dest
}

rr_array3_binop <- function(lhs, rhs, op, ctx="array3_binop") {
  switch(
    as.character(op),
    "+" = lhs + rhs,
    "-" = lhs - rhs,
    "*" = lhs * rhs,
    "/" = lhs / rhs,
    "%%" = lhs %% rhs,
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )
}

rr_array3_compare <- function(lhs, rhs, cmp, ctx="array3_compare") {
  switch(
    as.character(cmp),
    "<" = lhs < rhs,
    "<=" = lhs <= rhs,
    ">" = lhs > rhs,
    ">=" = lhs >= rhs,
    "==" = lhs == rhs,
    "!=" = lhs != rhs,
    rr_value_error(paste0(ctx, " unsupported cmp: ", cmp), "E2001", ctx)
  )
}

rr_array3_reduce_apply <- function(values, op, ctx="array3_reduce") {
  switch(
    as.character(op),
    "sum" = sum(values),
    "prod" = prod(values),
    "min" = min(values),
    "max" = max(values),
    rr_value_error(paste0(ctx, " unsupported reduce op: ", op), "E2001", ctx)
  )
}

rr_array3_expand_gather_arg <- function(idx, n, limit, what, ctx="array3_gather") {
  if (length(idx) == 1L) {
    idx <- rep.int(idx, n)
  } else if (length(idx) != n) {
    rr_value_error(
      paste0(what, " length mismatch (", length(idx), " vs ", n, ")"),
      "E2001",
      ctx
    )
  }
  rr_array3_axis_index_vec(idx, limit, what, ctx)
}

rr_array3_gather_values <- function(base, i, j, k, ctx="array3_gather") {
  dims <- rr_array3_dims(base, ctx)
  n <- max(length(i), length(j), length(k))
  if (n <= 0L) return(base[integer(0)])
  ii <- rr_array3_expand_gather_arg(i, n, dims[[1L]], paste0(ctx, " dim1"), ctx)
  jj <- rr_array3_expand_gather_arg(j, n, dims[[2L]], paste0(ctx, " dim2"), ctx)
  kk <- rr_array3_expand_gather_arg(k, n, dims[[3L]], paste0(ctx, " dim3"), ctx)
  base[cbind(ii, jj, kk)]
}

rr_array3_assign_gather_values <- function(dest, values, i, j, k, ctx="array3_scatter") {
  dims <- rr_array3_dims(dest, ctx)
  n <- max(length(i), length(j), length(k))
  if (n <= 0L) return(dest)
  ii <- rr_array3_expand_gather_arg(i, n, dims[[1L]], paste0(ctx, " dim1"), ctx)
  jj <- rr_array3_expand_gather_arg(j, n, dims[[2L]], paste0(ctx, " dim2"), ctx)
  kk <- rr_array3_expand_gather_arg(k, n, dims[[3L]], paste0(ctx, " dim3"), ctx)
  vv <- rr_array3_normalize_values(values, n, paste0(ctx, " values"), ctx)
  dest[cbind(ii, jj, kk)] <- vv
  dest
}

rr_dim1_read_values <- function(base, fixed_a, fixed_b, idx, ctx="dim1_read") {
  rr_array3_materialize_axis_arg(base, 1L, fixed_a, fixed_b, idx, "base", ctx)
}

rr_dim2_read_values <- function(base, fixed_a, fixed_b, idx, ctx="dim2_read") {
  rr_array3_materialize_axis_arg(base, 2L, fixed_a, fixed_b, idx, "base", ctx)
}

rr_dim3_read_values <- function(base, fixed_a, fixed_b, idx, ctx="dim3_read") {
  rr_array3_materialize_axis_arg(base, 3L, fixed_a, fixed_b, idx, "base", ctx)
}

rr_dim1_assign_values <- function(dest, values, fixed_a, fixed_b, start, end, ctx="dim1_assign") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[1L]], "dim1", ctx)
  rr_array3_assign_axis(dest, values, 1L, fixed_a, fixed_b, idx, ctx)
}

rr_dim2_assign_values <- function(dest, values, fixed_a, fixed_b, start, end, ctx="dim2_assign") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[2L]], "dim2", ctx)
  rr_array3_assign_axis(dest, values, 2L, fixed_a, fixed_b, idx, ctx)
}

rr_dim3_assign_values <- function(dest, values, fixed_a, fixed_b, start, end, ctx="dim3_assign") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[3L]], "dim3", ctx)
  rr_array3_assign_axis(dest, values, 3L, fixed_a, fixed_b, idx, ctx)
}

rr_dim1_binop_assign <- function(dest, lhs_src, rhs_src, fixed_a, fixed_b, start, end, op, ctx="dim1_map") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[1L]], "dim1", ctx)
  lv <- rr_array3_materialize_axis_arg(lhs_src, 1L, fixed_a, fixed_b, idx, "lhs", ctx)
  rv <- rr_array3_materialize_axis_arg(rhs_src, 1L, fixed_a, fixed_b, idx, "rhs", ctx)
  rr_array3_assign_axis(dest, rr_array3_binop(lv, rv, op, ctx), 1L, fixed_a, fixed_b, idx, ctx)
}

rr_dim2_binop_assign <- function(dest, lhs_src, rhs_src, fixed_a, fixed_b, start, end, op, ctx="dim2_map") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[2L]], "dim2", ctx)
  lv <- rr_array3_materialize_axis_arg(lhs_src, 2L, fixed_a, fixed_b, idx, "lhs", ctx)
  rv <- rr_array3_materialize_axis_arg(rhs_src, 2L, fixed_a, fixed_b, idx, "rhs", ctx)
  rr_array3_assign_axis(dest, rr_array3_binop(lv, rv, op, ctx), 2L, fixed_a, fixed_b, idx, ctx)
}

rr_dim3_binop_assign <- function(dest, lhs_src, rhs_src, fixed_a, fixed_b, start, end, op, ctx="dim3_map") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[3L]], "dim3", ctx)
  lv <- rr_array3_materialize_axis_arg(lhs_src, 3L, fixed_a, fixed_b, idx, "lhs", ctx)
  rv <- rr_array3_materialize_axis_arg(rhs_src, 3L, fixed_a, fixed_b, idx, "rhs", ctx)
  rr_array3_assign_axis(dest, rr_array3_binop(lv, rv, op, ctx), 3L, fixed_a, fixed_b, idx, ctx)
}

rr_array3_binop_cube_assign <- function(dest, lhs_src, rhs_src, i_start, i_end, j_start, j_end, k_start, k_end, op, ctx="array3_cube_map") {
  dims <- rr_array3_dims(dest, ctx)
  ii <- rr_array3_range_idx(i_start, i_end, dims[[1L]], "dim1", ctx)
  jj <- rr_array3_range_idx(j_start, j_end, dims[[2L]], "dim2", ctx)
  kk <- rr_array3_range_idx(k_start, k_end, dims[[3L]], "dim3", ctx)
  if (length(ii) == 0L || length(jj) == 0L || length(kk) == 0L) return(dest)

  to_cube_slice <- function(src, label) {
    if (is.array(src) && length(dim(src)) == 3L) {
      sd <- rr_array3_dims(src, ctx)
      if (max(ii) > sd[[1L]] || max(jj) > sd[[2L]] || max(kk) > sd[[3L]]) {
        rr_bounds_error(
          paste0(ctx, " ", label, " source out of bounds"),
          "E2007",
          ctx
        )
      }
      return(src[ii, jj, kk, drop = FALSE])
    }
    if (length(src) != 1L) {
      rr_value_error(
        paste0(ctx, " ", label, " source must be scalar or rank-3 array"),
        "E2001",
        ctx
      )
    }
    src
  }

  lv <- to_cube_slice(lhs_src, "lhs")
  rv <- to_cube_slice(rhs_src, "rhs")
  dest[ii, jj, kk] <- rr_array3_binop(lv, rv, op, ctx)
  dest
}

rr_tile_array3_binop_cube_assign <- function(dest, lhs_src, rhs_src, i_start, i_end, j_start, j_end, k_start, k_end, op, tile_i, tile_j, tile_k, ctx="tile_array3_cube_map") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  is <- to_i1(i_start, paste0(ctx, " dim1_start"))
  ie <- to_i1(i_end, paste0(ctx, " dim1_end"))
  js <- to_i1(j_start, paste0(ctx, " dim2_start"))
  je <- to_i1(j_end, paste0(ctx, " dim2_end"))
  ks <- to_i1(k_start, paste0(ctx, " dim3_start"))
  ke <- to_i1(k_end, paste0(ctx, " dim3_end"))
  ti <- to_i1(tile_i, paste0(ctx, " tile_dim1"))
  tj <- to_i1(tile_j, paste0(ctx, " tile_dim2"))
  tk <- to_i1(tile_k, paste0(ctx, " tile_dim3"))
  if (ti <= 0L || tj <= 0L || tk <= 0L) {
    rr_value_error(paste0(ctx, " tile sizes must be > 0"), "E2001", ctx)
  }
  if (ie < is || je < js || ke < ks) return(dest)
  i_cur <- is
  while (i_cur <= ie) {
    i_chunk_end <- min(ie, i_cur + ti - 1L)
    j_cur <- js
    while (j_cur <= je) {
      j_chunk_end <- min(je, j_cur + tj - 1L)
      k_cur <- ks
      while (k_cur <= ke) {
        k_chunk_end <- min(ke, k_cur + tk - 1L)
        dest <- rr_array3_binop_cube_assign(
          dest, lhs_src, rhs_src,
          i_cur, i_chunk_end,
          j_cur, j_chunk_end,
          k_cur, k_chunk_end,
          op, ctx
        )
        k_cur <- k_chunk_end + 1L
      }
      j_cur <- j_chunk_end + 1L
    }
    i_cur <- i_chunk_end + 1L
  }
  dest
}

rr_dim1_ifelse_assign <- function(dest, cond_lhs, cond_rhs, cmp, then_src, else_src, fixed_a, fixed_b, start, end, ctx="dim1_ifelse") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[1L]], "dim1", ctx)
  lhs <- rr_array3_materialize_axis_arg(cond_lhs, 1L, fixed_a, fixed_b, idx, "cond_lhs", ctx)
  rhs <- rr_array3_materialize_axis_arg(cond_rhs, 1L, fixed_a, fixed_b, idx, "cond_rhs", ctx)
  yes <- rr_array3_materialize_axis_arg(then_src, 1L, fixed_a, fixed_b, idx, "then", ctx)
  no <- rr_array3_materialize_axis_arg(else_src, 1L, fixed_a, fixed_b, idx, "else", ctx)
  cond <- rr_array3_compare(lhs, rhs, cmp, ctx)
  rr_array3_assign_axis(dest, rr_ifelse_strict(cond, yes, no, ctx), 1L, fixed_a, fixed_b, idx, ctx)
}

rr_dim2_ifelse_assign <- function(dest, cond_lhs, cond_rhs, cmp, then_src, else_src, fixed_a, fixed_b, start, end, ctx="dim2_ifelse") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[2L]], "dim2", ctx)
  lhs <- rr_array3_materialize_axis_arg(cond_lhs, 2L, fixed_a, fixed_b, idx, "cond_lhs", ctx)
  rhs <- rr_array3_materialize_axis_arg(cond_rhs, 2L, fixed_a, fixed_b, idx, "cond_rhs", ctx)
  yes <- rr_array3_materialize_axis_arg(then_src, 2L, fixed_a, fixed_b, idx, "then", ctx)
  no <- rr_array3_materialize_axis_arg(else_src, 2L, fixed_a, fixed_b, idx, "else", ctx)
  cond <- rr_array3_compare(lhs, rhs, cmp, ctx)
  rr_array3_assign_axis(dest, rr_ifelse_strict(cond, yes, no, ctx), 2L, fixed_a, fixed_b, idx, ctx)
}

rr_dim3_ifelse_assign <- function(dest, cond_lhs, cond_rhs, cmp, then_src, else_src, fixed_a, fixed_b, start, end, ctx="dim3_ifelse") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[3L]], "dim3", ctx)
  lhs <- rr_array3_materialize_axis_arg(cond_lhs, 3L, fixed_a, fixed_b, idx, "cond_lhs", ctx)
  rhs <- rr_array3_materialize_axis_arg(cond_rhs, 3L, fixed_a, fixed_b, idx, "cond_rhs", ctx)
  yes <- rr_array3_materialize_axis_arg(then_src, 3L, fixed_a, fixed_b, idx, "then", ctx)
  no <- rr_array3_materialize_axis_arg(else_src, 3L, fixed_a, fixed_b, idx, "else", ctx)
  cond <- rr_array3_compare(lhs, rhs, cmp, ctx)
  rr_array3_assign_axis(dest, rr_ifelse_strict(cond, yes, no, ctx), 3L, fixed_a, fixed_b, idx, ctx)
}

rr_dim1_call_assign <- function(dest, callee, fixed_a, fixed_b, start, end, ..., ctx="dim1_call") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[1L]], "dim1", ctx)
  args <- lapply(list(...), function(arg) rr_array3_materialize_axis_arg(arg, 1L, fixed_a, fixed_b, idx, "arg", ctx))
  out <- rr_call_map_vector_eval(as.character(callee), args)
  rr_array3_assign_axis(dest, out, 1L, fixed_a, fixed_b, idx, ctx)
}

rr_dim2_call_assign <- function(dest, callee, fixed_a, fixed_b, start, end, ..., ctx="dim2_call") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[2L]], "dim2", ctx)
  args <- lapply(list(...), function(arg) rr_array3_materialize_axis_arg(arg, 2L, fixed_a, fixed_b, idx, "arg", ctx))
  out <- rr_call_map_vector_eval(as.character(callee), args)
  rr_array3_assign_axis(dest, out, 2L, fixed_a, fixed_b, idx, ctx)
}

rr_dim3_call_assign <- function(dest, callee, fixed_a, fixed_b, start, end, ..., ctx="dim3_call") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(dest, ctx)[[3L]], "dim3", ctx)
  args <- lapply(list(...), function(arg) rr_array3_materialize_axis_arg(arg, 3L, fixed_a, fixed_b, idx, "arg", ctx))
  out <- rr_call_map_vector_eval(as.character(callee), args)
  rr_array3_assign_axis(dest, out, 3L, fixed_a, fixed_b, idx, ctx)
}

rr_dim1_assign_index_values <- function(dest, values, fixed_a, fixed_b, idx, ctx="dim1_scatter") {
  rr_array3_assign_axis(dest, values, 1L, fixed_a, fixed_b, idx, ctx)
}

rr_dim2_assign_index_values <- function(dest, values, fixed_a, fixed_b, idx, ctx="dim2_scatter") {
  rr_array3_assign_axis(dest, values, 2L, fixed_a, fixed_b, idx, ctx)
}

rr_dim3_assign_index_values <- function(dest, values, fixed_a, fixed_b, idx, ctx="dim3_scatter") {
  rr_array3_assign_axis(dest, values, 3L, fixed_a, fixed_b, idx, ctx)
}

rr_dim1_sum_range <- function(base, fixed_a, fixed_b, start, end, ctx="dim1_sum") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[1L]], "dim1", ctx)
  sum(rr_array3_materialize_axis_arg(base, 1L, fixed_a, fixed_b, idx, "base", ctx))
}

rr_dim2_sum_range <- function(base, fixed_a, fixed_b, start, end, ctx="dim2_sum") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[2L]], "dim2", ctx)
  sum(rr_array3_materialize_axis_arg(base, 2L, fixed_a, fixed_b, idx, "base", ctx))
}

rr_dim3_sum_range <- function(base, fixed_a, fixed_b, start, end, ctx="dim3_sum") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[3L]], "dim3", ctx)
  sum(rr_array3_materialize_axis_arg(base, 3L, fixed_a, fixed_b, idx, "base", ctx))
}

rr_dim1_reduce_range <- function(base, fixed_a, fixed_b, start, end, op, ctx="dim1_reduce") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[1L]], "dim1", ctx)
  rr_array3_reduce_apply(rr_array3_materialize_axis_arg(base, 1L, fixed_a, fixed_b, idx, "base", ctx), op, ctx)
}

rr_tile_dim1_binop_assign <- function(dest, lhs_src, rhs_src, fixed_a, fixed_b, start, end, op, tile, ctx="tile_dim1_map") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  s <- to_i1(start, paste0(ctx, " start"))
  e <- to_i1(end, paste0(ctx, " end"))
  t <- to_i1(tile, paste0(ctx, " tile"))
  if (t <= 0L) rr_value_error(paste0(ctx, " tile must be > 0"), "E2001", ctx)
  if (e < s) return(dest)
  cur <- s
  while (cur <= e) {
    chunk_end <- min(e, cur + t - 1L)
    dest <- rr_dim1_binop_assign(dest, lhs_src, rhs_src, fixed_a, fixed_b, cur, chunk_end, op, ctx)
    cur <- chunk_end + 1L
  }
  dest
}

rr_tile_dim1_reduce_range <- function(base, fixed_a, fixed_b, start, end, op, tile, ctx="tile_dim1_reduce") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  s <- to_i1(start, paste0(ctx, " start"))
  e <- to_i1(end, paste0(ctx, " end"))
  t <- to_i1(tile, paste0(ctx, " tile"))
  if (t <= 0L) rr_value_error(paste0(ctx, " tile must be > 0"), "E2001", ctx)
  if (e < s) {
    return(switch(
      as.character(op),
      "sum" = 0,
      "prod" = 1,
      "min" = Inf,
      "max" = -Inf,
      rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
    ))
  }
  acc <- switch(
    as.character(op),
    "sum" = 0,
    "prod" = 1,
    "min" = Inf,
    "max" = -Inf,
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )
  cur <- s
  while (cur <= e) {
    chunk_end <- min(e, cur + t - 1L)
    chunk <- rr_dim1_reduce_range(base, fixed_a, fixed_b, cur, chunk_end, op, ctx)
    acc <- switch(
      as.character(op),
      "sum" = acc + chunk,
      "prod" = acc * chunk,
      "min" = min(acc, chunk),
      "max" = max(acc, chunk),
      rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
    )
    cur <- chunk_end + 1L
  }
  acc
}

rr_can_dim1_reduce_range <- function(base, fixed_a, fixed_b, start, end) {
  if (!is.array(base) || length(dim(base)) != 3L) return(FALSE)
  to_i1 <- function(v) {
    if (length(v) != 1L || is.na(v) || !is.numeric(v) || v != floor(v)) return(NA_integer_)
    as.integer(v)
  }
  dims <- as.integer(dim(base))
  fa <- to_i1(fixed_a)
  fb <- to_i1(fixed_b)
  s <- to_i1(start)
  e <- to_i1(end)
  if (is.na(fa) || is.na(fb) || is.na(s) || is.na(e)) return(FALSE)
  if (fa < 1L || fa > dims[[2L]] || fb < 1L || fb > dims[[3L]]) return(FALSE)
  if (e < s) return(TRUE)
  s >= 1L && e <= dims[[1L]]
}

rr_can_array3_reduce_cube <- function(base, i_start, i_end, j_start, j_end, k_start, k_end) {
  if (!is.array(base) || length(dim(base)) != 3L) return(FALSE)
  to_i1 <- function(v) {
    if (length(v) != 1L || is.na(v) || !is.numeric(v) || v != floor(v)) return(NA_integer_)
    as.integer(v)
  }
  dims <- as.integer(dim(base))
  is <- to_i1(i_start)
  ie <- to_i1(i_end)
  js <- to_i1(j_start)
  je <- to_i1(j_end)
  ks <- to_i1(k_start)
  ke <- to_i1(k_end)
  if (is.na(is) || is.na(ie) || is.na(js) || is.na(je) || is.na(ks) || is.na(ke)) return(FALSE)
  if (ie < is || je < js || ke < ks) return(TRUE)
  is >= 1L && ie <= dims[[1L]] &&
    js >= 1L && je <= dims[[2L]] &&
    ks >= 1L && ke <= dims[[3L]]
}

rr_dim2_reduce_range <- function(base, fixed_a, fixed_b, start, end, op, ctx="dim2_reduce") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[2L]], "dim2", ctx)
  rr_array3_reduce_apply(rr_array3_materialize_axis_arg(base, 2L, fixed_a, fixed_b, idx, "base", ctx), op, ctx)
}

rr_dim3_reduce_range <- function(base, fixed_a, fixed_b, start, end, op, ctx="dim3_reduce") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[3L]], "dim3", ctx)
  rr_array3_reduce_apply(rr_array3_materialize_axis_arg(base, 3L, fixed_a, fixed_b, idx, "base", ctx), op, ctx)
}

rr_array3_reduce_cube <- function(base, i_start, i_end, j_start, j_end, k_start, k_end, op, ctx="array3_cube_reduce") {
  dims <- rr_array3_dims(base, ctx)
  ii <- rr_array3_range_idx(i_start, i_end, dims[[1L]], "dim1", ctx)
  jj <- rr_array3_range_idx(j_start, j_end, dims[[2L]], "dim2", ctx)
  kk <- rr_array3_range_idx(k_start, k_end, dims[[3L]], "dim3", ctx)
  if (length(ii) == 0L || length(jj) == 0L || length(kk) == 0L) {
    return(switch(
      as.character(op),
      "sum" = 0,
      "prod" = 1,
      "min" = Inf,
      "max" = -Inf,
      rr_value_error(paste0(ctx, " unsupported reduce op: ", op), "E2001", ctx)
    ))
  }
  rr_array3_reduce_apply(base[ii, jj, kk, drop = FALSE], op, ctx)
}

rr_tile_array3_reduce_cube <- function(base, i_start, i_end, j_start, j_end, k_start, k_end, op, tile_i, tile_j, tile_k, ctx="tile_array3_cube_reduce") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  is <- to_i1(i_start, paste0(ctx, " dim1_start"))
  ie <- to_i1(i_end, paste0(ctx, " dim1_end"))
  js <- to_i1(j_start, paste0(ctx, " dim2_start"))
  je <- to_i1(j_end, paste0(ctx, " dim2_end"))
  ks <- to_i1(k_start, paste0(ctx, " dim3_start"))
  ke <- to_i1(k_end, paste0(ctx, " dim3_end"))
  ti <- to_i1(tile_i, paste0(ctx, " tile_dim1"))
  tj <- to_i1(tile_j, paste0(ctx, " tile_dim2"))
  tk <- to_i1(tile_k, paste0(ctx, " tile_dim3"))
  if (ti <= 0L || tj <= 0L || tk <= 0L) {
    rr_value_error(paste0(ctx, " tile sizes must be > 0"), "E2001", ctx)
  }
  if (ie < is || je < js || ke < ks) {
    return(switch(
      as.character(op),
      "sum" = 0,
      "prod" = 1,
      "min" = Inf,
      "max" = -Inf,
      rr_value_error(paste0(ctx, " unsupported reduce op: ", op), "E2001", ctx)
    ))
  }
  acc <- switch(
    as.character(op),
    "sum" = 0,
    "prod" = 1,
    "min" = Inf,
    "max" = -Inf,
    rr_value_error(paste0(ctx, " unsupported reduce op: ", op), "E2001", ctx)
  )
  i_cur <- is
  while (i_cur <= ie) {
    i_chunk_end <- min(ie, i_cur + ti - 1L)
    j_cur <- js
    while (j_cur <= je) {
      j_chunk_end <- min(je, j_cur + tj - 1L)
      k_cur <- ks
      while (k_cur <= ke) {
        k_chunk_end <- min(ke, k_cur + tk - 1L)
        chunk <- rr_array3_reduce_cube(
          base,
          i_cur, i_chunk_end,
          j_cur, j_chunk_end,
          k_cur, k_chunk_end,
          op, ctx
        )
        acc <- switch(
          as.character(op),
          "sum" = acc + chunk,

          "prod" = acc * chunk,
          "min" = min(acc, chunk),
          "max" = max(acc, chunk),
          rr_value_error(paste0(ctx, " unsupported reduce op: ", op), "E2001", ctx)
        )
        k_cur <- k_chunk_end + 1L
      }
      j_cur <- j_chunk_end + 1L
    }
    i_cur <- i_chunk_end + 1L
  }
  acc
}

rr_dim1_shift_assign <- function(dest, src, fixed_a, fixed_b, d_start, d_end, s_start, s_end, ctx="dim1_shift") {
  dest_dims <- rr_array3_dims(dest, ctx)
  src_dims <- rr_array3_dims(src, ctx)
  d_idx <- rr_array3_range_idx(d_start, d_end, dest_dims[[1L]], "dim1 dest", ctx)
  s_idx <- rr_array3_range_idx(s_start, s_end, src_dims[[1L]], "dim1 src", ctx)
  if (length(d_idx) != length(s_idx)) rr_value_error(paste0(ctx, " length mismatch"), "E2001", ctx)
  vals <- rr_array3_materialize_axis_arg(src, 1L, fixed_a, fixed_b, s_idx, "src", ctx)
  rr_array3_assign_axis(dest, vals, 1L, fixed_a, fixed_b, d_idx, ctx)
}

rr_dim2_shift_assign <- function(dest, src, fixed_a, fixed_b, d_start, d_end, s_start, s_end, ctx="dim2_shift") {
  dest_dims <- rr_array3_dims(dest, ctx)
  src_dims <- rr_array3_dims(src, ctx)
  d_idx <- rr_array3_range_idx(d_start, d_end, dest_dims[[2L]], "dim2 dest", ctx)
  s_idx <- rr_array3_range_idx(s_start, s_end, src_dims[[2L]], "dim2 src", ctx)
  if (length(d_idx) != length(s_idx)) rr_value_error(paste0(ctx, " length mismatch"), "E2001", ctx)
  vals <- rr_array3_materialize_axis_arg(src, 2L, fixed_a, fixed_b, s_idx, "src", ctx)
  rr_array3_assign_axis(dest, vals, 2L, fixed_a, fixed_b, d_idx, ctx)
}

rr_dim3_shift_assign <- function(dest, src, fixed_a, fixed_b, d_start, d_end, s_start, s_end, ctx="dim3_shift") {
  dest_dims <- rr_array3_dims(dest, ctx)
  src_dims <- rr_array3_dims(src, ctx)
  d_idx <- rr_array3_range_idx(d_start, d_end, dest_dims[[3L]], "dim3 dest", ctx)
  s_idx <- rr_array3_range_idx(s_start, s_end, src_dims[[3L]], "dim3 src", ctx)
  if (length(d_idx) != length(s_idx)) rr_value_error(paste0(ctx, " length mismatch"), "E2001", ctx)
  vals <- rr_array3_materialize_axis_arg(src, 3L, fixed_a, fixed_b, s_idx, "src", ctx)
  rr_array3_assign_axis(dest, vals, 3L, fixed_a, fixed_b, d_idx, ctx)
}

rr_dim1_recur_add_const <- function(base, fixed_a, fixed_b, start, end, delta, ctx="dim1_recur") {
  dims <- rr_array3_dims(base, ctx)
  s <- rr_array3_scalar_index(start, paste0(ctx, " start"), ctx)
  e <- rr_array3_scalar_index(end, paste0(ctx, " end"), ctx)
  if (e < s) return(base)
  if (s <= 1L) rr_bounds_error("recurrence requires start >= 2", "E2007", ctx)
  idx <- rr_array3_range_idx(s, e, dims[[1L]], "dim1", ctx)
  prev <- rr_array3_materialize_axis_arg(base, 1L, fixed_a, fixed_b, s - 1L, "base", ctx)
  step <- as.numeric(delta)
  vals <- as.numeric(prev) + cumsum(rep(step, length(idx)))
  rr_array3_assign_axis(base, vals, 1L, fixed_a, fixed_b, idx, ctx)
}

rr_dim2_recur_add_const <- function(base, fixed_a, fixed_b, start, end, delta, ctx="dim2_recur") {
  dims <- rr_array3_dims(base, ctx)
  s <- rr_array3_scalar_index(start, paste0(ctx, " start"), ctx)
  e <- rr_array3_scalar_index(end, paste0(ctx, " end"), ctx)
  if (e < s) return(base)
  if (s <= 1L) rr_bounds_error("recurrence requires start >= 2", "E2007", ctx)
  idx <- rr_array3_range_idx(s, e, dims[[2L]], "dim2", ctx)
  prev <- rr_array3_materialize_axis_arg(base, 2L, fixed_a, fixed_b, s - 1L, "base", ctx)
  step <- as.numeric(delta)
  vals <- as.numeric(prev) + cumsum(rep(step, length(idx)))
  rr_array3_assign_axis(base, vals, 2L, fixed_a, fixed_b, idx, ctx)
}

rr_dim3_recur_add_const <- function(base, fixed_a, fixed_b, start, end, delta, ctx="dim3_recur") {
  dims <- rr_array3_dims(base, ctx)
  s <- rr_array3_scalar_index(start, paste0(ctx, " start"), ctx)
  e <- rr_array3_scalar_index(end, paste0(ctx, " end"), ctx)
  if (e < s) return(base)
  if (s <= 1L) rr_bounds_error("recurrence requires start >= 2", "E2007", ctx)
  idx <- rr_array3_range_idx(s, e, dims[[3L]], "dim3", ctx)
  prev <- rr_array3_materialize_axis_arg(base, 3L, fixed_a, fixed_b, s - 1L, "base", ctx)
  step <- as.numeric(delta)
  vals <- as.numeric(prev) + cumsum(rep(step, length(idx)))
  rr_array3_assign_axis(base, vals, 3L, fixed_a, fixed_b, idx, ctx)
}
