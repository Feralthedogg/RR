use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use RR::hir::def::{HirItem, HirStmt, ModuleId, Ty};
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
fn typed_hints_and_fn_short_are_lowered() {
    let src = r#"
fn add(a: f64, b: i64) -> f64 = a + b

fn main() {
  z: int = 10L
  add(1.5, z)
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse_program().expect("parse");
    let mut lowerer = Lowerer::new();
    let (hir_mod, symbols) = lowerer.lower_module(ast, ModuleId(0)).expect("lower");

    let mut add_fn = None;
    let mut main_fn = None;
    for it in hir_mod.items {
        if let HirItem::Fn(f) = it {
            let name = symbols.get(&f.name).cloned().unwrap_or_default();
            if name == "add" {
                add_fn = Some(f.clone());
            } else if name == "main" {
                main_fn = Some(f.clone());
            }
        }
    }

    let add_fn = add_fn.expect("add fn");
    assert_eq!(add_fn.ret_ty, Some(Ty::Double));
    assert_eq!(add_fn.params.len(), 2);
    assert_eq!(add_fn.params[0].ty, Some(Ty::Double));
    assert_eq!(add_fn.params[1].ty, Some(Ty::Int));

    let main_fn = main_fn.expect("main fn");
    let typed_let = main_fn.body.stmts.iter().find_map(|s| match s {
        HirStmt::Let {
            name, ty: Some(t), ..
        } if symbols.get(name).map(|n| n == "z").unwrap_or(false) => Some(t.clone()),
        _ => None,
    });
    assert_eq!(typed_let, Some(Ty::Int));
}

#[test]
fn hybrid_surface_syntax_compiles_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping hybrid syntax runtime test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("hybrid_syntax");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join("hybrid_syntax.rr");
    let out_r = out_dir.join("hybrid_syntax_o2.R");

    let src = r#"
fn add(a: float, b: float) -> float = a + b

main <- function() {
  x: int = 10L
  y = add(1.0, x)
  print(y)
  y
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
    let out = stdout.replace("\r\n", "\n");
    assert!(
        out.contains("[1] 11"),
        "unexpected output:\n{}\nstderr:\n{}",
        out,
        stderr
    );
}

#[test]
fn native_for_range_and_compound_assign_lower() {
    let src = r#"
fn main(n) {
  let s = 0L
  for i in 1L..n {
    s += i
  }
  s
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse_program().expect("parse");
    let mut lowerer = Lowerer::new();
    let (hir_mod, symbols) = lowerer.lower_module(ast, ModuleId(0)).expect("lower");

    let main_fn = hir_mod
        .items
        .into_iter()
        .find_map(|it| match it {
            HirItem::Fn(f) if symbols.get(&f.name).map(|n| n == "main").unwrap_or(false) => Some(f),
            _ => None,
        })
        .expect("main fn");

    let for_stmt = main_fn.body.stmts.iter().find_map(|s| match s {
        HirStmt::For { iter, body, .. } => Some((iter, body)),
        _ => None,
    });
    let (iter, body) = for_stmt.expect("for stmt");

    match iter {
        RR::hir::def::HirForIter::Range {
            start,
            end,
            inclusive,
            ..
        } => {
            assert!(*inclusive, "for-range must stay inclusive");
            assert!(matches!(start, RR::hir::def::HirExpr::Lit(_)));
            assert!(matches!(
                end,
                RR::hir::def::HirExpr::Local(_) | RR::hir::def::HirExpr::Global(_)
            ));
        }
        _ => panic!("expected canonical range iterator"),
    }

    let assign_stmt = body.stmts.iter().find_map(|s| match s {
        HirStmt::Assign { value, .. } => Some(value),
        _ => None,
    });
    let value = assign_stmt.expect("compound assignment lowered to assign");
    assert!(matches!(
        value,
        RR::hir::def::HirExpr::Binary {
            op: RR::hir::def::HirBinOp::Add,
            ..
        }
    ));
}

#[test]
fn compound_assign_supports_index_and_field_targets() {
    let src = r#"
fn main() {
  let arr = [1L, 2L, 3L]
  let rec = {x: 10L}
  arr[1L] += 2L
  rec.x -= 3L
  arr[1L] + rec.x
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse_program().expect("parse");
    let mut lowerer = Lowerer::new();
    let (hir_mod, symbols) = lowerer.lower_module(ast, ModuleId(0)).expect("lower");

    let main_fn = hir_mod
        .items
        .into_iter()
        .find_map(|it| match it {
            HirItem::Fn(f) if symbols.get(&f.name).map(|n| n == "main").unwrap_or(false) => Some(f),
            _ => None,
        })
        .expect("main fn");

    let mut saw_index_assign = false;
    let mut saw_field_assign = false;
    for stmt in &main_fn.body.stmts {
        if let HirStmt::Assign { target, value, .. } = stmt {
            match target {
                RR::hir::def::HirLValue::Index { .. } => {
                    saw_index_assign = matches!(
                        value,
                        RR::hir::def::HirExpr::Binary {
                            op: RR::hir::def::HirBinOp::Add,
                            ..
                        }
                    );
                }
                RR::hir::def::HirLValue::Field { .. } => {
                    saw_field_assign = matches!(
                        value,
                        RR::hir::def::HirExpr::Binary {
                            op: RR::hir::def::HirBinOp::Sub,
                            ..
                        }
                    );
                }
                _ => {}
            }
        }
    }

    assert!(saw_index_assign, "index compound assignment not lowered");
    assert!(saw_field_assign, "field compound assignment not lowered");
}
