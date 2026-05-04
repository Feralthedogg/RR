use super::access::MemoryLayout;
use super::schedule::{PolyBackendUsed, SchedulePlan, SchedulePlanKind, ScheduleRelation};
use super::{LoopDimension, ScopRegion};
use crate::mir::opt::poly::affine::AffineSymbol;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MultiStmtCostBreakdown {
    pub data_stmt_count: u32,
    pub reused_read_bases: u32,
    pub write_disjoint_stmt_pairs: u32,
    pub reduction_like_stmt_count: u32,
}

pub fn estimate_relation_cost(
    scop: &ScopRegion,
    relation: &ScheduleRelation,
    kind: SchedulePlanKind,
) -> u32 {
    let mut cost = 0u32;
    let innermost = relation
        .output_expressions
        .last()
        .and_then(|expr| expr.terms.iter().next())
        .and_then(|(symbol, coeff)| match (symbol, coeff) {
            (AffineSymbol::LoopIv(name), 1) => Some(name.as_str()),
            _ => None,
        });
    for stmt in &scop.statements {
        for access in &stmt.accesses {
            cost +=
                match access.memref.layout {
                    MemoryLayout::Dense1D => 1,
                    MemoryLayout::ColumnMajor2D => {
                        if access.subscripts.len() >= 2 {
                            let row_dim = access.subscripts[0].terms.iter().next().and_then(
                                |(symbol, coeff)| match (symbol, coeff) {
                                    (AffineSymbol::LoopIv(name), 1) => Some(name.as_str()),
                                    _ => None,
                                },
                            );
                            if innermost.is_some() && innermost == row_dim {
                                1
                            } else {
                                3
                            }
                        } else {
                            3
                        }
                    }
                    MemoryLayout::ColumnMajor3D => {
                        let dim1 = access.subscripts.first().and_then(|expr| {
                            expr.terms.iter().next().and_then(|(symbol, coeff)| {
                                match (symbol, coeff) {
                                    (AffineSymbol::LoopIv(name), 1) => Some(name.as_str()),
                                    _ => None,
                                }
                            })
                        });
                        if innermost.is_some() && innermost == dim1 {
                            2
                        } else {
                            4
                        }
                    }
                };
        }
    }

    if kind == SchedulePlanKind::None {
        cost += 16;
    } else if kind == SchedulePlanKind::Interchange {
        cost += 1;
    } else if kind == SchedulePlanKind::Skew2D {
        cost = cost.saturating_sub(8);
    }

    cost
}

fn constant_trip_extent(dim: &LoopDimension) -> Option<u32> {
    if !dim.lower_bound.terms.is_empty() || !dim.upper_bound.terms.is_empty() || dim.step == 0 {
        return None;
    }
    let step = dim.step.unsigned_abs();
    if step == 0 {
        return None;
    }
    let lower = dim.lower_bound.constant;
    let upper = dim.upper_bound.constant;
    if upper < lower {
        return Some(0);
    }
    let span = (upper - lower) as u64;
    Some((span / step + 1) as u32)
}

fn constant_iteration_volume(scop: &ScopRegion) -> Option<u32> {
    let mut volume = 1u32;
    for dim in &scop.dimensions {
        let extent = constant_trip_extent(dim)?;
        volume = volume.checked_mul(extent)?;
    }
    Some(volume)
}

fn tile_volume(schedule: &SchedulePlan) -> Option<u32> {
    match schedule.kind {
        SchedulePlanKind::Tile1D => Some(schedule.tile_size? as u32),
        SchedulePlanKind::Tile2D => {
            Some((schedule.tile_rows? as u32).saturating_mul(schedule.tile_cols? as u32))
        }
        SchedulePlanKind::Tile3D => Some(
            (schedule.tile_depth? as u32)
                .saturating_mul(schedule.tile_rows? as u32)
                .saturating_mul(schedule.tile_cols? as u32),
        ),
        _ => None,
    }
}

fn env_flag_is_enabled(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn env_flag_is_disabled(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        )
    })
}

