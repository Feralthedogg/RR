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

rr_set_vector_fallback_base_trip <- function(n) {
  v <- suppressWarnings(as.integer(n))
  if (is.na(v) || v < 0L) v <- 12L
  .rr_env$vector_fallback_base_trip <- v
}

rr_set_vector_fallback_helper_scale <- function(n) {
  v <- suppressWarnings(as.integer(n))
  if (is.na(v) || v < 0L) v <- 4L
  .rr_env$vector_fallback_helper_scale <- v
}

rr_set_native_lib <- function(path) {
  if (is.null(path) || !nzchar(as.character(path))) {
    .rr_env$native_lib <- ""
    .rr_env$native_lib_from_env <- FALSE
    .rr_env$native_loaded <- FALSE
    return(invisible(NULL))
  }
  .rr_env$native_lib <- normalizePath(as.character(path), winslash = "/", mustWork = FALSE)
  .rr_env$native_lib_from_env <- FALSE
  .rr_env$native_loaded <- FALSE
}

rr_set_native_roots <- function(paths) {
  if (is.null(paths) || length(paths) < 1L) {
    .rr_env$native_anchor_roots <- character(0)
    return(invisible(NULL))
  }
  vals <- vapply(
    as.character(paths),
    function(p) normalizePath(p, winslash = "/", mustWork = FALSE),
    character(1)
  )
  vals <- unique(vals[nzchar(vals)])
  .rr_env$native_anchor_roots <- vals
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
.rr_env$vector_fallback_base_trip <- suppressWarnings(as.integer(Sys.getenv("RR_VECTOR_FALLBACK_BASE_TRIP", "12")))
if (is.na(.rr_env$vector_fallback_base_trip) || .rr_env$vector_fallback_base_trip < 0L) {
  .rr_env$vector_fallback_base_trip <- 12L
}
.rr_env$vector_fallback_helper_scale <- suppressWarnings(as.integer(Sys.getenv("RR_VECTOR_FALLBACK_HELPER_SCALE", "4")))
if (is.na(.rr_env$vector_fallback_helper_scale) || .rr_env$vector_fallback_helper_scale < 0L) {
  .rr_env$vector_fallback_helper_scale <- 4L
}
.rr_env$native_autobuild <- tolower(Sys.getenv("RR_NATIVE_AUTOBUILD", "1"))
.rr_env$native_autobuild <- .rr_env$native_autobuild %in% c("1", "true", "yes", "on")
.rr_env$native_anchor_roots <- character(0)

rr_native_lib_ext <- function() {
  if (identical(.Platform$OS.type, "windows")) return(".dll")
  if (identical(tolower(Sys.info()[["sysname"]]), "darwin")) return(".dylib")
  ".so"
}

rr_native_script_path <- function() {
  args <- commandArgs(trailingOnly = FALSE)
  file_arg <- grep("^--file=", args, value = TRUE)
  if (length(file_arg) < 1L) return("")
  path <- sub("^--file=", "", file_arg[[1L]])
  if (!nzchar(path)) return("")
  normalizePath(path, winslash = "/", mustWork = FALSE)
}

rr_native_candidate_roots <- function() {
  roots <- character(0)
  seen <- character(0)
  if (!is.null(.rr_env$native_anchor_roots) && length(.rr_env$native_anchor_roots) > 0L) {
    for (root in .rr_env$native_anchor_roots) {
      if (nzchar(root) && !(root %in% seen)) {
        roots <- c(roots, root)
        seen <- c(seen, root)
      }
    }
  }
  script_path <- rr_native_script_path()
  if (nzchar(script_path)) {
    cur <- dirname(script_path)
    while (nzchar(cur) && !(cur %in% seen)) {
      roots <- c(roots, cur)
      seen <- c(seen, cur)
      parent <- dirname(cur)
      if (!nzchar(parent) || identical(parent, cur)) break
      cur <- parent
    }
  }
  roots
}

rr_native_find_existing <- function() {
  ext <- rr_native_lib_ext()
  roots <- rr_native_candidate_roots()
  if (length(roots) < 1L) return("")
  names <- c(paste0("rr_native", ext), paste0("librr_native", ext))
  for (root in roots) {
    candidates <- c(
      file.path(root, "native", names),
      file.path(root, "target", "native", names),
      file.path(root, names)
    )
    for (candidate in candidates) {
      if (file.exists(candidate)) {
        return(normalizePath(candidate, winslash = "/", mustWork = FALSE))
      }
    }
  }
  ""
}

rr_native_maybe_build <- function() {
  if (!isTRUE(.rr_env$native_autobuild)) return("")
  ext <- rr_native_lib_ext()
  roots <- rr_native_candidate_roots()
  if (length(roots) < 1L) return("")

  src <- ""
  root_hit <- ""
  for (root in roots) {
    probe <- file.path(root, "native", "rr_native.c")
    if (file.exists(probe)) {
      src <- probe
      root_hit <- root
      break
    }
  }
  if (!nzchar(src)) return("")

  out_dir <- file.path(root_hit, "target", "native")
  dir.create(out_dir, recursive = TRUE, showWarnings = FALSE)
  out_path <- file.path(out_dir, paste0("rr_native", ext))
  if (file.exists(out_path)) {
    return(normalizePath(out_path, winslash = "/", mustWork = FALSE))
  }

  r_bin <- file.path(
    R.home("bin"),
    if (identical(.Platform$OS.type, "windows")) "R.exe" else "R"
  )
  if (!file.exists(r_bin)) {
    r_bin <- Sys.which("R")
  }
  if (!nzchar(r_bin)) return("")

  build_out <- tryCatch(
    suppressWarnings(system2(
      r_bin,
      c("CMD", "SHLIB", src, "-o", out_path),
      stdout = TRUE,
      stderr = TRUE
    )),
    error = function(e) NULL
  )
  if (is.null(build_out)) return("")
  status <- attr(build_out, "status")
  if (is.null(status)) status <- 0L
  if (status != 0L || !file.exists(out_path)) return("")
  normalizePath(out_path, winslash = "/", mustWork = FALSE)
}

rr_native_resolve_lib <- function() {
  env_path <- Sys.getenv("RR_NATIVE_LIB", "")
  if (nzchar(env_path)) {
    .rr_env$native_lib_from_env <- TRUE
    return(normalizePath(env_path, winslash = "/", mustWork = FALSE))
  }
  .rr_env$native_lib_from_env <- FALSE
  needs_native <- (!is.null(.rr_env$native_backend) && .rr_env$native_backend != "off") ||
    (!is.null(.rr_env$parallel_mode) && .rr_env$parallel_mode != "off")
  if (!needs_native) return("")
  existing <- rr_native_find_existing()
  if (nzchar(existing)) return(existing)
  rr_native_maybe_build()
}

.rr_env$native_lib <- rr_native_resolve_lib()
.rr_env$native_loaded <- FALSE

rr_native_try_load <- function() {
  if (isTRUE(.rr_env$native_loaded)) return(TRUE)
  if (is.null(.rr_env$native_lib) || !nzchar(.rr_env$native_lib)) {
    .rr_env$native_lib <- rr_native_resolve_lib()
  }
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

rr_parallel_typed_vec_r <- function(label, impl, slice_slots, args) {
  if (!requireNamespace("parallel", quietly = TRUE)) return(NULL)
  if (identical(.Platform$OS.type, "windows")) return(NULL)
  if (length(slice_slots) == 0L) return(NULL)

  lens <- integer(length(slice_slots))
  for (i in seq_along(slice_slots)) {
    slot <- as.integer(slice_slots[[i]])
    if (is.na(slot) || slot < 1L || slot > length(args)) return(NULL)
    v <- args[[slot]]
    if (!is.atomic(v)) return(NULL)
    lens[[i]] <- length(v)
  }
  if (length(lens) == 0L || any(lens <= 0L)) return(NULL)
  n <- lens[[1L]]
  if (any(lens != n)) return(NULL)

  cores <- rr_parallel_resolve_cores(n)
  if (cores <= 1L) return(NULL)
  breaks <- min(cores, n)
  if (breaks <= 1L) return(NULL)
  chunks <- split(seq_len(n), as.integer(cut(seq_len(n), breaks = breaks, labels = FALSE)))

  parts <- parallel::mclapply(
    chunks,
    function(ix) {
      chunk_args <- args
      for (slot in slice_slots) {
        slot_i <- as.integer(slot)
        chunk_args[[slot_i]] <- chunk_args[[slot_i]][ix]
      }
      tryCatch(
        list(ok = TRUE, val = do.call(impl, chunk_args)),
        error = function(e) list(ok = FALSE, err = conditionMessage(e))
      )
    },
    mc.cores = cores
  )

  if (length(parts) == 0L) return(NULL)
  ok <- vapply(parts, function(part) is.list(part) && isTRUE(part$ok), logical(1))
  if (!all(ok)) return(NULL)

  vals <- lapply(parts, function(part) part$val)
  expected <- vapply(chunks, length, integer(1))
  actual <- vapply(vals, length, integer(1))
  if (!all(actual == expected)) return(NULL)
  if (any(vapply(vals, function(part) !is.atomic(part), logical(1)))) return(NULL)

  unlist(vals, use.names = FALSE)
}

rr_parallel_typed_vec_call <- function(label, impl, slice_slots, ...) {
  args <- list(...)
  if (length(slice_slots) == 0L) {
    return(do.call(impl, args))
  }

  trip_lengths <- integer(length(slice_slots))
  for (i in seq_along(slice_slots)) {
    slot <- as.integer(slice_slots[[i]])
    if (is.na(slot) || slot < 1L || slot > length(args)) {
      return(do.call(impl, args))
    }
    trip_lengths[[i]] <- length(args[[slot]])
  }
  if (length(trip_lengths) == 0L || any(trip_lengths <= 0L)) {
    return(do.call(impl, args))
  }

  n <- trip_lengths[[1L]]
  if (any(trip_lengths != n) || !rr_parallel_enabled(n)) {
    return(do.call(impl, args))
  }

  backend <- .rr_env$parallel_backend
  if (backend %in% c("auto", "r")) {
    out <- rr_parallel_typed_vec_r(label, impl, slice_slots, args)
    if (!is.null(out)) return(out)
  }

  if (.rr_env$parallel_mode == "required") {
    rr_fail(
      "RR.RuntimeError",
      "E1031",
      paste0("parallel backend failed for typed wrapper ", label),
      "parallel backend"
    )
  }
  do.call(impl, args)
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

rr_dim2_reduce_range <- function(base, fixed_a, fixed_b, start, end, op, ctx="dim2_reduce") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[2L]], "dim2", ctx)
  rr_array3_reduce_apply(rr_array3_materialize_axis_arg(base, 2L, fixed_a, fixed_b, idx, "base", ctx), op, ctx)
}

rr_dim3_reduce_range <- function(base, fixed_a, fixed_b, start, end, op, ctx="dim3_reduce") {
  idx <- rr_array3_range_idx(start, end, rr_array3_dims(base, ctx)[[3L]], "dim3", ctx)
  rr_array3_reduce_apply(rr_array3_materialize_axis_arg(base, 3L, fixed_a, fixed_b, idx, "base", ctx), op, ctx)
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

# -----------------------------------
