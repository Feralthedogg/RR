use super::*;
#[test]
pub(crate) fn removes_dead_unused_scalar_index_reads_and_pure_call_bindings() {
    let input = "\
Sym_287 <- function(temp, q_r, q_i, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T <- temp[i]\n\
qr <- q_r[i]\n\
qi <- q_i[i]\n\
es_ice <- (6.11 * exp(T))\n\
rr_mark(1, 1);\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("qr <- q_r[i]"), "{out}");
    assert!(!out.contains("qi <- q_i[i]"), "{out}");
    assert!(!out.contains("es_ice <- (6.11 * exp(T))"), "{out}");
    assert!(out.contains("rr_mark(1, 1);"), "{out}");
}

#[test]
pub(crate) fn inlines_single_use_named_scalar_index_reads() {
    let input = "\
Sym_287 <- function(temp, q_v, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T <- temp[i]\n\
qv <- q_v[i]\n\
T_c <- (T - 273.15)\n\
if ((qv > 0.01)) {\n\
  rr_mark(1, 1);\n\
  print(T_c)\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("T <- temp[i]"), "{out}");
    assert!(
        out.contains("T_c <- (temp[i] - 273.15)") || out.contains("T_c <- ((temp[i]) - 273.15)"),
        "{out}"
    );
    assert!(
        out.contains("if (((q_v[i]) > 0.01)) {")
            || out.contains("if ((q_v[i] > 0.01)) {")
            || out.contains("if ((qv > 0.01)) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn inlines_single_use_named_scalar_index_reads_across_if_boundaries() {
    let input = "\
Sym_287 <- function(temp, q_v, q_c, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T_c <- (temp[i] - 273.15)\n\
qc <- q_c[i]\n\
if ((T_c < (-(5)))) {\n\
  if ((qc > 0.0001)) {\n\
    rate <- (0.01 * qc)\n\
  }\n\
}\n\
qv <- q_v[i]\n\
if ((T_c < (-(15)))) {\n\
  if ((qv > 0.01)) {\n\
    print(T_c)\n\
  }\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("if ((q_c[i] > 0.0001)) {")
            || out.contains("if (((q_c[i]) > 0.0001)) {")
            || out.contains("if ((qc > 0.0001)) {"),
        "{out}"
    );
    assert!(
        out.contains("if ((q_v[i] > 0.01)) {")
            || out.contains("if (((q_v[i]) > 0.01)) {")
            || out.contains("if ((qv > 0.01)) {"),
        "{out}"
    );
}

#[test]
pub(crate) fn inlines_two_use_named_scalar_index_reads_across_if_boundaries() {
    let input = "\
Sym_287 <- function(temp, q_s, q_g, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
if (!(i <= size)) break\n\
T_c <- (temp[i] - 273.15)\n\
qs <- q_s[i]\n\
qg <- q_g[i]\n\
if ((T_c > 0)) {\n\
  melt_rate <- 0\n\
  if ((qs > 0)) {\n\
    melt_rate <- (qs * 0.05)\n\
  }\n\
  if ((qg > 0)) {\n\
    melt_rate <- (melt_rate + (qg * 0.02))\n\
  }\n\
}\n\
i <- (i + 1)\n\
next\n\
  }\n\
  return(0)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(
        out.contains("if ((q_s[i] > 0)) {")
            || out.contains("if (((q_s[i]) > 0)) {")
            || out.contains("if ((qs > 0)) {"),
        "{out}"
    );
    assert!(
        out.contains("melt_rate <- (q_s[i] * 0.05)")
            || out.contains("melt_rate <- ((q_s[i]) * 0.05)")
            || out.contains("melt_rate <- (qs * 0.05)"),
        "{out}"
    );
    assert!(
        out.contains("if ((q_g[i] > 0)) {")
            || out.contains("if (((q_g[i]) > 0)) {")
            || out.contains("if ((qg > 0)) {"),
        "{out}"
    );
    assert!(
        out.contains("melt_rate <- (melt_rate + (q_g[i] * 0.02))")
            || out.contains("melt_rate <- (melt_rate + ((q_g[i]) * 0.02))")
            || out.contains("melt_rate <- (melt_rate + (qg * 0.02))"),
        "{out}"
    );
}

#[test]
pub(crate) fn inlines_immediate_single_use_named_scalar_expr_into_following_assignment() {
    let input = "\
Sym_287 <- function(q_c, i) \n\
{\n\
  if ((q_c[i] > 0.0001)) {\n\
rate <- (0.01 * q_c[i])\n\
tendency_T <- (rate * L_f)\n\
  }\n\
  return(tendency_T)\n\
}\n";
    let out = optimize_emitted_r(input, true);
    assert!(!out.contains("rate <- (0.01 * q_c[i])"), "{out}");
    assert!(
        out.contains("tendency_T <- ((0.01 * q_c[i]) * L_f)")
            || out.contains("tendency_T <- (((0.01 * q_c[i]) * L_f))")
            || out.contains("tendency_T <- ((0.01 * (q_c[i])) * L_f)"),
        "{out}"
    );
}
