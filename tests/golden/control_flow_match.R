loop_ops <- function(n) {
  s <- 0L
  for (i in seq.int(1L, n)) {
    if (i == 3L) {
      next
    }
    if (i == 6L) {
      break
    }
    s <- s + i
  }
  s
}

fact <- function(n) {
  acc <- 1L
  i <- 1L
  while (i <= n) {
    acc <- acc * i
    i <- i + 1L
  }
  acc
}

match_list <- function(v) {
  if (is.null(v)) return(0L)
  if (length(v) >= 2L) {
    a <- v[1L]
    b <- v[2L]
    rest_len <- length(v) - 2L
    return(a + b + rest_len)
  }
  if (length(v) == 1L) return(v[1L])
  0L
}

match_record <- function(v) {
  if (!is.list(v)) return(0L)
  nms <- names(v)
  if (is.null(nms)) return(0L)
  has_a <- isTRUE("a" %in% nms)
  has_b <- isTRUE("b" %in% nms)
  if (has_a && has_b) return(v[["a"]] + v[["b"]])
  if (has_a) return(v[["a"]])
  0L
}

match_record_after_list <- function(v) {
  nms <- names(v)
  list_matchable <- is.null(nms) || all(is.na(nms) | nms == "")
  if (list_matchable && length(v) >= 2L) {
    a <- v[1L]
    b <- v[2L]
    rest_len <- length(v) - 2L
    return(a * 100L + b + rest_len)
  }
  if (!is.list(v)) return(0L)
  nms <- names(v)
  if (is.null(nms)) return(0L)
  has_a <- isTRUE("a" %in% nms)
  has_b <- isTRUE("b" %in% nms)
  if (has_a && has_b) return(v[["a"]] + v[["b"]])
  0L
}

main <- function() {
  print(loop_ops(10L))
  print(fact(6L))
  print(match_list(c(10L, 20L, 30L, 40L)))
  print(match_list(c(7L)))
  print(match_list(NULL))
  print(match_record(list(a = 3L, b = 4L)))
  print(match_record(list(a = 9L)))
  print(match_record(list(c = 1L)))
  print(match_record_after_list(list(a = 3L, b = 4L)))
  print(match_record_after_list(c(3L, 4L, 5L)))
  0L
}

print(main())
