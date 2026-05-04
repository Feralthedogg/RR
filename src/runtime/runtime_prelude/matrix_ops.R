# --- RR runtime: matrix helpers ---
rr_row_binop_assign <- function(dest, lhs_src, rhs_src, row, c_start, c_end, op, ctx="row_map") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }

  if (!is.matrix(dest)) {
    rr_type_error(paste0(ctx, " dest must be a matrix"), "E1002", ctx)
  }

  r <- to_i1(row, paste0(ctx, " row"))
  cs <- to_i1(c_start, paste0(ctx, " col_start"))
  ce <- to_i1(c_end, paste0(ctx, " col_end"))
  if (ce < cs) return(dest)
  if (r < 1L || r > nrow(dest)) {
    rr_bounds_error(paste0(ctx, " row out of bounds: ", r), "E2007", ctx)
  }
  if (cs < 1L || ce > ncol(dest)) {
    rr_bounds_error(
      paste0(ctx, " col range out of bounds: [", cs, ", ", ce, "]"),
      "E2007",
      ctx
    )
  }

  to_row_vec <- function(src, label) {
    if (is.matrix(src)) {
      if (r > nrow(src) || ce > ncol(src)) {
        rr_bounds_error(
          paste0(ctx, " ", label, " source out of bounds"),
          "E2007",
          ctx
        )
      }
      src[r, cs:ce]
    } else {
      if (length(src) != 1L) {
        rr_value_error(
          paste0(ctx, " ", label, " source must be scalar or matrix"),
          "E2001",
          ctx
        )
      }
      src
    }
  }

  lv <- to_row_vec(lhs_src, "lhs")
  rv <- to_row_vec(rhs_src, "rhs")

  out <- switch(
    as.character(op),
    "+" = lv + rv,
    "-" = lv - rv,
    "*" = lv * rv,
    "/" = lv / rv,
    "%%" = lv %% rv,
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )

  dest[r, cs:ce] <- out
  dest
}

rr_col_binop_assign <- function(dest, lhs_src, rhs_src, col, r_start, r_end, op, ctx="col_map") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }

  if (!is.matrix(dest)) {
    rr_type_error(paste0(ctx, " dest must be a matrix"), "E1002", ctx)
  }

  c <- to_i1(col, paste0(ctx, " col"))
  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  if (re < rs) return(dest)
  if (c < 1L || c > ncol(dest)) {
    rr_bounds_error(paste0(ctx, " col out of bounds: ", c), "E2007", ctx)
  }
  if (rs < 1L || re > nrow(dest)) {
    rr_bounds_error(
      paste0(ctx, " row range out of bounds: [", rs, ", ", re, "]"),
      "E2007",
      ctx
    )
  }

  to_col_vec <- function(src, label) {
    if (is.matrix(src)) {
      if (re > nrow(src) || c > ncol(src)) {
        rr_bounds_error(
          paste0(ctx, " ", label, " source out of bounds"),
          "E2007",
          ctx
        )
      }
      src[rs:re, c]
    } else {
      if (length(src) != 1L) {
        rr_value_error(
          paste0(ctx, " ", label, " source must be scalar or matrix"),
          "E2001",
          ctx
        )
      }
      src
    }
  }

  lv <- to_col_vec(lhs_src, "lhs")
  rv <- to_col_vec(rhs_src, "rhs")

  out <- switch(
    as.character(op),
    "+" = lv + rv,
    "-" = lv - rv,
    "*" = lv * rv,
    "/" = lv / rv,
    "%%" = lv %% rv,
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )

  dest[rs:re, c] <- out
  dest
}

rr_matrix_binop_assign <- function(dest, lhs_src, rhs_src, r_start, r_end, c_start, c_end, op, ctx="matrix_map") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }

  if (!is.matrix(dest)) {
    rr_type_error(paste0(ctx, " dest must be a matrix"), "E1002", ctx)
  }

  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  cs <- to_i1(c_start, paste0(ctx, " col_start"))
  ce <- to_i1(c_end, paste0(ctx, " col_end"))
  if (re < rs || ce < cs) return(dest)
  if (rs < 1L || re > nrow(dest) || cs < 1L || ce > ncol(dest)) {
    rr_bounds_error(
      paste0(ctx, " matrix range out of bounds: rows [", rs, ", ", re, "] cols [", cs, ", ", ce, "]"),
      "E2007",
      ctx
    )
  }

  to_matrix_slice <- function(src, label) {
    if (is.matrix(src)) {
      if (re > nrow(src) || ce > ncol(src)) {
        rr_bounds_error(
          paste0(ctx, " ", label, " source out of bounds"),
          "E2007",
          ctx
        )
      }
      src[rs:re, cs:ce]
    } else {
      if (length(src) != 1L) {
        rr_value_error(
          paste0(ctx, " ", label, " source must be scalar or matrix"),
          "E2001",
          ctx
        )
      }
      src
    }
  }

  lv <- to_matrix_slice(lhs_src, "lhs")
  rv <- to_matrix_slice(rhs_src, "rhs")

  out <- switch(
    as.character(op),
    "+" = lv + rv,
    "-" = lv - rv,
    "*" = lv * rv,
    "/" = lv / rv,
    "%%" = lv %% rv,
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )

  dest[rs:re, cs:ce] <- out
  dest
}

