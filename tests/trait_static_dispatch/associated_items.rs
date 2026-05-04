use super::*;

#[test]
pub(crate) fn ufcs_double_colon_trait_call_dispatches_statically() {
    let src = r#"
trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass
  }
}

fn main() {
  let b: Body = {mass: 2.0}
  Physical::energy(b)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_ufcs_double_colon.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("Physical::energy"),
        "UFCS trait call must lower before R emission:\n{output}"
    );
}

#[test]
pub(crate) fn trait_default_method_is_materialized_for_impl() {
    let src = r#"
trait Physical {
  fn energy(self: Self) -> float {
    self.mass
  }
}

impl Physical for Body {
}

fn main() {
  let b: Body = {mass: 2.0}
  Physical::energy(b)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_default_method.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("Physical::energy")
            && (output.contains("[[\"mass\"]]") || output.contains("return(((2.0)))")),
        "default trait method must lower to concrete field access before R emission:\n{output}"
    );
}

#[test]
pub(crate) fn associated_function_dispatches_statically_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping associated function runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Factory {
  fn make() -> Self
}

impl Factory for Body {
  fn make() -> Body {
    {mass: 4.0}
  }
}

fn main() {
  let b = Factory::make::<Body>()
  b.mass
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_associated_function_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("associated function should compile");

    assert!(
        !output.contains("Factory::make") && !output.contains("Factory.make"),
        "associated function must lower statically:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("trait_assoc_fn");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_assoc_fn.R");
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
    assert!(stdout.contains("[1] 4"), "unexpected R output:\n{stdout}");
}

#[test]
pub(crate) fn default_associated_function_is_materialized_for_impl() {
    let src = r#"
trait Factory {
  fn make() -> Self {
    {mass: 6.0}
  }
}

impl Factory for Body {
}

fn main() {
  let b = Factory::make::<Body>()
  b.mass
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_default_associated_function.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("default associated function should compile");

    assert!(
        !output.contains("Factory::make") && output.contains("6.0"),
        "default associated function must materialize as a concrete helper:\n{output}"
    );
}

#[test]
pub(crate) fn explicit_type_trait_method_dispatch_does_not_require_receiver_hint() {
    let src = r#"
trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass * self.velocity * self.velocity * 0.5
  }
}

fn main() {
  let b = {mass: 2.0, velocity: 3.0}
  Physical::energy::<Body>(b)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_explicit_type_method_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("explicit type trait method dispatch should compile");

    assert!(
        !output.contains("Physical::energy") && !output.contains("Physical.energy"),
        "explicit type trait method call must lower statically:\n{output}"
    );
}

#[test]
pub(crate) fn fully_qualified_associated_items_dispatch_statically_and_run() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!(
                "Skipping fully-qualified associated item runtime test: Rscript not available."
            );
            return;
        }
    };

    let src = r#"
trait Scale {
  const FACTOR: float
}

impl Scale for Body {
  const FACTOR: float = 2.0
}

trait Factory {
  fn make() -> Body
}

impl Factory for Body {
  fn make() -> Body {
    {mass: <Body as Scale>::FACTOR}
  }
}

trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass * 3.0
  }
}

fn main() {
  let b = <Body as Factory>::make()
  <Body as Physical>::energy(b) + <Body as Scale>::FACTOR
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_fully_qualified_associated_items.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("fully-qualified associated items should compile");

    assert!(
        !output.contains(" as ")
            && !output.contains("<Body")
            && !output.contains("Factory::")
            && !output.contains("Physical::"),
        "fully-qualified associated items must lower before R emission:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_fully_qualified_items");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_fully_qualified_items.R");
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
    assert!(stdout.contains("[1] 8"), "unexpected R output:\n{stdout}");
}

#[test]
pub(crate) fn const_generic_trait_impl_monomorphizes_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping const generic trait runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait StaticLen {
  fn len(self: Self) -> int
}

impl<const N> StaticLen for StaticVec<N> {
  fn len(self: StaticVec<N>) -> int {
    N
  }
}

fn add_len<const N>(x: StaticVec<N>) -> int {
  x.len() + N
}

fn main() {
  let v: StaticVec<3> = {values: [1, 2, 3]}
  add_len(v)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_const_generic_impl.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("const generic trait impl should compile");

    assert!(
        output.contains("+ 3L") && output.contains("return(3L)"),
        "const generic call should monomorphize with the const value:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_const_generic_impl");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_const_generic_impl.R");
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
    assert!(stdout.contains("[1] 6"), "unexpected R output:\n{stdout}");
}

