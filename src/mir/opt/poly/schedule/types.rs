use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyBackendKind {
    Heuristic,
    Isl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyBackendUsed {
    Heuristic,
    Isl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleRelation {
    pub input_dimensions: Vec<String>,
    pub output_expressions: Vec<AffineExpr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulePlanKind {
    None,
    Identity,
    Interchange,
    Skew2D,
    Tile1D,
    Tile2D,
    Tile3D,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulePlan {
    pub kind: SchedulePlanKind,
    pub relation: ScheduleRelation,
    pub backend: PolyBackendUsed,
    pub tile_size: Option<usize>,
    pub tile_depth: Option<usize>,
    pub tile_rows: Option<usize>,
    pub tile_cols: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TilePolicy {
    pub(crate) enable_1d: bool,
    pub(crate) skew_2d_mode: AutoChoice,
    pub(crate) allow_skew_with_tiles: bool,
    pub(crate) tile_size: usize,
    pub(crate) enable_2d: bool,
    pub(crate) enable_3d: bool,
    pub(crate) tile_depth: usize,
    pub(crate) tile_rows: usize,
    pub(crate) tile_cols: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoChoice {
    Auto,
    ForceOn,
    ForceOff,
}

pub fn parse_backend_name(raw: &str) -> PolyBackendKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "auto" | "isl" => PolyBackendKind::Isl,
        "heuristic" => PolyBackendKind::Heuristic,
        "fallback" => PolyBackendKind::Heuristic,
        _ => PolyBackendKind::Heuristic,
    }
}

pub(crate) fn backend_from_setting(raw: Option<&str>, _build_has_isl: bool) -> PolyBackendKind {
    match raw
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("isl") => PolyBackendKind::Isl,
        None | Some("auto") => PolyBackendKind::Isl,
        Some(_) => PolyBackendKind::Heuristic,
    }
}

pub fn backend_from_env() -> PolyBackendKind {
    backend_from_setting(
        std::env::var("RR_POLY_BACKEND").ok().as_deref(),
        option_env!("RR_HAS_ISL") == Some("1"),
    )
}
