use std::fs;
use std::path::PathBuf;
use std::process::Command;

use RR::compiler::{OptLevel, compile_with_config};
use RR::hir::def::{HirItem, ModuleId};
use RR::hir::lower::Lowerer;
use RR::syntax::parse::Parser;
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};

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

    assert!(
        output.contains("__rr_sroa_x"),
        "record chain should lower through scalar SROA temps:\n{output}"
    );
    assert!(
        !output.contains(".__rr_inline_expr_0 <- list(")
            && !output.contains("final_state <- ((list(")
            && !output.contains("final_state <- (list("),
        "hot trait record chain should not keep avoidable list temporaries:\n{output}"
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

#[test]
fn generic_trait_return_inference_allows_unannotated_locals_and_method_chains() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => Some(p),
        _ => None,
    };

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

print(main())
"#;

    let (output, _) = compile_with_config(
        "generic_trait_return_inference_method_chain.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("unannotated trait-return locals and method chains should compile");

    assert!(
        !output.contains("$translate") && !output.contains("$scale"),
        "receiver method chain must lower statically instead of field dispatch:\n{output}"
    );
    let max_line_len = output.lines().map(str::len).max().unwrap_or(0);
    assert!(
        max_line_len < 700,
        "trait method chain emission should avoid aggregate AST bloat:\n{output}"
    );
    assert!(
        output.contains(".__rr_inline_expr_"),
        "optimized trait method chain should use let-lifted inline temps:\n{output}"
    );

    if let Some(rscript) = rscript {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let out_dir = root
            .join("target")
            .join("tests")
            .join("generic_trait_return_inference_method_chain");
        fs::create_dir_all(&out_dir).expect("create out dir");
        let out_r = out_dir.join("generic_trait_return_inference_method_chain.R");
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
            stdout.contains("[1] 15"),
            "unexpected R output:\n{}",
            stdout
        );
    }
}

