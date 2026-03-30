mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn methods_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping methods direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("methods_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "methods"
import r { globalenv } from "base"

fn rr_tmp_dispatch(obj) {
  return methods.standardGeneric("rr_tmp_generic")
}

fn rr_tmp_impl(obj) {
  return 1.0
}

fn use_methods() -> int {
  let class_ok = methods.isClass("MethodDefinition")
  let generic_ok = methods.isGeneric("show")
  let method_ok = methods.hasMethod("show", "ANY")
  let exists_ok = methods.existsMethod("show", "ANY")
  let class_def = methods.getClass("MethodDefinition")
  let class_def2 = methods.getClassDef("MethodDefinition")
  let classes = methods.getClasses()
  let fun_obj = methods.getFunction("show")
  let load_actions = methods.getLoadActions(where = globalenv())
  let package_name = methods.getPackageName(where = globalenv())
  let slots2 = methods.getSlots("MethodDefinition")
  let generic_obj = methods.getGeneric("show")
  let all_generics = methods.getGenerics()
  let group = methods.getGroup("Arith")
  let members = methods.getGroupMembers("Arith")
  let formals = methods.formalArgs(generic_obj)
  let supers2 = methods.getAllSuperClasses(class_def)
  let sealed_class = methods.isSealedClass("MethodDefinition")
  let sealed_method = methods.isSealedMethod("show", "ANY")
  let class_def_ok = methods.isClassDef(class_def)
  let exists_fun = methods.existsFunction("show")
  let has_load_action = methods.hasLoadAction("missing-action", where = globalenv())
  let has_arg_now = methods.hasArg("foo")
  let found_fun = methods.findFunction("show")
  let has_methods = methods.hasMethods("show")
  let group_ok = methods.isGroup("Arith")
  let grammar_ok = methods.isGrammarSymbol("if")
  let rematched_ok = methods.isRematched("show")
  let xs3_ok = methods.isXS3Class(methods.getClass("oldClass"))
  let adjacency = methods.classesToAM("MethodDefinition")
  let signatures = methods.findMethodSignatures("show")
  let cache_ok = methods.cacheMetaData(where = globalenv())
  let found_class = methods.findClass("MethodDefinition")
  let unique_fun = methods.findUnique("show", "rr probe", where = globalenv())
  let ref_class = methods.getRefClass("envRefClass")
  let inherited_report = methods.testInheritedMethods("show")
  let validity_obj = methods.getValidity(class_def)
  let numeric_wrap_gen = methods.setClass("RRTmpNumericWrap", contains = "numeric")
  let wrapped_numeric = methods.new("RRTmpNumericWrap", 1.0)
  let data_part = methods.getDataPart(wrapped_numeric)
  let created = methods.new("MethodDefinition")
  let method_obj = methods.getMethod("show", "ANY")
  let cached_method = methods.cacheMethod("show", list("ANY"), method_obj, fdef = generic_obj)
  let found_method = methods.findMethod("show", "ANY")
  let dispatch_methods = methods.getMethodsForDispatch(generic_obj)
  let selected = methods.selectMethod("show", "ANY")
  let shown = methods.show(class_def)
  let class_gen = methods.setClass("RRTmpClass", slots = c(x = "numeric"))
  let created_s4 = methods.new("RRTmpClass", x = 1.0)
  let is_instance = methods.is(created_s4, "RRTmpClass")
  let slot_value = methods.slot(created_s4, "x")
  let valid_ok = methods.validObject(created_s4)
  let virtual_ok = methods.isVirtualClass("RRTmpClass")
  let union_ok = methods.isClassUnion("numeric")
  let test_virtual = methods.testVirtual(character(), NULL, NULL, globalenv())
  let can_coerce = methods.canCoerce("numeric", "character")
  let generic_name = methods.setGeneric("rr_tmp_generic", rr_tmp_dispatch)
  let method_name = methods.setMethod("rr_tmp_generic", "RRTmpClass", rr_tmp_impl)
  let supers = methods.extends("RRTmpClass")
  let slots = methods.slotNames(class_def)
  let found = methods.findMethods("show")
  print(class_ok)
  print(generic_ok)
  print(method_ok)
  print(exists_ok)
  print(length(class_def))
  print(length(class_def2))
  print(length(classes))
  print(length(fun_obj))
  print(length(load_actions))
  print(length(package_name))
  print(length(slots2))
  print(length(generic_obj))
  print(length(all_generics))
  print(length(group))
  print(length(members))
  print(length(formals))
  print(length(supers2))
  print(sealed_class)
  print(sealed_method)
  print(class_def_ok)
  print(exists_fun)
  print(has_load_action)
  print(has_arg_now)
  print(length(found_fun))
  print(has_methods)
  print(group_ok)
  print(grammar_ok)
  print(rematched_ok)
  print(xs3_ok)
  print(length(adjacency))
  print(length(signatures))
  print(length(cache_ok))
  print(length(found_class))
  print(length(unique_fun))
  print(length(ref_class))
  print(length(cached_method))
  print(length(inherited_report))
  print(length(validity_obj))
  print(length(numeric_wrap_gen))
  print(length(wrapped_numeric))
  print(length(data_part))
  print(length(created))
  print(length(method_obj))
  print(length(found_method))
  print(length(dispatch_methods))
  print(length(selected))
  print(length(shown))
  print(length(class_gen))
  print(length(created_s4))
  print(is_instance)
  print(length(slot_value))
  print(valid_ok)
  print(virtual_ok)
  print(union_ok)
  print(test_virtual)
  print(can_coerce)
  print(length(generic_name))
  print(length(method_name))
  print(length(supers))
  print(length(slots))
  print(length(found))
  return length(slots)
}

print(use_methods())
"#;

    let rr_path = out_dir.join("methods_direct_interop.rr");
    let o0 = out_dir.join("methods_direct_interop_o0.R");
    let o2 = out_dir.join("methods_direct_interop_o2.R");

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
}

#[test]
fn methods_standard_generic_stays_on_direct_surface() {
    let src = r#"
import r default from "methods"

fn rr_tmp_dispatch(obj) {
  return methods.standardGeneric("rr_tmp_generic")
}

fn rr_tmp_impl(obj) {
  return 1.0
}

fn build_methods() -> int {
  let generic_name = methods.setGeneric("rr_tmp_generic", rr_tmp_dispatch)
  let method_name = methods.setMethod("rr_tmp_generic", "ANY", rr_tmp_impl)
  print(length(generic_name))
  print(length(method_name))
  return 0
}

print(build_methods())
"#;

    let (ok, stdout, stderr) = run_compile_case(
        "methods_standard_generic_direct_surface",
        src,
        "methods_standard_generic_direct_surface.rr",
        "-O1",
        &[],
    );

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "methods::standardGeneric should stay on the direct surface, got stderr:\n{stderr}"
    );
}
