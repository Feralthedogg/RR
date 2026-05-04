use super::super::ScopRegion;
use super::super::affine::{AffineExpr, AffineSymbol};
use super::super::schedule::{SchedulePlan, SchedulePlanKind};
use super::*;
#[cfg(rr_has_isl)]
pub(crate) mod imp {
    use super::super::super::affine::AffineConstraintKind;
    use super::*;
    use std::collections::BTreeSet;
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int, c_void};

    #[repr(C)]
    struct isl_ctx {
        pub(crate) _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_set {
        pub(crate) _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_map {
        pub(crate) _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_union_set {
        pub(crate) _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_union_map {
        pub(crate) _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_schedule_constraints {
        pub(crate) _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_schedule {
        pub(crate) _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_schedule_node {
        pub(crate) _private: [u8; 0],
    }

    const ISL_SCHEDULE_NODE_BAND: i32 = 0;
    const ISL_SCHEDULE_NODE_DOMAIN: i32 = 2;
    const ISL_SCHEDULE_NODE_FILTER: i32 = 6;
    const ISL_SCHEDULE_NODE_LEAF: i32 = 7;
    const ISL_SCHEDULE_NODE_SEQUENCE: i32 = 10;
    const ISL_SCHEDULE_NODE_SET: i32 = 11;
    const ISL_ON_ERROR_CONTINUE: c_int = 1;

    // SAFETY: These declarations model ISL/libc FFI entry points. The C ABI and
    // raw pointer signatures cannot be expressed safely in Rust, so callers keep
    // ownership and lifetime checks on the Rust side before crossing this boundary.
    unsafe extern "C" {
        fn isl_ctx_alloc() -> *mut isl_ctx;
        fn isl_ctx_free(ctx: *mut isl_ctx);
        fn isl_options_set_on_error(ctx: *mut isl_ctx, val: c_int) -> c_int;

        fn isl_set_read_from_str(ctx: *mut isl_ctx, s: *const c_char) -> *mut isl_set;
        fn isl_union_set_read_from_str(ctx: *mut isl_ctx, s: *const c_char) -> *mut isl_union_set;
        fn isl_union_set_from_set(set: *mut isl_set) -> *mut isl_union_set;

        fn isl_map_read_from_str(ctx: *mut isl_ctx, s: *const c_char) -> *mut isl_map;
        fn isl_union_map_from_map(map: *mut isl_map) -> *mut isl_union_map;
        fn isl_union_map_read_from_str(ctx: *mut isl_ctx, s: *const c_char) -> *mut isl_union_map;
        fn isl_union_map_to_str(map: *const isl_union_map) -> *mut c_char;
        fn isl_union_map_add_map(map: *mut isl_union_map, map2: *mut isl_map)
        -> *mut isl_union_map;
        fn isl_union_map_union(
            map1: *mut isl_union_map,
            map2: *mut isl_union_map,
        ) -> *mut isl_union_map;
        fn isl_union_map_coalesce(map: *mut isl_union_map) -> *mut isl_union_map;
        fn isl_union_map_is_empty(map: *const isl_union_map) -> c_int;
        fn isl_union_map_free(map: *mut isl_union_map) -> *mut isl_union_map;
        fn isl_map_is_empty(map: *mut isl_map) -> c_int;

        fn isl_schedule_constraints_on_domain(
            domain: *mut isl_union_set,
        ) -> *mut isl_schedule_constraints;
        fn isl_schedule_constraints_set_validity(
            sc: *mut isl_schedule_constraints,
            validity: *mut isl_union_map,
        ) -> *mut isl_schedule_constraints;
        fn isl_schedule_constraints_set_proximity(
            sc: *mut isl_schedule_constraints,
            proximity: *mut isl_union_map,
        ) -> *mut isl_schedule_constraints;
        fn isl_schedule_constraints_set_conditional_validity(
            sc: *mut isl_schedule_constraints,
            validity: *mut isl_union_map,
        ) -> *mut isl_schedule_constraints;
        fn isl_schedule_constraints_compute_schedule(
            sc: *mut isl_schedule_constraints,
        ) -> *mut isl_schedule;
        fn isl_schedule_constraints_free(
            sc: *mut isl_schedule_constraints,
        ) -> *mut isl_schedule_constraints;
        fn isl_schedule_from_domain(domain: *mut isl_union_set) -> *mut isl_schedule;

        fn isl_schedule_to_str(schedule: *const isl_schedule) -> *mut c_char;
        fn isl_schedule_get_root(schedule: *const isl_schedule) -> *mut isl_schedule_node;
        fn isl_schedule_free(schedule: *mut isl_schedule) -> *mut isl_schedule;

        fn isl_schedule_node_get_type(node: *const isl_schedule_node) -> c_int;
        fn isl_schedule_node_n_children(node: *const isl_schedule_node) -> isize;
        fn isl_schedule_node_get_child(
            node: *const isl_schedule_node,
            pos: c_int,
        ) -> *mut isl_schedule_node;
        fn isl_schedule_node_band_n_member(node: *const isl_schedule_node) -> isize;
        fn isl_schedule_node_band_get_partial_schedule_union_map(
            node: *const isl_schedule_node,
        ) -> *mut isl_union_map;
        fn isl_schedule_node_free(node: *mut isl_schedule_node) -> *mut isl_schedule_node;

        fn free(ptr: *mut c_void);
    }

    fn configure_isl_context(ctx: *mut isl_ctx) {
        if ctx.is_null() {
            return;
        }
        // SAFETY: `ctx` is a freshly allocated ISL context owned by the caller.
        // The libisl FFI option call/raw pointer cannot be expressed safely.
        // RR uses null/error states as optimizer misses instead of stderr noise.
        unsafe {
            let _ = isl_options_set_on_error(ctx, ISL_ON_ERROR_CONTINUE);
        }
    }

    fn node_type_name(kind: i32) -> &'static str {
        match kind {
            ISL_SCHEDULE_NODE_BAND => "band",
            ISL_SCHEDULE_NODE_DOMAIN => "domain",
            ISL_SCHEDULE_NODE_FILTER => "filter",
            ISL_SCHEDULE_NODE_LEAF => "leaf",
            ISL_SCHEDULE_NODE_SEQUENCE => "sequence",
            ISL_SCHEDULE_NODE_SET => "set",
            _ => "other",
        }
    }

    fn take_isl_string(ptr: *mut c_char) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        // SAFETY: `ptr` comes from ISL/libc string-returning FFI in this module.
        // Converting and freeing that raw pointer cannot be expressed safely in
        // Rust, so this helper owns both steps in one place.
        let out = unsafe {
            let out = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            free(ptr.cast::<c_void>());
            out
        };
        Some(out)
    }

    fn sanitize(raw: &str) -> String {
        raw.chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn symbol_name(symbol: &AffineSymbol) -> String {
        match symbol {
            AffineSymbol::LoopIv(name) => sanitize(name),
            AffineSymbol::Param(name) => format!("p_{}", sanitize(name)),
            AffineSymbol::Invariant(name) => format!("inv_{}", sanitize(name)),
            AffineSymbol::Length(name) => format!("len_{}", sanitize(name)),
        }
    }

    fn expr_to_isl(expr: &AffineExpr) -> String {
        let mut parts = Vec::new();
        for (symbol, coeff) in &expr.terms {
            let name = symbol_name(symbol);
            let term = match *coeff {
                1 => name,
                -1 => format!("-{name}"),
                coeff => format!("{coeff}*{name}"),
            };
            parts.push(term);
        }
        if expr.constant != 0 || parts.is_empty() {
            parts.push(expr.constant.to_string());
        }
        parts.join(" + ").replace("+ -", "- ")
    }

    fn collect_params(scop: &ScopRegion, plan: &SchedulePlan) -> Vec<String> {
        let mut params = BTreeSet::new();
        for constraint in &scop.iteration_space.constraints {
            for symbol in constraint.expr.terms.keys() {
                if !matches!(symbol, AffineSymbol::LoopIv(_)) {
                    params.insert(symbol_name(symbol));
                }
            }
        }
        for expr in &plan.relation.output_expressions {
            for symbol in expr.terms.keys() {
                if !matches!(symbol, AffineSymbol::LoopIv(_)) {
                    params.insert(symbol_name(symbol));
                }
            }
        }
        params.into_iter().collect()
    }

    fn scop_domain_to_str(scop: &ScopRegion, plan: &SchedulePlan) -> String {
        let params = collect_params(scop, plan);
        let param_prefix = if params.is_empty() {
            String::new()
        } else {
            format!("[{}] -> ", params.join(", "))
        };
        let dims = scop
            .dimensions
            .iter()
            .map(|dim| symbol_name(&AffineSymbol::LoopIv(dim.iv_name.clone())))
            .collect::<Vec<_>>();
        let mut constraints = Vec::new();
        for (idx, dim_name) in dims.iter().enumerate() {
            let Some(lower) = scop.iteration_space.constraints.get(idx * 2) else {
                continue;
            };
            let Some(upper) = scop.iteration_space.constraints.get(idx * 2 + 1) else {
                continue;
            };
            if matches!(lower.kind, AffineConstraintKind::LowerBound) {
                constraints.push(format!("{} <= {}", expr_to_isl(&lower.expr), dim_name));
            }
            if matches!(upper.kind, AffineConstraintKind::UpperBound) {
                constraints.push(format!("{} <= {}", dim_name, expr_to_isl(&upper.expr)));
            }
        }
        let body = if constraints.is_empty() {
            solver_statement_ids(scop)
                .into_iter()
                .map(|stmt_id| format!("S{stmt_id}[{}]", dims.join(", ")))
                .collect::<Vec<_>>()
                .join("; ")
        } else {
            solver_statement_ids(scop)
                .into_iter()
                .map(|stmt_id| {
                    format!(
                        "S{stmt_id}[{}] : {}",
                        dims.join(", "),
                        constraints.join(" and ")
                    )
                })
                .collect::<Vec<_>>()
                .join("; ")
        };
        format!("{param_prefix}{{ {body} }}")
    }

    fn schedule_map_to_str(scop: &ScopRegion, plan: &SchedulePlan) -> Option<String> {
        if plan.kind == SchedulePlanKind::None || plan.relation.input_dimensions.is_empty() {
            return None;
        }
        let params = collect_params(scop, plan);
        let param_prefix = if params.is_empty() {
            String::new()
        } else {
            format!("[{}] -> ", params.join(", "))
        };
        let inputs = plan
            .relation
            .input_dimensions
            .iter()
            .map(|name| symbol_name(&AffineSymbol::LoopIv(name.clone())))
            .collect::<Vec<_>>();
        let outputs = plan
            .relation
            .output_expressions
            .iter()
            .map(expr_to_isl)
            .collect::<Vec<_>>();
        Some(format!(
            "{param_prefix}{{ {} }}",
            solver_statement_ids(scop)
                .into_iter()
                .map(|stmt_id| format!(
                    "S{stmt_id}[{}] -> [{}]",
                    inputs.join(", "),
                    outputs.join(", ")
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ))
    }

    fn first_band_info(root: *mut isl_schedule_node) -> (usize, Option<String>) {
        if root.is_null() {
            return (0, None);
        }
        // SAFETY: `root` comes from ISL schedule traversal and stays valid for this walk.
        // Descending through child raw pointers and freeing each child cannot be
        // expressed safely in Rust, so this helper localizes the FFI recursion.
        unsafe {
            let ty = isl_schedule_node_get_type(root);
            if ty == ISL_SCHEDULE_NODE_BAND {
                let members = isl_schedule_node_band_n_member(root).max(0) as usize;
                let partial = isl_schedule_node_band_get_partial_schedule_union_map(root);
                let partial_str = take_isl_string(isl_union_map_to_str(partial));
                let _ = isl_union_map_free(partial);
                return (members, partial_str);
            }
            let n_children = isl_schedule_node_n_children(root).max(0) as usize;
            for pos in 0..n_children {
                let child = isl_schedule_node_get_child(root, pos as c_int);
                let info = first_band_info(child);
                let _ = isl_schedule_node_free(child);
                if info.0 > 0 || info.1.is_some() {
                    return info;
                }
            }
            (0, None)
        }
    }

    fn build_schedule_constraints(
        ctx: *mut isl_ctx,
        domain_c: &CString,
        validity_c: Option<&CString>,
        proximity_c: Option<&CString>,
        conditional_validity_c: Option<&CString>,
    ) -> (
        *mut isl_schedule_constraints,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        // SAFETY: `ctx` and the `CString` inputs stay owned for this full call.
        // Constructing ISL constraint objects and transferring their raw pointer
        // ownership through FFI cannot be expressed safely in Rust.
        unsafe {
            let domain_union = isl_union_set_read_from_str(ctx, domain_c.as_ptr());
            if domain_union.is_null() {
                return (std::ptr::null_mut(), None, None, None);
            }

            let mut constraints = isl_schedule_constraints_on_domain(domain_union);
            let validity_roundtrip = if let Some(validity_c) = validity_c {
                let validity_map = isl_union_map_read_from_str(ctx, validity_c.as_ptr());
                if !validity_map.is_null() {
                    let rendered = take_isl_string(isl_union_map_to_str(validity_map));
                    constraints = isl_schedule_constraints_set_validity(constraints, validity_map);
                    rendered
                } else {
                    None
                }
            } else {
                None
            };
            let proximity_roundtrip = if let Some(proximity_c) = proximity_c {
                let proximity_map = isl_union_map_read_from_str(ctx, proximity_c.as_ptr());
                if !proximity_map.is_null() {
                    let rendered = take_isl_string(isl_union_map_to_str(proximity_map));
                    constraints =
                        isl_schedule_constraints_set_proximity(constraints, proximity_map);
                    rendered
                } else {
                    None
                }
            } else {
                None
            };
            let conditional_validity_roundtrip = if let Some(conditional_validity_c) =
                conditional_validity_c
            {
                let conditional_validity_map =
                    isl_union_map_read_from_str(ctx, conditional_validity_c.as_ptr());
                if !conditional_validity_map.is_null() {
                    let rendered = take_isl_string(isl_union_map_to_str(conditional_validity_map));
                    constraints = isl_schedule_constraints_set_conditional_validity(
                        constraints,
                        conditional_validity_map,
                    );
                    rendered
                } else {
                    None
                }
            } else {
                None
            };

            (
                constraints,
                validity_roundtrip,
                proximity_roundtrip,
                conditional_validity_roundtrip,
            )
        }
    }

    fn schedule_shape_flags(root: *mut isl_schedule_node) -> (bool, bool) {
        if root.is_null() {
            return (false, false);
        }
        // SAFETY: `root` is an ISL-owned schedule node for this traversal.
        // Inspecting node types and walking/freeing child raw pointers cannot be
        // expressed safely in Rust, so this helper contains the FFI recursion.
        unsafe {
            let ty = isl_schedule_node_get_type(root);
            let mut has_sequence = ty == ISL_SCHEDULE_NODE_SEQUENCE;
            let mut has_filter = ty == ISL_SCHEDULE_NODE_FILTER;
            let n_children = isl_schedule_node_n_children(root).max(0) as usize;
            for pos in 0..n_children {
                let child = isl_schedule_node_get_child(root, pos as c_int);
                let (child_sequence, child_filter) = schedule_shape_flags(child);
                let _ = isl_schedule_node_free(child);
                has_sequence |= child_sequence;
                has_filter |= child_filter;
            }
            (has_sequence, has_filter)
        }
    }

    pub fn map_roundtrip_if_non_empty(raw: &str) -> Option<String> {
        let raw_c = CString::new(raw).ok()?;
        // SAFETY: This block owns the temporary ISL ctx/map handles created from
        // local `CString` input. Pairing the FFI raw pointer allocations and frees
        // cannot be expressed safely in Rust, so the lifecycle stays together here.
        unsafe {
            let ctx = isl_ctx_alloc();
            if ctx.is_null() {
                return None;
            }
            configure_isl_context(ctx);
            let map = isl_map_read_from_str(ctx, raw_c.as_ptr());
            if map.is_null() {
                isl_ctx_free(ctx);
                return None;
            }
            let empty = isl_map_is_empty(map) != 0;
            if empty {
                let _ = isl_union_map_free(isl_union_map_from_map(map));
                isl_ctx_free(ctx);
                return None;
            }
            let umap = isl_union_map_from_map(map);
            let rendered = take_isl_string(isl_union_map_to_str(umap));
            let _ = isl_union_map_free(umap);
            isl_ctx_free(ctx);
            rendered
        }
    }

    pub fn union_maps_roundtrip(maps: &[String]) -> Option<String> {
        if maps.is_empty() {
            return None;
        }
        // SAFETY: This block owns the temporary ISL ctx/union-map handles created
        // from local `CString` input. Aggregating and freeing those FFI raw
        // pointers cannot be expressed safely in Rust, so cleanup stays paired here.
        unsafe {
            let ctx = isl_ctx_alloc();
            if ctx.is_null() {
                return None;
            }
            configure_isl_context(ctx);
            let mut union: *mut isl_union_map = std::ptr::null_mut();
            for raw in maps {
                let raw_c = match CString::new(raw.as_str()) {
                    Ok(raw_c) => raw_c,
                    Err(_) => continue,
                };
                let map = isl_map_read_from_str(ctx, raw_c.as_ptr());
                if map.is_null() {
                    continue;
                }
                if isl_map_is_empty(map) != 0 {
                    let _ = isl_union_map_free(isl_union_map_from_map(map));
                    continue;
                }
                union = if union.is_null() {
                    isl_union_map_from_map(map)
                } else {
                    isl_union_map_add_map(union, map)
                };
            }
            if union.is_null() {
                isl_ctx_free(ctx);
                return None;
            }
            union = isl_union_map_coalesce(union);
            if union.is_null() || isl_union_map_is_empty(union) != 0 {
                let _ = isl_union_map_free(union);
                isl_ctx_free(ctx);
                return None;
            }
            let rendered = take_isl_string(isl_union_map_to_str(union));
            let _ = isl_union_map_free(union);
            isl_ctx_free(ctx);
            rendered
        }
    }

    fn materialize_schedule_artifacts_from_strings(
        domain: String,
        candidate_schedule_map: Option<String>,
        validity: Option<String>,
        proximity: Option<String>,
        coincidence: Option<String>,
        conditional_validity: Option<String>,
        conditional_validity_candidate: Option<String>,
    ) -> Option<IslArtifacts> {
        let domain_c = CString::new(domain.clone()).ok()?;
        let candidate_c = candidate_schedule_map
            .as_ref()
            .and_then(|raw| CString::new(raw.as_str()).ok());
        let validity_c = validity
            .as_ref()
            .and_then(|raw| CString::new(raw.as_str()).ok());
        let proximity_c = proximity
            .as_ref()
            .and_then(|raw| CString::new(raw.as_str()).ok());
        let coincidence_c = coincidence
            .as_ref()
            .and_then(|raw| CString::new(raw.as_str()).ok());
        let conditional_validity_c = conditional_validity
            .as_ref()
            .and_then(|raw| CString::new(raw.as_str()).ok());

        // SAFETY: All `CString` inputs are owned locally for this full call.
        // ISL schedule materialization uses FFI raw pointer lifetimes, ownership
        // transfers, and explicit frees that cannot be expressed safely in Rust.
        unsafe {
            let ctx = isl_ctx_alloc();
            if ctx.is_null() {
                return None;
            }
            configure_isl_context(ctx);

            let (
                constraints,
                validity_roundtrip,
                proximity_roundtrip,
                conditional_validity_roundtrip,
            ) = build_schedule_constraints(
                ctx,
                &domain_c,
                validity_c.as_ref(),
                proximity_c.as_ref(),
                conditional_validity_c.as_ref(),
            );
            let coincidence_roundtrip = if let Some(coincidence_c) = &coincidence_c {
                let coincidence_map = isl_union_map_read_from_str(ctx, coincidence_c.as_ptr());
                if !coincidence_map.is_null() {
                    let rendered = take_isl_string(isl_union_map_to_str(coincidence_map));
                    let _ = isl_union_map_free(coincidence_map);
                    rendered
                } else {
                    None
                }
            } else {
                None
            };
            let mut conditional_validity_applied =
                conditional_validity_c.is_some() && conditional_validity_roundtrip.is_some();
            let mut schedule = if constraints.is_null() {
                std::ptr::null_mut()
            } else {
                isl_schedule_constraints_compute_schedule(constraints)
            };
            if schedule.is_null() && conditional_validity_applied {
                conditional_validity_applied = false;
                let (retry_constraints, _, _, _) = build_schedule_constraints(
                    ctx,
                    &domain_c,
                    validity_c.as_ref(),
                    proximity_c.as_ref(),
                    None,
                );
                schedule = if retry_constraints.is_null() {
                    std::ptr::null_mut()
                } else {
                    isl_schedule_constraints_compute_schedule(retry_constraints)
                };
            }
            let schedule = if schedule.is_null() {
                let fallback_domain = isl_union_set_read_from_str(ctx, domain_c.as_ptr());
                if fallback_domain.is_null() {
                    isl_ctx_free(ctx);
                    return None;
                }
                isl_schedule_from_domain(fallback_domain)
            } else {
                schedule
            };
            if schedule.is_null() {
                isl_ctx_free(ctx);
                return None;
            }

            let computed_schedule = take_isl_string(isl_schedule_to_str(schedule))?;
            let root = isl_schedule_get_root(schedule);
            let root_type = node_type_name(isl_schedule_node_get_type(root)).to_string();
            let (contains_sequence_node, contains_filter_node) = schedule_shape_flags(root);
            let (first_band_members, first_band_partial_schedule) = first_band_info(root);
            let _ = isl_schedule_node_free(root);
            let _ = isl_schedule_free(schedule);

            let candidate_schedule_roundtrip = if let Some(candidate_c) = &candidate_c {
                let umap = isl_union_map_read_from_str(ctx, candidate_c.as_ptr());
                if umap.is_null() {
                    None
                } else {
                    let roundtrip = take_isl_string(isl_union_map_to_str(umap));
                    let _ = isl_union_map_free(umap);
                    roundtrip
                }
            } else {
                None
            };

            isl_ctx_free(ctx);

            Some(IslArtifacts {
                domain,
                validity: validity_roundtrip,
                proximity: proximity_roundtrip,
                coincidence: coincidence_roundtrip,
                conditional_validity: conditional_validity_roundtrip,
                conditional_validity_applied,
                conditional_validity_candidate,
                candidate_schedule_map,
                candidate_schedule_roundtrip,
                computed_schedule,
                root_type,
                contains_sequence_node,
                contains_filter_node,
                first_band_members,
                first_band_partial_schedule,
            })
        }
    }

    pub fn materialize_schedule_artifacts(
        scop: &ScopRegion,
        plan: &SchedulePlan,
        validity: Option<&str>,
        proximity: Option<&str>,
        coincidence: Option<&str>,
        conditional_validity: Option<&str>,
        conditional_validity_candidate: Option<&str>,
    ) -> Option<IslArtifacts> {
        let domain = scop_domain_to_str(scop, plan);
        let candidate_schedule_map = schedule_map_to_str(scop, plan);
        // NOTE: ISL conditional-validity maps can fail below Rust's error
        // boundary on some linked libisl builds. Keep the optimizer hermetic by
        // recording the candidate and using the normal schedule constraints in
        // process; the backend emits a fallback hint when the condition is not
        // applied.
        let conditional_validity_for_isl = if conditional_validity.is_some() {
            None
        } else {
            conditional_validity.map(ToOwned::to_owned)
        };
        materialize_schedule_artifacts_from_strings(
            domain,
            candidate_schedule_map,
            validity.map(ToOwned::to_owned),
            proximity.map(ToOwned::to_owned),
            coincidence.map(ToOwned::to_owned),
            conditional_validity_for_isl,
            conditional_validity_candidate.map(ToOwned::to_owned),
        )
    }
}

#[cfg(not(rr_has_isl))]
pub(crate) mod imp {
    use super::*;

    pub fn map_roundtrip_if_non_empty(_raw: &str) -> Option<String> {
        None
    }

    pub fn union_maps_roundtrip(_maps: &[String]) -> Option<String> {
        None
    }

    pub fn materialize_schedule_artifacts(
        _scop: &ScopRegion,
        _plan: &SchedulePlan,
        _validity: Option<&str>,
        _proximity: Option<&str>,
        _coincidence: Option<&str>,
        _conditional_validity: Option<&str>,
        _conditional_validity_candidate: Option<&str>,
    ) -> Option<IslArtifacts> {
        None
    }
}

pub fn map_roundtrip_if_non_empty(raw: &str) -> Option<String> {
    imp::map_roundtrip_if_non_empty(raw)
}

pub fn union_maps_roundtrip(maps: &[String]) -> Option<String> {
    imp::union_maps_roundtrip(maps)
}

pub fn materialize_schedule_artifacts(
    scop: &ScopRegion,
    plan: &SchedulePlan,
    validity: Option<&str>,
    proximity: Option<&str>,
    coincidence: Option<&str>,
    conditional_validity: Option<&str>,
    conditional_validity_candidate: Option<&str>,
) -> Option<IslArtifacts> {
    imp::materialize_schedule_artifacts(
        scop,
        plan,
        validity,
        proximity,
        coincidence,
        conditional_validity,
        conditional_validity_candidate,
    )
}

pub fn isl_available() -> bool {
    cfg!(rr_has_isl)
}
