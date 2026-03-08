use std::fmt::Write;

#[derive(Clone, Debug)]
pub struct GeneratedCase {
    pub name: String,
    pub rr_src: String,
    pub ref_r_src: String,
}

#[derive(Clone, Copy, Debug)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi - lo + 1) as u32;
        lo + (self.next_u32() % span) as i32
    }
}

pub fn generate_cases(seed: u64, count: usize) -> Vec<GeneratedCase> {
    let mut rng = Lcg::new(seed);
    let mut cases = Vec::with_capacity(count);
    for idx in 0..count {
        let family = idx % 8;
        let case = match family {
            0 => branch_vec_fold_case(idx, &mut rng),
            1 => recurrence_case(idx, &mut rng),
            2 => matrix_fold_case(idx, &mut rng),
            3 => nested_loop_case(idx, &mut rng),
            4 => call_chain_case(idx, &mut rng),
            5 => tail_recursion_case(idx, &mut rng),
            6 => record_state_case(idx, &mut rng),
            _ => stats_namespace_case(idx, &mut rng),
        };
        cases.push(case);
    }
    cases
}

fn branch_vec_fold_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let n = rng.range_i32(6, 18);
    let scale = rng.range_i32(2, 6);
    let shift = rng.range_i32(-5, 7);
    let bias = rng.range_i32(2, 6);
    let cutoff = rng.range_i32(3, n + 5);
    let name = format!("branch_vec_fold_{idx:02}");
    let rr_src = format!(
        r#"
fn adjust(v, k, bias) {{
  if ((v % k) == 0L) {{
    return v + bias;
  }} else {{
    return v - bias;
  }}
}}

fn kernel(n, scale, shift, bias, cutoff) {{
  let xs = seq_len(n);
  let ys = seq_len(n);
  let i = 1L;
  let acc = 0L;
  while (i <= length(xs)) {{
    let base = (xs[i] * scale) + shift;
    if (base > cutoff) {{
      ys[i] = adjust(base, bias, i);
    }} else {{
      ys[i] = adjust(base + bias, bias, i);
    }}
    acc = acc + ys[i];
    i = i + 1L;
  }}
  print(acc);
  print(sum(ys));
  print(ys[length(ys)]);
  return acc + sum(ys) + ys[length(ys)];
}}

print(kernel({n}L, {scale}L, {shift}L, {bias}L, {cutoff}L));
"#
    );
    let ref_r_src = format!(
        r#"
adjust <- function(v, k, bias) {{
  if ((v %% k) == 0L) {{
    v + bias
  }} else {{
    v - bias
  }}
}}

kernel <- function(n, scale, shift, bias, cutoff) {{
  xs <- seq_len(n)
  ys <- seq_len(n)
  i <- 1L
  acc <- 0L
  while (i <= length(xs)) {{
    base <- (xs[i] * scale) + shift
    if (base > cutoff) {{
      ys[i] <- adjust(base, bias, i)
    }} else {{
      ys[i] <- adjust(base + bias, bias, i)
    }}
    acc <- acc + ys[i]
    i <- i + 1L
  }}
  print(acc)
  print(sum(ys))
  print(ys[length(ys)])
  acc + sum(ys) + ys[length(ys)]
}}

print(kernel({n}L, {scale}L, {shift}L, {bias}L, {cutoff}L))
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

fn recurrence_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let n = rng.range_i32(5, 16);
    let seed = rng.range_i32(2, 12);
    let a = rng.range_i32(2, 7);
    let b = rng.range_i32(-6, 5);
    let modulo = rng.range_i32(3, 9);
    let name = format!("recurrence_{idx:02}");
    let rr_src = format!(
        r#"
fn main() {{
  let n = {n}L;
  let xs = seq_len(n);
  let acc = seq_len(n);
  acc[1L] = {seed}L;
  let i = 2L;
  while (i <= n) {{
    let step = ((xs[i] * {a}L) + {b}L) % {modulo}L;
    acc[i] = acc[i - 1L] + step;
    i = i + 1L;
  }}
  print(acc[n]);
  print(sum(acc));
  return acc[n] + sum(acc);
}}

print(main());
"#
    );
    let ref_r_src = format!(
        r#"
main <- function() {{
  n <- {n}L
  xs <- seq_len(n)
  acc <- seq_len(n)
  acc[1L] <- {seed}L
  i <- 2L
  while (i <= n) {{
    step <- ((xs[i] * {a}L) + {b}L) %% {modulo}L
    acc[i] <- acc[i - 1L] + step
    i <- i + 1L
  }}
  print(acc[n])
  print(sum(acc))
  acc[n] + sum(acc)
}}

print(main())
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

fn matrix_fold_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let rows = rng.range_i32(2, 4);
    let cols = rng.range_i32(3, 5);
    let tweak = rng.range_i32(1, 5);
    let row_pick = rng.range_i32(1, rows);
    let col_pick = rng.range_i32(1, cols);
    let name = format!("matrix_fold_{idx:02}");
    let rr_src = format!(
        r#"
fn mix(v, tweak) {{
  if ((v % 2L) == 0L) {{
    return v + tweak;
  }} else {{
    return v - tweak;
  }}
}}

fn main() {{
  let rows = {rows}L;
  let cols = {cols}L;
  let vals = seq_len(rows * cols);
  let m = matrix(vals, rows, cols);
  let rs = rowSums(m);
  let cs = colSums(m);
  let total = 0L;
  let i = 1L;
  while (i <= length(rs)) {{
    total = total + mix(rs[i], {tweak}L);
    i = i + 1L;
  }}
  let j = 1L;
  while (j <= length(cs)) {{
    total = total + mix(cs[j], {tweak}L + 1L);
    j = j + 1L;
  }}
  print(total);
  print(m[{row_pick}L, {col_pick}L]);
  return total + m[{row_pick}L, {col_pick}L];
}}

print(main());
"#
    );
    let ref_r_src = format!(
        r#"
mix <- function(v, tweak) {{
  if ((v %% 2L) == 0L) {{
    v + tweak
  }} else {{
    v - tweak
  }}
}}

main <- function() {{
  rows <- {rows}L
  cols <- {cols}L
  vals <- seq_len(rows * cols)
  m <- matrix(vals, rows, cols)
  rs <- rowSums(m)
  cs <- colSums(m)
  total <- 0L
  i <- 1L
  while (i <= length(rs)) {{
    total <- total + mix(rs[i], {tweak}L)
    i <- i + 1L
  }}
  j <- 1L
  while (j <= length(cs)) {{
    total <- total + mix(cs[j], {tweak}L + 1L)
    j <- j + 1L
  }}
  print(total)
  print(m[{row_pick}L, {col_pick}L])
  total + m[{row_pick}L, {col_pick}L]
}}

print(main())
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

fn nested_loop_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let outer = rng.range_i32(3, 7);
    let inner = rng.range_i32(4, 8);
    let a = rng.range_i32(2, 6);
    let b = rng.range_i32(1, 5);
    let name = format!("nested_loop_{idx:02}");
    let rr_src = format!(
        r#"
fn score(i, j, a, b) {{
  let v = (i * a) - (j * b);
  if (v > 0L) {{
    return v;
  }} else {{
    return 0L - v;
  }}
}}

fn main() {{
  let outer = {outer}L;
  let inner = {inner}L;
  let acc = 0L;
  let diag = 0L;
  let i = 1L;
  while (i <= outer) {{
    let j = 1L;
    while (j <= inner) {{
      let s = score(i, j, {a}L, {b}L);
      acc = acc + s;
      if (i == j) {{
        diag = diag + s;
      }}
      j = j + 1L;
    }}
    i = i + 1L;
  }}
  print(acc);
  print(diag);
  return acc + diag;
}}

print(main());
"#
    );
    let ref_r_src = format!(
        r#"
score <- function(i, j, a, b) {{
  v <- (i * a) - (j * b)
  if (v > 0L) {{
    v
  }} else {{
    0L - v
  }}
}}

main <- function() {{
  outer <- {outer}L
  inner <- {inner}L
  acc <- 0L
  diag <- 0L
  i <- 1L
  while (i <= outer) {{
    j <- 1L
    while (j <= inner) {{
      s <- score(i, j, {a}L, {b}L)
      acc <- acc + s
      if (i == j) {{
        diag <- diag + s
      }}
      j <- j + 1L
    }}
    i <- i + 1L
  }}
  print(acc)
  print(diag)
  acc + diag
}}

print(main())
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

fn call_chain_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let n = rng.range_i32(6, 18);
    let a = rng.range_i32(2, 5);
    let b = rng.range_i32(3, 9);
    let name = format!("call_chain_{idx:02}");
    let rr_src = format!(
        r#"
fn tweak(x, k) {{
  return (x * k) - (k - 1L);
}}

fn project(v, a, b) {{
  if (v > b) {{
    return tweak(v, a) - b;
  }} else {{
    return tweak(v + b, a);
  }}
}}

fn main() {{
  let n = {n}L;
  let xs = seq_len(n);
  let ys = seq_len(n);
  let i = 1L;
  while (i <= n) {{
    ys[i] = project(xs[i], {a}L, {b}L);
    i = i + 1L;
  }}
  print(sum(ys));
  print(ys[1L]);
  print(ys[n]);
  return sum(ys) + ys[1L] + ys[n];
}}

print(main());
"#
    );
    let ref_r_src = format!(
        r#"
tweak <- function(x, k) {{
  (x * k) - (k - 1L)
}}

project <- function(v, a, b) {{
  if (v > b) {{
    tweak(v, a) - b
  }} else {{
    tweak(v + b, a)
  }}
}}

main <- function() {{
  n <- {n}L
  xs <- seq_len(n)
  ys <- seq_len(n)
  i <- 1L
  while (i <= n) {{
    ys[i] <- project(xs[i], {a}L, {b}L)
    i <- i + 1L
  }}
  print(sum(ys))
  print(ys[1L])
  print(ys[n])
  sum(ys) + ys[1L] + ys[n]
}}

print(main())
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

fn tail_recursion_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let n = rng.range_i32(6, 22);
    let k = rng.range_i32(2, 6);
    let bias = rng.range_i32(1, 5);
    let name = format!("tail_recur_{idx:02}");
    let rr_src = format!(
        r#"
fn step(n, k, bias) {{
  if ((n % 2L) == 0L) {{
    return (n * k) + bias;
  }} else {{
    return (n * k) - bias;
  }}
}}

fn recur(n, k, bias, acc) {{
  if (n <= 0L) {{
    return acc;
  }} else {{
    return recur(n - 1L, k, bias, acc + step(n, k, bias));
  }}
}}

fn main() {{
  let out = recur({n}L, {k}L, {bias}L, 0L);
  print(out);
  return out;
}}

print(main());
"#
    );
    let ref_r_src = format!(
        r#"
step <- function(n, k, bias) {{
  if ((n %% 2L) == 0L) {{
    (n * k) + bias
  }} else {{
    (n * k) - bias
  }}
}}

recur <- function(n, k, bias, acc) {{
  if (n <= 0L) {{
    acc
  }} else {{
    recur(n - 1L, k, bias, acc + step(n, k, bias))
  }}
}}

main <- function() {{
  out <- recur({n}L, {k}L, {bias}L, 0L)
  print(out)
  out
}}

print(main())
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

fn record_state_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let n = rng.range_i32(4, 10);
    let gain = rng.range_i32(2, 6);
    let shift = rng.range_i32(-3, 4);
    let name = format!("record_state_{idx:02}");
    let rr_src = format!(
        r#"
fn update_box(box, step, gain, shift) {{
  box.total = box.total + ((step * gain) + shift);
  if ((step % 2L) == 0L) {{
    box.evens = box.evens + step;
  }} else {{
    box.odds = box.odds + step;
  }}
  return box;
}}

fn main() {{
  let box = {{total: 0L, evens: 0L, odds: 0L}};
  let i = 1L;
  while (i <= {n}L) {{
    let box = update_box(box, i, {gain}L, {shift}L);
    i = i + 1L;
  }}
  print(box.total);
  print(box.evens);
  print(box.odds);
  return box.total + box.evens + box.odds;
}}

print(main());
"#
    );
    let ref_r_src = format!(
        r#"
update_box <- function(box, step, gain, shift) {{
  box$total <- box$total + ((step * gain) + shift)
  if ((step %% 2L) == 0L) {{
    box$evens <- box$evens + step
  }} else {{
    box$odds <- box$odds + step
  }}
  box
}}

main <- function() {{
  box <- list(total = 0L, evens = 0L, odds = 0L)
  i <- 1L
  while (i <= {n}L) {{
    box_inner <- update_box(box, i, {gain}L, {shift}L)
    i <- i + 1L
  }}
  print(box$total)
  print(box$evens)
  print(box$odds)
  box$total + box$evens + box$odds
}}

print(main())
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

fn stats_namespace_case(idx: usize, rng: &mut Lcg) -> GeneratedCase {
    let n = rng.range_i32(6, 14);
    let scale = rng.range_i32(2, 5);
    let shift = rng.range_i32(-2, 6);
    let q = if rng.range_i32(0, 1) == 0 {
        "0.25"
    } else {
        "0.75"
    };
    let name = format!("stats_namespace_{idx:02}");
    let rr_src = format!(
        r#"
import r default from "stats";

fn main() {{
  let xs = seq_len({n}L);
  let ys = seq_len({n}L);
  let i = 1L;
  while (i <= {n}L) {{
    ys[i] = (xs[i] * {scale}L) + {shift}L;
    i = i + 1L;
  }}
  let center = stats.median(ys);
  let spread = stats.sd(ys);
  let qv = stats.quantile(ys, probs = c({q}));
  print(center);
  print(spread);
  print(qv[1L]);
  return center + qv[1L];
}}

print(main());
"#
    );
    let ref_r_src = format!(
        r#"
main <- function() {{
  xs <- seq_len({n}L)
  ys <- seq_len({n}L)
  i <- 1L
  while (i <= {n}L) {{
    ys[i] <- (xs[i] * {scale}L) + {shift}L
    i <- i + 1L
  }}
  center <- stats::median(ys)
  spread <- stats::sd(ys)
  qv <- stats::quantile(ys, probs = c({q}))
  print(center)
  print(spread)
  print(qv[1L])
  center + qv[1L]
}}

print(main())
"#
    );
    GeneratedCase {
        name,
        rr_src,
        ref_r_src,
    }
}

pub fn suite_summary(cases: &[GeneratedCase]) -> String {
    let mut out = String::new();
    for case in cases {
        let _ = writeln!(&mut out, "{}", case.name);
    }
    out
}
