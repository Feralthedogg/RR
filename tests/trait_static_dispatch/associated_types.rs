use super::*;

#[test]
pub(crate) fn associated_type_substitutes_into_impl_signature() {
    let src = r#"
trait Measure {
  type Output
  fn value(self: Self) -> Self::Output
}

impl Measure for Body {
  type Output = float

  fn value(self: Body) -> float {
    self.mass
  }
}

fn main() {
  let b: Body = {mass: 2.0}
  Measure::value(b)
}

print(main())
"#;

    compile_with_config(
        "trait_associated_type_signature.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("associated type signature should compile");
}

#[test]
pub(crate) fn associated_type_projection_bound_monomorphizes_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping associated type projection runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait ScalarLabel {
  fn label(self: Self) -> str
}

impl ScalarLabel for float {
  fn label(self: float) -> str {
    "float"
  }
}

trait Container {
  type Item
  fn get(self: Self) -> Self::Item
}

impl Container for FloatBox {
  type Item = float

  fn get(self: FloatBox) -> float {
    self.value
  }
}

fn label_item<T>(x: T) -> str where T: Container, <T as Container>::Item: ScalarLabel {
  let item: <T as Container>::Item = x.get()
  item.label()
}

fn main() {
  let boxed: FloatBox = {value: 2.0}
  label_item(boxed)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_associated_type_projection_bound.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("associated type projection bound should compile");

    assert!(
        output.contains("return(\"float\")"),
        "associated type projection should materialize the concrete impl body:\n{output}"
    );
    assert!(
        !output.contains("item.label"),
        "projection-bound method call must not emit dynamic R dispatch:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_assoc_type_projection");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_assoc_type_projection.R");
    fs::write(&out_r, output).expect("write emitted R");

    let run = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&out_r)
        .output()
        .expect("run Rscript");
    assert!(
        run.status.success(),
        "R failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n");
    assert!(
        stdout.contains("[1] \"float\""),
        "unexpected R output:\n{}",
        stdout
    );
}

