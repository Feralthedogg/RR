use std::fs;
use std::path::PathBuf;
use std::process::Command;

use rr::compiler::internal::hir::def::{HirItem, ModuleId};
use rr::compiler::internal::hir::lower::Lowerer;
use rr::compiler::internal::syntax::parse::Parser;
use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, compile_with_config};

fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    Some("Rscript".to_string())
}

fn rscript_available(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn trait_where_bounds_are_preserved_in_hir() {
    let src = r#"
trait Numeric {}
trait Parallel {}

fn solve<T>(x: T) -> T where T: Numeric + Parallel {
  x
}
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse_program().expect("parse");
    let mut lowerer = Lowerer::new();
    let hir_mod = lowerer.lower_module(ast, ModuleId(0)).expect("lower");
    let symbols = lowerer.into_symbols();

    let solve = hir_mod
        .items
        .into_iter()
        .find_map(|item| match item {
            HirItem::Fn(f) if symbols.get(&f.name).is_some_and(|name| name == "solve") => Some(f),
            _ => None,
        })
        .expect("solve function");

    assert_eq!(solve.type_params, vec!["T"]);
    assert_eq!(solve.where_bounds.len(), 1);
    assert_eq!(solve.where_bounds[0].type_name, "T");
    let bound_names = solve.where_bounds[0]
        .trait_names
        .iter()
        .map(|sym| symbols.get(sym).cloned().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(bound_names, vec!["Numeric", "Parallel"]);
}

#[test]
fn where_keyword_remains_valid_named_call_argument() {
    let src = r#"
fn main() {
  methods.getLoadActions(where = globalenv())
  let found = utils.getAnywhere("mean")
  found.where
}
"#;
    let mut parser = Parser::new(src);
    parser
        .parse_program()
        .expect("where should remain valid in R named arguments and field selectors");
}

#[test]
fn trait_method_dispatch_requires_static_receiver_type_hint() {
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
  let b = {mass: 2.0}
  Physical.energy(b)
}
"#;

    let err = compile_with_config(
        "trait_missing_receiver_type.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("trait dispatch without a receiver type hint must fail");

    assert!(
        err.message.contains("explicit static type hint"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn receiver_method_sugar_reports_missing_static_receiver_type_hint() {
    let src = r#"
trait Transformable {
  fn translate(self: Self, offset: Self) -> Self
}

impl Transformable for Vec2 {
  fn translate(self: Vec2, offset: Vec2) -> Vec2 {
    {x: self.x + offset.x, y: self.y + offset.y}
  }
}

fn main() {
  let moved = {x: 1.0, y: 2.0}
  let delta = {x: 3.0, y: 4.0}
  moved.translate(delta)
}
"#;

    let err = compile_with_config(
        "trait_receiver_missing_type_hint.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("receiver method sugar without a receiver type hint must fail clearly");

    assert!(
        err.message.contains(
            "receiver method 'translate' requires a receiver with an explicit static type hint"
        ),
        "unexpected error: {err:?}"
    );
    let rendered = format!("{err:?}");
    assert!(
        !rendered.contains("dataframe schema"),
        "receiver method hint error must not mention dataframe internals: {rendered}"
    );
}

#[test]
fn receiver_method_sugar_reports_missing_impl_for_concrete_receiver() {
    let src = r#"
trait Transformable {
  fn translate(self: Self, offset: Self) -> Self
}

impl Transformable for Vec2 {
  fn translate(self: Vec2, offset: Vec2) -> Vec2 {
    {x: self.x + offset.x, y: self.y + offset.y}
  }
}

fn main() {
  let moved: Rock = {x: 1.0, y: 2.0}
  let delta: Rock = {x: 3.0, y: 4.0}
  moved.translate(delta)
}
"#;

    let err = compile_with_config(
        "trait_receiver_missing_impl.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("receiver method sugar without a matching impl must fail clearly");

    assert!(
        err.message
            .contains("cannot resolve receiver method 'translate' for receiver type 'Rock'"),
        "unexpected error: {err:?}"
    );
    let rendered = format!("{err:?}");
    assert!(
        !rendered.contains("unknown field") && !rendered.contains("dataframe schema"),
        "missing impl error must not fall through to field diagnostics: {rendered}"
    );
}

#[test]
fn receiver_trait_method_name_does_not_block_unbound_dotted_call() {
    let src = r#"
trait MethodNameCollision {
  fn getLoadActions(self: Self) -> float
}

fn main() {
  methods.getLoadActions(where = globalenv())
}
"#;

    let mut parser = Parser::new(src);
    let ast = parser.parse_program().expect("parse");
    let mut lowerer = Lowerer::new();
    lowerer
        .lower_module(ast, ModuleId(0))
        .expect("unbound dotted R calls must not be treated as receiver trait methods");
}

#[test]
fn trait_method_dispatch_compiles_to_static_impl_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping trait static dispatch runtime test: Rscript not available.");
            return;
        }
    };

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
  let b: Body = {mass: 2.0, velocity: 3.0}
  Physical.energy(b)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_static_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("Physical.energy"),
        "trait call should be resolved before R emission:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_static_dispatch");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_static_dispatch.R");
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
    assert!(stdout.contains("[1] 9"), "unexpected R output:\n{}", stdout);
}

#[test]
fn receiver_method_sugar_dispatches_statically_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping receiver trait dispatch runtime test: Rscript not available.");
            return;
        }
    };

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
  let b: Body = {mass: 2.0, velocity: 3.0}
  b.energy()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_receiver_method_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("b.energy"),
        "receiver method sugar must not be emitted as dynamic R dispatch:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_receiver_method_dispatch");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_receiver_method_dispatch.R");
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
    assert!(stdout.contains("[1] 9"), "unexpected R output:\n{}", stdout);
}

