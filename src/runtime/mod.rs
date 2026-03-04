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

rr_set_type_mode <- function(mode) {
  m <- tolower(as.character(mode))
  if (!(m %in% c("strict", "gradual"))) {
    m <- "strict"
  }
  .rr_env$type_mode <- m
}

rr_set_native_backend <- function(mode) {
  m <- tolower(as.character(mode))
  if (!(m %in% c("off", "optional", "required"))) {
    m <- "off"
  }
  .rr_env$native_backend <- m
}

rr_set_parallel_mode <- function(mode) {
  m <- tolower(as.character(mode))
  if (!(m %in% c("off", "optional", "required"))) {
    m <- "off"
  }
  .rr_env$parallel_mode <- m
}

rr_set_parallel_backend <- function(backend) {
  b <- tolower(as.character(backend))
  if (!(b %in% c("auto", "r", "openmp"))) {
    b <- "auto"
  }
  .rr_env$parallel_backend <- b
}

rr_set_parallel_threads <- function(n) {
  v <- suppressWarnings(as.integer(n))
  if (is.na(v) || v < 0L) v <- 0L
  .rr_env$parallel_threads <- v
}

rr_set_parallel_min_trip <- function(n) {
  v <- suppressWarnings(as.integer(n))
  if (is.na(v) || v < 0L) v <- 4096L
  .rr_env$parallel_min_trip <- v
}

rr_set_native_lib <- function(path) {
  if (is.null(path) || !nzchar(as.character(path))) {
    .rr_env$native_lib <- ""
    .rr_env$native_loaded <- FALSE
    return(invisible(NULL))
  }
  .rr_env$native_lib <- as.character(path)
  .rr_env$native_loaded <- FALSE
}

.rr_env$type_mode <- "strict"
.rr_env$native_backend <- tolower(Sys.getenv("RR_NATIVE_BACKEND", "off"))
if (!(.rr_env$native_backend %in% c("off", "optional", "required"))) {
  .rr_env$native_backend <- "off"
}
.rr_env$parallel_mode <- tolower(Sys.getenv("RR_PARALLEL_MODE", "off"))
if (!(.rr_env$parallel_mode %in% c("off", "optional", "required"))) {
  .rr_env$parallel_mode <- "off"
}
.rr_env$parallel_backend <- tolower(Sys.getenv("RR_PARALLEL_BACKEND", "auto"))
if (!(.rr_env$parallel_backend %in% c("auto", "r", "openmp"))) {
  .rr_env$parallel_backend <- "auto"
}
.rr_env$parallel_threads <- suppressWarnings(as.integer(Sys.getenv("RR_PARALLEL_THREADS", "0")))
if (is.na(.rr_env$parallel_threads) || .rr_env$parallel_threads < 0L) {
  .rr_env$parallel_threads <- 0L
}
.rr_env$parallel_min_trip <- suppressWarnings(as.integer(Sys.getenv("RR_PARALLEL_MIN_TRIP", "4096")))
if (is.na(.rr_env$parallel_min_trip) || .rr_env$parallel_min_trip < 0L) {
  .rr_env$parallel_min_trip <- 4096L
}
.rr_env$native_lib <- Sys.getenv("RR_NATIVE_LIB", "")
.rr_env$native_loaded <- FALSE

rr_native_try_load <- function() {
  if (isTRUE(.rr_env$native_loaded)) return(TRUE)
  if (is.null(.rr_env$native_lib) || !nzchar(.rr_env$native_lib)) return(FALSE)
  ok <- tryCatch({
    dyn.load(.rr_env$native_lib)
    TRUE
  }, error = function(e) FALSE)
  .rr_env$native_loaded <- isTRUE(ok)
  isTRUE(ok)
}

