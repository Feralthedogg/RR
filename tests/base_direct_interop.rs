mod common;

use common::{run_compile_case, run_rscript};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    Some("Rscript".to_string())
}

fn rscript_available(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn base_direct_interop_core_helpers_match_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping base direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("base_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = out_dir.join("case.rr");
    let o0_path = out_dir.join("case_o0.R");
    let o2_path = out_dir.join("case_o2.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"
import r { set.seed } from "base"
import r { set.seed } from "base"

fn main() {
  let chars = base.character(3L)
  let flags = base.logical(4L)
  let ids = base.integer(2L)
  let vals = base.double(2L)
  let via_char_vector = base.vector("character", 3L)
  let via_list_vector = base.vector("list", 2L)
  let nums = base.numeric(3L)
  let seq = base.c(1L, 2L, 3L)
  let dup = base.rep(2L, 4L)
  let keep = base.any(c(FALSE, TRUE, FALSE))
  let ok = base.all(c(TRUE, TRUE, TRUE))
  let picks = base.which(c(FALSE, TRUE, FALSE, TRUE))
  let p = base.prod(c(2L, 3L, 4L))
  let s = base.sum(base.c(1L, 2L, 3L))
  let m = base.mean(c(2.0, 4.0, 6.0))
  let a = base.paste("alpha", "beta")
  let b = base.paste0("x", 2L)
  let c = base.sprintf("%s-%d", "z", 3L)
  let nm = base.list(base.c("r1", "r2"), base.c("c1", "c2"))
  let mat = base.matrix(base.c(1.0, 2.0, 3.0, 4.0), nrow = 2L, ncol = 2L, dimnames = nm)
  let dims = base.dim(mat)
  let dn = base.dimnames(mat)
  let rows = base.nrow(mat)
  let cols = base.ncol(mat)
  let n = base.length(seq)
  let out = base.cat("cat", "line", "\n", sep = "-")

  print(length(chars))
  print(length(flags))
  print(length(ids))
  print(length(vals))
  print(length(via_char_vector))
  print(length(via_list_vector))
  print(length(nums))
  print(seq)
  print(dup)
  print(keep)
  print(ok)
  print(picks)
  print(p)
  print(s)
  print(m)
  print(a)
  print(b)
  print(c)
  print(dims)
  print(dn)
  print(rows)
  print(cols)
  print(n)
  print(out)
}

main()
"#;

    fs::write(&rr_src, src).expect("failed to write RR source");

    let status_o0 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o0_path)
        .arg("-O0")
        .status()
        .expect("failed to compile O0 case");
    assert!(status_o0.success(), "O0 compile failed");

    let status_o2 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o2_path)
        .arg("-O2")
        .status()
        .expect("failed to compile O2 case");
    assert!(status_o2.success(), "O2 compile failed");

    let o0 = run_rscript(&rscript, &o0_path);
    let o2 = run_rscript(&rscript, &o2_path);

    assert_eq!(o0.status, 0, "O0 runtime failed:\n{}", o0.stderr);
    assert_eq!(o2.status, 0, "O2 runtime failed:\n{}", o2.stderr);
    assert_eq!(o0.stdout, o2.stdout, "O0/O2 stdout mismatch");
    assert_eq!(o0.stderr, o2.stderr, "O0/O2 stderr mismatch");
}

#[test]
fn base_direct_interop_matrix_helpers_match_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping base direct matrix runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("base_direct_interop_matrix");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = out_dir.join("case.rr");
    let o0_path = out_dir.join("case_o0.R");
    let o2_path = out_dir.join("case_o2.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"

fn main() {
  let seq = base.seq_len(4L)
  let mat = base.matrix(base.c(1.0, 2.0, 3.0, 4.0), nrow = 2L, ncol = 2L)
  print(seq)
  print(base.seq_along(seq))
  print(base.t(mat))
  print(base.diag(mat))
  print(base.rbind(mat, mat))
  print(base.cbind(mat, mat))
  print(base.rowSums(mat))
  print(base.colSums(mat))
  print(base.crossprod(mat))
  print(base.tcrossprod(mat))
}

main()
"#;

    fs::write(&rr_src, src).expect("failed to write RR source");

    let status_o0 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o0_path)
        .arg("-O0")
        .status()
        .expect("failed to compile O0 case");
    assert!(status_o0.success(), "O0 compile failed");

    let status_o2 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o2_path)
        .arg("-O2")
        .status()
        .expect("failed to compile O2 case");
    assert!(status_o2.success(), "O2 compile failed");

    let o0 = run_rscript(&rscript, &o0_path);
    let o2 = run_rscript(&rscript, &o2_path);

    assert_eq!(o0.status, 0, "O0 runtime failed:\n{}", o0.stderr);
    assert_eq!(o2.status, 0, "O2 runtime failed:\n{}", o2.stderr);
    assert_eq!(o0.stdout, o2.stdout, "O0/O2 stdout mismatch");
    assert_eq!(o0.stderr, o2.stderr, "O0/O2 stderr mismatch");
}

