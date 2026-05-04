# --- RR runtime prelude entrypoint ---
# Rust embeds the same files via src/runtime/source.rs.
.rr_runtime_dir <- local({
  frames <- sys.frames()
  ofiles <- vapply(frames, function(frame) {
    value <- frame$ofile
    if (is.null(value)) "" else as.character(value)
  }, character(1))
  current <- tail(ofiles[nzchar(ofiles)], 1)
  if (length(current) == 0L) "." else dirname(current)
})
.rr_runtime_files <- file.path(
  .rr_runtime_dir,
  "runtime_prelude",
  c(
    "config.R",
    "indexing.R",
    "matrix_ops.R",
    "array3_ops.R",
    "records_closures.R",
    "reductions.R"
  )
)
.rr_runtime_source <- paste(vapply(.rr_runtime_files, function(path) {
  paste(readLines(path, warn = FALSE), collapse = "\n")
}, character(1)), collapse = "\n")
eval(parse(text = .rr_runtime_source), envir = parent.frame())
rm(.rr_runtime_dir, .rr_runtime_files, .rr_runtime_source)
# -----------------------------------