rr_tile_matrix_binop_assign <- function(dest, lhs_src, rhs_src, r_start, r_end, c_start, c_end, op, tile_r, tile_c, ctx="tile_matrix_map") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  rs <- to_i1(r_start, paste0(ctx, " row_start"))

  re <- to_i1(r_end, paste0(ctx, " row_end"))
  cs <- to_i1(c_start, paste0(ctx, " col_start"))
  ce <- to_i1(c_end, paste0(ctx, " col_end"))
  tr <- to_i1(tile_r, paste0(ctx, " tile_rows"))
  tc <- to_i1(tile_c, paste0(ctx, " tile_cols"))
  if (tr <= 0L || tc <= 0L) rr_value_error(paste0(ctx, " tile sizes must be > 0"), "E2001", ctx)
  if (re < rs || ce < cs) return(dest)
  r_cur <- rs
  while (r_cur <= re) {
    r_chunk_end <- min(re, r_cur + tr - 1L)
    c_cur <- cs
    while (c_cur <= ce) {
      c_chunk_end <- min(ce, c_cur + tc - 1L)
      dest <- rr_matrix_binop_assign(
        dest, lhs_src, rhs_src, r_cur, r_chunk_end, c_cur, c_chunk_end, op, ctx
      )
      c_cur <- c_chunk_end + 1L
    }
    r_cur <- r_chunk_end + 1L
  }
  dest
}

rr_row_sum_range <- function(base, row, c_start, c_end, ctx="row_sum") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  if (!is.matrix(base)) {
    rr_type_error(paste0(ctx, " base must be a matrix"), "E1002", ctx)
  }
  r <- to_i1(row, paste0(ctx, " row"))
  cs <- to_i1(c_start, paste0(ctx, " col_start"))
  ce <- to_i1(c_end, paste0(ctx, " col_end"))
  if (ce < cs) return(0)
  if (r < 1L || r > nrow(base)) {
    rr_bounds_error(paste0(ctx, " row out of bounds: ", r), "E2007", ctx)
  }
  if (cs < 1L || ce > ncol(base)) {
    rr_bounds_error(
      paste0(ctx, " col range out of bounds: [", cs, ", ", ce, "]"),
      "E2007",
      ctx
    )
  }
  sum(base[r, cs:ce])
}

rr_col_sum_range <- function(base, col, r_start, r_end, ctx="col_sum") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  if (!is.matrix(base)) {
    rr_type_error(paste0(ctx, " base must be a matrix"), "E1002", ctx)
  }
  c <- to_i1(col, paste0(ctx, " col"))
  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  if (re < rs) return(0)
  if (c < 1L || c > ncol(base)) {
    rr_bounds_error(paste0(ctx, " col out of bounds: ", c), "E2007", ctx)
  }
  if (rs < 1L || re > nrow(base)) {
    rr_bounds_error(
      paste0(ctx, " row range out of bounds: [", rs, ", ", re, "]"),
      "E2007",
      ctx
    )
  }
  sum(base[rs:re, c])
}

rr_col_reduce_range <- function(base, col, r_start, r_end, op, ctx="col_reduce") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  if (!is.matrix(base)) {
    rr_type_error(paste0(ctx, " base must be a matrix"), "E1002", ctx)
  }
  c <- to_i1(col, paste0(ctx, " col"))
  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  if (re < rs) {
    return(switch(
      as.character(op),
      "sum" = 0,
      "prod" = 1,
      "min" = Inf,
      "max" = -Inf,
      rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
    ))
  }
  if (c < 1L || c > ncol(base)) {
    rr_bounds_error(paste0(ctx, " col out of bounds: ", c), "E2007", ctx)
  }
  if (rs < 1L || re > nrow(base)) {
    rr_bounds_error(
      paste0(ctx, " row range out of bounds: [", rs, ", ", re, "]"),
      "E2007",
      ctx
    )
  }
  vals <- base[rs:re, c]
  switch(
    as.character(op),
    "sum" = sum(vals),
    "prod" = prod(vals),
    "min" = min(vals),
    "max" = max(vals),
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )
}

rr_tile_col_binop_assign <- function(dest, lhs_src, rhs_src, col, r_start, r_end, op, tile, ctx="tile_col_map") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  t <- to_i1(tile, paste0(ctx, " tile"))
  if (t <= 0L) rr_value_error(paste0(ctx, " tile must be > 0"), "E2001", ctx)
  if (re < rs) return(dest)
  cur <- rs
  while (cur <= re) {
    chunk_end <- min(re, cur + t - 1L)
    dest <- rr_col_binop_assign(dest, lhs_src, rhs_src, col, cur, chunk_end, op, ctx)
    cur <- chunk_end + 1L
  }
  dest
}