rr_native_call <- function(sym, fallback, ...) {
  backend <- .rr_env$native_backend
  if (is.null(backend) || backend == "off") {
    return(fallback(...))
  }
  loaded <- rr_native_try_load()
  if (!loaded) {
    if (backend == "required") {
      rr_fail(
        "RR.RuntimeError",
        "E2001",
        paste0("native backend required but library is not loaded: ", .rr_env$native_lib),
        "native backend",
        "Set RR_NATIVE_LIB to a valid shared library or use --native-backend optional/off."
      )
    }
    return(fallback(...))
  }
  out <- tryCatch(
    list(ok = TRUE, val = do.call(".Call", c(list(sym), list(...)))),
    error = function(e) list(ok = FALSE, err = conditionMessage(e))
  )
  if (!isTRUE(out$ok)) {
    if (backend == "required") {
      rr_fail(
        "RR.RuntimeError",
        "E2001",
        paste0("native call failed for symbol ", sym, ": ", out$err),
        "native backend"
      )
    }
    return(fallback(...))
  }
  out$val
}

rr_parallel_native_call <- function(sym, fallback, ...) {
  mode <- .rr_env$parallel_mode
  if (is.null(mode) || mode == "off") {
    return(fallback(...))
  }
  loaded <- rr_native_try_load()
  if (!loaded) {
    if (mode == "required") {
      rr_fail(
        "RR.RuntimeError",
        "E1031",
        paste0("parallel backend requires native library but load failed: ", .rr_env$native_lib),
        "parallel backend",
        "Set RR_NATIVE_LIB to a valid shared library or use --parallel-mode optional/off."
      )
    }
    return(fallback(...))
  }
  out <- tryCatch(
    list(ok = TRUE, val = do.call(".Call", c(list(sym), list(...)))),
    error = function(e) list(ok = FALSE, err = conditionMessage(e))
  )
  if (!isTRUE(out$ok)) {
    if (mode == "required") {
      rr_fail(
        "RR.RuntimeError",
        "E1031",
        paste0("parallel native call failed for symbol ", sym, ": ", out$err),
        "parallel backend"
      )
    }
    return(fallback(...))
  }
  out$val
}

rr_parallel_enabled <- function(n) {
  mode <- .rr_env$parallel_mode
  if (is.null(mode) || mode == "off") return(FALSE)
  if (is.na(n) || n <= 0L) return(FALSE)
  n >= as.integer(.rr_env$parallel_min_trip)
}

rr_parallel_resolve_cores <- function(n) {
  cores <- suppressWarnings(as.integer(.rr_env$parallel_threads))
  if (is.na(cores) || cores <= 0L) {
    dc <- suppressWarnings(parallel::detectCores(logical = FALSE))
    if (is.na(dc) || dc <= 0L) {
      dc <- suppressWarnings(parallel::detectCores(logical = TRUE))
    }
    if (is.na(dc) || dc <= 0L) {
      cores <- 1L
    } else {
      cores <- as.integer(dc)
    }
  }
  if (cores < 1L) cores <- 1L
  if (!is.na(n) && n > 0L) {
    cores <- min(cores, as.integer(n))
  }
  cores
}

rr_parallel_binop_r <- function(op, a, b) {
  if (!requireNamespace("parallel", quietly = TRUE)) return(NULL)
  if (identical(.Platform$OS.type, "windows")) return(NULL)
  la <- length(a)
  lb <- length(b)
  if (la == 0L || lb == 0L) return(NULL)
  if (!(la == lb || la == 1L || lb == 1L)) return(NULL)
  n <- max(la, lb)
  cores <- rr_parallel_resolve_cores(n)
  if (cores <= 1L) return(NULL)
  chunks <- split(seq_len(n), as.integer(cut(seq_len(n), breaks = min(cores, n), labels = FALSE)))
  parts <- parallel::mclapply(
    chunks,
    function(ix) {
      av <- if (la == 1L) rep(a[1L], length(ix)) else a[ix]
      bv <- if (lb == 1L) rep(b[1L], length(ix)) else b[ix]
      switch(
        op,
        add = av + bv,
        sub = av - bv,
        mul = av * bv,
        div = av / bv,
        pmax = pmax(av, bv),
        pmin = pmin(av, bv),
        NULL
      )
    },
    mc.cores = cores
  )
  if (any(vapply(parts, is.null, logical(1)))) return(NULL)
  unlist(parts, use.names = FALSE)
}