#[test]
pub(crate) fn lifetime_params_and_hrtb_bounds_are_erased_for_static_dispatch() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping lifetime/HRTB trait runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Borrowable {
  fn id(self: Self) -> Self
}

impl Borrowable for Body {
  fn id(self: Body) -> Body {
    self
  }
}

fn passthrough<'a, T>(x: T) -> T where for<'a> T: Borrowable {
  x.id()
}

fn main() {
  let b: Body = {mass: 5.0}
  let out: Body = passthrough(b)
  out.mass
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_lifetime_hrtb_erased.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("lifetime params and HRTB bounds should parse and compile");

    assert!(
        !output.contains("'a") && !output.contains("for<"),
        "lifetimes should be erased before R emission:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_lifetime_hrtb");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_lifetime_hrtb.R");
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
    assert!(stdout.contains("[1] 5"), "unexpected R output:\n{stdout}");
}

#[test]
pub(crate) fn concrete_gat_family_instance_substitutes_in_trait_signature() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping GAT trait runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Project {
  type Out<T>
  fn get_float(self: Self) -> <Self as Project>::Out<float>
}

impl Project for FloatBox {
  type Out<float> = float

  fn get_float(self: FloatBox) -> float {
    self.value
  }
}

fn main() {
  let b: FloatBox = {value: 7.0}
  b.get_float()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_concrete_gat_family_instance.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("concrete GAT family instance should compile");

    assert!(
        !output.contains("get_float(self")
            && (output.contains("[[\"value\"]]") || output.contains("return(((7.0)))")),
        "GAT-backed trait method should lower to the concrete impl body:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("trait_concrete_gat");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_concrete_gat.R");
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
    assert!(stdout.contains("[1] 7"), "unexpected R output:\n{stdout}");
}

#[test]
pub(crate) fn generic_specialization_prefers_more_specific_generic_pattern() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping generic specialization runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Tag {
  fn tag(self: Self) -> str
}

impl<T> Tag for Wrap<T> {
  fn tag(self: Wrap<T>) -> str {
    "generic"
  }
}

impl<T> Tag for Wrap<Inner<T>> {
  fn tag(self: Wrap<Inner<T>>) -> str {
    "inner"
  }
}

fn main() {
  let x: Wrap<Inner<Body>> = {value: {value: {mass: 1.0}}}
  x.tag()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_generic_specialization.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("more-specific generic impl should compile");

    assert!(
        output.contains("\"inner\"") && !output.contains("\"generic\""),
        "more-specific generic impl should be selected:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_generic_specialization");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_generic_specialization.R");
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
        stdout.contains("[1] \"inner\""),
        "unexpected R output:\n{stdout}"
    );
}

#[test]
pub(crate) fn dyn_trait_binding_preserves_concrete_static_dispatch() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping dyn trait binding runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass * 2.0
  }
}

fn main() {
  let b: Body = {mass: 4.0}
  let obj: dyn Physical = b
  obj.energy()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_dyn_binding_static_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("dyn trait binding should compile when initialized from a concrete impl");

    assert!(
        !output.contains("obj.energy") && !output.contains("dyn Physical"),
        "dyn binding should erase before R emission and dispatch statically:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("trait_dyn_binding");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_dyn_binding.R");
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
    assert!(stdout.contains("[1] 8"), "unexpected R output:\n{stdout}");
}

#[test]
pub(crate) fn generic_associated_function_monomorphizes_and_requires_bound() {
    let ok_src = r#"
trait Factory {
  fn make() -> Self
}

impl Factory for Body {
  fn make() -> Body {
    {mass: 8.0}
  }
}

fn make_default<T>() -> T where T: Factory {
  Factory::make::<T>()
}

fn main() {
  let b: Body = make_default()
  b.mass
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_generic_associated_function.rr",
        ok_src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("generic associated function should monomorphize");

    assert!(
        !output.contains("Factory::make") && !output.contains("Factory.make"),
        "generic associated function must lower through concrete specialization:\n{output}"
    );

    let missing_bound_src = r#"
trait Factory {
  fn make() -> Self
}

fn make_default<T>() -> T {
  Factory::make::<T>()
}
"#;

    let err = compile_with_config(
        "trait_generic_associated_function_missing_bound.rr",
        missing_bound_src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("generic associated function should require a matching bound");

    assert!(
        err.message
            .contains("generic static trait method 'Factory.make' requires bound `T: Factory`"),
        "unexpected error: {err:?}"
    );
}