rr_tile_col_reduce_range <- function(base, col, r_start, r_end, op, tile, ctx="tile_col_reduce") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  t <- to_i1(tile, paste0(ctx, " tile"))
  if (t <= 0L) rr_value_error(paste0(ctx, " tile must be > 0"), "E2001", ctx)
  if (re < rs) {
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
  cur <- rs
  while (cur <= re) {
    chunk_end <- min(re, cur + t - 1L)
    chunk <- rr_col_reduce_range(base, col, cur, chunk_end, op, ctx)
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

rr_can_col_reduce_range <- function(base, col, r_start, r_end) {
  to_i1 <- function(v) {
    if (length(v) != 1L || is.na(v) || !is.numeric(v) || v != floor(v)) return(NA_integer_)
    as.integer(v)
  }
  if (!is.matrix(base)) return(FALSE)
  c <- to_i1(col)
  rs <- to_i1(r_start)
  re <- to_i1(r_end)
  if (is.na(c) || is.na(rs) || is.na(re)) return(FALSE)
  if (re < rs) return(TRUE)
  c >= 1L && c <= ncol(base) && rs >= 1L && re <= nrow(base)
}

rr_matrix_reduce_rect <- function(base, r_start, r_end, c_start, c_end, op, ctx="matrix_reduce") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  if (!is.matrix(base)) {
    rr_type_error(paste0(ctx, " base must be a matrix"), "E1002", ctx)
  }
  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  cs <- to_i1(c_start, paste0(ctx, " col_start"))
  ce <- to_i1(c_end, paste0(ctx, " col_end"))
  if (re < rs || ce < cs) {
    return(switch(
      as.character(op),
      "sum" = 0,
      "prod" = 1,
      "min" = Inf,
      "max" = -Inf,
      rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
    ))
  }
  if (rs < 1L || re > nrow(base) || cs < 1L || ce > ncol(base)) {
    rr_bounds_error(
      paste0(ctx, " matrix range out of bounds: rows [", rs, ", ", re, "] cols [", cs, ", ", ce, "]"),
      "E2007",
      ctx
    )
  }
  slice <- base[rs:re, cs:ce]
  switch(
    as.character(op),
    "sum" = sum(slice),
    "prod" = prod(slice),
    "min" = min(slice),
    "max" = max(slice),
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )
}

rr_tile_matrix_reduce_rect <- function(base, r_start, r_end, c_start, c_end, op, tile_r, tile_c, ctx="tile_matrix_reduce") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  rs <- to_i1(r_start, paste0(ctx, " row_start"))
  re <- to_i1(r_end, paste0(ctx, " row_end"))
  cs <- to_i1(c_start, paste0(ctx, " col_start"))
  ce <- to_i1(c_end, paste0(ctx, " col_end"))
  tr <- to_i1(tile_r, paste0(ctx, " tile_rows"))
  tc <- to_i1(tile_c, paste0(ctx, " tile_cols"))
  if (tr <= 0L || tc <= 0L) rr_value_error(paste0(ctx, " tile sizes must be > 0"), "E2001", ctx)
  if (re < rs || ce < cs) {
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
  r_cur <- rs
  while (r_cur <= re) {
    r_chunk_end <- min(re, r_cur + tr - 1L)
    c_cur <- cs
    while (c_cur <= ce) {
      c_chunk_end <- min(ce, c_cur + tc - 1L)
      chunk <- rr_matrix_reduce_rect(base, r_cur, r_chunk_end, c_cur, c_chunk_end, op, ctx)
      acc <- switch(
        as.character(op),
        "sum" = acc + chunk,
        "prod" = acc * chunk,
        "min" = min(acc, chunk),
        "max" = max(acc, chunk),
        rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
      )
      c_cur <- c_chunk_end + 1L
    }
    r_cur <- r_chunk_end + 1L
  }
  acc
}

rr_can_matrix_reduce_rect <- function(base, r_start, r_end, c_start, c_end) {
  to_i1 <- function(v) {
    if (length(v) != 1L || is.na(v) || !is.numeric(v) || v != floor(v)) return(NA_integer_)
    as.integer(v)
  }
  if (!is.matrix(base)) return(FALSE)
  rs <- to_i1(r_start)
  re <- to_i1(r_end)
  cs <- to_i1(c_start)
  ce <- to_i1(c_end)
  if (is.na(rs) || is.na(re) || is.na(cs) || is.na(ce)) return(FALSE)
  if (re < rs || ce < cs) return(TRUE)
  rs >= 1L && re <= nrow(base) && cs >= 1L && ce <= ncol(base)
}