rr_parallel_unary_r <- function(op, a) {
  if (!requireNamespace("parallel", quietly = TRUE)) return(NULL)
  if (identical(.Platform$OS.type, "windows")) return(NULL)
  n <- length(a)
  if (n == 0L) return(NULL)
  cores <- rr_parallel_resolve_cores(n)
  if (cores <= 1L) return(NULL)
  chunks <- split(seq_len(n), as.integer(cut(seq_len(n), breaks = min(cores, n), labels = FALSE)))
  parts <- parallel::mclapply(
    chunks,
    function(ix) {
      av <- a[ix]
      switch(
        op,
        abs = abs(av),
        log = log(av),
        sqrt = sqrt(av),
        NULL
      )
    },
    mc.cores = cores
  )
  if (any(vapply(parts, is.null, logical(1)))) return(NULL)
  unlist(parts, use.names = FALSE)
}

rr_parallel_vec2_f64 <- function(native_sym, op, base, a, b) {
  n <- max(length(a), length(b))
  if (!rr_parallel_enabled(n)) {
    return(base(a, b))
  }

  backend <- .rr_env$parallel_backend
  if (backend %in% c("auto", "openmp")) {
    out <- rr_parallel_native_call(
      native_sym,
      function(...) NULL,
      a,
      b,
      as.integer(.rr_env$parallel_threads),
      as.integer(.rr_env$parallel_min_trip)
    )
    if (!is.null(out)) return(out)
    if (backend == "openmp") return(base(a, b))
  }

  if (backend %in% c("auto", "r")) {
    out <- rr_parallel_binop_r(op, a, b)
    if (!is.null(out)) return(out)
  }

  if (.rr_env$parallel_mode == "required") {
    rr_fail(
      "RR.RuntimeError",
      "E1031",
      "parallel backend failed and no fallback path was available",
      "parallel backend"
    )
  }
  base(a, b)
}

rr_parallel_vec1_f64 <- function(native_sym, op, base, a) {
  n <- length(a)
  if (!rr_parallel_enabled(n)) {
    return(base(a))
  }

  backend <- .rr_env$parallel_backend
  if (backend %in% c("auto", "openmp")) {
    out <- rr_parallel_native_call(
      native_sym,
      function(...) NULL,
      a,
      as.integer(.rr_env$parallel_threads),
      as.integer(.rr_env$parallel_min_trip)
    )
    if (!is.null(out)) return(out)
    if (backend == "openmp") return(base(a))
  }

  if (backend %in% c("auto", "r")) {
    out <- rr_parallel_unary_r(op, a)
    if (!is.null(out)) return(out)
  }

  if (.rr_env$parallel_mode == "required") {
    rr_fail(
      "RR.RuntimeError",
      "E1031",
      "parallel backend failed and no fallback path was available",
      "parallel backend"
    )
  }
  base(a)
}

rr_intrinsic_base_vec_add_f64 <- function(a, b) {
  rr_native_call("rr_vec_add_f64", function(x, y) x + y, a, b)
}

rr_intrinsic_base_vec_sub_f64 <- function(a, b) {
  rr_native_call("rr_vec_sub_f64", function(x, y) x - y, a, b)
}

rr_intrinsic_base_vec_mul_f64 <- function(a, b) {
  rr_native_call("rr_vec_mul_f64", function(x, y) x * y, a, b)
}

rr_intrinsic_base_vec_div_f64 <- function(a, b) {
  rr_native_call("rr_vec_div_f64", function(x, y) x / y, a, b)
}

rr_intrinsic_base_vec_abs_f64 <- function(a) {
  rr_native_call("rr_vec_abs_f64", function(x) abs(x), a)
}