#[test]
pub(crate) fn fully_qualified_associated_type_projection_disambiguates_same_assoc_names() {
    let src = r#"
trait ScalarLabel {
  fn label(self: Self) -> str
}

impl ScalarLabel for float {
  fn label(self: float) -> str {
    "float"
  }
}

trait First {
  type Item
  fn get_first(self: Self) -> Self::Item
}

trait Second {
  type Item
}

impl First for FloatBox {
  type Item = float

  fn get_first(self: FloatBox) -> float {
    self.value
  }
}

impl Second for FloatBox {
  type Item = str
}

fn label_first<T>(x: T) -> str where T: First + Second, <T as First>::Item: ScalarLabel {
  let item: <T as First>::Item = x.get_first()
  item.label()
}

fn main() {
  let boxed: FloatBox = {value: 2.0}
  label_first(boxed)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_fully_qualified_projection_disambiguates.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("fully-qualified projection should pick First.Item without Second.Item ambiguity");

    assert!(
        output.contains("return(\"float\")") && !output.contains("item.label"),
        "qualified projection should monomorphize to the First.Item impl:\n{output}"
    );
}

#[test]
pub(crate) fn qualified_projection_through_supertrait_bound_ignores_sibling_assoc_alias() {
    let src = r#"
trait ScalarLabel {
  fn label(self: Self) -> str
}

impl ScalarLabel for float {
  fn label(self: float) -> str {
    "float"
  }
}

trait First {
  type Item
  fn get_first(self: Self) -> Self::Item
}

trait Second {
  type Item
}

trait Child: First + Second {
}

impl First for FloatBox {
  type Item = float

  fn get_first(self: FloatBox) -> float {
    self.value
  }
}

impl Second for FloatBox {
  type Item = str
}

impl Child for FloatBox {
}

fn label_first<T>(x: T) -> str where T: Child, <T as First>::Item: ScalarLabel {
  let item: <T as First>::Item = First::get_first(x)
  item.label()
}

fn main() {
  let boxed: FloatBox = {value: 2.0}
  label_first(boxed)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_supertrait_projection_disambiguates.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("qualified owner projection should not be blocked by sibling supertrait assoc names");

    assert!(
        output.contains("return(\"float\")") && !output.contains("item.label"),
        "owner-qualified supertrait projection should lower statically:\n{output}"
    );
}

#[test]
pub(crate) fn associated_type_projection_method_requires_projection_bound() {
    let src = r#"
trait ScalarLabel {
  fn label(self: Self) -> str
}

trait Container {
  type Item
  fn get(self: Self) -> Self::Item
}

fn label_item<T>(x: T) -> str where T: Container {
  let item: <T as Container>::Item = x.get()
  item.label()
}
"#;

    let err = compile_with_config(
        "trait_associated_type_projection_missing_bound.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("projection receiver method should require a projection bound");

    assert!(
        err.message
            .contains("generic receiver type '<T as Container>::Item' uses method 'label'"),
        "unexpected error: {err:?}"
    );
}

#[test]
pub(crate) fn associated_type_projection_bound_is_checked_at_call_site() {
    let src = r#"
trait ScalarLabel {
  fn label(self: Self) -> str
}

trait Container {
  type Item
  fn get(self: Self) -> Self::Item
}

impl Container for RockBox {
  type Item = Rock

  fn get(self: RockBox) -> Rock {
    self.value
  }
}

fn label_item<T>(x: T) -> str where T: Container, <T as Container>::Item: ScalarLabel {
  let item: <T as Container>::Item = x.get()
  item.label()
}

fn main() {
  let boxed: RockBox = {value: {mass: 2.0}}
  label_item(boxed)
}
"#;

    let err = compile_with_config(
        "trait_associated_type_projection_unsatisfied.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("projection bound should be validated for the concrete associated type");

    assert!(
        err.message.contains(
            "generic bound '<T as Container>::Item' requires trait 'ScalarLabel' for 'Rock'"
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
pub(crate) fn associated_const_dispatches_statically_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping associated const runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Scale {
  const FACTOR: float
}

impl Scale for Body {
  const FACTOR: float = 2.5
}

fn main() {
  Scale::FACTOR::<Body>() + 0.5
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_associated_const_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("associated const should compile");

    assert!(
        !output.contains("Scale::FACTOR") && !output.contains("Scale.FACTOR"),
        "associated const selection must lower statically:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("trait_assoc_const");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_assoc_const.R");
    fs::write(&out_r, output).expect("write emitted R");

    let run = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&out_r)
        .output()
        .expect("run Rscript");
    assert!(
        run.status.success(),
        "R failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n");
    assert!(stdout.contains("[1] 3"), "unexpected R output:\n{stdout}");
}

#[test]
pub(crate) fn associated_const_default_materializes_for_impl() {
    let src = r#"
trait Scale {
  const FACTOR: float = 2.0
}

impl Scale for Body {
}

fn main() {
  Scale::FACTOR::<Body>()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_associated_const_default.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("default associated const should compile");

    assert!(
        !output.contains("Scale::FACTOR") && output.contains("2.0"),
        "default associated const must materialize as a concrete helper:\n{output}"
    );
}

#[test]
pub(crate) fn associated_const_must_be_provided_without_default() {
    let src = r#"
trait Scale {
  const FACTOR: float
}

impl Scale for Body {
}

fn main() {
  Scale::FACTOR::<Body>()
}
"#;

    let err = compile_with_config(
        "trait_associated_const_missing.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("impl missing associated const must fail");

    assert!(
        err.message.contains("missing associated const 'FACTOR'"),
        "unexpected error: {err:?}"
    );
}

#[test]
pub(crate) fn generic_associated_const_uses_bound_and_monomorphizes() {
    let src = r#"
trait Scale {
  const FACTOR: float
}

impl Scale for Body {
  const FACTOR: float = 4.0
}

fn scale_factor<T>() -> float where T: Scale {
  Scale::FACTOR::<T>()
}

fn main() {
  scale_factor::<Body>()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_generic_associated_const.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("generic associated const should monomorphize");

    assert!(
        !output.contains("Scale::FACTOR") && !output.contains("Scale.FACTOR"),
        "generic associated const must lower through the concrete specialization:\n{output}"
    );
}

#[test]
pub(crate) fn associated_type_must_be_provided_by_impl() {
    let src = r#"
trait Measure {
  type Output
  fn value(self: Self) -> Self::Output
}

impl Measure for Body {
  fn value(self: Body) -> float {
    self.mass
  }
}

fn main() {
  let b: Body = {mass: 2.0}
  Measure::value(b)
}
"#;

    let err = compile_with_config(
        "trait_associated_type_missing.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("impl missing associated type must fail");

    assert!(
        err.message.contains("missing associated type 'Output'"),
        "unexpected error: {err:?}"
    );
}

#[test]
pub(crate) fn supertrait_bound_must_be_satisfied_for_dispatch() {
    let src = r#"
trait Eqish {
  fn eqish(self: Self) -> bool
}

trait Ordish: Eqish {
  fn score(self: Self) -> float
}

impl Ordish for Body {
  fn score(self: Body) -> float {
    self.mass
  }
}

fn main() {
  let b: Body = {mass: 2.0}
  Ordish::score(b)
}
"#;

    let err = compile_with_config(
        "trait_supertrait_missing_impl.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("supertrait obligation must be enforced at dispatch");

    assert!(
        err.message.contains("no impl of trait 'Ordish'"),
        "unexpected error: {err:?}"
    );
}

#[test]
pub(crate) fn repeated_type_param_impl_does_not_overlap_inconsistent_exact_impl() {
    let src = r#"
trait Label {
  fn label(self: Self) -> str
}

impl<T> Label for Pair<T,T> {
  fn label(self: Pair<T,T>) -> str {
    "same"
  }
}

impl Label for Pair<int,float> {
  fn label(self: Pair<int,float>) -> str {
    "mixed"
  }
}

fn main() {
  let p: Pair<int,float> = {left: 1, right: 2.0}
  p.label()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_repeated_type_param_overlap.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("inconsistent exact impl should not overlap Pair<T,T>");

    assert!(
        output.contains("\"mixed\"") && !output.contains("\"same\""),
        "exact mixed impl should be selected without overlap false positive:\n{output}"
    );
}

#[test]
pub(crate) fn exact_impl_specializes_generic_impl_for_static_dispatch() {
    let src = r#"
trait Show {
  fn show(self: Self) -> str
}

impl<T> Show for Box<T> {
  fn show(self: Box<T>) -> str {
    "generic"
  }
}

impl Show for Box<Body> {
fn show(self: Box<Body>) -> str {
    "body"
  }
}

fn main() {
  let b: Body = {mass: 1.0}
  let boxed: Box<Body> = {value: b}
  boxed.show()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_specialized_impl.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("exact impl should specialize generic impl");

    assert!(
        output.contains("\"body\"") && !output.contains("\"generic\""),
        "exact impl should be selected without instantiating the generic impl:\n{output}"
    );
}

#[test]
pub(crate) fn negative_impl_blocks_blanket_generic_impl() {
    let src = r#"
trait Show {
  fn show(self: Self) -> str
}

impl<T> Show for Box<T> {
  fn show(self: Box<T>) -> str {
    "generic"
  }
}

impl !Show for Box<Rock> {
}

fn main() {
  let rock: Rock = {mass: 1.0}
  let boxed: Box<Rock> = {value: rock}
  boxed.show()
}
"#;

    let err = compile_with_config(
        "trait_negative_impl_blocks_blanket.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("negative impl must block blanket impl dispatch");

    assert!(
        err.message
            .contains("negative impl explicitly prevents trait 'Show' for 'Box<Rock>'"),
        "unexpected error: {err:?}"
    );
}

#[test]
pub(crate) fn supertrait_where_bound_satisfies_parent_trait_call() {
    let src = r#"
trait Eqish {
  fn eqish(self: Self) -> bool
}

trait Ordish: Eqish {
  fn score(self: Self) -> float
}

impl Eqish for Body {
  fn eqish(self: Body) -> bool {
    true
  }
}

impl Ordish for Body {
  fn score(self: Body) -> float {
    self.mass
  }
}

fn check<T>(x: T) -> bool where T: Ordish {
  Eqish::eqish(x)
}

fn main() {
  let b: Body = {mass: 1.0}
  check(b)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_supertrait_where_bound.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("supertrait where bound should satisfy parent trait call");

    assert!(
        !output.contains("Eqish::eqish"),
        "supertrait-bound trait call must lower statically:\n{output}"
    );
}

#[test]
pub(crate) fn index_operator_trait_dispatches_statically() {
    let src = r#"
trait Index {
  fn index(self: Self, i: int) -> float
}

impl Index for VecBox {
  fn index(self: VecBox, i: int) -> float {
    self.values[i]
  }
}

fn main() {
  let v: VecBox = {values: c(10.0, 20.0)}
  v[2L]
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_index_operator_dispatch.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("Index trait should lower bracket syntax");

    assert!(
        !output.contains("v[2"),
        "Index trait dispatch should replace source bracket call:\n{output}"
    );
}