#[test]
fn receiver_method_sugar_reports_ambiguous_trait_methods() {
    let src = r#"
trait A {
  fn score(self: Self) -> float
}

trait B {
  fn score(self: Self) -> float
}

impl A for Thing {
  fn score(self: Thing) -> float {
    self.x
  }
}

impl B for Thing {
  fn score(self: Thing) -> float {
    self.x + 1.0
  }
}

fn main() {
  let t: Thing = {x: 1.0}
  t.score()
}
"#;

    let err = compile_with_config(
        "trait_receiver_ambiguous.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("ambiguous receiver method dispatch must fail");

    assert!(
        err.message.contains("ambiguous trait method")
            && err.message.contains("Trait.method(receiver, ...)"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn add_operator_trait_dispatches_statically_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping Add operator trait dispatch runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Add {
  fn add(self: Self, rhs: Self) -> Self
}

impl Add for Vec2 {
  fn add(self: Vec2, rhs: Vec2) -> Vec2 {
    {x: self.x + rhs.x, y: self.y + rhs.y}
  }
}

fn main() {
  let a: Vec2 = {x: 1.0, y: 2.0}
  let b: Vec2 = {x: 3.0, y: 4.0}
  let c = a + b
  c.x + c.y
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_add_operator_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("(a + b)"),
        "operator overload must not be emitted as native R list addition:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_add_operator_dispatch");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_add_operator_dispatch.R");
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
        stdout.contains("[1] 10"),
        "unexpected R output:\n{}",
        stdout
    );
}

#[test]
fn trait_record_chain_scalarizes_in_emitted_r() {
    let src = r#"
trait Add {
  fn add(self: Self, rhs: Self) -> Self
}

trait Neg {
  fn neg(self: Self) -> Self
}

trait Transformable {
  fn translate(self: Self, offset: Self) -> Self
  fn scale(self: Self, factor: float) -> Self
}

impl Add for Vec2 {
  fn add(self: Vec2, rhs: Vec2) -> Vec2 {
    {x: self.x + rhs.x, y: self.y + rhs.y}
  }
}

impl Neg for Vec2 {
  fn neg(self: Vec2) -> Vec2 {
    {x: 0.0 - self.x, y: 0.0 - self.y}
  }
}

impl Transformable for Vec2 {
  fn translate(self: Vec2, offset: Vec2) -> Vec2 {
    self + offset
  }

  fn scale(self: Vec2, factor: float) -> Vec2 {
    {x: self.x * factor, y: self.y * factor}
  }
}

fn simulate_rebound<T>(entity: T, velocity: T, time: float) -> T
  where T: Add + Neg + Transformable
{
  let moved = entity + velocity
  let rebounded_vel = -velocity
  moved.translate(rebounded_vel).scale(time)
}

fn main() {
  let initial_pos: Vec2 = {x: 10.0, y: 15.0}
  let velocity: Vec2 = {x: 2.0, y: -3.0}
  let final_state = simulate_rebound(initial_pos, velocity, 1.5)
  final_state.x
}

main()
"#;

    let (output, _) = compile_with_config(
        "trait_record_chain_sroa.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    let scalar_rebound_x = "(((10.0 + 2.0) + (0.0 - 2.0)) * 1.5)";
    let folded_rebound_x = "return((15.0))";
    assert!(
        output.contains("__rr_sroa_x")
            || output.contains(scalar_rebound_x)
            || output.contains(folded_rebound_x),
        "record chain should either use scalar SROA temps or fold to a scalar expression:\n{output}"
    );
    assert!(
        !output.contains("$translate")
            && !output.contains("$scale")
            && !output.contains("Sym_35(")
            && !output.contains("[[\"x\"]]")
            && !output.contains("[[\"y\"]]")
            && !output.contains("list(")
            && !output.contains(".__rr_inline_expr_0 <- list(")
            && !output.contains("final_state <- ((list(")
            && !output.contains("final_state <- (list("),
        "hot trait record chain should not keep dynamic dispatch, record allocation, or field projection:\n{output}"
    );
}

#[test]
fn neg_operator_trait_dispatches_statically_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping Neg operator trait dispatch runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Neg {
  fn neg(self: Self) -> Self
}

impl Neg for Vec1 {
  fn neg(self: Vec1) -> Vec1 {
    {x: 0.0 - self.x}
  }
}

fn main() {
  let a: Vec1 = {x: 3.0}
  let b = -a
  b.x
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "trait_neg_operator_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("(-a)"),
        "Neg operator overload must not be emitted as native R list negation:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("trait_neg_operator_dispatch");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("trait_neg_operator_dispatch.R");
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
    assert!(stdout.contains("[1] -3"), "unexpected R output:\n{stdout}");
}

#[test]
fn generic_neg_operator_trait_bound_monomorphizes_and_requires_bound() {
    let ok_src = r#"
trait Neg {
  fn neg(self: Self) -> Self
}

impl Neg for Vec1 {
  fn neg(self: Vec1) -> Vec1 {
    {x: 0.0 - self.x}
  }
}

fn negate<T>(x: T) -> T where T: Neg {
  -x
}

fn main() {
  let a: Vec1 = {x: 3.0}
  let b = negate(a)
  b.x
}
"#;

    let (output, _) = compile_with_config(
        "generic_neg_operator_trait_bound.rr",
        ok_src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("Neg-bound generic operator should monomorphize");

    assert!(
        !output.contains("(-x)"),
        "generic Neg specialization must not emit native R negation:\n{output}"
    );

    let missing_bound_src = r#"
trait Neg {
  fn neg(self: Self) -> Self
}

fn negate<T>(x: T) -> T {
  -x
}
"#;

    let err = compile_with_config(
        "generic_neg_operator_missing_bound.rr",
        missing_bound_src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("generic unary operator should require an explicit trait bound");

    assert!(
        err.message
            .contains("generic operator 'neg' requires bound `T: Neg`"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn generic_trait_bound_monomorphizes_receiver_method_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!(
                "Skipping generic trait monomorphization runtime test: Rscript not available."
            );
            return;
        }
    };

    let src = r#"
trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass * self.velocity * self.velocity * 0.5
  }
}

fn energy_of<T>(x: T) -> float where T: Physical {
  x.energy()
}

fn main() {
  let b: Body = {mass: 2.0, velocity: 3.0}
  energy_of(b)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "generic_trait_bound_monomorphizes_receiver_method.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("x.energy"),
        "generic specialization must not leave receiver sugar in emitted R:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("generic_trait_bound_monomorphizes_receiver_method");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("generic_trait_bound_monomorphizes_receiver_method.R");
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
    assert!(stdout.contains("[1] 9"), "unexpected R output:\n{}", stdout);
}

#[test]
fn generic_trait_bound_monomorphizes_explicit_trait_call_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping generic explicit trait call runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass * self.velocity * self.velocity * 0.5
  }
}

fn energy_of<T>(x: T) -> float where T: Physical {
  Physical.energy(x)
}

fn main() {
  let b: Body = {mass: 2.0, velocity: 3.0}
  energy_of(b)
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "generic_trait_bound_monomorphizes_explicit_trait_call.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("Physical.energy"),
        "generic specialization must resolve explicit trait calls before R emission:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("generic_trait_bound_monomorphizes_explicit_trait_call");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("generic_trait_bound_monomorphizes_explicit_trait_call.R");
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
    assert!(stdout.contains("[1] 9"), "unexpected R output:\n{}", stdout);
}

#[test]
fn generic_operator_trait_bound_monomorphizes_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping generic Add monomorphization runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Add {
  fn add(self: Self, rhs: Self) -> Self
}

impl Add for Vec2 {
  fn add(self: Vec2, rhs: Vec2) -> Vec2 {
    {x: self.x + rhs.x, y: self.y + rhs.y}
  }
}

fn sum_pair<T>(a: T, b: T) -> T where T: Add {
  a + b
}

fn main() {
  let a: Vec2 = {x: 1.0, y: 2.0}
  let b: Vec2 = {x: 3.0, y: 4.0}
  let c = sum_pair(a, b)
  c.x + c.y
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "generic_operator_trait_bound_monomorphizes.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("(a + b)"),
        "generic Add specialization must not emit native R list addition:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("generic_operator_trait_bound_monomorphizes");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("generic_operator_trait_bound_monomorphizes.R");
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
        stdout.contains("[1] 10"),
        "unexpected R output:\n{}",
        stdout
    );
}

#[path = "trait_static_dispatch/associated_items.rs"]
mod associated_items;
#[path = "trait_static_dispatch/associated_types.rs"]
mod associated_types;
#[path = "trait_static_dispatch/generic_inference.rs"]
mod generic_inference;
