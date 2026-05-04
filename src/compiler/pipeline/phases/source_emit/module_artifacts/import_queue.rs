use super::*;
pub(crate) fn hir_module_requires_source_lowering_for_metadata(
    module: &crate::hir::def::HirModule,
) -> bool {
    module.items.iter().any(|item| match item {
        crate::hir::def::HirItem::Trait(_) | crate::hir::def::HirItem::Impl(_) => true,
        crate::hir::def::HirItem::Fn(f) => !f.type_params.is_empty(),
        _ => false,
    })
}

pub(crate) fn public_impl_metadata_needs_external_traits(
    prog: &crate::syntax::ast::Program,
) -> bool {
    let public_traits = prog
        .stmts
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            crate::syntax::ast::StmtKind::TraitDecl(decl) if decl.public => {
                Some(decl.name.as_str())
            }
            _ => None,
        })
        .collect::<FxHashSet<_>>();

    prog.stmts.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            crate::syntax::ast::StmtKind::ImplDecl(decl)
                if decl.public && !public_traits.contains(decl.trait_name.as_str())
        )
    })
}

pub(crate) fn enqueue_ast_module_imports(
    stmts: &[crate::syntax::ast::Stmt],
    curr_path: &Path,
    loaded_paths: &mut FxHashSet<PathBuf>,
    queue: &mut std::collections::VecDeque<ModuleLoadJob>,
    next_mod_id: &mut u32,
) -> crate::error::RR<usize> {
    let mut targets = Vec::new();
    for stmt in stmts {
        let crate::syntax::ast::StmtKind::Import {
            source: crate::syntax::ast::ImportSource::Module,
            path,
            ..
        } = &stmt.kind
        else {
            continue;
        };
        let target = crate::pkg::resolve_import_path(curr_path, path)?;
        if loaded_paths.contains(&target) {
            continue;
        }
        if !target.is_absolute() {
            return Err(crate::error::RRException::new(
                "RR.ParseError",
                crate::error::RRCode::E0001,
                crate::error::Stage::Parse,
                format!(
                    "relative import resolution requires an absolute entry path; normalize '{}' before compiling",
                    curr_path.display()
                ),
            ));
        }
        loaded_paths.insert(target.clone());
        targets.push(target);
    }

    let mut jobs = Vec::with_capacity(targets.len());
    for target in targets {
        jobs.push(ModuleLoadJob {
            path: target,
            content: None,
            ast: None,
            mod_id: *next_mod_id,
            is_entry: false,
            imports_preloaded: false,
        });
        *next_mod_id += 1;
    }
    let enqueued = jobs.len();
    for job in jobs.into_iter().rev() {
        queue.push_front(job);
    }
    Ok(enqueued)
}

pub(crate) fn enqueue_module_imports(
    module: &crate::hir::def::HirModule,
    curr_path: &Path,
    loaded_paths: &mut FxHashSet<PathBuf>,
    queue: &mut std::collections::VecDeque<ModuleLoadJob>,
    next_mod_id: &mut u32,
) -> crate::error::RR<()> {
    for item in &module.items {
        if let crate::hir::def::HirItem::Import(imp) = item {
            let target = crate::pkg::resolve_import_path(curr_path, &imp.module)?;
            if !loaded_paths.contains(&target) {
                if !target.is_absolute() {
                    return Err(crate::error::RRException::new(
                        "RR.ParseError",
                        crate::error::RRCode::E0001,
                        crate::error::Stage::Parse,
                        format!(
                            "relative import resolution requires an absolute entry path; normalize '{}' before compiling",
                            curr_path.display()
                        ),
                    ));
                }
                loaded_paths.insert(target.clone());
                queue.push_back(ModuleLoadJob {
                    path: target,
                    content: None,
                    ast: None,
                    mod_id: *next_mod_id,
                    is_entry: false,
                    imports_preloaded: false,
                });
                *next_mod_id += 1;
            }
        }
    }
    Ok(())
}
