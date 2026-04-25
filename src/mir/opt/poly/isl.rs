use super::ScopRegion;
use super::affine::{AffineExpr, AffineSymbol};
use super::schedule::{SchedulePlan, SchedulePlanKind};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslArtifacts {
    pub domain: String,
    pub validity: Option<String>,
    pub proximity: Option<String>,
    pub coincidence: Option<String>,
    pub conditional_validity: Option<String>,
    pub conditional_validity_applied: bool,
    pub conditional_validity_candidate: Option<String>,
    pub candidate_schedule_map: Option<String>,
    pub candidate_schedule_roundtrip: Option<String>,
    pub computed_schedule: String,
    pub root_type: String,
    pub contains_sequence_node: bool,
    pub contains_filter_node: bool,
    pub first_band_members: usize,
    pub first_band_partial_schedule: Option<String>,
}

impl IslArtifacts {
    pub fn render(&self) -> String {
        format!(
            "domain={}; validity={}; proximity={}; coincidence={}; conditional_validity={}; conditional_validity_applied={}; conditional_validity_candidate={}; candidate_map={}; candidate_roundtrip={}; computed_schedule={}; root_type={}; contains_sequence_node={}; contains_filter_node={}; first_band_members={}; first_band_partial_schedule={}",
            self.domain,
            self.validity.as_deref().unwrap_or(""),
            self.proximity.as_deref().unwrap_or(""),
            self.coincidence.as_deref().unwrap_or(""),
            self.conditional_validity.as_deref().unwrap_or(""),
            usize::from(self.conditional_validity_applied),
            self.conditional_validity_candidate.as_deref().unwrap_or(""),
            self.candidate_schedule_map.as_deref().unwrap_or(""),
            self.candidate_schedule_roundtrip.as_deref().unwrap_or(""),
            self.computed_schedule,
            self.root_type,
            usize::from(self.contains_sequence_node),
            usize::from(self.contains_filter_node),
            self.first_band_members,
            self.first_band_partial_schedule.as_deref().unwrap_or(""),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IslTransformHints {
    pub inferred_plan: Option<SchedulePlan>,
    pub prefer_fission: bool,
    pub reason: String,
}

const ISL_MATERIALIZE_HELPER_CMD: &str = "__rr_poly_isl_materialize";

#[derive(Debug, Clone)]
struct MaterializeRequest {
    domain: String,
    candidate_schedule_map: Option<String>,
    validity: Option<String>,
    proximity: Option<String>,
    coincidence: Option<String>,
    conditional_validity: Option<String>,
    conditional_validity_candidate: Option<String>,
}

fn write_u64(writer: &mut dyn Write, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u64(reader: &mut dyn Read) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn write_bool(writer: &mut dyn Write, value: bool) -> io::Result<()> {
    writer.write_all(&[u8::from(value)])
}

fn read_bool(reader: &mut dyn Read) -> io::Result<bool> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0] != 0)
}

fn write_string(writer: &mut dyn Write, value: &str) -> io::Result<()> {
    write_u64(writer, value.len() as u64)?;
    writer.write_all(value.as_bytes())
}

fn read_string(reader: &mut dyn Read) -> io::Result<String> {
    let len = read_u64(reader)? as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn write_opt_string(writer: &mut dyn Write, value: Option<&str>) -> io::Result<()> {
    write_bool(writer, value.is_some())?;
    if let Some(value) = value {
        write_string(writer, value)?;
    }
    Ok(())
}

fn read_opt_string(reader: &mut dyn Read) -> io::Result<Option<String>> {
    if read_bool(reader)? {
        read_string(reader).map(Some)
    } else {
        Ok(None)
    }
}

fn write_materialize_request(path: &Path, request: &MaterializeRequest) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    write_string(&mut file, &request.domain)?;
    write_opt_string(&mut file, request.candidate_schedule_map.as_deref())?;
    write_opt_string(&mut file, request.validity.as_deref())?;
    write_opt_string(&mut file, request.proximity.as_deref())?;
    write_opt_string(&mut file, request.coincidence.as_deref())?;
    write_opt_string(&mut file, request.conditional_validity.as_deref())?;
    write_opt_string(&mut file, request.conditional_validity_candidate.as_deref())?;
    Ok(())
}

fn read_materialize_request(path: &Path) -> io::Result<MaterializeRequest> {
    let mut file = fs::File::open(path)?;
    Ok(MaterializeRequest {
        domain: read_string(&mut file)?,
        candidate_schedule_map: read_opt_string(&mut file)?,
        validity: read_opt_string(&mut file)?,
        proximity: read_opt_string(&mut file)?,
        coincidence: read_opt_string(&mut file)?,
        conditional_validity: read_opt_string(&mut file)?,
        conditional_validity_candidate: read_opt_string(&mut file)?,
    })
}

fn write_materialize_response(path: &Path, artifacts: Option<&IslArtifacts>) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    write_bool(&mut file, artifacts.is_some())?;
    let Some(artifacts) = artifacts else {
        return Ok(());
    };
    write_string(&mut file, &artifacts.domain)?;
    write_opt_string(&mut file, artifacts.validity.as_deref())?;
    write_opt_string(&mut file, artifacts.proximity.as_deref())?;
    write_opt_string(&mut file, artifacts.coincidence.as_deref())?;
    write_opt_string(&mut file, artifacts.conditional_validity.as_deref())?;
    write_bool(&mut file, artifacts.conditional_validity_applied)?;
    write_opt_string(
        &mut file,
        artifacts.conditional_validity_candidate.as_deref(),
    )?;
    write_opt_string(&mut file, artifacts.candidate_schedule_map.as_deref())?;
    write_opt_string(&mut file, artifacts.candidate_schedule_roundtrip.as_deref())?;
    write_string(&mut file, &artifacts.computed_schedule)?;
    write_string(&mut file, &artifacts.root_type)?;
    write_bool(&mut file, artifacts.contains_sequence_node)?;
    write_bool(&mut file, artifacts.contains_filter_node)?;
    write_u64(&mut file, artifacts.first_band_members as u64)?;
    write_opt_string(&mut file, artifacts.first_band_partial_schedule.as_deref())?;
    Ok(())
}

