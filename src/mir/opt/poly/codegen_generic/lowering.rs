use super::*;
pub(crate) fn lower_generic_map_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            GenericFissionRequest {
                lp,
                scop,
                schedule,
                preheader: site.preheader,
                exit_bb: site.exit_bb,
                skip_accessless_assigns: false,
                lower_one: rebuild_generic_loop_nest,
            },
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-map-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: scop not map-compatible",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        false,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

pub(crate) fn lower_generic_fission_sequence_with_builder(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
    lower_one: fn(&mut FnIR, &LoopInfo, &ScopRegion, &SchedulePlan, usize, usize, bool) -> bool,
) -> bool {
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_fissioned_sequence(
        &mut trial,
        GenericFissionRequest {
            lp,
            scop,
            schedule,
            preheader: site.preheader,
            exit_bb: site.exit_bb,
            skip_accessless_assigns,
            lower_one,
        },
    ) {
        return false;
    }
    *fn_ir = trial;
    true
}

pub(crate) fn lower_generic_tiled_1d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            GenericFissionRequest {
                lp,
                scop,
                schedule,
                preheader: site.preheader,
                exit_bb: site.exit_bb,
                skip_accessless_assigns,
                lower_one: rebuild_generic_tiled_1d_loop_nest,
            },
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-tile1d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_tiled_1d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: tiled rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-tile1d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

pub(crate) fn lower_generic_skew2d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            GenericFissionRequest {
                lp,
                scop,
                schedule,
                preheader: site.preheader,
                exit_bb: site.exit_bb,
                skip_accessless_assigns,
                lower_one: rebuild_generic_skewed_2d_loop_nest,
            },
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-skew2d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_skewed_2d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-skew2d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

pub(crate) fn lower_generic_tiled_2d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            GenericFissionRequest {
                lp,
                scop,
                schedule,
                preheader: site.preheader,
                exit_bb: site.exit_bb,
                skip_accessless_assigns,
                lower_one: rebuild_generic_tiled_2d_loop_nest,
            },
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-tile2d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_tiled_2d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: tiled2d rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-tile2d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

pub(crate) fn lower_generic_tiled_3d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            GenericFissionRequest {
                lp,
                scop,
                schedule,
                preheader: site.preheader,
                exit_bb: site.exit_bb,
                skip_accessless_assigns,
                lower_one: rebuild_generic_tiled_3d_loop_nest,
            },
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-tile3d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_tiled_3d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: tiled3d rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-tile3d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

pub(crate) fn lower_generic_reduce_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            GenericFissionRequest {
                lp,
                scop,
                schedule,
                preheader: site.preheader,
                exit_bb: site.exit_bb,
                skip_accessless_assigns: true,
                lower_one: rebuild_generic_loop_nest,
            },
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-reduce-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        true,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-reduction-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}
