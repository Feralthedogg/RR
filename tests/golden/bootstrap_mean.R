floor1 <- function(x) {
  x - (x %% 1.0)
}

vector_sum <- function(xs) {
  s <- 0.0
  i <- 1.0
  n <- length(xs)
  while (i <= n) {
    s <- s + xs[as.integer(i)]
    i <- i + 1.0
  }
  s
}

vector_mean <- function(xs) {
  vector_sum(xs) / length(xs)
}

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

unit_index <- function(u, n) {
  idx <- 1.0 + floor1(u * n)
  if (idx < 1.0) idx <- 1.0
  if (idx > n) idx <- n
  idx
}

print_metric <- function(name, value) {
  print(name)
  print(value)
  value
}

main <- function() {
  data <- c(3.0, 4.0, 5.0, 6.0, 8.0, 9.0, 10.0, 11.0)
  sample_n <- length(data)
  draws <- 32.0
  uniforms <- lcg_uniform(draws * sample_n, 24680.0)
  means <- rep.int(0.0, as.integer(draws))
  cursor <- 1.0
  d <- 1.0
  while (d <= draws) {
    s <- 0.0
    j <- 1.0
    while (j <= sample_n) {
      pick <- unit_index(uniforms[as.integer(cursor)], sample_n)
      s <- s + data[as.integer(pick)]
      cursor <- cursor + 1.0
      j <- j + 1.0
    }
    means[as.integer(d)] <- s / sample_n
    d <- d + 1.0
  }
  print_metric("bootstrap_base_mean", vector_mean(data))
  print_metric("bootstrap_resample_mean", vector_mean(means))
}

print(main())
