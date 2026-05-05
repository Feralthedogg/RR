use super::*;
pub(crate) fn identity_relation(scop: &ScopRegion) -> ScheduleRelation {
    ScheduleRelation {
        input_dimensions: scop
            .dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect(),
        output_expressions: scop
            .dimensions
            .iter()
            .map(|dim| AffineExpr::symbol(AffineSymbol::LoopIv(dim.iv_name.clone())))
            .collect(),
    }
}

pub(crate) fn interchange_relation(scop: &ScopRegion) -> ScheduleRelation {
    let mut dims = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    if dims.len() >= 2 {
        dims.rotate_left(1);
    }
    interchange_relation_from_order(scop, dims)
}

pub(crate) fn interchange_relation_from_order(
    scop: &ScopRegion,
    dims: Vec<String>,
) -> ScheduleRelation {
    ScheduleRelation {
        input_dimensions: scop
            .dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect(),
        output_expressions: dims
            .into_iter()
            .map(|name| AffineExpr::symbol(AffineSymbol::LoopIv(name)))
            .collect(),
    }
}

pub(crate) fn interchange_relations(scop: &ScopRegion) -> Vec<ScheduleRelation> {
    let dims = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    match dims.as_slice() {
        [a, b] => vec![interchange_relation_from_order(
            scop,
            vec![b.clone(), a.clone()],
        )],
        [a, b, c] => vec![
            interchange_relation_from_order(scop, vec![b.clone(), c.clone(), a.clone()]),
            interchange_relation_from_order(scop, vec![c.clone(), a.clone(), b.clone()]),
            interchange_relation_from_order(scop, vec![a.clone(), c.clone(), b.clone()]),
            interchange_relation_from_order(scop, vec![b.clone(), a.clone(), c.clone()]),
            interchange_relation_from_order(scop, vec![c.clone(), b.clone(), a.clone()]),
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn none_relation() -> ScheduleRelation {
    ScheduleRelation {
        input_dimensions: Vec::new(),
        output_expressions: Vec::new(),
    }
}

pub(crate) fn skew2d_relation(scop: &ScopRegion) -> ScheduleRelation {
    let input_dimensions = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    if input_dimensions.len() != 2 {
        return none_relation();
    }
    let outer = AffineExpr::symbol(AffineSymbol::LoopIv(input_dimensions[0].clone()));
    let mut skewed = AffineExpr::symbol(AffineSymbol::LoopIv(input_dimensions[1].clone()));
    skewed.add_assign(
        &AffineExpr::symbol(AffineSymbol::LoopIv(input_dimensions[0].clone())),
        1,
    );
    ScheduleRelation {
        input_dimensions,
        output_expressions: vec![outer, skewed],
    }
}
