use crate::typeck::builtin_sigs::vectorized_scalar_or_vector_double_type;
use crate::typeck::lattice::{PrimTy, TypeState};

pub(crate) fn infer_graphics_pkg_state(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "graphics::plot"
        | "graphics::plot.default"
        | "graphics::plot.design"
        | "graphics::plot.function"
        | "graphics::plot.new"
        | "graphics::plot.window"
        | "graphics::plot.xy"
        | "graphics::lines"
        | "graphics::lines.default"
        | "graphics::points"
        | "graphics::points.default"
        | "graphics::abline"
        | "graphics::title"
        | "graphics::box"
        | "graphics::text"
        | "graphics::text.default"
        | "graphics::segments"
        | "graphics::arrows"
        | "graphics::mtext"
        | "graphics::polygon"
        | "graphics::polypath"
        | "graphics::matplot"
        | "graphics::matlines"
        | "graphics::matpoints"
        | "graphics::pairs"
        | "graphics::pairs.default"
        | "graphics::stripchart"
        | "graphics::dotchart"
        | "graphics::layout.show"
        | "graphics::pie"
        | "graphics::symbols"
        | "graphics::smoothScatter"
        | "graphics::stem"
        | "graphics::contour"
        | "graphics::contour.default"
        | "graphics::image"
        | "graphics::image.default"
        | "graphics::assocplot"
        | "graphics::mosaicplot"
        | "graphics::fourfoldplot"
        | "graphics::clip"
        | "graphics::xspline"
        | "graphics::.filled.contour"
        | "graphics::filled.contour"
        | "graphics::cdplot"
        | "graphics::coplot"
        | "graphics::curve"
        | "graphics::close.screen"
        | "graphics::co.intervals"
        | "graphics::erase.screen"
        | "graphics::frame"
        | "graphics::grid"
        | "graphics::panel.smooth"
        | "graphics::rasterImage"
        | "graphics::rect"
        | "graphics::spineplot"
        | "graphics::stars"
        | "graphics::sunflowerplot"
        | "grDevices::png"
        | "grDevices::pdf" => Some(TypeState::null()),
        "graphics::persp" => Some(TypeState::matrix(PrimTy::Double, false)),
        "graphics::hist"
        | "graphics::hist.default"
        | "graphics::boxplot"
        | "graphics::boxplot.default"
        | "graphics::boxplot.matrix"
        | "graphics::barplot"
        | "graphics::barplot.default"
        | "graphics::bxp"
        | "graphics::par"
        | "graphics::screen"
        | "graphics::split.screen" => Some(TypeState::vector(PrimTy::Any, false)),
        "graphics::layout" => Some(TypeState::scalar(PrimTy::Int, false)),
        "grid::seekViewport" => Some(TypeState::scalar(PrimTy::Int, false)),
        "grid::grob" | "grid::convertX" | "grid::grid.legend" | "grid::vpTree"
        | "grid::grid.remove" => Some(TypeState::unknown()),
        "graphics::identify" => Some(TypeState::vector(PrimTy::Int, false)),
        "graphics::axTicks"
        | "graphics::Axis"
        | "graphics::axis.Date"
        | "graphics::axis.POSIXct"
        | "graphics::strwidth"
        | "graphics::strheight"
        | "graphics::grconvertX"
        | "graphics::grconvertY" => Some(TypeState::vector(PrimTy::Double, false)),
        "graphics::lcm" | "graphics::xinch" | "graphics::yinch" | "graphics::xyinch" => {
            vectorized_scalar_or_vector_double_type(arg_tys)
        }
        "grid::unit" | "grid::grobWidth" | "grid::grobHeight" => Some(TypeState::unknown()),
        "grid::grid.newpage"
        | "grid::pushViewport"
        | "grid::popViewport"
        | "grid::grid.pack"
        | "grid::grid.place"
        | "grid::grid.curve"
        | "grid::grid.bezier"
        | "grid::grid.path"
        | "grid::grid.polyline"
        | "grid::grid.raster"
        | "grid::grid.draw" => Some(TypeState::null()),
        "grid::grid.frame" => Some(TypeState::vector(PrimTy::Any, false)),
        callee
            if callee.starts_with("grid::")
                && !matches!(
                    callee,
                    "grid::grid.newpage"
                        | "grid::pushViewport"
                        | "grid::popViewport"
                        | "grid::grid.pack"
                        | "grid::grid.place"
                        | "grid::grid.curve"
                        | "grid::grid.bezier"
                        | "grid::grid.path"
                        | "grid::grid.polyline"
                        | "grid::grid.raster"
                        | "grid::grid.draw"
                        | "grid::seekViewport"
                        | "grid::unit"
                        | "grid::grobWidth"
                        | "grid::grobHeight"
                ) =>
        {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        _ => None,
    }
}
