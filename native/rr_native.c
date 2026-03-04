#include <R.h>
#include <R_ext/Rdynload.h>
#include <Rinternals.h>
#include <math.h>

#ifdef _OPENMP
#include <omp.h>
#endif

static R_len_t out_len(SEXP a, SEXP b) {
  R_len_t la = XLENGTH(a);
  R_len_t lb = XLENGTH(b);
  if (la == lb) return la;
  if (la == 1) return lb;
  if (lb == 1) return la;
  return la > lb ? la : lb;
}

static void warn_recycle(R_len_t la, R_len_t lb) {
  if (la == 0 || lb == 0) return;
  if (la == lb || la == 1 || lb == 1) return;
  if ((la % lb) != 0 && (lb % la) != 0) {
    warning("longer object length is not a multiple of shorter object length");
  }
}

static SEXP as_real(SEXP x) {
  if (TYPEOF(x) == REALSXP) {
    return x;
  }
  return PROTECT(coerceVector(x, REALSXP));
}

static SEXP rr_binop(SEXP a, SEXP b, int op) {
  int protect_count = 0;
  SEXP ar = a;
  SEXP br = b;
  if (TYPEOF(ar) != REALSXP) {
    ar = PROTECT(coerceVector(ar, REALSXP));
    protect_count++;
  }
  if (TYPEOF(br) != REALSXP) {
    br = PROTECT(coerceVector(br, REALSXP));
    protect_count++;
  }

  R_len_t la = XLENGTH(ar);
  R_len_t lb = XLENGTH(br);
  R_len_t n = out_len(ar, br);
  warn_recycle(la, lb);

  SEXP out = PROTECT(allocVector(REALSXP, n));
  protect_count++;
  double* pa = REAL(ar);
  double* pb = REAL(br);
  double* po = REAL(out);

  for (R_len_t i = 0; i < n; i++) {
    double av = pa[la == 0 ? 0 : i % la];
    double bv = pb[lb == 0 ? 0 : i % lb];
    switch (op) {
      case 0: po[i] = av + bv; break;
      case 1: po[i] = av - bv; break;
      case 2: po[i] = av * bv; break;
      case 3: po[i] = av / bv; break;
      case 4: po[i] = av > bv ? av : bv; break;
      case 5: po[i] = av < bv ? av : bv; break;
      default: po[i] = av / bv; break;
    }
  }

  UNPROTECT(protect_count);
  return out;
}

static int as_int_default(SEXP x, int fallback) {
  if (x == R_NilValue || XLENGTH(x) < 1) {
    return fallback;
  }
  int out = asInteger(x);
  if (out == NA_INTEGER) {
    return fallback;
  }
  return out;
}

static SEXP rr_binop_omp(SEXP a, SEXP b, SEXP threads_sxp, SEXP min_trip_sxp, int op) {
  int threads = as_int_default(threads_sxp, 0);
  int min_trip = as_int_default(min_trip_sxp, 4096);

  int protect_count = 0;
  SEXP ar = a;
  SEXP br = b;
  if (TYPEOF(ar) != REALSXP) {
    ar = PROTECT(coerceVector(ar, REALSXP));
    protect_count++;
  }
  if (TYPEOF(br) != REALSXP) {
    br = PROTECT(coerceVector(br, REALSXP));
    protect_count++;
  }

  R_len_t la = XLENGTH(ar);
  R_len_t lb = XLENGTH(br);
  R_len_t n = out_len(ar, br);
  warn_recycle(la, lb);

  if (n <= 0 || n < min_trip) {
    SEXP out = PROTECT(allocVector(REALSXP, n));
    protect_count++;
    double* pa = REAL(ar);
    double* pb = REAL(br);
    double* po = REAL(out);
    for (R_len_t i = 0; i < n; i++) {
      double av = pa[la == 0 ? 0 : i % la];
      double bv = pb[lb == 0 ? 0 : i % lb];
      switch (op) {
        case 0: po[i] = av + bv; break;
        case 1: po[i] = av - bv; break;
        case 2: po[i] = av * bv; break;
        case 3: po[i] = av / bv; break;
        case 4: po[i] = av > bv ? av : bv; break;
        case 5: po[i] = av < bv ? av : bv; break;
        default: po[i] = av / bv; break;
      }
    }
    UNPROTECT(protect_count);
    return out;
  }

  SEXP out = PROTECT(allocVector(REALSXP, n));
  protect_count++;
  double* pa = REAL(ar);
  double* pb = REAL(br);
  double* po = REAL(out);

#ifdef _OPENMP
  if (threads > 0) {
    omp_set_num_threads(threads);
  }
#pragma omp parallel for
#endif
  for (R_len_t i = 0; i < n; i++) {
    double av = pa[la == 0 ? 0 : i % la];
    double bv = pb[lb == 0 ? 0 : i % lb];
    switch (op) {
      case 0: po[i] = av + bv; break;
      case 1: po[i] = av - bv; break;
      case 2: po[i] = av * bv; break;
      case 3: po[i] = av / bv; break;
      case 4: po[i] = av > bv ? av : bv; break;
      case 5: po[i] = av < bv ? av : bv; break;
      default: po[i] = av / bv; break;
    }
  }

  UNPROTECT(protect_count);
  return out;
}