#[test]
fn base_direct_interop_numeric_helpers_match_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping base direct numeric runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("base_direct_interop_numeric");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = out_dir.join("case.rr");
    let o0_path = out_dir.join("case_o0.R");
    let o2_path = out_dir.join("case_o2.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"
import r { set.seed } from "base"

fn main() {
  print(base.abs(base.c(-1.0, 2.0)))
  print(base.min(base.c(3.0, 1.0, 2.0)))
  print(base.max(base.c(3.0, 1.0, 2.0)))
  print(base.pmax(base.c(1.0, 5.0), base.c(2.0, 4.0)))
  print(base.pmin(base.c(1.0, 5.0), base.c(2.0, 4.0)))
  print(base.sqrt(base.c(1.0, 4.0, 9.0)))
  print(base.log(1.0))
  print(base.log10(100.0))
  print(base.log2(8.0))
  print(base.exp(1.0))
  print(base.atan2(1.0, 1.0))
  print(base.sin(0.0))
  print(base.cos(0.0))
  print(base.tan(0.0))
  print(base.asin(0.0))
  print(base.acos(1.0))
  print(base.atan(1.0))
  print(base.sinh(0.0))
  print(base.cosh(0.0))
  print(base.tanh(0.0))
  print(base.sign(base.c(-1.0, 0.0, 1.0)))
  print(base.gamma(4.0))
  print(base.lgamma(4.0))
  print(base.floor(base.c(1.2, 2.8)))
  print(base.ceiling(base.c(1.2, 2.8)))
  print(base.trunc(base.c(1.2, 2.8)))
  print(base.round(base.c(1.2, 2.8)))
  print(base.rep.int(3L, 4L))
  print(base.is.na(base.c(1.0, 2.0)))
  print(base.is.finite(base.c(1.0, 2.0)))
  let shown = base.print(base.c(1L, 2L))
  print(shown)
}

main()
"#;

    fs::write(&rr_src, src).expect("failed to write RR source");

    let status_o0 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o0_path)
        .arg("-O0")
        .status()
        .expect("failed to compile O0 case");
    assert!(status_o0.success(), "O0 compile failed");

    let status_o2 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o2_path)
        .arg("-O2")
        .status()
        .expect("failed to compile O2 case");
    assert!(status_o2.success(), "O2 compile failed");

    let o0 = run_rscript(&rscript, &o0_path);
    let o2 = run_rscript(&rscript, &o2_path);

    assert_eq!(o0.status, 0, "O0 runtime failed:\n{}", o0.stderr);
    assert_eq!(o2.status, 0, "O2 runtime failed:\n{}", o2.stderr);
    assert_eq!(o0.stdout, o2.stdout, "O0/O2 stdout mismatch");
    assert_eq!(o0.stderr, o2.stderr, "O0/O2 stderr mismatch");
}

