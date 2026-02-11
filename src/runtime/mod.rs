pub mod runner;

pub const R_RUNTIME: &str = r#"
# --- RR runtime (auto-generated) ---

.rr_env <- new.env(parent=emptyenv())
.rr_env$line <- 1L
.rr_env$col  <- 1L
.rr_env$file <- "RR"
.rr_env$runtime_mode <- tolower(Sys.getenv("RR_RUNTIME_MODE", "debug"))
.rr_env$fast_runtime <- identical(.rr_env$runtime_mode, "release") ||
                       identical(Sys.getenv("RR_FAST_RUNTIME", "0"), "1")
.rr_env$strict_index_read <- identical(Sys.getenv("RR_STRICT_INDEX_READ", "0"), "1")
.rr_env$enable_marks <- !identical(Sys.getenv("RR_ENABLE_MARKS", "1"), "0")
if (.rr_env$fast_runtime && identical(Sys.getenv("RR_ENABLE_MARKS", ""), "")) {
  .rr_env$enable_marks <- FALSE
}

rr_mark <- function(line, col) {
  if (!.rr_env$enable_marks) return(invisible(NULL))
  .rr_env$line <- as.integer(line)
  .rr_env$col  <- as.integer(col)
}

rr_set_source <- function(file) {
  .rr_env$file <- as.character(file)
}

rr_escape_diag <- function(x) {
  x <- as.character(x)
  x <- gsub("\\n", " ", x)
  gsub("\\|", "/", x)
}

rr_loc <- function() {
  sprintf("%s:%d:%d", .rr_env$file, .rr_env$line, .rr_env$col)
}

rr_fail <- function(kind="RR.RuntimeError", code="E2001", msg, ctx=NULL, hint=NULL) {
  header <- sprintf("** (%s) %s: %s", kind, rr_loc(), msg)
  diag <- sprintf(
    "RRDIAG|kind=%s|code=%s|file=%s|line=%d|col=%d|msg=%s|ctx=%s|hint=%s",
    rr_escape_diag(kind),
    rr_escape_diag(code),
    rr_escape_diag(.rr_env$file),
    .rr_env$line,
    .rr_env$col,
    rr_escape_diag(msg),
    rr_escape_diag(if (is.null(ctx)) "" else ctx),
    rr_escape_diag(if (is.null(hint)) "" else hint)
  )
  lines <- c(header, diag)
  if (!is.null(ctx))  lines <- c(lines, sprintf("In: %s", ctx))
  if (!is.null(hint)) lines <- c(lines, sprintf("Hint: %s", hint))
  stop(paste(lines, collapse="\n"), call.=FALSE)
}

rr_type_error <- function(msg, code="E1002", ctx=NULL, hint=NULL) {
  rr_fail("RR.TypeError", code, msg, ctx, hint)
}

rr_bounds_error <- function(msg, code="E2007", ctx=NULL, hint=NULL) {
  rr_fail("RR.BoundsError", code, msg, ctx, hint)
}

rr_value_error <- function(msg, code="E2001", ctx=NULL, hint=NULL) {
  rr_fail("RR.ValueError", code, msg, ctx, hint)
}

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

rr_field_get <- function(base, name) {
  if (length(name) != 1) rr_type_error("field name must be scalar", "E1002", "field access")
  if (!is.character(name)) rr_type_error("field name must be character", "E1002", "field access")
  base[[name]]
}

rr_field_exists <- function(base, name) {
  if (length(name) != 1) rr_type_error("field name must be scalar", "E1002", "field access")
  if (!is.character(name)) rr_type_error("field name must be character", "E1002", "field access")
  nms <- names(base)
  if (is.null(nms)) return(FALSE)
  isTRUE(name %in% nms)
}

rr_field_set <- function(base, name, value) {
  if (length(name) != 1) rr_type_error("field name must be scalar", "E1002", "field assign")
  if (!is.character(name)) rr_type_error("field name must be character", "E1002", "field assign")
  base[[name]] <- value
  base
}

rr_named_list <- function(...) {
  xs <- list(...)
  n <- length(xs)
  if ((n %% 2L) != 0L) {
    rr_value_error("rr_named_list requires name/value pairs", "E2001", "record literal")
  }
  out <- list()
  i <- 1L
  while (i <= n) {
    nm <- xs[[i]]
    vv <- xs[[i + 1L]]
    if (length(nm) != 1 || !is.character(nm)) {
      rr_type_error("record field name must be scalar character", "E1002", "record literal")
    }
    out[[nm]] <- vv
    i <- i + 2L
  }
  out
}

rr_closure_make <- function(fn_obj, ...) {
  if (!is.function(fn_obj)) {
    rr_type_error("closure target must be a function", "E1002", "closure")
  }
  caps <- list(...)
  structure(list(fn = fn_obj, caps = caps), class = "rr_closure")
}

rr_call_closure <- function(callee, ...) {
  args <- list(...)
  if (inherits(callee, "rr_closure")) {
    fn <- callee$fn
    caps <- callee$caps
    if (!is.function(fn)) {
      rr_type_error("closure payload is not callable", "E1002", "call")
    }
    if (length(caps) == 0L) {
      return(fn(...))
    }
    return(do.call(fn, c(caps, args)))
  }
  if (is.function(callee)) {
    return(callee(...))
  }
  rr_type_error(
    paste0("callee is not a function: ", typeof(callee)),
    "E1002",
    "call"
  )
}

rr_list_rest <- function(base, start) {
  if (length(start) != 1L) rr_type_error("list rest start must be scalar", "E1002", "match")
  if (is.na(start)) rr_value_error("list rest start is NA", "E2001", "match")
  if (!is.numeric(start)) rr_type_error("list rest start must be numeric", "E1002", "match")
  if (start != floor(start)) rr_type_error("list rest start must be integer", "E1002", "match")
  start <- as.integer(start)
  if (start <= 1L) return(base)
  n <- length(base)
  if (start > n) return(base[0])
  base[start:n]
}

rr_recur_add_const <- function(base, start, end, delta) {
  if (length(start) != 1L || length(end) != 1L) {
    rr_type_error("recurrence bounds must be scalar", "E1002", "recurrence")
  }
  if (!is.numeric(start) || !is.numeric(end)) {
    rr_type_error("recurrence bounds must be numeric", "E1002", "recurrence")
  }
  s <- as.integer(start)
  e <- as.integer(end)
  if (is.na(s) || is.na(e)) {
    rr_value_error("recurrence bounds cannot be NA", "E2001", "recurrence")
  }
  if (s > e) return(base)
  if (s <= 1L) {
    rr_bounds_error(
      "recurrence requires start >= 2",
      "E2007",
      "recurrence",
      "Pattern expects a[i] = a[i-1] + k"
    )
  }
  n <- e - s + 1L
  step <- as.numeric(delta)
  base[s:e] <- base[s - 1L] + cumsum(rep(step, n))
  base
}

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
  rr_index1_write <- function(i, ctx="index") as.integer(i)
}

# -----------------------------------

"#;
