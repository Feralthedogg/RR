use super::*;
pub(crate) fn can_auto_skew_2d(
    scop: &ScopRegion,
    dep_state: DependenceState,
    policy: TilePolicy,
) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    if scop.dimensions.len() != 2
        || data_stmt_count <= 1
        || scop.dimensions.iter().any(|dim| dim.step != 1)
        || !matches!(dep_state, DependenceState::IdentityProven)
        || (!policy.allow_skew_with_tiles
            && (policy.enable_1d || policy.enable_2d || policy.enable_3d))
    {
        return false;
    }
    let loop_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<Vec<_>>();
    scop.statements
        .iter()
        .flat_map(|stmt| &stmt.accesses)
        .all(|access| {
            access.memref.layout == MemoryLayout::ColumnMajor2D
                && access_matches_loop_order(access, &loop_names)
        })
}

pub(crate) fn can_skew_2d(
    scop: &ScopRegion,
    dep_state: DependenceState,
    policy: TilePolicy,
) -> bool {
    match policy.skew_2d_mode {
        AutoChoice::ForceOff => false,
        AutoChoice::ForceOn => {
            scop.dimensions.len() == 2
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop.statements.iter().any(|stmt| !stmt.accesses.is_empty())
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| &stmt.accesses)
                    .all(|access| access.memref.layout == MemoryLayout::ColumnMajor2D)
        }
        AutoChoice::Auto => can_auto_skew_2d(scop, dep_state, policy),
    }
}

pub(crate) fn should_interchange(scop: &ScopRegion) -> bool {
    if !(scop.dimensions.len() == 2 || scop.dimensions.len() == 3) {
        return false;
    }
    let loop_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<Vec<_>>();
    let outer = &scop.dimensions[0].iv_name;
    let inner = &scop.dimensions[1].iv_name;
    if scop.dimensions.len() == 2 {
        return scop
            .statements
            .iter()
            .flat_map(|stmt| &stmt.accesses)
            .any(|access| {
                access.memref.layout == MemoryLayout::ColumnMajor2D
                    && access_matches_loop_order(access, &loop_names)
                    && access.subscripts.len() == 2
                    && matches!(
                        access.subscripts[0].terms.iter().next(),
                        Some((AffineSymbol::LoopIv(name), coeff)) if name == outer && *coeff == 1
                    )
                    && matches!(
                        access.subscripts[1].terms.iter().next(),
                        Some((AffineSymbol::LoopIv(name), coeff)) if name == inner && *coeff == 1
                    )
            });
    }

    let middle = &scop.dimensions[1].iv_name;
    let inner = &scop.dimensions[2].iv_name;
    scop.statements
        .iter()
        .flat_map(|stmt| &stmt.accesses)
        .any(|access| {
            access.memref.layout == MemoryLayout::ColumnMajor3D
                && access_matches_loop_order(access, &loop_names)
                && access.subscripts.len() == 3
                && matches!(
                    access.subscripts[0].terms.iter().next(),
                    Some((AffineSymbol::LoopIv(name), coeff)) if name == outer && *coeff == 1
                )
                && matches!(
                    access.subscripts[1].terms.iter().next(),
                    Some((AffineSymbol::LoopIv(name), coeff)) if name == middle && *coeff == 1
                )
                && matches!(
                    access.subscripts[2].terms.iter().next(),
                    Some((AffineSymbol::LoopIv(name), coeff)) if name == inner && *coeff == 1
                )
        })
}

pub(crate) fn access_matches_loop_order(
    access: &crate::mir::opt::poly::access::AccessRelation,
    loop_names: &[&str],
) -> bool {
    fn expr_is_loop_aligned(expr: &AffineExpr, expected: &str) -> bool {
        let mut expected_coeff = None;
        for (symbol, coeff) in &expr.terms {
            if let AffineSymbol::LoopIv(name) = symbol {
                if name == expected {
                    if expected_coeff.is_some() {
                        return false;
                    }
                    expected_coeff = Some(*coeff);
                } else {
                    return false;
                }
            }
        }
        expected_coeff == Some(1)
    }

    access.subscripts.len() == loop_names.len()
        && access
            .subscripts
            .iter()
            .zip(loop_names.iter())
            .all(|(expr, expected)| expr_is_loop_aligned(expr, expected))
}

pub(crate) fn can_tile_1d(scop: &ScopRegion, policy: TilePolicy) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    let iv_name = &scop.dimensions[0].iv_name;
    policy.enable_1d
        && scop.dimensions.len() == 1
        && scop.dimensions[0].step == 1
        && data_stmt_count >= 1
        && scop
            .statements
            .iter()
            .flat_map(|stmt| &stmt.accesses)
            .all(|access| match access.memref.layout {
                MemoryLayout::Dense1D => access.subscripts.len() == 1,
                MemoryLayout::ColumnMajor2D => {
                    access.subscripts.len() == 2
                        && matches!(
                            access.subscripts[0].terms.iter().next(),
                            Some((AffineSymbol::LoopIv(name), coeff)) if name == iv_name && *coeff == 1
                        )
                }
                MemoryLayout::ColumnMajor3D => {
                    access.subscripts.len() == 3
                        && matches!(
                            access.subscripts[0].terms.iter().next(),
                            Some((AffineSymbol::LoopIv(name), coeff)) if name == iv_name && *coeff == 1
                        )
                }
            })
}

pub(crate) fn can_tile_2d(scop: &ScopRegion, policy: TilePolicy) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    policy.enable_2d
        && scop.dimensions.len() == 2
        && scop.dimensions[0].step == 1
        && scop.dimensions[1].step == 1
        && data_stmt_count >= 1
        && {
            let loop_names = scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.as_str())
                .collect::<Vec<_>>();
            scop.statements
                .iter()
                .flat_map(|stmt| &stmt.accesses)
                .all(|access| {
                    access.memref.layout == MemoryLayout::ColumnMajor2D
                        && access_matches_loop_order(access, &loop_names)
                })
        }
}

pub(crate) fn can_tile_3d(scop: &ScopRegion, policy: TilePolicy) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    policy.enable_3d
        && scop.dimensions.len() == 3
        && scop.dimensions.iter().all(|dim| dim.step == 1)
        && data_stmt_count >= 1
        && {
            let loop_names = scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.as_str())
                .collect::<Vec<_>>();
            scop.statements
                .iter()
                .flat_map(|stmt| &stmt.accesses)
                .all(|access| {
                    access.memref.layout == MemoryLayout::ColumnMajor3D
                        && access_matches_loop_order(access, &loop_names)
                })
        }
}
