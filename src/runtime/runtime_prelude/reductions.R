# --- RR runtime: release fast paths and range reductions ---
# Fast-path rebinding for release mode:
# avoid per-call branch/check overhead in hot loops when compiler guarantees safety.
if (.rr_env$fast_runtime) {
  rr_mark <- function(line, col) invisible(NULL)
  rr_bool <- function(x, ctx="condition") isTRUE(x)
  rr_truthy1 <- rr_bool
  rr_i0 <- function(i, ctx="index") as.integer(i)
  rr_i0_read <- function(i, ctx="index") as.integer(i)
  rr_i1 <- function(i, ctx="index") as.integer(i)
  rr_index1_read <- function(base, i, ctx="index") base[as.integer(i)]
  rr_index1_read_idx <- function(base, i, ctx="index") {
    v <- base[as.integer(i)]
    if (is.integer(v)) v else as.integer(floor(v))
  }
  rr_index_vec_floor <- function(i, ctx="index") {
    if (is.integer(i)) i else as.integer(floor(i))
  }
  rr_index1_read_vec <- function(base, i, ctx="index") {
    if (is.integer(i)) base[i] else base[as.integer(i)]
  }
  rr_index1_read_vec_floor <- function(base, i, ctx="index") {
    base[rr_index_vec_floor(i, ctx)]
  }
  rr_wrap_index_vec <- function(x, y, w, h, ctx="wrap_index") {
    xx <- ifelse(x < 1, w, x)
    xx <- ifelse(xx > w, 1, xx)
    yy <- ifelse(y < 1, h, y)
    yy <- ifelse(yy > h, 1, yy)
    ((yy - 1) * w) + xx
  }
  rr_wrap_index_vec_i <- function(x, y, w, h, ctx="wrap_index") {
    as.integer(rr_wrap_index_vec(x, y, w, h, ctx))
  }
  rr_idx_cube_vec_i <- function(f, x, y, size, ctx="cube_index") {
    rr_round_rr <- function(v) {
      r <- v %% 1
      out <- v - r
      ifelse(r >= 0.5, out + 1, out)
    }
    ff <- rr_round_rr(f)
    xx <- rr_round_rr(x)
    yy <- rr_round_rr(y)
    ss <- rr_round_rr(size)
    ff <- pmin(pmax(ff, 1), 6)
    xx <- pmin(pmax(xx, 1), ss)
    yy <- pmin(pmax(yy, 1), ss)
    as.integer((((ff - 1) * ss) * ss) + ((xx - 1) * ss) + yy)
  }
  rr_index1_write <- function(i, ctx="index") as.integer(i)
  rr_assign_slice <- function(dest, start, end, values, ctx="slice_assign") {
    s <- as.integer(start)
    e <- as.integer(end)
    if (e < s) return(dest)
    dest[s:e] <- values
    dest
  }
  rr_assign_index_vec <- function(dest, idx, values, ctx="index_assign") {
    ii <- if (is.integer(idx)) idx else as.integer(idx)
    dest[ii] <- values
    dest
  }
}

rr_can_same_or_scalar <- function(a, b) {
  la <- length(a); lb <- length(b)
  la == lb || la == 1L || lb == 1L || la == 0L || lb == 0L
}

rr_reduce_range <- function(base, start, end, op, ctx="reduce_range") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }
  s <- to_i1(start, paste0(ctx, " start"))
  e <- to_i1(end, paste0(ctx, " end"))
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
  if (s < 1L || e > length(base)) {
    rr_bounds_error(
      paste0(ctx, " range out of bounds: [", s, ", ", e, "]"),
      "E2007",
      ctx
    )
  }
  vals <- base[s:e]
  switch(
    as.character(op),
    "sum" = sum(vals),
    "prod" = prod(vals),
    "min" = min(vals),
    "max" = max(vals),
    rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
  )
}

rr_can_reduce_range <- function(base, start, end) {
  to_i1 <- function(v) {
    if (length(v) != 1L || is.na(v) || !is.numeric(v) || v != floor(v)) return(NA_integer_)
    as.integer(v)
  }
  s <- to_i1(start)
  e <- to_i1(end)
  if (is.na(s) || is.na(e)) return(FALSE)
  if (e < s) return(TRUE)
  s >= 1L && e <= length(base)
}

rr_tile_map_range <- function(dest, lhs_src, rhs_src, start, end, op, tile, ctx="tile_map") {
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
  if (s < 1L || e > length(dest)) {
    rr_bounds_error(
      paste0(ctx, " range out of bounds: [", s, ", ", e, "]"),
      "E2007",
      ctx
    )
  }

  materialize <- function(src, cs, ce, label) {
    if (length(src) == 1L) {
      rep.int(src, ce - cs + 1L)
    } else {
      rr_index1_read_vec(src, cs:ce, paste0(ctx, " ", label))
    }
  }

  cur <- s
  while (cur <= e) {
    chunk_end <- min(e, cur + t - 1L)
    lv <- materialize(lhs_src, cur, chunk_end, "lhs")
    rv <- materialize(rhs_src, cur, chunk_end, "rhs")
    out <- switch(
      as.character(op),
      "+" = lv + rv,
      "-" = lv - rv,
      "*" = lv * rv,
      "/" = lv / rv,
      "%%" = lv %% rv,
      rr_value_error(paste0(ctx, " unsupported op: ", op), "E2001", ctx)
    )
    dest <- rr_assign_slice(dest, cur, chunk_end, out, ctx)
    cur <- chunk_end + 1L
  }
  dest
}

rr_tile_reduce_range <- function(base, start, end, op, tile, ctx="tile_reduce") {
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
    chunk <- rr_reduce_range(base, cur, chunk_end, op, ctx)
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

# -----------------------------------
