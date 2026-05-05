# --- RR runtime: scalar checks, indexing, vector helpers ---
rr_bool <- function(x, ctx="condition") {
  if (.rr_env$fast_runtime &&
      length(x) == 1L &&
      is.logical(x) &&
      !is.na(x)) {
    return(isTRUE(x))
  }
  if (length(x) != 1) rr_type_error(paste0(ctx, " must be scalar boolean"), "E1002", ctx)
  if (!is.logical(x)) rr_type_error(paste0(ctx, " must be logical"), "E1002", ctx)
  if (is.na(x)) rr_value_error(
    paste0(ctx, " is NA"),
    "E2001",
    ctx,
    "Check for missing values (NA) in logical expressions."
  )
  isTRUE(x)
}

rr_truthy1 <- function(x, ctx="condition") {
  rr_bool(x, ctx)
}

rr_i0 <- function(i, ctx="index") {
  if (length(i)!=1) rr_type_error(paste0(ctx, " must be scalar"), "E1002", ctx)
  if (is.na(i)) rr_value_error(paste0(ctx, " is NA"), "E2001", ctx)
  if (!is.numeric(i)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
  if (i != floor(i)) rr_type_error(paste0(ctx, " must be integer"), "E1002", ctx)
  i <- as.integer(i)
  if (i < 0L) rr_bounds_error(
    paste0(ctx, " must be >= 0"),
    "E2007",
    ctx,
    "RR uses 0-based indexing internally."
  )
  i
}

rr_i0_read <- function(i, ctx="index") {
  if (length(i)!=1) rr_type_error(paste0(ctx, " must be scalar"), "E1002", ctx)
  if (is.na(i)) return(NA_integer_)
  if (!is.numeric(i)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
  if (i != floor(i)) rr_type_error(paste0(ctx, " must be integer"), "E1002", ctx)
  i <- as.integer(i)
  if (i < 0L) rr_bounds_error(
    paste0(ctx, " must be >= 0"),
    "E2007",
    ctx,
    "RR uses 0-based indexing internally."
  )
  i
}

rr_index1_read_strict <- function(base, i, ctx="index") {
  if (length(i)!=1) rr_type_error(paste0(ctx, " must be scalar"), "E1002", ctx)
  if (is.na(i)) rr_value_error(paste0(ctx, " is NA"), "E2001", ctx)
  if (!is.numeric(i)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
  if (i != floor(i)) rr_type_error(paste0(ctx, " must be integer"), "E1002", ctx)
  i <- as.integer(i)
  if (i < 1L) rr_bounds_error(
    paste0(ctx, " must be >= 1"),
    "E2007",
    ctx,
    "R indexing is 1-based at runtime."
  )
  base[i]
}

rr_index1_read <- function(base, i, ctx="index") {
  if (.rr_env$strict_index_read) {
    return(rr_index1_read_strict(base, i, ctx))
  }
  if (.rr_env$fast_runtime &&
      length(i) == 1L &&
      !is.na(i) &&
      is.numeric(i) &&
      i == floor(i)) {
    ii <- as.integer(i)
    if (ii >= 1L) return(base[ii])
  }
  if (length(i)!=1) rr_type_error(paste0(ctx, " must be scalar"), "E1002", ctx)
  # Keep R semantics for logical NA indexing: x[NA] -> length(x) NA vector.
  if (is.na(i)) return(base[NA])
  if (!is.numeric(i)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
  if (i != floor(i)) rr_type_error(paste0(ctx, " must be integer"), "E1002", ctx)
  i <- as.integer(i)
  if (i < 1L) rr_bounds_error(
    paste0(ctx, " must be >= 1"),
    "E2007",
    ctx,
    "R indexing is 1-based at runtime."
  )
  base[i]
}

rr_index1_read_vec <- function(base, idx, ctx="index") {
  if (is.integer(idx)) {
    ii <- idx
    if (.rr_env$strict_index_read && anyNA(ii)) {
      rr_value_error(paste0(ctx, " contains NA"), "E2001", ctx)
    }
  } else {
    if (!is.numeric(idx)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
    if (.rr_env$strict_index_read && anyNA(idx)) {
      rr_value_error(paste0(ctx, " contains NA"), "E2001", ctx)
    }
    non_na <- !is.na(idx)
    if (any(idx[non_na] != floor(idx[non_na]))) {
      rr_type_error(paste0(ctx, " must be integer"), "E1002", ctx)
    }
    ii <- as.integer(idx)
  }
  if (any(ii[!is.na(ii)] < 1L)) {
    rr_bounds_error(
      paste0(ctx, " must be >= 1"),
      "E2007",
      ctx,
      "R indexing is 1-based at runtime."
    )
  }
  base[ii]
}

rr_gather <- function(base, idx, ctx="gather") {
  rr_index1_read_vec(base, idx, ctx)
}

rr_index1_read_idx <- function(base, i, ctx="index") {
  v <- rr_index1_read(base, i, ctx)
  if (length(v) != 1L) rr_type_error(paste0(ctx, " must be scalar"), "E1002", ctx)
  if (is.na(v)) rr_value_error(paste0(ctx, " is NA"), "E2001", ctx)
  if (is.integer(v)) {
    if (v < 1L) {
      rr_bounds_error(
        paste0(ctx, " must be >= 1"),
        "E2007",
        ctx,
        "R indexing is 1-based at runtime."
      )
    }
    return(v)
  }
  if (!is.numeric(v)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
  if (v != floor(v)) rr_type_error(paste0(ctx, " must be integer"), "E1002", ctx)
  ii <- as.integer(v)
  if (ii < 1L) {
    rr_bounds_error(
      paste0(ctx, " must be >= 1"),
      "E2007",
      ctx,
      "R indexing is 1-based at runtime."
    )
  }
  ii
}

rr_index_vec_floor <- function(idx, ctx="index") {
  if (is.integer(idx)) {
    ii <- idx
  } else {
    if (!is.numeric(idx)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
    floored <- floor(idx)
    ii <- as.integer(floored)
  }
  if (.rr_env$strict_index_read && anyNA(ii)) {
    rr_value_error(paste0(ctx, " contains NA"), "E2001", ctx)
  }
  if (any(ii[!is.na(ii)] < 1L)) {
    rr_bounds_error(
      paste0(ctx, " must be >= 1"),
      "E2007",
      ctx,
      "R indexing is 1-based at runtime."
    )
  }
  ii
}

rr_index1_read_vec_floor <- function(base, idx, ctx="index") {
  ii <- rr_index_vec_floor(idx, ctx)
  base[ii]
}

rr_wrap_index_vec <- function(x, y, w, h, ctx="wrap_index") {
  if (length(w) != 1L || length(h) != 1L) {
    rr_type_error(paste0(ctx, " w/h must be scalar"), "E1002", ctx)
  }
  if (is.na(w) || is.na(h)) {
    rr_value_error(paste0(ctx, " w/h cannot be NA"), "E2001", ctx)
  }
  if (!is.numeric(w) || !is.numeric(h)) {
    rr_type_error(paste0(ctx, " w/h must be numeric"), "E1002", ctx)
  }
  if (w != floor(w) || h != floor(h)) {
    rr_type_error(paste0(ctx, " w/h must be integer"), "E1002", ctx)
  }
  if (w < 1 || h < 1) {
    rr_bounds_error(
      paste0(ctx, " w/h must be >= 1"),
      "E2007",
      ctx
    )
  }
  xx <- rr_ifelse_strict(x < 1, w, x, paste0(ctx, ".x<1"))
  xx <- rr_ifelse_strict(xx > w, 1, xx, paste0(ctx, ".x>w"))
  yy <- rr_ifelse_strict(y < 1, h, y, paste0(ctx, ".y<1"))
  yy <- rr_ifelse_strict(yy > h, 1, yy, paste0(ctx, ".y>h"))
  ((yy - 1) * w) + xx
}

# Integer-index specialization for compiler-emitted wrap-index helpers.
# This keeps scalar shape checks while using a lighter vector path and
# returning integer indices for downstream gather fast-paths.
rr_wrap_index_vec_i <- function(x, y, w, h, ctx="wrap_index") {
  if (length(w) != 1L || length(h) != 1L) {
    rr_type_error(paste0(ctx, " w/h must be scalar"), "E1002", ctx)
  }
  if (is.na(w) || is.na(h)) {
    rr_value_error(paste0(ctx, " w/h cannot be NA"), "E2001", ctx)
  }
  if (!is.numeric(w) || !is.numeric(h)) {
    rr_type_error(paste0(ctx, " w/h must be numeric"), "E1002", ctx)
  }
  if (w != floor(w) || h != floor(h)) {
    rr_type_error(paste0(ctx, " w/h must be integer"), "E1002", ctx)
  }
  if (w < 1 || h < 1) {
    rr_bounds_error(
      paste0(ctx, " w/h must be >= 1"),
      "E2007",
      ctx
    )
  }
  xx <- ifelse(x < 1, w, x)
  xx <- ifelse(xx > w, 1, xx)
  yy <- ifelse(y < 1, h, y)
  yy <- ifelse(yy > h, 1, yy)
  as.integer(((yy - 1) * w) + xx)
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

rr_index1_write <- function(i, ctx="index") {
  if (.rr_env$fast_runtime &&
      length(i) == 1L &&
      !is.na(i) &&
      is.numeric(i) &&
      i == floor(i)) {
    ii <- as.integer(i)
    if (ii >= 1L) return(ii)
  }
  if (length(i)!=1) rr_type_error(paste0(ctx, " must be scalar"), "E1002", ctx)
  if (is.na(i)) rr_value_error(paste0(ctx, " is NA"), "E2001", ctx)
  if (!is.numeric(i)) rr_type_error(paste0(ctx, " must be numeric"), "E1002", ctx)
  if (i != floor(i)) rr_type_error(paste0(ctx, " must be integer"), "E1002", ctx)
  i <- as.integer(i)
  if (i < 1L) rr_bounds_error(
    paste0(ctx, " must be >= 1"),
    "E2007",
    ctx,
    "R indexing is 1-based at runtime."
  )
  i
}

rr_i0_write <- function(i, ctx="index") {
  rr_i0(i, ctx)
}

rr_i1 <- function(i, ctx="index") {
  i <- rr_i0(i, ctx)
  if (i < 1L) rr_bounds_error(
    paste0(ctx, " must be >= 1"),
    "E2007",
    ctx,
    "R indexing is 1-based at runtime."
  )
  i
}

rr_range <- function(a, b) {
  a <- rr_i0(a, "range start"); b <- rr_i0(b, "range end")
  if (a <= b) seq.int(a, b) else integer(0)
}

rr_indices <- function(x) {
  n <- length(x)
  if (n <= 0) integer(0) else seq.int(0L, as.integer(n - 1L))
}

rr_same_len <- function(a,b, ctx="vector op") {
  la <- length(a); lb <- length(b)
  if (la != lb) {
    rr_value_error(
      paste0(ctx, " length mismatch (", la, " vs ", lb, ")"),
      "E2001",
      ctx,
      "Expected equal lengths for zip-style vector operation."
    )
  }
}

rr_can_same_len <- function(a, b) {
  length(a) == length(b)
}

rr_same_or_scalar <- function(a,b, ctx="vector op") {
  la <- length(a); lb <- length(b)
  # Follow R recycling semantics:
  # - equal length: ok
  # - scalar recycling: ok
  # - non-scalar recycling: allowed, warn when non-multiple
  if (la == lb || la == 1L || lb == 1L) return(invisible(TRUE))
  if (la == 0L || lb == 0L) return(invisible(TRUE))
  if ((la %% lb) != 0L && (lb %% la) != 0L) {
    warning(
      paste0(
        ctx,
        ": longer object length is not a multiple of shorter object length (",
        la,
        " vs ",
        lb,
        ")"
      ),
      call. = FALSE
    )
  }
  invisible(TRUE)
}

rr_same_matrix_shape_or_scalar <- function(dest, src, ctx="matrix op") {
  if (!is.matrix(dest)) {
    rr_type_error(paste0(ctx, " dest must be a matrix"), "E1002", ctx)
  }
  if (is.matrix(src)) {
    if (nrow(dest) != nrow(src) || ncol(dest) != ncol(src)) {
      rr_value_error(
        paste0(
          ctx,
          " shape mismatch (",
          nrow(dest),
          "x",
          ncol(dest),
          " vs ",
          nrow(src),
          "x",
          ncol(src),
          ")"
        ),
        "E2001",
        ctx
      )
    }
    return(invisible(TRUE))
  }
  if (length(src) == 1L) {
    return(invisible(TRUE))
  }
  rr_value_error(
    paste0(ctx, " source must be a scalar or matrix"),
    "E2001",
    ctx
  )
}

rr_can_same_matrix_shape_or_scalar <- function(dest, src) {
  if (!is.matrix(dest)) return(FALSE)
  if (is.matrix(src)) {
    return(nrow(dest) == nrow(src) && ncol(dest) == ncol(src))
  }
  length(src) == 1L
}

rr_same_array3_shape_or_scalar <- function(dest, src, ctx="array3 op") {
  if (!is.array(dest) || length(dim(dest)) != 3L) {
    rr_type_error(paste0(ctx, " dest must be a rank-3 array"), "E1002", ctx)
  }
  if (is.array(src)) {
    sd <- dim(src)
    dd <- dim(dest)
    if (length(sd) != 3L || any(dd != sd)) {
      rr_value_error(
        paste0(
          ctx,
          " shape mismatch (",
          paste(dd, collapse = "x"),
          " vs ",
          paste(sd, collapse = "x"),
          ")"
        ),
        "E2001",
        ctx
      )
    }
    return(invisible(TRUE))
  }
  if (length(src) == 1L) {
    return(invisible(TRUE))
  }
  rr_value_error(
    paste0(ctx, " source must be a scalar or rank-3 array"),
    "E2001",
    ctx
  )
}

rr_can_same_array3_shape_or_scalar <- function(dest, src) {
  if (!is.array(dest) || length(dim(dest)) != 3L) return(FALSE)
  if (is.array(src)) {
    sd <- dim(src)
    dd <- dim(dest)
    return(length(sd) == 3L && all(dd == sd))
  }
  length(src) == 1L
}

rr_vector_scalar_fallback_enabled <- function(n, helper_cost) {
  nn <- suppressWarnings(as.integer(n))
  hc <- suppressWarnings(as.integer(helper_cost))
  if (is.na(nn) || nn <= 0L) return(FALSE)
  if (is.na(hc) || hc < 0L) hc <- 0L
  base_trip <- suppressWarnings(as.integer(.rr_env$vector_fallback_base_trip))
  helper_scale <- suppressWarnings(as.integer(.rr_env$vector_fallback_helper_scale))
  if (is.na(base_trip) || base_trip < 0L) base_trip <- 12L
  if (is.na(helper_scale) || helper_scale < 0L) helper_scale <- 4L
  nn <= (base_trip + (helper_scale * hc))
}

rr_call_map_trip_len <- function(args, vector_slots, ctx="call_map") {
  slots <- as.integer(vector_slots)
  if (length(slots) < 1L) return(0L)
  n <- NA_integer_
  for (slot in slots) {
    if (is.na(slot) || slot < 1L || slot > length(args)) {
      rr_value_error(paste0(ctx, " vector slot is invalid"), "E2001", ctx)
    }
    len <- length(args[[slot]])
    if (is.na(n)) {
      n <- as.integer(len)
    } else if (len != n) {
      rr_value_error(
        paste0(ctx, " vector slot length mismatch (", len, " vs ", n, ")"),
        "E2001",
        ctx
      )
    }
  }
  if (is.na(n)) 0L else n
}

rr_call_map_resolve_fun <- function(callee, ctx="call_map") {
  tryCatch(
    match.fun(callee),
    error = function(e) {
      rr_value_error(
        paste0(ctx, " unresolved callee: ", as.character(callee)),
        "E2001",
        ctx
      )
    }
  )
}

rr_call_map_scalar_assign <- function(out, start_idx, fun, args, vector_slots, n, ctx="call_map") {
  if (n <= 0L) return(out)
  slots <- as.integer(vector_slots)
  for (k in seq_len(n)) {
    scalar_args <- args
    for (slot in slots) {
      scalar_args[[slot]] <- scalar_args[[slot]][k]
    }
    value <- do.call(fun, scalar_args)
    if (length(value) != 1L) {
      rr_value_error(
        paste0(ctx, " scalar fallback expected length 1 but got ", length(value)),
        "E2001",
        ctx
      )
    }
    out[start_idx + k - 1L] <- value
  }
  out
}

rr_call_map_backend_required <- function(callee) {
  callee <- as.character(callee)
  intrinsic_like <- callee %in% c("abs", "log", "sqrt", "pmax", "pmin")
  intrinsic_like &&
    (identical(.rr_env$native_backend, "required") ||
      identical(.rr_env$parallel_mode, "required"))
}

rr_call_map_vector_eval <- function(callee, args) {
  callee <- as.character(callee)
  if (identical(callee, "abs") && length(args) == 1L) {
    return(rr_intrinsic_vec_abs_f64(args[[1L]]))
  }
  if (identical(callee, "log") && length(args) == 1L) {
    return(rr_intrinsic_vec_log_f64(args[[1L]]))
  }
  if (identical(callee, "sqrt") && length(args) == 1L) {
    return(rr_intrinsic_vec_sqrt_f64(args[[1L]]))
  }
  if (identical(callee, "pmax") && length(args) == 2L) {
    return(rr_intrinsic_vec_pmax_f64(args[[1L]], args[[2L]]))
  }
  if (identical(callee, "pmin") && length(args) == 2L) {
    return(rr_intrinsic_vec_pmin_f64(args[[1L]], args[[2L]]))
  }
  fun <- rr_call_map_resolve_fun(callee)
  do.call(fun, args)
}

rr_call_map_whole_auto <- function(dest, callee, helper_cost, vector_slots, ...) {
  args <- list(...)
  n <- rr_call_map_trip_len(args, vector_slots, "call_map")
  if (n <= 0L) {
    return(rr_call_map_vector_eval(callee, args))
  }
  if (rr_call_map_backend_required(callee) || !rr_vector_scalar_fallback_enabled(n, helper_cost)) {
    mapped <- rr_call_map_vector_eval(callee, args)
    if (length(dest) == n) {
      rr_same_len(dest, mapped, "call_map")
    }
    return(mapped)
  }
  fun <- rr_call_map_resolve_fun(callee)
  out <- dest
  rr_same_len(out, args[[as.integer(vector_slots[[1L]])]], "call_map")
  rr_call_map_scalar_assign(out, 1L, fun, args, vector_slots, n, "call_map")
}

rr_call_map_slice_auto <- function(dest, start, end, callee, helper_cost, vector_slots, ...) {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }

  s <- to_i1(start, "call_map start")
  e <- to_i1(end, "call_map end")
  if (e < s) return(dest)

  args <- list(...)
  n <- rr_call_map_trip_len(args, vector_slots, "call_map")
  expected <- e - s + 1L
  if (n != expected) {
    rr_value_error(
      paste0("call_map length mismatch (", n, " vs ", expected, ")"),
      "E2001",
      "call_map"
    )
  }
  if (rr_call_map_backend_required(callee) || !rr_vector_scalar_fallback_enabled(n, helper_cost)) {
    mapped <- rr_call_map_vector_eval(callee, args)
    return(rr_assign_slice(dest, s, e, mapped, "call_map"))
  }
  fun <- rr_call_map_resolve_fun(callee)
  out <- dest
  rr_call_map_scalar_assign(out, s, fun, args, vector_slots, n, "call_map")
}

rr_which_true <- function(mask) {
  which(mask %in% TRUE)
}

rr_ifelse_strict <- function(cond, yes, no, ctx="condition") {
  if (!is.logical(cond)) {
    rr_type_error(paste0(ctx, " must be logical"), "E1002", ctx)
  }
  if (anyNA(cond)) {
    rr_value_error(
      paste0(ctx, " is NA"),
      "E2001",
      ctx,
      "Vectorized condition contains NA; scalar if semantics require TRUE/FALSE."
    )
  }
  ifelse(cond, yes, no)
}

rr_shift_assign <- function(dest, src, d_start, d_end, s_start, s_end, ctx="shift") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }

  ds <- to_i1(d_start, paste0(ctx, " dest_start"))
  de <- to_i1(d_end, paste0(ctx, " dest_end"))
  ss <- to_i1(s_start, paste0(ctx, " src_start"))
  se <- to_i1(s_end, paste0(ctx, " src_end"))

  if (de < ds) return(dest)
  if (ds < 1L) rr_bounds_error(paste0(ctx, " dest_start must be >= 1"), "E2007", paste0(ctx, " dest_start"))
  if (ss < 1L) rr_bounds_error(paste0(ctx, " src_start must be >= 1"), "E2007", paste0(ctx, " src_start"))

  n_dst <- de - ds + 1L
  n_src <- se - ss + 1L
  if (n_dst != n_src) {
    rr_value_error(
      paste0(ctx, " length mismatch (", n_dst, " vs ", n_src, ")"),
      "E2001",
      ctx
    )
  }
  if (de > length(dest)) {
    rr_bounds_error(
      paste0(ctx, " destination end out of bounds: ", de, " > ", length(dest)),
      "E2007",
      ctx
    )
  }
  if (se > length(src)) {
    rr_bounds_error(
      paste0(ctx, " source end out of bounds: ", se, " > ", length(src)),
      "E2007",
      ctx
    )
  }

  dest[ds:de] <- src[ss:se]
  dest
}

rr_assign_slice <- function(dest, start, end, values, ctx="slice_assign") {
  to_i1 <- function(v, what) {
    if (length(v) != 1L) rr_type_error(paste0(what, " must be scalar"), "E1002", what)
    if (is.na(v)) rr_value_error(paste0(what, " is NA"), "E2001", what)
    if (!is.numeric(v)) rr_type_error(paste0(what, " must be numeric"), "E1002", what)
    if (v != floor(v)) rr_type_error(paste0(what, " must be integer"), "E1002", what)
    as.integer(v)
  }

  s <- to_i1(start, paste0(ctx, " start"))
  e <- to_i1(end, paste0(ctx, " end"))
  if (e < s) return(dest)
  if (s < 1L) rr_bounds_error(paste0(ctx, " start must be >= 1"), "E2007", paste0(ctx, " start"))
  if (e > length(dest)) {
    rr_bounds_error(
      paste0(ctx, " end out of bounds: ", e, " > ", length(dest)),
      "E2007",
      ctx
    )
  }
  expected <- e - s + 1L
  if (length(values) != expected) {
    rr_value_error(
      paste0(ctx, " length mismatch (", length(values), " vs ", expected, ")"),
      "E2001",
      ctx
    )
  }
  dest[s:e] <- values
  dest
}

rr_assign_index_vec <- function(dest, idx, values, ctx="index_assign") {
  ii <- rr_index_vec_floor(idx, paste0(ctx, " idx"))
  n <- length(ii)
  vv <- values
  if (length(vv) != 1L && length(vv) != n) {
    rr_value_error(
      paste0(ctx, " value/index length mismatch (", length(vv), " vs ", n, ")"),
      "E2001",
      ctx
    )
  }
  dest[ii] <- vv
  dest
}
