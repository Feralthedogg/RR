use crate::error::{RRCode, RRException, Stage};
use crate::syntax::ast::{Expr, ExprKind, Stmt, StmtKind};
use crate::syntax::parse::Parser;
use std::path::Path;

fn expr_contains_plain_main_call(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Call {
            callee,
            type_args: _,
            args,
        } => {
            matches!(&callee.kind, ExprKind::Name(name) if name == "main")
                || expr_contains_plain_main_call(callee)
                || args.iter().any(expr_contains_plain_main_call)
        }
        ExprKind::Unary { rhs, .. } => expr_contains_plain_main_call(rhs),
        ExprKind::Formula { lhs, rhs } => {
            lhs.as_deref().is_some_and(expr_contains_plain_main_call)
                || expr_contains_plain_main_call(rhs)
        }
        ExprKind::Binary { lhs, rhs, .. } => {
            expr_contains_plain_main_call(lhs) || expr_contains_plain_main_call(rhs)
        }
        ExprKind::Range { a, b } => {
            expr_contains_plain_main_call(a) || expr_contains_plain_main_call(b)
        }
        ExprKind::Lambda { body, .. } => body.stmts.iter().any(stmt_contains_plain_main_call),
        ExprKind::NamedArg { value, .. } => expr_contains_plain_main_call(value),
        ExprKind::Index { base, idx } => {
            expr_contains_plain_main_call(base) || idx.iter().any(expr_contains_plain_main_call)
        }
        ExprKind::Field { base, .. } => expr_contains_plain_main_call(base),
        ExprKind::VectorLit(items) => items.iter().any(expr_contains_plain_main_call),
        ExprKind::RecordLit(items) => items
            .iter()
            .any(|(_, expr)| expr_contains_plain_main_call(expr)),
        ExprKind::Pipe { lhs, rhs_call } => {
            expr_contains_plain_main_call(lhs) || expr_contains_plain_main_call(rhs_call)
        }
        ExprKind::Try { expr } => expr_contains_plain_main_call(expr),
        ExprKind::Match { scrutinee, arms } => {
            expr_contains_plain_main_call(scrutinee)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_deref()
                        .is_some_and(expr_contains_plain_main_call)
                        || expr_contains_plain_main_call(&arm.body)
                })
        }
        ExprKind::Unquote(expr) => expr_contains_plain_main_call(expr),
        ExprKind::Lit(_) | ExprKind::Name(_) | ExprKind::ColRef(_) | ExprKind::Column(_) => false,
    }
}

fn stmt_contains_plain_main_call(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Let { init, .. } => init.as_ref().is_some_and(expr_contains_plain_main_call),
        StmtKind::Assign { value, .. } => expr_contains_plain_main_call(value),
        StmtKind::FnDecl { .. }
        | StmtKind::TraitDecl(_)
        | StmtKind::ImplDecl(_)
        | StmtKind::Export(_)
        | StmtKind::Import { .. } => false,
        StmtKind::If {
            cond,
            then_blk,
            else_blk,
        } => {
            expr_contains_plain_main_call(cond)
                || then_blk.stmts.iter().any(stmt_contains_plain_main_call)
                || else_blk
                    .as_ref()
                    .is_some_and(|blk| blk.stmts.iter().any(stmt_contains_plain_main_call))
        }
        StmtKind::While { cond, body } => {
            expr_contains_plain_main_call(cond)
                || body.stmts.iter().any(stmt_contains_plain_main_call)
        }
        StmtKind::For { iter, body, .. } => {
            expr_contains_plain_main_call(iter)
                || body.stmts.iter().any(stmt_contains_plain_main_call)
        }
        StmtKind::Return { value } => value.as_ref().is_some_and(expr_contains_plain_main_call),
        StmtKind::ExprStmt { expr } | StmtKind::Expr(expr) => expr_contains_plain_main_call(expr),
        StmtKind::Break | StmtKind::Next | StmtKind::UnsafeRBlock { .. } => false,
    }
}

fn source_defines_main_function(source: &str) -> Result<(bool, bool), RRException> {
    let mut parser = Parser::new(source);
    let program = parser.parse_program()?;
    let has_main_fn = program.stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::FnDecl { name, .. } => name == "main",
        StmtKind::Export(fndecl) => fndecl.name == "main",
        _ => false,
    });
    let has_top_level_main_call = program.stmts.iter().any(stmt_contains_plain_main_call);
    Ok((has_main_fn, has_top_level_main_call))
}

fn source_with_main_call_appended(source: &str) -> String {
    let mut patched = source.to_string();
    if !patched.ends_with('\n') {
        patched.push('\n');
    }
    patched.push_str("\nmain()\n");
    patched
}

pub fn prepare_project_entry_source(
    input_path: &Path,
    source: &str,
    command: &str,
) -> Result<String, RRException> {
    let (has_main_fn, has_top_level_main_call) = source_defines_main_function(source)?;
    if !has_main_fn {
        return Err(RRException::new(
            "RR.SemanticError",
            RRCode::E1001,
            Stage::Parse,
            format!(
                "project entry '{}' must define fn main()",
                input_path.display()
            ),
        )
        .help(format!(
            "add `fn main() {{ ... }}` to the entry file before running `RR {}`",
            command
        )));
    }
    if has_top_level_main_call {
        return Ok(source.to_string());
    }

    Ok(source_with_main_call_appended(source))
}

pub fn prepare_single_file_build_source(source: &str) -> Result<String, RRException> {
    let (has_main_fn, has_top_level_main_call) = source_defines_main_function(source)?;
    if has_main_fn && !has_top_level_main_call {
        return Ok(source_with_main_call_appended(source));
    }
    Ok(source.to_string())
}
