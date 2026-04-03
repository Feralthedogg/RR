use crate::typeck::builtin_sigs::vectorized_scalar_or_vector_double_type;
use crate::typeck::lattice::{PrimTy, TypeState};

pub(super) fn infer_grdevices_state(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "grDevices::jpeg" | "grDevices::bmp" | "grDevices::tiff" => Some(TypeState::null()),
        "grDevices::dev.size" => Some(TypeState::vector(PrimTy::Double, false)),
        "grDevices::dev.off"
        | "grDevices::dev.cur"
        | "grDevices::dev.next"
        | "grDevices::dev.prev" => Some(TypeState::scalar(PrimTy::Int, false)),
        "graphics::axis" => Some(TypeState::vector(PrimTy::Double, false)),
        "graphics::locator" => Some(TypeState::vector(PrimTy::Any, false)),
        "graphics::rug" => Some(TypeState::vector(PrimTy::Double, false)),
        "grDevices::rgb"
        | "grDevices::hsv"
        | "grDevices::gray"
        | "grDevices::gray.colors"
        | "grDevices::palette.colors"
        | "grDevices::palette.pals"
        | "grDevices::hcl.colors"
        | "grDevices::colors"
        | "grDevices::heat.colors"
        | "grDevices::terrain.colors"
        | "grDevices::topo.colors"
        | "grDevices::cm.colors"
        | "grDevices::rainbow"
        | "grDevices::adjustcolor"
        | "grDevices::palette"
        | "grDevices::densCols" => Some(TypeState::vector(PrimTy::Char, false)),
        "grDevices::n2mfrow" => Some(TypeState::vector(PrimTy::Int, false)),
        "grDevices::col2rgb" => Some(TypeState::matrix(PrimTy::Int, false)),
        "grDevices::rgb2hsv" | "grDevices::convertColor" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "grDevices::as.raster" => Some(TypeState::matrix(PrimTy::Char, false)),
        "grDevices::axisTicks" | "grDevices::extendrange" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "grDevices::cm" => vectorized_scalar_or_vector_double_type(arg_tys),
        "grDevices::boxplot.stats"
        | "grDevices::contourLines"
        | "grDevices::dev.capabilities"
        | "grDevices::dev.capture"
        | "grDevices::check.options"
        | "grDevices::colorConverter"
        | "grDevices::colorRamp"
        | "grDevices::colorRampPalette"
        | "grDevices::getGraphicsEvent"
        | "grDevices::getGraphicsEventEnv"
        | "grDevices::recordPlot"
        | "grDevices::as.graphicsAnnot"
        | "grDevices::make.rgb"
        | "grDevices::pdf.options"
        | "grDevices::pdfFonts"
        | "grDevices::ps.options"
        | "grDevices::postscriptFonts"
        | "grDevices::quartz.options"
        | "grDevices::quartzFont"
        | "grDevices::quartzFonts"
        | "grDevices::X11.options"
        | "grDevices::X11Font"
        | "grDevices::X11Fonts"
        | "grDevices::cairoSymbolFont"
        | "grDevices::CIDFont"
        | "grDevices::Type1Font"
        | "grDevices::Hershey"
        | "grDevices::glyphAnchor"
        | "grDevices::glyphFont"
        | "grDevices::glyphFontList"
        | "grDevices::glyphHeight"
        | "grDevices::glyphHeightBottom"
        | "grDevices::glyphInfo"
        | "grDevices::glyphJust"
        | "grDevices::glyphWidth"
        | "grDevices::glyphWidthLeft"
        | "grDevices::.axisPars"
        | "grDevices::.clipPath"
        | "grDevices::.defineGroup"
        | "grDevices::.devUp"
        | "grDevices::.linearGradientPattern"
        | "grDevices::.mask"
        | "grDevices::.opIndex"
        | "grDevices::.radialGradientPattern"
        | "grDevices::.ruleIndex"
        | "grDevices::.setClipPath"
        | "grDevices::.setMask"
        | "grDevices::.setPattern"
        | "grDevices::.tilingPattern"
        | "grDevices::.useGroup" => Some(TypeState::vector(PrimTy::Any, false)),
        "grDevices::chull" | "grDevices::dev.list" => Some(TypeState::vector(PrimTy::Int, false)),
        "grDevices::dev.set"
        | "grDevices::nclass.FD"
        | "grDevices::nclass.scott"
        | "grDevices::nclass.Sturges" => Some(TypeState::scalar(PrimTy::Int, false)),
        "grDevices::dev.interactive"
        | "grDevices::deviceIsInteractive"
        | "grDevices::is.raster" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "grDevices::blues9"
        | "grDevices::grey"
        | "grDevices::grey.colors"
        | "grDevices::grSoftVersion"
        | "grDevices::hcl"
        | "grDevices::hcl.pals"
        | "grDevices::colorspaces"
        | "grDevices::colours" => Some(TypeState::vector(PrimTy::Char, false)),
        "grDevices::trans3d"
        | "grDevices::xy.coords"
        | "grDevices::xyTable"
        | "grDevices::xyz.coords" => Some(TypeState::vector(PrimTy::Any, false)),
        "grDevices::bitmap"
        | "grDevices::cairo_pdf"
        | "grDevices::cairo_ps"
        | "grDevices::dev.control"
        | "grDevices::dev.copy"
        | "grDevices::dev.copy2eps"
        | "grDevices::dev.copy2pdf"
        | "grDevices::dev.flush"
        | "grDevices::dev.hold"
        | "grDevices::dev.new"
        | "grDevices::dev.print"
        | "grDevices::devAskNewPage"
        | "grDevices::dev2bitmap"
        | "grDevices::embedFonts"
        | "grDevices::embedGlyphs"
        | "grDevices::graphics.off"
        | "grDevices::pictex"
        | "grDevices::postscript"
        | "grDevices::quartz"
        | "grDevices::quartz.save"
        | "grDevices::recordGraphics"
        | "grDevices::replayPlot"
        | "grDevices::savePlot"
        | "grDevices::setEPS"
        | "grDevices::setGraphicsEventEnv"
        | "grDevices::setGraphicsEventHandlers"
        | "grDevices::setPS"
        | "grDevices::svg"
        | "grDevices::x11"
        | "grDevices::X11"
        | "grDevices::xfig" => Some(TypeState::null()),
        "graphics::legend" => Some(TypeState::vector(PrimTy::Any, false)),
        _ => None,
    }
}
