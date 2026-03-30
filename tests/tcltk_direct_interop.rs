mod common;

use common::run_compile_case;

#[test]
fn tcltk_direct_surface_compiles_without_opaque_warning() {
    let src = r#"
import r default from "tcltk"

fn marker() -> string {
  let obj = tcltk.tclObj("alpha")
  let obj2 = tcltk.as.tclObj("beta")
  let var = tcltk.tclVar("gamma")
  let arr = tcltk.tclArray()
  let svc = tcltk.tclServiceMode()
  let raw = tcltk.tcl("set", "x", "1")
  let value = tcltk.tclvalue(obj)
  let added = tcltk.addTclPath(".")
  let req = tcltk.tclRequire("Tcl")
  let ver = tcltk.tclVersion()
  let btn = tcltk.tkbutton()
  let cvs = tcltk.tkcanvas()
  let frm = tcltk.tkframe()
  let lbl = tcltk.tklabel()
  let men = tcltk.tkmenu()
  let msg = tcltk.tkmessage()
  let scale = tcltk.tkscale()
  let scroll = tcltk.tkscrollbar()
  let top = tcltk.tktoplevel()
  let chooser = tcltk.tkchooseDirectory()
  let openf = tcltk.tkgetOpenFile()
  let savef = tcltk.tkgetSaveFile()
  let mbox = tcltk.tkmessageBox()
  let grid = tcltk.tkgrid(frm)
  let pack = tcltk.tkpack(btn)
  let place = tcltk.tkplace(lbl)
  let ttkb = tcltk.ttkbutton()
  let ttkf = tcltk.ttkframe()
  let ttkl = tcltk.ttklabel()
  let ttkn = tcltk.ttknotebook()
  let ttkp = tcltk.ttkprogressbar()
  let ttkt = tcltk.ttktreeview()
  let pb = tcltk.tkProgressBar()
  let prev = tcltk.getTkProgressBar(pb)
  let before = tcltk.setTkProgressBar(pb, 1.0)
  let ok = tcltk.is.tclObj(obj2)
  let win = tcltk.is.tkwin(obj)
  let dir = tcltk.tclfile.dir("/tmp/demo.txt")
  let tail = tcltk.tclfile.tail("/tmp/demo.txt")
  print(arr)
  print(svc)
  print(raw)
  print(value)
  print(added)
  print(req)
  print(ver)
  print(btn)
  print(cvs)
  print(frm)
  print(lbl)
  print(men)
  print(msg)
  print(scale)
  print(scroll)
  print(top)
  print(chooser)
  print(openf)
  print(savef)
  print(mbox)
  print(grid)
  print(pack)
  print(place)
  print(ttkb)
  print(ttkf)
  print(ttkl)
  print(ttkn)
  print(ttkp)
  print(ttkt)
  print(prev)
  print(before)
  print(ok)
  print(win)
  print(dir)
  print(tail)
  return tail
}

print(marker())
"#;

    let (ok, stdout, stderr) = run_compile_case("tcltk_direct_interop", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "tcltk helper batch should stay on direct surface, got stderr:\n{stderr}"
    );
}
