lcg_uniform <- function(n, seed) {
  out <- rep.int(0.0, as.integer(n))
  s <- seed
  i <- 1.0
  while (i <= n) {
    s <- (s * 1103515245.0 + 12345.0) %% 2147483648.0
    out[as.integer(i)] <- s / 2147483648.0
    i <- i + 1.0
  }
  out
}

print_metric <- function(name, value) {
  print(name)
  print(value)
  value
}

main <- function() {
  n <- 256.0
  xs <- lcg_uniform(n, 12345.0)
  ys <- lcg_uniform(n, 67890.0)
  inside <- 0.0
  i <- 1.0
  while (i <= n) {
    dx <- xs[as.integer(i)] - 0.5
    dy <- ys[as.integer(i)] - 0.5
    if (dx * dx + dy * dy <= 0.25) {
      inside <- inside + 1.0
    }
    i <- i + 1.0
  }
  estimate <- 4.0 * inside / n
  print_metric("monte_carlo_pi", estimate)
}

print(main())
