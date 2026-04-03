use crate::typeck::builtin_sigs::vectorized_scalar_or_vector_double_term;
use crate::typeck::term::TypeTerm;

pub(super) fn infer_grdevices_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    match callee {
        "grDevices::jpeg" | "grDevices::bmp" | "grDevices::tiff" => Some(TypeTerm::Null),
        "grDevices::dev.size" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "grDevices::dev.off"
        | "grDevices::dev.cur"
        | "grDevices::dev.next"
        | "grDevices::dev.prev" => Some(TypeTerm::Int),
        "graphics::axis" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "graphics::identify" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "graphics::locator" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "graphics::rug" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
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
        | "grDevices::densCols" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "grDevices::n2mfrow" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "grDevices::col2rgb" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Int))),
        "grDevices::rgb2hsv" | "grDevices::convertColor" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "grDevices::as.raster" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "grDevices::axisTicks" | "grDevices::extendrange" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "grDevices::cm" => vectorized_scalar_or_vector_double_term(arg_terms),
        "grDevices::boxplot.stats" => Some(TypeTerm::NamedList(vec![
            (
                "stats".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n".to_string(), TypeTerm::Int),
            (
                "conf".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "out".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "grDevices::chull" | "grDevices::dev.list" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Int)))
        }
        "grDevices::dev.set"
        | "grDevices::nclass.FD"
        | "grDevices::nclass.scott"
        | "grDevices::nclass.Sturges" => Some(TypeTerm::Int),
        "grDevices::dev.interactive"
        | "grDevices::deviceIsInteractive"
        | "grDevices::is.raster" => Some(TypeTerm::Logical),
        "grDevices::blues9"
        | "grDevices::grey"
        | "grDevices::grey.colors"
        | "grDevices::grSoftVersion"
        | "grDevices::hcl"
        | "grDevices::hcl.pals"
        | "grDevices::colorspaces"
        | "grDevices::colours" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "grDevices::contourLines"
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
        | "grDevices::.useGroup" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "grDevices::trans3d" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "grDevices::xy.coords" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("xlab".to_string(), TypeTerm::Any),
            ("ylab".to_string(), TypeTerm::Any),
        ])),
        "grDevices::xyTable" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "number".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
        ])),
        "grDevices::xyz.coords" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "z".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("xlab".to_string(), TypeTerm::Any),
            ("ylab".to_string(), TypeTerm::Any),
            ("zlab".to_string(), TypeTerm::Any),
        ])),
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
        | "grDevices::xfig" => Some(TypeTerm::Null),
        "graphics::legend" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        _ => None,
    }
}