SEXP rr_vec_add_f64(SEXP a, SEXP b) { return rr_binop(a, b, 0); }
SEXP rr_vec_sub_f64(SEXP a, SEXP b) { return rr_binop(a, b, 1); }
SEXP rr_vec_mul_f64(SEXP a, SEXP b) { return rr_binop(a, b, 2); }
SEXP rr_vec_div_f64(SEXP a, SEXP b) { return rr_binop(a, b, 3); }

SEXP rr_vec_add_f64_omp(SEXP a, SEXP b, SEXP threads, SEXP min_trip) {
  return rr_binop_omp(a, b, threads, min_trip, 0);
}
SEXP rr_vec_sub_f64_omp(SEXP a, SEXP b, SEXP threads, SEXP min_trip) {
  return rr_binop_omp(a, b, threads, min_trip, 1);
}
SEXP rr_vec_mul_f64_omp(SEXP a, SEXP b, SEXP threads, SEXP min_trip) {
  return rr_binop_omp(a, b, threads, min_trip, 2);
}
SEXP rr_vec_div_f64_omp(SEXP a, SEXP b, SEXP threads, SEXP min_trip) {
  return rr_binop_omp(a, b, threads, min_trip, 3);
}

SEXP rr_vec_abs_f64(SEXP a) {
  SEXP ar = PROTECT(coerceVector(a, REALSXP));
  R_len_t n = XLENGTH(ar);
  SEXP out = PROTECT(allocVector(REALSXP, n));
  double* pa = REAL(ar);
  double* po = REAL(out);
  for (R_len_t i = 0; i < n; i++) {
    po[i] = pa[i] < 0 ? -pa[i] : pa[i];
  }
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_abs_f64_omp(SEXP a, SEXP threads_sxp, SEXP min_trip_sxp) {
  int threads = as_int_default(threads_sxp, 0);
  int min_trip = as_int_default(min_trip_sxp, 4096);
  SEXP ar = PROTECT(coerceVector(a, REALSXP));
  R_len_t n = XLENGTH(ar);
  SEXP out = PROTECT(allocVector(REALSXP, n));
  double* pa = REAL(ar);
  double* po = REAL(out);
  if (n <= 0 || n < min_trip) {
    for (R_len_t i = 0; i < n; i++) {
      po[i] = pa[i] < 0 ? -pa[i] : pa[i];
    }
    UNPROTECT(2);
    return out;
  }
#ifdef _OPENMP
  if (threads > 0) {
    omp_set_num_threads(threads);
  }
#pragma omp parallel for
#endif
  for (R_len_t i = 0; i < n; i++) {
    po[i] = pa[i] < 0 ? -pa[i] : pa[i];
  }
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_log_f64(SEXP a) {
  SEXP call = PROTECT(lang2(install("log"), a));
  SEXP out = PROTECT(eval(call, R_BaseEnv));
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_log_f64_omp(SEXP a, SEXP threads_sxp, SEXP min_trip_sxp) {
  int threads = as_int_default(threads_sxp, 0);
  int min_trip = as_int_default(min_trip_sxp, 4096);
  SEXP ar = PROTECT(coerceVector(a, REALSXP));
  R_len_t n = XLENGTH(ar);
  SEXP out = PROTECT(allocVector(REALSXP, n));
  double* pa = REAL(ar);
  double* po = REAL(out);
  if (n <= 0 || n < min_trip) {
    for (R_len_t i = 0; i < n; i++) {
      po[i] = log(pa[i]);
    }
    UNPROTECT(2);
    return out;
  }
#ifdef _OPENMP
  if (threads > 0) {
    omp_set_num_threads(threads);
  }
#pragma omp parallel for
#endif
  for (R_len_t i = 0; i < n; i++) {
    po[i] = log(pa[i]);
  }
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_sqrt_f64(SEXP a) {
  SEXP call = PROTECT(lang2(install("sqrt"), a));
  SEXP out = PROTECT(eval(call, R_BaseEnv));
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_sqrt_f64_omp(SEXP a, SEXP threads_sxp, SEXP min_trip_sxp) {
  int threads = as_int_default(threads_sxp, 0);
  int min_trip = as_int_default(min_trip_sxp, 4096);
  SEXP ar = PROTECT(coerceVector(a, REALSXP));
  R_len_t n = XLENGTH(ar);
  SEXP out = PROTECT(allocVector(REALSXP, n));
  double* pa = REAL(ar);
  double* po = REAL(out);
  if (n <= 0 || n < min_trip) {
    for (R_len_t i = 0; i < n; i++) {
      po[i] = sqrt(pa[i]);
    }
    UNPROTECT(2);
    return out;
  }
#ifdef _OPENMP
  if (threads > 0) {
    omp_set_num_threads(threads);
  }
#pragma omp parallel for
#endif
  for (R_len_t i = 0; i < n; i++) {
    po[i] = sqrt(pa[i]);
  }
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_pmax_f64(SEXP a, SEXP b) {
  SEXP call = PROTECT(lang3(install("pmax"), a, b));
  SEXP out = PROTECT(eval(call, R_BaseEnv));
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_pmax_f64_omp(SEXP a, SEXP b, SEXP threads, SEXP min_trip) {
  return rr_binop_omp(a, b, threads, min_trip, 4);
}

SEXP rr_vec_pmin_f64(SEXP a, SEXP b) {
  SEXP call = PROTECT(lang3(install("pmin"), a, b));
  SEXP out = PROTECT(eval(call, R_BaseEnv));
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_pmin_f64_omp(SEXP a, SEXP b, SEXP threads, SEXP min_trip) {
  return rr_binop_omp(a, b, threads, min_trip, 5);
}

SEXP rr_vec_sum_f64(SEXP a) {
  SEXP call = PROTECT(lang2(install("sum"), a));
  SEXP out = PROTECT(eval(call, R_BaseEnv));
  UNPROTECT(2);
  return out;
}

SEXP rr_vec_mean_f64(SEXP a) {
  SEXP call = PROTECT(lang2(install("mean"), a));
  SEXP out = PROTECT(eval(call, R_BaseEnv));
  UNPROTECT(2);
  return out;
}

static const R_CallMethodDef rr_call_methods[] = {
    {"rr_vec_add_f64", (DL_FUNC) &rr_vec_add_f64, 2},
    {"rr_vec_sub_f64", (DL_FUNC) &rr_vec_sub_f64, 2},
    {"rr_vec_mul_f64", (DL_FUNC) &rr_vec_mul_f64, 2},
    {"rr_vec_div_f64", (DL_FUNC) &rr_vec_div_f64, 2},
    {"rr_vec_add_f64_omp", (DL_FUNC) &rr_vec_add_f64_omp, 4},
    {"rr_vec_sub_f64_omp", (DL_FUNC) &rr_vec_sub_f64_omp, 4},
    {"rr_vec_mul_f64_omp", (DL_FUNC) &rr_vec_mul_f64_omp, 4},
    {"rr_vec_div_f64_omp", (DL_FUNC) &rr_vec_div_f64_omp, 4},
    {"rr_vec_abs_f64", (DL_FUNC) &rr_vec_abs_f64, 1},
    {"rr_vec_abs_f64_omp", (DL_FUNC) &rr_vec_abs_f64_omp, 3},
    {"rr_vec_log_f64", (DL_FUNC) &rr_vec_log_f64, 1},
    {"rr_vec_log_f64_omp", (DL_FUNC) &rr_vec_log_f64_omp, 3},
    {"rr_vec_sqrt_f64", (DL_FUNC) &rr_vec_sqrt_f64, 1},
    {"rr_vec_sqrt_f64_omp", (DL_FUNC) &rr_vec_sqrt_f64_omp, 3},
    {"rr_vec_pmax_f64", (DL_FUNC) &rr_vec_pmax_f64, 2},
    {"rr_vec_pmax_f64_omp", (DL_FUNC) &rr_vec_pmax_f64_omp, 4},
    {"rr_vec_pmin_f64", (DL_FUNC) &rr_vec_pmin_f64, 2},
    {"rr_vec_pmin_f64_omp", (DL_FUNC) &rr_vec_pmin_f64_omp, 4},
    {"rr_vec_sum_f64", (DL_FUNC) &rr_vec_sum_f64, 1},
    {"rr_vec_mean_f64", (DL_FUNC) &rr_vec_mean_f64, 1},
    {NULL, NULL, 0}
};

void R_init_rr_native(DllInfo* dll) {
  R_registerRoutines(dll, NULL, rr_call_methods, NULL, NULL);
  R_useDynamicSymbols(dll, FALSE);
}
