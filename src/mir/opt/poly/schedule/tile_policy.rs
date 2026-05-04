use super::*;
pub(crate) fn tile_policy_from_env() -> TilePolicy {
    let enable_1d = std::env::var("RR_POLY_TILE_1D").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let tile_size = std::env::var("RR_POLY_TILE_SIZE")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(64);
    let skew_2d_mode = match std::env::var("RR_POLY_SKEW_2D")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("1" | "true" | "yes" | "on") => AutoChoice::ForceOn,
        Some("0" | "false" | "no" | "off") => AutoChoice::ForceOff,
        Some("auto") | None => AutoChoice::Auto,
        Some(_) => AutoChoice::Auto,
    };
    let enable_2d = std::env::var("RR_POLY_TILE_2D").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let enable_3d = std::env::var("RR_POLY_TILE_3D").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let tile_depth = std::env::var("RR_POLY_TILE_DEPTH")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(4);
    let tile_rows = std::env::var("RR_POLY_TILE_ROWS")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(8);
    let tile_cols = std::env::var("RR_POLY_TILE_COLS")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(8);
    TilePolicy {
        enable_1d,
        skew_2d_mode,
        allow_skew_with_tiles: false,
        tile_size,
        enable_2d,
        enable_3d,
        tile_depth,
        tile_rows,
        tile_cols,
    }
}

pub(crate) fn solver_tile_policy(requested: PolyBackendKind, base: TilePolicy) -> TilePolicy {
    if requested != PolyBackendKind::Isl {
        return base;
    }
    TilePolicy {
        enable_1d: true,
        enable_2d: true,
        enable_3d: true,
        allow_skew_with_tiles: true,
        skew_2d_mode: match base.skew_2d_mode {
            AutoChoice::ForceOff => AutoChoice::ForceOff,
            AutoChoice::ForceOn | AutoChoice::Auto => AutoChoice::Auto,
        },
        ..base
    }
}

pub(crate) fn dedup_positive_usizes(values: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let mut out = values
        .into_iter()
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();
    out.sort_unstable();
    out.dedup();
    out
}

pub(crate) fn solver_tile1d_variants(policy: TilePolicy, requested: PolyBackendKind) -> Vec<usize> {
    if requested != PolyBackendKind::Isl {
        return vec![policy.tile_size];
    }
    dedup_positive_usizes([policy.tile_size / 2, policy.tile_size, policy.tile_size * 2])
}

pub(crate) fn solver_tile2d_variants(
    policy: TilePolicy,
    requested: PolyBackendKind,
) -> Vec<(usize, usize)> {
    if requested != PolyBackendKind::Isl {
        return vec![(policy.tile_rows, policy.tile_cols)];
    }
    let row_variants =
        dedup_positive_usizes([policy.tile_rows / 2, policy.tile_rows, policy.tile_rows * 2]);
    let col_variants =
        dedup_positive_usizes([policy.tile_cols / 2, policy.tile_cols, policy.tile_cols * 2]);
    let mut out = Vec::new();
    for rows in &row_variants {
        for cols in &col_variants {
            out.push((*rows, *cols));
        }
    }
    out
}

pub(crate) fn solver_tile3d_variants(
    policy: TilePolicy,
    requested: PolyBackendKind,
) -> Vec<(usize, usize, usize)> {
    if requested != PolyBackendKind::Isl {
        return vec![(policy.tile_depth, policy.tile_rows, policy.tile_cols)];
    }
    let depth_variants = dedup_positive_usizes([
        policy.tile_depth / 2,
        policy.tile_depth,
        policy.tile_depth * 2,
    ]);
    let row_variants =
        dedup_positive_usizes([policy.tile_rows / 2, policy.tile_rows, policy.tile_rows * 2]);
    let col_variants =
        dedup_positive_usizes([policy.tile_cols / 2, policy.tile_cols, policy.tile_cols * 2]);
    let mut out = Vec::new();
    for depth in &depth_variants {
        for rows in &row_variants {
            for cols in &col_variants {
                out.push((*depth, *rows, *cols));
            }
        }
    }
    out
}
