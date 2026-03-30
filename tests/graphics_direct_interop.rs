mod common;

use common::{normalize, rscript_available, rscript_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn graphics_and_grdevices_direct_surface_match_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping graphics direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("graphics_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"
import r { plot, lines, points, abline, title, box, text, axis, axTicks, strwidth, strheight, grconvertX, grconvertY, clip, xspline, pie, symbols, smoothScatter, stem, segments, arrows, mtext, rug, polygon, hist, boxplot, par, layout, layout.show, matplot, matlines, matpoints, pairs, stripchart, dotchart, contour, image, persp, assocplot, mosaicplot, fourfoldplot, legend } from "graphics"
import r default from "grDevices"

fn render_plot() -> int {
  let outfile = "graphics_direct_interop.png"
  let jpg_out = "graphics_direct_interop.jpg"
  let bmp_out = "graphics_direct_interop.bmp"
  let tiff_out = "graphics_direct_interop.tiff"
  grDevices.png(filename = outfile, width = 320, height = 240)
  plot(c(1.0, 2.0, 3.0), c(1.0, 4.0, 9.0), type = "l")
  lines(c(1.0, 2.0, 3.0), c(1.0, 2.0, 3.0), col = "tomato")
  points(c(1.0, 2.0, 3.0), c(1.0, 4.0, 9.0), pch = 19, col = "navy")
  abline(a = 0.0, b = 1.0, lty = 2)
  title(main = "graphics helpers")
  box()
  text(2.0, 4.0, "peak")
  segments(1.0, 1.0, 3.0, 9.0, col = "gray40")
  arrows(1.0, 9.0, 3.0, 1.0, length = 0.08)
  mtext("outer note")
  let rugs = rug(c(1.0, 2.0, 3.0))
  polygon(c(1.0, 2.0, 3.0), c(1.0, 3.0, 1.0), border = "steelblue")
  let ticks = axis(1)
  let converted_x = grconvertX(c(0.1, 0.9), from = "nfc", to = "user")
  let converted_y = grconvertY(c(0.1, 0.9), from = "nfc", to = "user")
  clip(1.0, 3.0, 1.0, 9.0)
  xspline(c(1.0, 2.0, 3.0), c(1.0, 2.0, 1.0), open = true)
  let placement = legend(
    "topright",
    legend = c("signal", "trend"),
    col = c("black", "tomato"),
    lty = c(1, 1)
  )
  let hist_obj = hist(c(1.0, 2.0, 2.0, 3.0), plot = false)
  let box_obj = boxplot(c(1.0, 2.0, 3.0), plot = false)
  let par_obj = par()
  let layout_id = layout(matrix(c(1, 2), 1, 2))
  layout.show(1)
  let mat = matrix(c(1.0, 2.0, 3.0, 2.0, 3.0, 4.0), 3, 2)
  matplot(mat, type = "l")
  matlines(mat)
  matpoints(mat)
  pairs(mat)
  let strips = stripchart(c(1.0, 2.0, 3.0))
  dotchart(c(1.0, 2.0, 3.0))
  let z = matrix(c(1.0, 2.0, 3.0, 2.0, 3.0, 4.0, 3.0, 4.0, 5.0), 3, 3)
  contour(z)
  image(z)
  let surface = persp(z)
  assocplot(matrix(c(10.0, 5.0, 6.0, 9.0), 2, 2))
  mosaicplot(matrix(c(10.0, 5.0, 6.0, 9.0), 2, 2))
  fourfoldplot(matrix(c(10.0, 5.0, 6.0, 9.0), 2, 2))
  let ticks2 = axTicks(1)
  let widths = strwidth(c("a", "bb"))
  let heights = strheight(c("a", "bb"))
  pie(c(1.0, 2.0, 3.0))
  symbols(c(1.0, 2.0, 3.0), c(1.0, 2.0, 3.0), circles = c(1.0, 2.0, 3.0), inches = false)
  let has_kernsmooth = base.requireNamespace("KernSmooth", quietly = true)
  if (has_kernsmooth) {
    smoothScatter(c(1.0, 2.0, 3.0), c(1.0, 2.0, 3.0))
  }
  stem(c(1.1, 1.2, 2.3))
  print(length(rugs))
  print(length(ticks))
  print(converted_x)
  print(converted_y)
  print(length(ticks2))
  print(length(placement))
  print(length(hist_obj))
  print(length(box_obj))
  print(length(par_obj))
  print(layout_id)
  print(length(strips))
  print(length(surface))
  print(length(widths))
  print(length(heights))
  let closed_png = grDevices.dev.off()

  print(grDevices.dev.size())

  print(grDevices.rgb(1.0, 0.0, 0.0))
  print(grDevices.hsv(0.0, 1.0, 1.0))
  print(grDevices.gray(0.5))
  print(grDevices.gray.colors(4))
  print(grDevices.hcl.colors(4))
  print(grDevices.colors())
  print(grDevices.heat.colors(4))
  print(grDevices.terrain.colors(4))
  print(grDevices.topo.colors(4))
  print(grDevices.cm.colors(4))
  print(grDevices.rainbow(4))
  print(grDevices.palette.colors())
  print(grDevices.palette.pals())
  print(grDevices.palette())
  print(grDevices.n2mfrow(5))
  print(grDevices.densCols(c(1.0, 2.0, 3.0), c(1.0, 2.0, 3.0)))
  print(grDevices.adjustcolor("red", 0.5))
  let rast = grDevices.as.raster(matrix(c("red", "blue", "green", "black"), nrow = 2L))
  print(dim(rast))
  print(grDevices.is.raster(rast))
  print(grDevices.axisTicks(c(0.0, 1.0), log = false, nint = 5L))
  let bp_stats = grDevices.boxplot.stats(c(1.0, 2.0, 3.0, 100.0))
  print(bp_stats.stats)
  print(bp_stats.n)
  print(grDevices.chull(c(0.0, 1.0, 1.0, 0.0), c(0.0, 0.0, 1.0, 1.0)))
  let contours = grDevices.contourLines(z = matrix(c(1.0, 2.0, 2.0, 3.0), nrow = 2L))
  print(length(contours))
  let caps = grDevices.dev.capabilities()
  print(length(caps))
  print(grDevices.extendrange(c(1.0, 3.0)))
  print(grDevices.hcl(0.0, 100.0, 65.0))
  print(length(grDevices.hcl.pals()))
  print(grDevices.grey(0.5))
  print(grDevices.grey.colors(4L))
  print(grDevices.nclass.FD(c(1.0, 2.0, 3.0, 4.0, 5.0)))
  print(grDevices.nclass.scott(c(1.0, 2.0, 3.0, 4.0, 5.0)))
  print(grDevices.nclass.Sturges(c(1.0, 2.0, 3.0, 4.0, 5.0)))
  grDevices.pdf(file = NULL)
  plot(c(1.0, 2.0, 3.0), c(1.0, 2.0, 3.0))
  print(grDevices.dev.list())
  print(grDevices.dev.interactive())
  print(grDevices.deviceIsInteractive("pdf"))
  print(grDevices.dev.set(grDevices.dev.cur()))
  let closed_null_pdf = grDevices.dev.off()
  print(closed_null_pdf)
  let td = grDevices.trans3d(c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0), diag(4L))
  print(length(td))
  let xyc = grDevices.xy.coords(c(1.0, 2.0, 3.0), c(4.0, 5.0, 6.0))
  print(length(xyc))
  let xyt = grDevices.xyTable(c(1.0, 1.0, 2.0), c(3.0, 3.0, 4.0))
  print(length(xyt))
  let xyz = grDevices.xyz.coords(c(1.0, 2.0), c(3.0, 4.0), c(5.0, 6.0))
  print(length(xyz))
  print(dim(grDevices.col2rgb(c("red", "blue"))))
  print(dim(grDevices.rgb2hsv(grDevices.col2rgb(c("red", "blue")))))
  print(dim(grDevices.convertColor(
    matrix(c(1.0, 0.0, 0.0, 0.0, 1.0, 0.0), ncol = 3L, byrow = true),
    from = "sRGB",
    to = "Lab"
  )))

  grDevices.jpeg(filename = jpg_out, width = 240, height = 180)
  plot(c(1.0, 2.0), c(1.0, 2.0), type = "p")
  let closed_jpg = grDevices.dev.off()

  grDevices.bmp(filename = bmp_out, width = 240, height = 180)
  plot(c(1.0, 2.0), c(2.0, 1.0), type = "p")
  let closed_bmp = grDevices.dev.off()

  grDevices.tiff(filename = tiff_out, width = 240, height = 180)
  plot(c(1.0, 2.0), c(1.5, 1.5), type = "p")
  let closed_tiff = grDevices.dev.off()

  print(closed_jpg)
  print(closed_bmp)
  print(closed_tiff)
  return closed_png
}

print(render_plot())
"#;

    let rr_path = out_dir.join("graphics_direct_interop.rr");
    let o0 = out_dir.join("graphics_direct_interop_o0.R");
    let o2 = out_dir.join("graphics_direct_interop_o2.R");

    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o0)
        .arg("-O0")
        .status()
        .expect("failed to compile O0");
    assert!(status.success(), "O0 compile failed");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o2)
        .arg("-O2")
        .status()
        .expect("failed to compile O2");
    assert!(status.success(), "O2 compile failed");

    let run_o0 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o0)
        .output()
        .expect("failed to execute O0 Rscript");
    let run_o2 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o2)
        .output()
        .expect("failed to execute O2 Rscript");

    assert_eq!(
        run_o0.status.code().unwrap_or(-1),
        0,
        "O0 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o0.stderr)
    );
    assert_eq!(
        run_o2.status.code().unwrap_or(-1),
        0,
        "O2 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o2.stderr)
    );

    let stdout_o0 = normalize(&String::from_utf8_lossy(&run_o0.stdout));
    let stdout_o2 = normalize(&String::from_utf8_lossy(&run_o2.stdout));
    let stderr_o0 = normalize(&String::from_utf8_lossy(&run_o0.stderr));
    let stderr_o2 = normalize(&String::from_utf8_lossy(&run_o2.stderr));

    assert_eq!(stdout_o0, stdout_o2, "stdout mismatch O0 vs O2");
    assert_eq!(stderr_o0, stderr_o2, "stderr mismatch O0 vs O2");

    let png_path = out_dir.join("graphics_direct_interop.png");
    let meta = fs::metadata(&png_path).expect("expected graphics PNG output");
    assert!(meta.len() > 0, "expected non-empty graphics PNG output");
    for ext in ["jpg", "bmp", "tiff"] {
        let path = out_dir.join(format!("graphics_direct_interop.{ext}"));
        let meta = fs::metadata(&path).expect("expected grDevices output");
        assert!(meta.len() > 0, "expected non-empty {ext} output");
    }
}