#[test]
fn generic_trait_bound_requires_concrete_impl_at_call_site() {
    let src = r#"
trait Physical {
  fn energy(self: Self) -> float
}

fn energy_of<T>(x: T) -> float where T: Physical {
  x.energy()
}

fn main() {
  let r: Rock = {mass: 1.0}
  energy_of(r)
}
"#;

    let err = compile_with_config(
        "generic_trait_bound_missing_impl.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("generic call without concrete impl must fail");

    assert!(
        err.message.contains("requires trait 'Physical' for 'Rock'")
            && err.message.contains("no impl was found"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn generic_trait_method_requires_declared_where_bound() {
    let src = r#"
trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass
  }
}

fn energy_of<T>(x: T) -> float {
  x.energy()
}

fn main() {
  let b: Body = {mass: 2.0}
  energy_of(b)
}
"#;

    let err = compile_with_config(
        "generic_trait_method_missing_bound.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("generic trait method use without where bound must fail");

    assert!(
        err.message.contains("without a matching trait bound"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn generic_type_parameter_inference_rejects_conflicting_concrete_types() {
    let src = r#"
fn first<T>(a: T, b: T) -> T {
  a
}

fn main() {
  let a: A = {x: 1.0}
  let b: B = {x: 2.0}
  first(a, b)
}
"#;

    let err = compile_with_config(
        "generic_type_parameter_conflict.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("conflicting generic type parameter inference must fail");

    assert!(
        err.message
            .contains("generic type parameter 'T' inferred as both 'A' and 'B'"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn explicit_turbofish_monomorphizes_generic_function_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping explicit turbofish runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
fn id<T>(x: T) -> T {
  x
}

fn main() {
  let b: Body = {mass: 2.0}
  let out = id::<Body>(b)
  out.mass
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "explicit_turbofish_generic_call.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("id::<"),
        "explicit turbofish syntax must not survive to emitted R:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("explicit_turbofish");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("explicit_turbofish.R");
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
    assert!(stdout.contains("[1] 2"), "unexpected R output:\n{}", stdout);
}

#[test]
fn generic_return_type_inference_uses_annotated_let_type_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping generic return inference runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
fn make<T>() -> T {
  {mass: 4.0}
}

fn main() {
  let b: Body = make()
  b.mass
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "generic_return_type_inference.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("make<"),
        "generic source type syntax must not survive to emitted R:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("generic_return_type_inference");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("generic_return_type_inference.R");
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
    assert!(stdout.contains("[1] 4"), "unexpected R output:\n{}", stdout);
}

#[test]
fn generic_impl_block_instantiates_for_concrete_receiver_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping generic impl runtime test: Rscript not available.");
            return;
        }
    };

    let src = r#"
trait Physical {
  fn marker(self: Self) -> float
}

trait Mass {
  fn mass(self: Self) -> float
}

impl Physical for Body {
  fn marker(self: Body) -> float {
    self.mass
  }
}

impl<T> Mass for Box<T> where T: Physical {
  fn mass(self: Box<T>) -> float {
    self.value.mass
  }
}

fn main() {
  let b: Body = {mass: 5.0}
  let boxed: Box<Body> = {value: b}
  boxed.mass()
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "generic_impl_block_dispatch.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !output.contains("boxed.mass"),
        "generic impl method syntax must not survive to emitted R:\n{output}"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("generic_impl_block");
    fs::create_dir_all(&out_dir).expect("create out dir");
    let out_r = out_dir.join("generic_impl_block.R");
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
    assert!(stdout.contains("[1] 5"), "unexpected R output:\n{}", stdout);
}

#[test]
fn generic_impl_block_enforces_where_bound_at_dispatch_site() {
    let src = r#"
trait Physical {
  fn marker(self: Self) -> float
}

trait Mass {
  fn mass(self: Self) -> float
}

impl<T> Mass for Box<T> where T: Physical {
  fn mass(self: Box<T>) -> float {
    self.value.mass
  }
}

fn main() {
  let rock: Rock = {mass: 1.0}
  let boxed: Box<Rock> = {value: rock}
  boxed.mass()
}
"#;

    let err = compile_with_config(
        "generic_impl_missing_where_bound.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("generic impl dispatch must enforce where bounds");

    assert!(
        err.message.contains("requires trait 'Physical' for 'Rock'")
            && err.message.contains("no impl was found"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn imported_trait_and_impl_are_visible_to_entry_module_and_run() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!(
                "Skipping cross-module trait visibility runtime test: Rscript not available."
            );
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root
        .join("target")
        .join("tests")
        .join("cross_module_trait_visibility")
        .join(format!("{}", std::process::id()));
    fs::create_dir_all(&sandbox).expect("create sandbox");
    let main_path = sandbox.join("main.rr");
    let traits_path = sandbox.join("traits.rr");
    fs::write(
        &traits_path,
        r#"
export trait Physical {
  fn energy(self: Self) -> float
}

export impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass * self.velocity * self.velocity * 0.5
  }
}
"#,
    )
    .expect("write traits module");

    let src = r#"
import "./traits.rr"

fn main() {
  let b: Body = {mass: 2.0, velocity: 3.0}
  b.energy()
}

print(main())
"#;
    fs::write(&main_path, src).expect("write main module");

    let (output, _) = compile_with_config(
        main_path.to_str().expect("utf8 path"),
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
        "cross-module trait dispatch must lower statically:\n{output}"
    );

    let out_r = sandbox.join("cross_module_trait_visibility.R");
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
fn imported_private_trait_metadata_is_not_visible_to_entry_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root
        .join("target")
        .join("tests")
        .join("cross_module_private_trait_visibility")
        .join(format!("{}", std::process::id()));
    fs::create_dir_all(&sandbox).expect("create sandbox");
    let main_path = sandbox.join("main.rr");
    let traits_path = sandbox.join("traits.rr");
    fs::write(
        &traits_path,
        r#"
trait Physical {
  fn energy(self: Self) -> float
}

impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass
  }
}
"#,
    )
    .expect("write traits module");

    let src = r#"
import "./traits.rr"

fn energy_of<T>(x: T) -> float where T: Physical {
  x.energy()
}

fn main() {
  let b: Body = {mass: 2.0}
  energy_of(b)
}
"#;
    fs::write(&main_path, src).expect("write main module");

    let err = compile_with_config(
        main_path.to_str().expect("utf8 path"),
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("non-exported imported trait metadata must stay private");

    assert!(
        err.message
            .contains("unknown trait 'Physical' in where clause")
            || err
                .message
                .contains("uses method 'energy' without a matching trait bound"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn ufcs_double_colon_trait_call_dispatches_statically() {
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
fn trait_default_method_is_materialized_for_impl() {
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
fn associated_function_dispatches_statically_and_runs() {
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
fn default_associated_function_is_materialized_for_impl() {
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
fn explicit_type_trait_method_dispatch_does_not_require_receiver_hint() {
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
fn fully_qualified_associated_items_dispatch_statically_and_run() {
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
fn const_generic_trait_impl_monomorphizes_and_runs() {
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
fn lifetime_params_and_hrtb_bounds_are_erased_for_static_dispatch() {
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
fn concrete_gat_family_instance_substitutes_in_trait_signature() {
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
fn generic_specialization_prefers_more_specific_generic_pattern() {
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
fn dyn_trait_binding_preserves_concrete_static_dispatch() {
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
fn generic_associated_function_monomorphizes_and_requires_bound() {
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

#[test]
fn associated_type_substitutes_into_impl_signature() {
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
fn associated_type_projection_bound_monomorphizes_and_runs() {
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
fn fully_qualified_associated_type_projection_disambiguates_same_assoc_names() {
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
fn qualified_projection_through_supertrait_bound_ignores_sibling_assoc_alias() {
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
fn associated_type_projection_method_requires_projection_bound() {
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
fn associated_type_projection_bound_is_checked_at_call_site() {
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
fn associated_const_dispatches_statically_and_runs() {
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
fn associated_const_default_materializes_for_impl() {
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
fn associated_const_must_be_provided_without_default() {
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
fn generic_associated_const_uses_bound_and_monomorphizes() {
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
fn associated_type_must_be_provided_by_impl() {
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
fn supertrait_bound_must_be_satisfied_for_dispatch() {
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
fn repeated_type_param_impl_does_not_overlap_inconsistent_exact_impl() {
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
fn exact_impl_specializes_generic_impl_for_static_dispatch() {
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
fn negative_impl_blocks_blanket_generic_impl() {
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
fn supertrait_where_bound_satisfies_parent_trait_call() {
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
fn index_operator_trait_dispatches_statically() {
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