#[test]
fn base_direct_interop_string_helpers_match_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping base direct string runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("base_direct_interop_string");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = out_dir.join("case.rr");
    let o0_path = out_dir.join("case_o0.R");
    let o2_path = out_dir.join("case_o2.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"

fn main() {
  let txt = base.c("Alpha", "beta", "gamma")
  let nums = base.c(3.0, 1.0, 2.0)
  print(base.tolower(txt))
  print(base.toupper(txt))
  print(base.nchar(txt))
  print(base.nzchar(base.c("a", "")))
  print(base.substr(base.c("alphabet", "beta"), 2L, 4L))
  print(base.sub("a", "x", txt))
  print(base.gsub("a", "x", txt))
  print(base.grepl("a", txt))
  print(base.grep("a", txt))
  print(base.startsWith(txt, "A"))
  print(base.endsWith(txt, "a"))
  print(base.trimws(base.c(" a ", "b ")))
  print(base.chartr("ab", "xy", txt))
  print(base.strsplit(base.c("a,b", "c,d"), ","))
  print(base.regexpr("a", txt))
  print(base.gregexpr("a", txt))
  print(base.regexec("a", txt))
  print(base.agrep("a", txt))
  print(base.agrepl("a", txt))
  print(base.which.min(nums))
  print(base.which.max(nums))
  print(base.isTRUE(TRUE))
  print(base.isFALSE(FALSE))
  print(base.lengths(base.list(base.c(1L), base.c(1L, 2L), base.c(1L, 2L, 3L))))
  print(base.union(base.c(1L, 2L, 3L), base.c(2L, 3L, 4L)))
  print(base.intersect(base.c(1L, 2L, 3L), base.c(2L, 3L, 4L)))
  print(base.setdiff(base.c(1L, 2L, 3L), base.c(2L, 3L, 4L)))
  print(base.seq(1L, 4L))
  print(base.seq(1.0, 2.0, 0.5))
  print(base.ifelse(base.c(TRUE, FALSE), base.c(1L, 2L), base.c(3L, 4L)))
  print(base.ifelse(TRUE, 1L, 3L))
  print(base.rank(base.c(30.0, 10.0, 20.0)))
}

main()
"#;

    fs::write(&rr_src, src).expect("failed to write RR source");

    let status_o0 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o0_path)
        .arg("-O0")
        .status()
        .expect("failed to compile O0 case");
    assert!(status_o0.success(), "O0 compile failed");

    let status_o2 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o2_path)
        .arg("-O2")
        .status()
        .expect("failed to compile O2 case");
    assert!(status_o2.success(), "O2 compile failed");

    let o0 = run_rscript(&rscript, &o0_path);
    let o2 = run_rscript(&rscript, &o2_path);

    assert_eq!(o0.status, 0, "O0 runtime failed:\n{}", o0.stderr);
    assert_eq!(o2.status, 0, "O2 runtime failed:\n{}", o2.stderr);
    assert_eq!(o0.stdout, o2.stdout, "O0/O2 stdout mismatch");
    assert_eq!(o0.stderr, o2.stderr, "O0/O2 stderr mismatch");
}

#[test]
fn base_direct_interop_sampling_helpers_compile_without_opaque_warning() {
    let src = r#"
import r * as base from "base"

fn main() -> int {
  let a = base.sample(base.c(10L, 20L, 30L, 40L), 2L)
  let b = base.sample(5L, 3L)
  let c = base.sample.int(5L, 3L)
  let d = base.rank(base.c(30.0, 10.0, 20.0))
  print(a)
  print(b)
  print(c)
  print(d)
  return base.length(a) + base.length(b) + base.length(c) + base.length(d)
}

print(main())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("base_direct_sampling_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "base sampling helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}

#[test]
fn base_direct_interop_factor_helpers_match_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping base direct factor runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("base_direct_interop_factor");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = out_dir.join("case.rr");
    let o0_path = out_dir.join("case_o0.R");
    let o2_path = out_dir.join("case_o2.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"

fn main() {
  let f = base.factor(base.c("a", "b", "a"))
  let ct = base.cut(base.c(1.0, 2.0, 3.0), 2L)
  let tb = base.table(base.c("a", "b", "a"))
  print(f)
  print(ct)
  print(tb)
  print(base.length(f))
  print(base.length(ct))
  print(base.length(tb))
}

main()
"#;

    fs::write(&rr_src, src).expect("failed to write RR source");

    let status_o0 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o0_path)
        .arg("-O0")
        .status()
        .expect("failed to compile O0 case");
    assert!(status_o0.success(), "O0 compile failed");

    let status_o2 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o2_path)
        .arg("-O2")
        .status()
        .expect("failed to compile O2 case");
    assert!(status_o2.success(), "O2 compile failed");

    let o0 = run_rscript(&rscript, &o0_path);
    let o2 = run_rscript(&rscript, &o2_path);

    assert_eq!(o0.status, 0, "O0 runtime failed:\n{}", o0.stderr);
    assert_eq!(o2.status, 0, "O2 runtime failed:\n{}", o2.stderr);
    assert_eq!(o0.stdout, o2.stdout, "O0/O2 stdout mismatch");
    assert_eq!(o0.stderr, o2.stderr, "O0/O2 stderr mismatch");
}
