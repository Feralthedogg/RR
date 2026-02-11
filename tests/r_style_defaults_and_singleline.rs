use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use RR::hir::def::{HirItem, ModuleId, Ty};
use RR::hir::lower::Lowerer;
use RR::syntax::parse::Parser;

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

fn run_rscript(path: &str, script: &Path) -> (i32, String, String) {
    let output = Command::new(path)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn default_params_are_preserved_in_hir_and_typed() {
    let src = r#"
f <- function(a = 0.0, b = 0L, c = TRUE, d = "x") {
  a + b
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse_program().expect("parse");
    let mut lowerer = Lowerer::new();
    let (hir_mod, _symbols) = lowerer.lower_module(ast, ModuleId(0)).expect("lower");

    let f = hir_mod
        .items
        .into_iter()
        .find_map(|it| match it {
            HirItem::Fn(f) => Some(f),
            _ => None,
        })
        .expect("fn item");
    assert_eq!(f.params.len(), 4);
    assert!(f.params[0].default.is_some());
    assert!(f.params[1].default.is_some());
    assert!(f.params[2].default.is_some());
    assert!(f.params[3].default.is_some());
    assert_eq!(f.params[0].ty, Some(Ty::Double));
    assert_eq!(f.params[1].ty, Some(Ty::Int));
    assert_eq!(f.params[2].ty, Some(Ty::Logical));
    assert_eq!(f.params[3].ty, Some(Ty::Char));
}

#[test]
fn single_line_control_forms_compile_and_run() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping single_line_control_forms test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_style_defaults_and_singleline");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("singleline.rr");
    let out_r = out_dir.join("singleline_o2.R");

    let src = r#"
main <- function() {
  s <- 0L
  i <- 0L
  while (i < 5L) i <- i + 1L
  for (k in 1L..5L) s <- s + k
  if (i == 5L) s <- s + 100L else s <- 0L
  print(s)
  s
}

print(main())
"#;
    fs::write(&rr_path, src).expect("write source");

    let status = Command::new(rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_r)
        .arg("--no-runtime")
        .arg("-O2")
        .status()
        .expect("run RR");
    assert!(status.success(), "compile failed");

    let (code, stdout, stderr) = run_rscript(&rscript, &out_r);
    assert_eq!(code, 0, "R failed: {stderr}");
    assert_eq!(stdout.replace("\r\n", "\n"), "[1] 115\n[1] 115\n");
}

#[test]
fn single_line_if_newline_followup_stmt_is_not_postfix_chained() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping newline postfix chaining test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_style_defaults_and_singleline");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("singleline_if_newline.rr");
    let out_r = out_dir.join("singleline_if_newline_o1.R");

    let src = r#"
idx.cube <- function(f, x, y, size) {
  ff <- round(f)
  if (ff < 1L) ff <- 1L
  (ff - 1L) * size * size + y
}

print(idx.cube(2L, 1L, 3L, 4L))
"#;
    fs::write(&rr_path, src).expect("write source");

    let status = Command::new(rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_r)
        .arg("--no-runtime")
        .arg("-O1")
        .status()
        .expect("run RR");
    assert!(status.success(), "compile failed");

    let (code, stdout, stderr) = run_rscript(&rscript, &out_r);
    assert_eq!(code, 0, "R failed: {stderr}");
    assert_eq!(stdout.replace("\r\n", "\n"), "[1] 19\n");
}

#[test]
fn no_paren_if_while_forms_compile_and_run() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping no_paren_if_while test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_style_defaults_and_singleline");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("no_paren_if_while.rr");
    let out_r = out_dir.join("no_paren_if_while_o2.R");

    let src = r#"
main <- function() {
  i <- 0L
  s <- 0L
  while i < 4L {
    i <- i + 1L
  }
  if i == 4L {
    s <- 42L
  } else {
    s <- -1L
  }
  print(s)
  s
}

print(main())
"#;
    fs::write(&rr_path, src).expect("write source");

    let status = Command::new(rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_r)
        .arg("--no-runtime")
        .arg("-O2")
        .status()
        .expect("run RR");
    assert!(status.success(), "compile failed");

    let (code, stdout, stderr) = run_rscript(&rscript, &out_r);
    assert_eq!(code, 0, "R failed: {stderr}");
    assert_eq!(stdout.replace("\r\n", "\n"), "[1] 42\n[1] 42\n");
}
