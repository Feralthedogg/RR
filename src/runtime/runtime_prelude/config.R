# --- RR runtime: configuration, native backend, diagnostics ---
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
    .rr_env$native_dll <- NULL
    return(invisible(NULL))
  }
  .rr_env$native_lib <- normalizePath(as.character(path), winslash = "/", mustWork = FALSE)
  .rr_env$native_lib_from_env <- FALSE
  .rr_env$native_loaded <- FALSE
  .rr_env$native_dll <- NULL
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
.rr_env$native_backend <- "off"
.rr_env$parallel_mode <- "off"
.rr_env$parallel_backend <- "auto"
.rr_env$parallel_threads <- 0L
.rr_env$parallel_min_trip <- 4096L
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

rr_native_openmp_enabled <- function() {
  !identical(.rr_env$parallel_mode, "off") &&
    (.rr_env$parallel_backend %in% c("auto", "openmp"))
}

rr_native_lib_stem <- function() {
  if (rr_native_openmp_enabled()) "rr_native_omp" else "rr_native"
}

rr_native_openmp_env <- function() {
  if (!rr_native_openmp_enabled()) return(character(0))
  sysname <- tolower(Sys.info()[["sysname"]])
  if (identical(sysname, "darwin")) {
    prefixes <- c("/opt/homebrew/opt/libomp", "/usr/local/opt/libomp")
    for (prefix in prefixes) {
      include_dir <- file.path(prefix, "include")
      lib_dir <- file.path(prefix, "lib")
      if (file.exists(file.path(include_dir, "omp.h")) && dir.exists(lib_dir)) {
        return(c(
          sprintf("PKG_CPPFLAGS=-Xpreprocessor -fopenmp -I%s", include_dir),
          sprintf("PKG_LIBS=-L%s -lomp", lib_dir)
        ))
      }
    }
    return(character(0))
  }
  c("PKG_CPPFLAGS=-fopenmp", "PKG_LIBS=-fopenmp")
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
  stem <- rr_native_lib_stem()
  names <- c(paste0(stem, ext), paste0("lib", stem, ext))
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
  out_path <- file.path(out_dir, paste0(rr_native_lib_stem(), ext))
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

  build_env <- rr_native_openmp_env()
  restore_env <- list()
  if (length(build_env) > 0L) {
    for (entry in build_env) {
      parts <- strsplit(entry, "=", fixed = TRUE)[[1L]]
      if (length(parts) < 2L) next
      key <- parts[[1L]]
      value <- paste(parts[-1L], collapse = "=")
      restore_env[[key]] <- Sys.getenv(key, unset = NA_character_)
      env_arg <- list(value)
      names(env_arg) <- key
      do.call(Sys.setenv, env_arg)
    }
  }
  on.exit({
    if (length(restore_env) > 0L) {
      for (key in names(restore_env)) {
        value <- restore_env[[key]]
        if (is.na(value)) {
          Sys.unsetenv(key)
        } else {
          env_arg <- list(value)
          names(env_arg) <- key
          do.call(Sys.setenv, env_arg)
        }
      }
    }
  }, add = TRUE)
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
.rr_env$native_dll <- NULL

rr_native_try_load <- function() {
  if (isTRUE(.rr_env$native_loaded) && !is.null(.rr_env$native_dll)) return(TRUE)
  if (is.null(.rr_env$native_lib) || !nzchar(.rr_env$native_lib)) {
    .rr_env$native_lib <- rr_native_resolve_lib()
  }
  if (is.null(.rr_env$native_lib) || !nzchar(.rr_env$native_lib)) return(FALSE)
  dll <- tryCatch(
    dyn.load(.rr_env$native_lib),
    error = function(e) NULL
  )
  ok <- !is.null(dll)
  .rr_env$native_dll <- dll
  .rr_env$native_loaded <- isTRUE(ok)
  isTRUE(ok)
}

rr_native_symbol_info <- function(sym) {
  if (is.null(.rr_env$native_dll)) return(NULL)
  tryCatch(
    getNativeSymbolInfo(sym, PACKAGE = .rr_env$native_dll),
    error = function(e) NULL
  )
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
  native_sym <- rr_native_symbol_info(sym)
  if (is.null(native_sym)) {
    if (backend == "required") {
      rr_fail(
        "RR.RuntimeError",
        "E2001",
        paste0("native symbol not found: ", sym),
        "native backend"
      )
    }
    return(fallback(...))
  }
  out <- tryCatch(
    list(ok = TRUE, val = do.call(".Call", c(list(native_sym), list(...)))),
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
  native_sym <- rr_native_symbol_info(sym)
  if (is.null(native_sym)) {
    if (mode == "required") {
      rr_fail(
        "RR.RuntimeError",
        "E1031",
        paste0("parallel native symbol not found: ", sym),
        "parallel backend"
      )
    }
    return(fallback(...))
  }
  out <- tryCatch(
    list(ok = TRUE, val = do.call(".Call", c(list(native_sym), list(...)))),
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
  matrix_dims <- NULL
  matrix_dimnames <- NULL
  matrix_rows <- NULL
  matrix_cols <- NULL
  for (i in seq_along(slice_slots)) {
    slot <- as.integer(slice_slots[[i]])
    if (is.na(slot) || slot < 1L || slot > length(args)) return(NULL)
    v <- args[[slot]]
    if (!is.atomic(v)) return(NULL)
    if (is.matrix(v)) {
      dims <- dim(v)
      if (length(dims) != 2L) return(NULL)
      if (is.null(matrix_dims)) {
        matrix_dims <- dims
        matrix_dimnames <- dimnames(v)
        matrix_rows <- dims[[1L]]
        matrix_cols <- dims[[2L]]
      } else if (!identical(matrix_dims, dims)) {
        return(NULL)
      }
    } else if (!is.null(matrix_dims)) {
      return(NULL)
    }
    lens[[i]] <- length(v)
  }
  if (length(lens) == 0L || any(lens <= 0L)) return(NULL)
  n <- lens[[1L]]
  if (any(lens != n)) return(NULL)

  cores <- rr_parallel_resolve_cores(n)
  if (cores <= 1L) return(NULL)
  if (!is.null(matrix_dims)) {
    if (is.null(matrix_cols) || matrix_cols <= 1L) return(NULL)
    breaks <- min(cores, matrix_cols)
    if (breaks <= 1L) return(NULL)
    chunks <- split(
      seq_len(matrix_cols),
      as.integer(cut(seq_len(matrix_cols), breaks = breaks, labels = FALSE))
    )
  } else {
    breaks <- min(cores, n)
    if (breaks <= 1L) return(NULL)
    chunks <- split(seq_len(n), as.integer(cut(seq_len(n), breaks = breaks, labels = FALSE)))
  }

  parts <- parallel::mclapply(
    chunks,
    function(ix) {
      chunk_args <- args
      for (slot in slice_slots) {
        slot_i <- as.integer(slot)
        if (!is.null(matrix_dims)) {
          chunk_args[[slot_i]] <- chunk_args[[slot_i]][, ix, drop = FALSE]
        } else {
          chunk_args[[slot_i]] <- chunk_args[[slot_i]][ix]
        }
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
  if (!is.null(matrix_dims)) {
    expected_cols <- vapply(chunks, length, integer(1))
    vals <- Map(
      function(part, cols) {
        if (is.matrix(part)) {
          dims <- dim(part)
          if (length(dims) != 2L || dims[[1L]] != matrix_rows || dims[[2L]] != cols) return(NULL)
          return(part)
        }
        if (!is.atomic(part) || length(part) != (matrix_rows * cols)) return(NULL)
        dim(part) <- c(matrix_rows, cols)
        part
      },
      vals,
      expected_cols
    )
    if (any(vapply(vals, is.null, logical(1)))) return(NULL)
    out <- do.call(cbind, unname(vals))
    if (!identical(dim(out), matrix_dims)) return(NULL)
    dim(out) <- matrix_dims
    if (!is.null(matrix_dimnames)) dimnames(out) <- matrix_dimnames
    return(out)
  }
  expected <- vapply(chunks, length, integer(1))
  actual <- vapply(vals, length, integer(1))
  if (!all(actual == expected)) return(NULL)
  if (any(vapply(vals, function(part) !is.atomic(part), logical(1)))) return(NULL)

  has_names <- any(vapply(vals, function(part) !is.null(names(part)), logical(1)))
  out <- if (has_names) {
    do.call(c, unname(vals))
  } else {
    unlist(vals, use.names = FALSE)
  }
  out
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
  rr_parallel_vec2_f64("rr_vec_pmax_f64_omp", "pmax", rr_intrinsic_base_vec_pmax_f64, a, b)
}

rr_parallel_vec_pmin_f64 <- function(a, b) {
  rr_parallel_vec2_f64("rr_vec_pmin_f64_omp", "pmin", rr_intrinsic_base_vec_pmin_f64, a, b)
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