rr_intrinsic_base_vec_log_f64 <- function(a) {
  rr_native_call("rr_vec_log_f64", function(x) log(x), a)
}

rr_intrinsic_base_vec_sqrt_f64 <- function(a) {
  rr_native_call("rr_vec_sqrt_f64", function(x) sqrt(x), a)
}

rr_intrinsic_base_vec_pmax_f64 <- function(a, b) {
  rr_native_call("rr_vec_pmax_f64", function(x, y) pmax(x, y), a, b)
}

rr_intrinsic_base_vec_pmin_f64 <- function(a, b) {
  rr_native_call("rr_vec_pmin_f64", function(x, y) pmin(x, y), a, b)
}

rr_parallel_vec_add_f64 <- function(a, b) {
  rr_parallel_vec2_f64("rr_vec_add_f64_omp", "add", rr_intrinsic_base_vec_add_f64, a, b)
}

rr_parallel_vec_sub_f64 <- function(a, b) {
  rr_parallel_vec2_f64("rr_vec_sub_f64_omp", "sub", rr_intrinsic_base_vec_sub_f64, a, b)
}

rr_parallel_vec_mul_f64 <- function(a, b) {
  rr_parallel_vec2_f64("rr_vec_mul_f64_omp", "mul", rr_intrinsic_base_vec_mul_f64, a, b)
}

rr_parallel_vec_div_f64 <- function(a, b) {
  rr_parallel_vec2_f64("rr_vec_div_f64_omp", "div", rr_intrinsic_base_vec_div_f64, a, b)
}

rr_parallel_vec_abs_f64 <- function(a) {
  rr_parallel_vec1_f64("rr_vec_abs_f64_omp", "abs", rr_intrinsic_base_vec_abs_f64, a)
}

rr_parallel_vec_log_f64 <- function(a) {
  rr_parallel_vec1_f64("rr_vec_log_f64_omp", "log", rr_intrinsic_base_vec_log_f64, a)
}

rr_parallel_vec_sqrt_f64 <- function(a) {
  rr_parallel_vec1_f64("rr_vec_sqrt_f64_omp", "sqrt", rr_intrinsic_base_vec_sqrt_f64, a)
}

rr_parallel_vec_pmax_f64 <- function(a, b) {
  rr_intrinsic_base_vec_pmax_f64(a, b)
}

rr_parallel_vec_pmin_f64 <- function(a, b) {
  rr_intrinsic_base_vec_pmin_f64(a, b)
}

rr_intrinsic_vec_add_f64 <- function(a, b) {
  rr_parallel_vec_add_f64(a, b)
}

rr_intrinsic_vec_sub_f64 <- function(a, b) {
  rr_parallel_vec_sub_f64(a, b)
}

rr_intrinsic_vec_mul_f64 <- function(a, b) {
  rr_parallel_vec_mul_f64(a, b)
}

rr_intrinsic_vec_div_f64 <- function(a, b) {
  rr_parallel_vec_div_f64(a, b)
}

rr_intrinsic_vec_abs_f64 <- function(a) {
  rr_parallel_vec_abs_f64(a)
}

rr_intrinsic_vec_log_f64 <- function(a) {
  rr_parallel_vec_log_f64(a)
}

rr_intrinsic_vec_sqrt_f64 <- function(a) {
  rr_parallel_vec_sqrt_f64(a)
}

rr_intrinsic_vec_pmax_f64 <- function(a, b) {
  rr_parallel_vec_pmax_f64(a, b)
}

rr_intrinsic_vec_pmin_f64 <- function(a, b) {
  rr_parallel_vec_pmin_f64(a, b)
}

rr_intrinsic_vec_sum_f64 <- function(a) {
  rr_native_call("rr_vec_sum_f64", function(x) sum(x), a)
}

rr_intrinsic_vec_mean_f64 <- function(a) {
  rr_native_call("rr_vec_mean_f64", function(x) mean(x), a)
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