fn read_materialize_response(path: &Path) -> io::Result<Option<IslArtifacts>> {
    let mut file = fs::File::open(path)?;
    if !read_bool(&mut file)? {
        return Ok(None);
    }
    Ok(Some(IslArtifacts {
        domain: read_string(&mut file)?,
        validity: read_opt_string(&mut file)?,
        proximity: read_opt_string(&mut file)?,
        coincidence: read_opt_string(&mut file)?,
        conditional_validity: read_opt_string(&mut file)?,
        conditional_validity_applied: read_bool(&mut file)?,
        conditional_validity_candidate: read_opt_string(&mut file)?,
        candidate_schedule_map: read_opt_string(&mut file)?,
        candidate_schedule_roundtrip: read_opt_string(&mut file)?,
        computed_schedule: read_string(&mut file)?,
        root_type: read_string(&mut file)?,
        contains_sequence_node: read_bool(&mut file)?,
        contains_filter_node: read_bool(&mut file)?,
        first_band_members: read_u64(&mut file)? as usize,
        first_band_partial_schedule: read_opt_string(&mut file)?,
    }))
}

fn helper_temp_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rr_poly_isl_{label}_{}_{}.bin",
        std::process::id(),
        nonce
    ))
}

fn current_rr_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let stem = exe.file_stem()?.to_string_lossy();
    if stem == "RR" { Some(exe) } else { None }
}

