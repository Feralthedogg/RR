use crate::typeck::builtin_sigs::vectorized_scalar_or_vector_double_term;
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_graphics_pkg_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
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
        | "grDevices::pdf" => Some(TypeTerm::Null),
        "graphics::persp" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
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
        | "graphics::split.screen" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "graphics::layout" => Some(TypeTerm::Int),
        "grid::seekViewport" => Some(TypeTerm::Int),
        "grid::grob" | "grid::convertX" | "grid::grid.legend" | "grid::vpTree"
        | "grid::grid.remove" => Some(TypeTerm::Any),
        "graphics::axTicks"
        | "graphics::Axis"
        | "graphics::axis.Date"
        | "graphics::axis.POSIXct"
        | "graphics::strwidth"
        | "graphics::strheight"
        | "graphics::grconvertX"
        | "graphics::grconvertY" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "graphics::lcm" | "graphics::xinch" | "graphics::yinch" | "graphics::xyinch" => {
            vectorized_scalar_or_vector_double_term(arg_terms)
        }
        "grid::unit" | "grid::grobWidth" | "grid::grobHeight" => Some(TypeTerm::Any),
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
        | "grid::grid.draw" => Some(TypeTerm::Null),
        "grid::grid.frame" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
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
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        _ => None,
    }
}
