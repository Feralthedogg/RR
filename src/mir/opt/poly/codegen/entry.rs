#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolyCodegenPlan {
    pub emitted: bool,
}

pub fn lower_schedule_tree(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tree: &ScheduleTree,
) -> PolyCodegenPlan {
    fn is_fission_sequence(node: &ScheduleTreeNode) -> bool {
        match node {
            ScheduleTreeNode::Sequence(nodes) if nodes.len() > 1 => nodes.iter().all(|node| {
                matches!(
                    node,
                    ScheduleTreeNode::Filter(filter) if filter.statement_ids.len() == 1
                )
            }),
            _ => false,
        }
    }

    fn scop_subset(scop: &ScopRegion, stmt_ids: &[usize]) -> ScopRegion {
        ScopRegion {
            header: scop.header,
            latch: scop.latch,
            exits: scop.exits.clone(),
            dimensions: scop.dimensions.clone(),
            iteration_space: scop.iteration_space.clone(),
            parameters: scop.parameters.clone(),
            statements: scop
                .statements
                .iter()
                .filter(|stmt| stmt_ids.contains(&stmt.id))
                .cloned()
                .collect(),
        }
    }

    fn transform_plan(
        transform: &ScheduleTransform,
        backend: super::schedule::PolyBackendUsed,
    ) -> SchedulePlan {
        SchedulePlan {
            kind: transform.plan_kind,
            relation: transform.relation.clone(),
            backend,
            tile_size: transform.tile_size,
            tile_depth: transform.tile_depth,
            tile_rows: transform.tile_rows,
            tile_cols: transform.tile_cols,
        }
    }

    if is_fission_sequence(&tree.root)
        && lower_fission_sequence_generic(fn_ir, lp, scop, &tree.to_primary_plan())
    {
        return PolyCodegenPlan { emitted: true };
    }

    // When a fission sequence splits statements into per-filter groups, the
    // per-filter `rec` path processes them one at a time.  The first successful
    // codegen mutates the preheader (Goto → If), which causes
    // `vector_apply_site` to return None for subsequent filters.
    //
    // Work around this by trying the specialized *multi-statement* codegen on
    // the full (unfissioned) SCoP before falling into per-filter processing.
    // The `build_multi_*` helpers already collect all matching statements and
    // emit them in a single `finish_vector_assignments_versioned` call.
    //
    // When the primary schedule (e.g. Skew2D) lacks specialized multi-statement
    // codegen, we also attempt Interchange which has `build_multi_*` helpers.
    if is_fission_sequence(&tree.root) {
        let plan = tree.to_primary_plan();
        let result = match plan.kind {
            SchedulePlanKind::Identity => lower_identity_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Interchange => lower_interchange_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Skew2D => lower_skew2d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Tile1D => lower_tile1d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Tile2D => lower_tile2d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Tile3D => lower_tile3d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::None => PolyCodegenPlan { emitted: false },
        };
        if result.emitted {
            return result;
        }
        // Skew2D/Identity lack specialized multi-statement codegen; fall back to
        // Interchange which has `build_multi_nested_2d_*` helpers.
        if matches!(
            plan.kind,
            SchedulePlanKind::Skew2D | SchedulePlanKind::Identity
        ) && scop.dimensions.len() >= 2
        {
            let xchg_plan = SchedulePlan {
                kind: SchedulePlanKind::Interchange,
                relation: super::schedule::interchange_relation(scop),
                backend: plan.backend,
                tile_size: None,
                tile_depth: None,
                tile_rows: None,
                tile_cols: None,
            };
            let result = lower_interchange_schedule(fn_ir, lp, scop, &xchg_plan);
            if result.emitted {
                return result;
            }
        }
    }

    fn rec(
        fn_ir: &mut FnIR,
        lp: &LoopInfo,
        scop: &ScopRegion,
        tree: &ScheduleTree,
        node: &ScheduleTreeNode,
    ) -> PolyCodegenPlan {
        match node {
            ScheduleTreeNode::Sequence(nodes) => {
                let mut emitted_any = false;
                for child in nodes {
                    let next = rec(fn_ir, lp, scop, tree, child);
                    if next.emitted {
                        emitted_any = true;
                    }
                }
                PolyCodegenPlan {
                    emitted: emitted_any,
                }
            }
            ScheduleTreeNode::Filter(filter) => {
                let subset = scop_subset(scop, &filter.statement_ids);
                for child in &filter.children {
                    let next = rec(fn_ir, lp, &subset, tree, child);
                    if next.emitted {
                        return next;
                    }
                }
                PolyCodegenPlan { emitted: false }
            }
            ScheduleTreeNode::Leaf => PolyCodegenPlan { emitted: false },
            ScheduleTreeNode::Band(band) => {
                for child in &band.children {
                    let next = rec(fn_ir, lp, scop, tree, child);
                    if next.emitted {
                        return next;
                    }
                }
                PolyCodegenPlan { emitted: false }
            }
            ScheduleTreeNode::Transform(transform) => {
                let emitted = if transform.plan_kind != SchedulePlanKind::None {
                    let plan = transform_plan(transform, tree.backend);
                    match transform.plan_kind {
                        SchedulePlanKind::Identity => {
                            lower_identity_schedule(fn_ir, lp, scop, &plan)
                        }
                        SchedulePlanKind::Interchange => {
                            lower_interchange_schedule(fn_ir, lp, scop, &plan)
                        }
                        SchedulePlanKind::Skew2D => lower_skew2d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::Tile1D => lower_tile1d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::Tile2D => lower_tile2d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::Tile3D => lower_tile3d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::None => PolyCodegenPlan { emitted: false },
                    }
                } else {
                    PolyCodegenPlan { emitted: false }
                };
                if emitted.emitted {
                    return emitted;
                }
                for child in &transform.children {
                    let next = rec(fn_ir, lp, scop, tree, child);
                    if next.emitted {
                        return next;
                    }
                }
                PolyCodegenPlan { emitted: false }
            }
        }
    }

    rec(fn_ir, lp, scop, tree, &tree.root)
}