fn try_materialize_via_helper(request: &MaterializeRequest) -> Option<Option<IslArtifacts>> {
    let rr_bin = current_rr_binary()?;
    let request_path = helper_temp_path("request");
    let response_path = helper_temp_path("response");
    if write_materialize_request(&request_path, request).is_err() {
        let _ = fs::remove_file(&request_path);
        return Some(None);
    }

    let status = Command::new(rr_bin)
        .arg(ISL_MATERIALIZE_HELPER_CMD)
        .arg(&request_path)
        .arg(&response_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let response = match status {
        Ok(status) if status.success() => read_materialize_response(&response_path).ok(),
        _ => Some(None),
    };

    let _ = fs::remove_file(&request_path);
    let _ = fs::remove_file(&response_path);
    Some(response.unwrap_or(None))
}

pub fn snapshot_schedule_artifacts(
    scop: &ScopRegion,
    plan: &SchedulePlan,
    validity: Option<&str>,
    proximity: Option<&str>,
    coincidence: Option<&str>,
    conditional_validity: Option<&str>,
    conditional_validity_candidate: Option<&str>,
) -> IslArtifacts {
    let candidate_schedule_map = {
        let inputs = plan.relation.input_dimensions.to_vec();
        let outputs = plan
            .relation
            .output_expressions
            .iter()
            .map(|expr| {
                let mut parts = Vec::new();
                for (symbol, coeff) in &expr.terms {
                    let name = match symbol {
                        AffineSymbol::LoopIv(name)
                        | AffineSymbol::Param(name)
                        | AffineSymbol::Invariant(name)
                        | AffineSymbol::Length(name) => name.clone(),
                    };
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
            })
            .collect::<Vec<_>>();
        if inputs.is_empty() || outputs.is_empty() {
            None
        } else {
            Some(format!(
                "{{ {} }}",
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
    };
    let domain = format!(
        "{{ {} }}",
        solver_statement_ids(scop)
            .into_iter()
            .map(|stmt_id| {
                let dims = scop
                    .dimensions
                    .iter()
                    .map(|dim| dim.iv_name.clone())
                    .collect::<Vec<_>>();
                format!("S{stmt_id}[{}]", dims.join(", "))
            })
            .collect::<Vec<_>>()
            .join("; ")
    );
    IslArtifacts {
        domain: domain.clone(),
        validity: validity.map(ToOwned::to_owned),
        proximity: proximity.map(ToOwned::to_owned),
        coincidence: coincidence.map(ToOwned::to_owned),
        conditional_validity: conditional_validity.map(ToOwned::to_owned),
        conditional_validity_applied: false,
        conditional_validity_candidate: conditional_validity_candidate.map(ToOwned::to_owned),
        candidate_schedule_map: candidate_schedule_map.clone(),
        candidate_schedule_roundtrip: candidate_schedule_map.clone(),
        computed_schedule: candidate_schedule_map
            .clone()
            .unwrap_or_else(|| domain.clone()),
        root_type: "domain".to_string(),
        contains_sequence_node: false,
        contains_filter_node: !solver_statement_ids(scop).is_empty(),
        first_band_members: plan.relation.output_expressions.len(),
        first_band_partial_schedule: candidate_schedule_map,
    }
}

fn solver_statement_ids(scop: &ScopRegion) -> Vec<usize> {
    let ids = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .map(|stmt| stmt.id)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        scop.statements.iter().map(|stmt| stmt.id).collect()
    } else {
        ids
    }
}

pub fn infer_plan_from_first_band(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    partial: Option<&str>,
) -> Option<SchedulePlan> {
    let partial = partial?;
    let start = partial.rfind('[')?;
    let end = partial[start..].find(']')? + start;
    let inside = partial.get(start + 1..end)?.trim();
    if inside.is_empty() {
        return None;
    }
    let order = inside
        .split(',')
        .map(|part| part.trim().to_string())
        .collect::<Vec<_>>();
    let dims = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    if order.len() != dims.len() || order.iter().any(|name| !dims.contains(name)) {
        return None;
    }
    let relation = super::schedule::ScheduleRelation {
        input_dimensions: dims.clone(),
        output_expressions: order
            .iter()
            .cloned()
            .map(|name| AffineExpr::symbol(AffineSymbol::LoopIv(name)))
            .collect(),
    };
    let kind = if order == dims {
        SchedulePlanKind::Identity
    } else {
        SchedulePlanKind::Interchange
    };
    Some(SchedulePlan {
        kind,
        relation,
        backend,
        tile_size: None,
        tile_depth: None,
        tile_rows: None,
        tile_cols: None,
    })
}

fn infer_skew2d_plan_from_partial(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    partial: Option<&str>,
) -> Option<SchedulePlan> {
    let partial = partial?;
    if scop.dimensions.len() != 2 {
        return None;
    }
    let outer = scop.dimensions[0].iv_name.clone();
    let inner = scop.dimensions[1].iv_name.clone();
    let outer_pat = super::affine::AffineSymbol::LoopIv(outer.clone());
    let inner_pat = super::affine::AffineSymbol::LoopIv(inner.clone());
    let outer_name = match &outer_pat {
        AffineSymbol::LoopIv(name) => name.as_str(),
        _ => unreachable!(),
    };
    let inner_name = match &inner_pat {
        AffineSymbol::LoopIv(name) => name.as_str(),
        _ => unreachable!(),
    };
    let looks_skewed =
        partial.contains(outer_name) && partial.contains(inner_name) && partial.contains(" + ");
    if !looks_skewed {
        return None;
    }
    let mut skewed = AffineExpr::symbol(AffineSymbol::LoopIv(inner.clone()));
    skewed.add_assign(&AffineExpr::symbol(AffineSymbol::LoopIv(outer.clone())), 1);
    Some(SchedulePlan {
        kind: SchedulePlanKind::Skew2D,
        relation: super::schedule::ScheduleRelation {
            input_dimensions: vec![outer.clone(), inner],
            output_expressions: vec![AffineExpr::symbol(AffineSymbol::LoopIv(outer)), skewed],
        },
        backend,
        tile_size: None,
        tile_depth: None,
        tile_rows: None,
        tile_cols: None,
    })
}

fn infer_tile_plan_from_artifacts(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    artifacts: &IslArtifacts,
) -> Option<SchedulePlan> {
    let text = artifacts.computed_schedule.as_str();
    let kind = if text.contains("chosen_kind=Tile3D")
        || artifacts.root_type == "band" && scop.dimensions.len() == 3
    {
        SchedulePlanKind::Tile3D
    } else if text.contains("chosen_kind=Tile2D")
        || (artifacts.root_type == "band" && scop.dimensions.len() == 2)
    {
        SchedulePlanKind::Tile2D
    } else if text.contains("chosen_kind=Tile1D")
        || (artifacts.root_type == "band" && scop.dimensions.len() == 1)
    {
        SchedulePlanKind::Tile1D
    } else {
        return None;
    };

    Some(SchedulePlan {
        kind,
        relation: super::schedule::ScheduleRelation {
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
        },
        backend,
        tile_size: (kind == SchedulePlanKind::Tile1D).then_some(64),
        tile_depth: (kind == SchedulePlanKind::Tile3D).then_some(4),
        tile_rows: matches!(kind, SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D).then_some(8),
        tile_cols: matches!(kind, SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D).then_some(8),
    })
}

pub fn infer_transform_hints(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    artifacts: &IslArtifacts,
) -> IslTransformHints {
    let inferred_plan = infer_tile_plan_from_artifacts(scop, backend, artifacts)
        .or_else(|| {
            infer_plan_from_first_band(
                scop,
                backend,
                artifacts.first_band_partial_schedule.as_deref(),
            )
        })
        .or_else(|| {
            infer_skew2d_plan_from_partial(
                scop,
                backend,
                artifacts.first_band_partial_schedule.as_deref(),
            )
        });
    let prefer_fission = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count()
        > 1
        && (artifacts.contains_sequence_node || artifacts.contains_filter_node);
    let mut reasons = Vec::new();
    if let Some(plan) = &inferred_plan {
        reasons.push(format!("hint_plan={:?}", plan.kind));
    }
    if prefer_fission {
        reasons.push("hint_fission=1".to_string());
    }
    if reasons.is_empty() {
        reasons.push("hint_none".to_string());
    }
    IslTransformHints {
        inferred_plan,
        prefer_fission,
        reason: reasons.join(","),
    }
}

#[cfg(rr_has_isl)]
mod imp {
    use super::super::affine::AffineConstraintKind;
    use super::*;
    use std::collections::BTreeSet;
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int, c_void};

    #[repr(C)]
    struct isl_ctx {
        _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_set {
        _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_map {
        _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_union_set {
        _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_union_map {
        _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_schedule_constraints {
        _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_schedule {
        _private: [u8; 0],
    }
    #[repr(C)]
    struct isl_schedule_node {
        _private: [u8; 0],
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
        if conditional_validity.is_some() {
            let request = super::MaterializeRequest {
                domain: domain.clone(),
                candidate_schedule_map: candidate_schedule_map.clone(),
                validity: validity.map(ToOwned::to_owned),
                proximity: proximity.map(ToOwned::to_owned),
                coincidence: coincidence.map(ToOwned::to_owned),
                conditional_validity: conditional_validity.map(ToOwned::to_owned),
                conditional_validity_candidate: conditional_validity_candidate
                    .map(ToOwned::to_owned),
            };
            if let Some(artifacts) = super::try_materialize_via_helper(&request) {
                return artifacts;
            }
            return None;
        }

        materialize_schedule_artifacts_from_strings(
            domain,
            candidate_schedule_map,
            validity.map(ToOwned::to_owned),
            proximity.map(ToOwned::to_owned),
            coincidence.map(ToOwned::to_owned),
            conditional_validity.map(ToOwned::to_owned),
            conditional_validity_candidate.map(ToOwned::to_owned),
        )
    }

    pub fn run_materialize_helper_from_cli(args: &[String]) -> Option<i32> {
        let [request_path, response_path] = args else {
            return Some(2);
        };
        let request = match super::read_materialize_request(Path::new(request_path)) {
            Ok(request) => request,
            Err(_) => return Some(2),
        };
        let artifacts = materialize_schedule_artifacts_from_strings(
            request.domain,
            request.candidate_schedule_map,
            request.validity,
            request.proximity,
            request.coincidence,
            request.conditional_validity,
            request.conditional_validity_candidate,
        );
        if super::write_materialize_response(Path::new(response_path), artifacts.as_ref()).is_err()
        {
            return Some(2);
        }
        Some(0)
    }
}

#[cfg(not(rr_has_isl))]
mod imp {
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

    pub fn run_materialize_helper_from_cli(args: &[String]) -> Option<i32> {
        let [_request_path, _response_path] = args else {
            return Some(2);
        };
        Some(0)
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

pub fn run_materialize_helper_from_cli(args: &[String]) -> Option<i32> {
    imp::run_materialize_helper_from_cli(args)
}

pub fn isl_available() -> bool {
    cfg!(rr_has_isl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::opt::poly::affine::PresburgerSet;
    use crate::mir::opt::poly::{LoopDimension, PolyStmt, PolyStmtKind};

    fn test_scop(stmt_count: usize) -> ScopRegion {
        ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
            parameters: Default::default(),
            statements: (0..stmt_count)
                .map(|id| PolyStmt {
                    id,
                    block: 0,
                    kind: PolyStmtKind::Assign {
                        dst: format!("v{id}"),
                    },
                    expr_root: None,
                    accesses: vec![crate::mir::opt::poly::access::AccessRelation {
                        statement_id: id,
                        kind: crate::mir::opt::poly::access::AccessKind::Write,
                        memref: crate::mir::opt::poly::access::MemRef {
                            base: id + 1,
                            name: format!("A{id}"),
                            rank: 2,
                            layout: crate::mir::opt::poly::access::MemoryLayout::ColumnMajor2D,
                        },
                        subscripts: vec![
                            AffineExpr::symbol(AffineSymbol::LoopIv("r".to_string())),
                            AffineExpr::symbol(AffineSymbol::LoopIv("c".to_string())),
                        ],
                    }],
                })
                .collect(),
        }
    }

    #[test]
    fn infer_transform_hints_detects_skew2d_from_partial_schedule() {
        let scop = test_scop(2);
        let artifacts = IslArtifacts {
            domain: String::new(),
            validity: None,
            proximity: None,
            coincidence: None,
            conditional_validity: None,
            conditional_validity_applied: false,
            conditional_validity_candidate: None,
            candidate_schedule_map: None,
            candidate_schedule_roundtrip: None,
            computed_schedule: String::new(),
            root_type: "domain".to_string(),
            contains_sequence_node: false,
            contains_filter_node: false,
            first_band_members: 2,
            first_band_partial_schedule: Some("{ S0[r, c] -> [r, c + r] }".to_string()),
        };
        let hints = infer_transform_hints(
            &scop,
            super::super::schedule::PolyBackendUsed::Isl,
            &artifacts,
        );
        assert_eq!(
            hints.inferred_plan.as_ref().map(|plan| plan.kind),
            Some(SchedulePlanKind::Skew2D)
        );
        assert!(hints.reason.contains("hint_plan=Skew2D"));
    }

    #[test]
    fn infer_transform_hints_prefers_fission_for_sequence_artifact() {
        let scop = test_scop(2);
        let artifacts = IslArtifacts {
            domain: String::new(),
            validity: None,
            proximity: None,
            coincidence: None,
            conditional_validity: None,
            conditional_validity_applied: false,
            conditional_validity_candidate: None,
            candidate_schedule_map: None,
            candidate_schedule_roundtrip: None,
            computed_schedule: String::new(),
            root_type: "domain".to_string(),
            contains_sequence_node: true,
            contains_filter_node: true,
            first_band_members: 2,
            first_band_partial_schedule: Some("{ S0[r, c] -> [r, c] }".to_string()),
        };
        let hints = infer_transform_hints(
            &scop,
            super::super::schedule::PolyBackendUsed::Isl,
            &artifacts,
        );
        assert!(hints.prefer_fission);
        assert!(hints.reason.contains("hint_fission=1"));
    }

    #[test]
    fn infer_transform_hints_accepts_non_rotated_3d_permutation_as_interchange() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(4),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(4),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(4),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Assign {
                    dst: "v0".to_string(),
                },
                expr_root: None,
                accesses: vec![],
            }],
        };
        let artifacts = IslArtifacts {
            domain: String::new(),
            validity: None,
            proximity: None,
            coincidence: None,
            conditional_validity: None,
            conditional_validity_applied: false,
            conditional_validity_candidate: None,
            candidate_schedule_map: None,
            candidate_schedule_roundtrip: None,
            computed_schedule: String::new(),
            root_type: "domain".to_string(),
            contains_sequence_node: false,
            contains_filter_node: false,
            first_band_members: 3,
            first_band_partial_schedule: Some("{ S0[i, j, k] -> [k, i, j] }".to_string()),
        };
        let hints = infer_transform_hints(
            &scop,
            super::super::schedule::PolyBackendUsed::Isl,
            &artifacts,
        );
        let inferred = hints.inferred_plan.expect("expected inferred plan");
        assert_eq!(inferred.kind, SchedulePlanKind::Interchange);
        assert_eq!(
            inferred
                .relation
                .output_expressions
                .iter()
                .map(|expr| {
                    expr.terms
                        .iter()
                        .next()
                        .and_then(|(symbol, _)| match symbol {
                            AffineSymbol::LoopIv(name) => Some(name.clone()),
                            _ => None,
                        })
                })
                .collect::<Vec<_>>(),
            vec![
                Some("k".to_string()),
                Some("i".to_string()),
                Some("j".to_string()),
            ]
        );
    }

    #[test]
    fn infer_transform_hints_detects_tile2d_from_artifact_choice() {
        let scop = test_scop(2);
        let artifacts = IslArtifacts {
            domain: String::new(),
            validity: None,
            proximity: None,
            coincidence: None,
            conditional_validity: None,
            conditional_validity_applied: false,
            conditional_validity_candidate: None,
            candidate_schedule_map: None,
            candidate_schedule_roundtrip: None,
            computed_schedule: "chosen_kind=Tile2D".to_string(),
            root_type: "band".to_string(),
            contains_sequence_node: false,
            contains_filter_node: false,
            first_band_members: 2,
            first_band_partial_schedule: Some("{ S0[r, c] -> [r, c] }".to_string()),
        };
        let hints = infer_transform_hints(
            &scop,
            super::super::schedule::PolyBackendUsed::Isl,
            &artifacts,
        );
        assert_eq!(
            hints.inferred_plan.as_ref().map(|plan| plan.kind),
            Some(SchedulePlanKind::Tile2D)
        );
    }

    #[test]
    fn infer_transform_hints_detects_tile3d_from_artifact_choice() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(4),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(4),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(4),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Assign {
                    dst: "v0".to_string(),
                },
                expr_root: None,
                accesses: vec![],
            }],
        };
        let artifacts = IslArtifacts {
            domain: String::new(),
            validity: None,
            proximity: None,
            coincidence: None,
            conditional_validity: None,
            conditional_validity_applied: false,
            conditional_validity_candidate: None,
            candidate_schedule_map: None,
            candidate_schedule_roundtrip: None,
            computed_schedule: "chosen_kind=Tile3D".to_string(),
            root_type: "band".to_string(),
            contains_sequence_node: false,
            contains_filter_node: false,
            first_band_members: 3,
            first_band_partial_schedule: Some("{ S0[i, j, k] -> [i, j, k] }".to_string()),
        };
        let hints = infer_transform_hints(
            &scop,
            super::super::schedule::PolyBackendUsed::Isl,
            &artifacts,
        );
        assert_eq!(
            hints.inferred_plan.as_ref().map(|plan| plan.kind),
            Some(SchedulePlanKind::Tile3D)
        );
    }
}
