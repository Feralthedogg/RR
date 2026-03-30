mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn grid_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping grid direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("grid_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let pdf_path = out_dir.join("grid_direct_interop.pdf");

    let src = format!(
        r#"
import r default from "grid"
import r default from "grDevices"

fn render_grid() -> int {{
  let outfile = "{outfile}"
  let unit_obj = grid.unit(1.0, "npc")
  let gp = grid.gpar(col = "red")
  let layout_obj = grid.grid.layout(2, 2)
  let vp = grid.viewport(width = unit_obj, height = unit_obj, gp = gp)
  let named_vp = grid.viewport(name = "rrvp", layout = layout_obj)
  let vp_stack = grid.vpStack(grid.viewport(), grid.viewport())
  let vp_list = grid.vpList(grid.viewport(), grid.viewport())
  let data_vp = grid.dataViewport(xData = c(1.0, 2.0, 3.0), yData = c(4.0, 5.0, 6.0))
  let circle = grid.circleGrob(gp = gp, vp = vp)
  let segs = grid.segmentsGrob(vp = vp)
  let pts = grid.pointsGrob(x = c(0.2, 0.8), y = c(0.2, 0.8), vp = vp)
  let ras = grid.rasterGrob(matrix(c("red", "blue", "blue", "red"), 2, 2), vp = vp)
  let poly = grid.polygonGrob()
  let pline = grid.polylineGrob()
  let xspline = grid.xsplineGrob(c(0.2, 0.5, 0.8), c(0.2, 0.8, 0.2))
  let frame = grid.frameGrob(layout = grid.grid.layout(1, 1))
  let packed = grid.packGrob(frame, grid.rectGrob(), row = 1, col = 1)
  let placed = grid.placeGrob(frame, grid.textGrob("placed"), row = 1, col = 1)
  let roundrect = grid.roundrectGrob()
  let line_grob = grid.linesGrob()
  let curve = grid.curveGrob(0.1, 0.1, 0.9, 0.9)
  let null_grob = grid.nullGrob()
  let bezier = grid.bezierGrob(x = c(0.1, 0.4, 0.6, 0.9), y = c(0.1, 0.9, 0.1, 0.9))
  let path = grid.pathGrob(x = c(0.1, 0.9, 0.9, 0.1), y = c(0.1, 0.1, 0.9, 0.9))
  let width = grid.grobWidth(grid.textGrob("measure"))
  let height = grid.grobHeight(grid.textGrob("measure"))
  let gl = grid.gList(circle, segs, pts, poly, pline, xspline, line_grob)
  let grob = grid.grobTree(
    grid.rectGrob(gp = gp, vp = vp),
    grid.textGrob("hi", vp = vp),
    circle,
    segs,
    pts,
    ras,
    poly,
    pline,
    xspline,
    frame,
    roundrect,
    line_grob,
    curve,
    null_grob,
    bezier,
    path
  )
  grDevices.pdf(file = outfile, width = 4.0, height = 3.0)
  grid.grid.newpage()
  let pushed = grid.pushViewport(named_vp)
  let drawn_frame = grid.grid.frame(name = "fg", layout = grid.grid.layout(1, 1))
  let packed_drawn = grid.grid.pack("fg", grid.rectGrob(gp = gp), row = 1, col = 1)
  let placed_drawn = grid.grid.place("fg", grid.textGrob("placed"), row = 1, col = 1)
  let current_vp = grid.current.viewport()
  let seek_result = grid.seekViewport("rrvp")
  let up_path = grid.upViewport(0)
  let popped = grid.popViewport(0)
  let drawn_curve = grid.grid.curve(0.1, 0.1, 0.9, 0.9)
  let drawn_bezier = grid.grid.bezier(x = c(0.1, 0.4, 0.6, 0.9), y = c(0.1, 0.9, 0.1, 0.9))
  let drawn_path = grid.grid.path(x = c(0.1, 0.9, 0.9, 0.1), y = c(0.1, 0.1, 0.9, 0.9))
  let drawn_circle = grid.grid.circle(gp = gp)
  let drawn_points = grid.grid.points(x = c(0.2, 0.8), y = c(0.2, 0.8), gp = gp)
  let drawn_lines = grid.grid.lines(gp = gp)
  let drawn_segments = grid.grid.segments(gp = gp)
  let drawn_polygon = grid.grid.polygon(gp = gp)
  let drawn_polyline = grid.grid.polyline()
  let drawn_raster = grid.grid.raster(matrix(c("red", "blue", "blue", "red"), 2, 2))
  let drawn_rect = grid.grid.rect(gp = gp)
  let drawn_text = grid.grid.text("caption", gp = gp)
  let drawn = grid.grid.draw(grob)
  print(length(unit_obj))
  print(length(gp))
  print(length(layout_obj))
  print(length(vp_stack))
  print(length(vp_list))
  print(length(data_vp))
  print(length(vp))
  print(length(pushed))
  print(length(drawn_frame))
  print(length(packed_drawn))
  print(length(placed_drawn))
  print(length(current_vp))
  print(seek_result)
  print(length(up_path))
  print(length(popped))
  print(length(circle))
  print(length(segs))
  print(length(pts))
  print(length(ras))
  print(length(poly))
  print(length(pline))
  print(length(xspline))
  print(length(frame))
  print(length(packed))
  print(length(placed))
  print(length(roundrect))
  print(length(line_grob))
  print(length(curve))
  print(length(null_grob))
  print(length(bezier))
  print(length(path))
  print(length(width))
  print(length(height))
  print(length(gl))
  print(length(grob))
  print(length(drawn_curve))
  print(length(drawn_bezier))
  print(length(drawn_path))
  print(length(drawn_circle))
  print(length(drawn_points))
  print(length(drawn_lines))
  print(length(drawn_segments))
  print(length(drawn_polygon))
  print(length(drawn_polyline))
  print(length(drawn_raster))
  print(length(drawn_rect))
  print(length(drawn_text))
  print(length(drawn))
  return grDevices.dev.off()
}}

print(render_grid())
"#,
        outfile = pdf_path.display()
    );

    let rr_path = out_dir.join("grid_direct_interop.rr");
    let o0 = out_dir.join("grid_direct_interop_o0.R");
    let o2 = out_dir.join("grid_direct_interop_o2.R");

    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let run_o0 = run_rscript(&rscript, &o0);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(run_o0.status, 0, "O0 runtime failed:\n{}", run_o0.stderr);
    assert_eq!(run_o2.status, 0, "O2 runtime failed:\n{}", run_o2.stderr);
    assert_eq!(
        normalize(&run_o0.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&run_o0.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch O0 vs O2"
    );

    let meta = fs::metadata(&pdf_path).expect("expected grid PDF output");
    assert!(meta.len() > 0, "expected non-empty grid PDF output");
}