fn env_usize(key: &str, default_value: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

pub fn r_backend_cost_model_enabled() -> bool {
    !env_flag_is_disabled("RR_POLY_R_COST_MODEL")
        && !env_flag_is_disabled("RR_POLY_R_CODE_SIZE_MODEL")
}

pub fn schedule_forced_by_env(schedule: &SchedulePlan) -> bool {
    match schedule.kind {
        SchedulePlanKind::Tile1D => env_flag_is_enabled("RR_POLY_TILE_1D"),
        SchedulePlanKind::Tile2D => env_flag_is_enabled("RR_POLY_TILE_2D"),
        SchedulePlanKind::Tile3D => env_flag_is_enabled("RR_POLY_TILE_3D"),
        SchedulePlanKind::Skew2D => env_flag_is_enabled("RR_POLY_SKEW_2D"),
        _ => false,
    }
}

fn r_backend_tile_code_growth_units(scop: &ScopRegion, schedule: &SchedulePlan) -> u32 {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count() as u32;
    match schedule.kind {
        SchedulePlanKind::Tile1D => 4 + data_stmt_count,
        SchedulePlanKind::Tile2D => 10 + data_stmt_count.saturating_mul(2),
        SchedulePlanKind::Tile3D => 18 + data_stmt_count.saturating_mul(3),
        _ => 0,
    }
}

pub fn r_backend_code_size_reject_reason(
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> Option<&'static str> {
    if !r_backend_cost_model_enabled()
        || schedule.backend != PolyBackendUsed::Isl
        || schedule_forced_by_env(schedule)
        || !matches!(
            schedule.kind,
            SchedulePlanKind::Tile1D | SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D
        )
    {
        return None;
    }

    let min_profitable_volume = env_usize("RR_POLY_R_MIN_TILE_VOLUME", 65_536);
    match constant_iteration_volume(scop) {
        Some(volume) if volume >= min_profitable_volume => {
            let code_growth_limit = env_usize("RR_POLY_R_MAX_CODE_GROWTH", 18);
            if r_backend_tile_code_growth_units(scop, schedule) > code_growth_limit {
                Some("estimated R code growth exceeds tile budget")
            } else {
                None
            }
        }
        Some(_) => Some("constant trip count too small for R tile lowering"),
        None => Some("dynamic trip count cannot justify R tile code growth"),
    }
}

fn r_backend_code_size_penalty(scop: &ScopRegion, schedule: &SchedulePlan) -> u32 {
    if r_backend_code_size_reject_reason(scop, schedule).is_some() {
        10_000 + r_backend_tile_code_growth_units(scop, schedule).saturating_mul(32)
    } else {
        0
    }
}

fn repeated_read_base_count(scop: &ScopRegion) -> u32 {
    let mut counts = BTreeMap::new();
    for stmt in &scop.statements {
        for access in &stmt.accesses {
            if matches!(access.kind, super::access::AccessKind::Read) {
                *counts.entry(access.memref.base).or_insert(0u32) += 1;
            }
        }
    }
    counts
        .into_values()
        .map(|count| count.saturating_sub(1))
        .sum()
}

fn stmt_read_write_sets(stmt: &super::PolyStmt) -> (BTreeMap<usize, u32>, BTreeMap<usize, u32>) {
    let mut reads = BTreeMap::new();
    let mut writes = BTreeMap::new();
    for access in &stmt.accesses {
        let entry = match access.kind {
            super::access::AccessKind::Read => reads.entry(access.memref.base),
            super::access::AccessKind::Write => writes.entry(access.memref.base),
        };
        *entry.or_insert(0) += 1;
    }
    (reads, writes)
}

pub fn analyze_multi_stmt_cost(scop: &ScopRegion) -> MultiStmtCostBreakdown {
    let data_stmts = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .collect::<Vec<_>>();
    if data_stmts.len() <= 1 {
        return MultiStmtCostBreakdown {
            data_stmt_count: data_stmts.len() as u32,
            ..MultiStmtCostBreakdown::default()
        };
    }

    let mut read_base_counts = BTreeMap::new();
    let mut stmt_sets = Vec::with_capacity(data_stmts.len());
    let mut reduction_like_stmt_count = 0u32;
    for stmt in data_stmts {
        let (reads, writes) = stmt_read_write_sets(stmt);
        for base in reads.keys() {
            *read_base_counts.entry(*base).or_insert(0u32) += 1;
        }
        if reads.keys().any(|base| writes.contains_key(base)) {
            reduction_like_stmt_count += 1;
        }
        stmt_sets.push((reads, writes));
    }

    let reused_read_bases = read_base_counts
        .into_values()
        .map(|count| count.saturating_sub(1))
        .sum();
    let mut write_disjoint_stmt_pairs = 0u32;
    for idx in 0..stmt_sets.len() {
        for jdx in idx + 1..stmt_sets.len() {
            let lhs_writes = &stmt_sets[idx].1;
            let rhs_writes = &stmt_sets[jdx].1;
            if !lhs_writes.is_empty()
                && !rhs_writes.is_empty()
                && lhs_writes.keys().all(|base| !rhs_writes.contains_key(base))
            {
                write_disjoint_stmt_pairs += 1;
            }
        }
    }

    MultiStmtCostBreakdown {
        data_stmt_count: stmt_sets.len() as u32,
        reused_read_bases,
        write_disjoint_stmt_pairs,
        reduction_like_stmt_count,
    }
}

pub fn estimate_fission_benefit(scop: &ScopRegion, schedule: &SchedulePlan) -> i32 {
    let breakdown = analyze_multi_stmt_cost(scop);
    let data_stmt_count = breakdown.data_stmt_count as i32;
    if data_stmt_count <= 1 {
        return 0;
    }

    let transform_bonus = match schedule.kind {
        SchedulePlanKind::Tile1D => 3,
        SchedulePlanKind::Tile2D => 4,
        SchedulePlanKind::Tile3D => 5,
        SchedulePlanKind::Skew2D => 4,
        SchedulePlanKind::Interchange => 1,
        SchedulePlanKind::Identity | SchedulePlanKind::None => 0,
    };
    let loop_body_pressure = (data_stmt_count - 1) * scop.dimensions.len() as i32;
    let reuse_bonus = breakdown.reused_read_bases.min(4) as i32;
    let write_disjoint_bonus = breakdown.write_disjoint_stmt_pairs.min(4) as i32;
    let reduction_pressure = breakdown.reduction_like_stmt_count.min(3) as i32;

    loop_body_pressure + transform_bonus + write_disjoint_bonus - reuse_bonus - reduction_pressure
}

pub fn describe_schedule_decision(scop: &ScopRegion, schedule: &SchedulePlan) -> String {
    let breakdown = analyze_multi_stmt_cost(scop);
    let r_backend_gate = r_backend_code_size_reject_reason(scop, schedule).unwrap_or("ok");
    format!(
        "kind={:?} cost={} fission_benefit={} stmts={} reused_reads={} write_disjoint_pairs={} reduction_pressure={} r_backend_gate={}",
        schedule.kind,
        estimate_schedule_cost(scop, schedule),
        estimate_fission_benefit(scop, schedule),
        breakdown.data_stmt_count,
        breakdown.reused_read_bases,
        breakdown.write_disjoint_stmt_pairs,
        breakdown.reduction_like_stmt_count,
        r_backend_gate,
    )
}

pub fn estimate_schedule_cost(scop: &ScopRegion, schedule: &SchedulePlan) -> u32 {
    let mut cost = estimate_relation_cost(scop, &schedule.relation, schedule.kind);
    let tile_bonus =
        ((scop.dimensions.len() as u32) * 2).saturating_add(scop.statements.len() as u32);
    let breakdown = analyze_multi_stmt_cost(scop);

    match schedule.kind {
        SchedulePlanKind::Tile1D | SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D => {
            if let (Some(iter_vol), Some(tile_vol)) =
                (constant_iteration_volume(scop), tile_volume(schedule))
            {
                if tile_vol == 0 || iter_vol <= tile_vol {
                    cost = cost.saturating_add(2);
                } else {
                    let full_tiles = (iter_vol / tile_vol).max(1);
                    let dynamic_bonus = full_tiles.min(6);
                    cost = cost.saturating_sub(tile_bonus.saturating_add(dynamic_bonus));
                }
            } else {
                cost = cost.saturating_sub(tile_bonus);
            }
        }
        SchedulePlanKind::Skew2D => {
            cost = cost.saturating_sub(
                breakdown
                    .write_disjoint_stmt_pairs
                    .saturating_add(breakdown.reused_read_bases)
                    .min(6),
            );
        }
        SchedulePlanKind::Interchange | SchedulePlanKind::Identity | SchedulePlanKind::None => {}
    }

    cost = cost.saturating_add(r_backend_code_size_penalty(scop, schedule));

    if matches!(
        schedule.kind,
        SchedulePlanKind::Identity | SchedulePlanKind::Interchange
    ) && breakdown.reduction_like_stmt_count > 0
    {
        cost = cost.saturating_add(breakdown.reduction_like_stmt_count.min(3));
    }

    cost
}
