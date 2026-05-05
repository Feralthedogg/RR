use super::*;

#[test]
pub(crate) fn generic_trait_return_inference_allows_unannotated_locals_and_method_chains() {
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
    let scalar_rebound_x = "(((10.0 + 2.0) + (0.0 - 2.0)) * 1.5)";
    let folded_rebound_x = "return((15.0))";
    assert!(
        output.contains(".__rr_inline_expr_")
            || output.contains(scalar_rebound_x)
            || output.contains(folded_rebound_x),
        "optimized trait method chain should use let-lifted inline temps or fold to a scalar expression:\n{output}"
    );
    assert!(
        !output.contains("list(")
            && !output.contains("Sym_35(")
            && !output.contains("[[\"x\"]]")
            && !output.contains("[[\"y\"]]"),
        "optimized trait method chain should avoid record allocation and field projection:\n{output}"
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
pub(crate) fn generic_trait_sroa_reduces_record_allocation_shape() {
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
  final_state.x + final_state.y
}

print(main())
"#;

    let (o0, _) = compile_with_config(
        "generic_trait_sroa_shape_o0.rr",
        src,
        OptLevel::O0,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("O0 trait chain should compile");
    let (o2, _) = compile_with_config(
        "generic_trait_sroa_shape_o2.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("O2 trait chain should compile");

    assert!(
        o2.len() < o0.len(),
        "optimized trait+SROA output should shrink the helper-heavy record chain: O0={} bytes, O2={} bytes\nO2 output:\n{o2}",
        o0.len(),
        o2.len()
    );
    let scalar_rebound_x = "(((10.0 + 2.0) + (0.0 - 2.0)) * 1.5)";
    let scalar_rebound_y = "(((15.0 + -3.0) + (0.0 - -3.0)) * 1.5)";
    let folded_sroa_fields =
        o2.contains("final_state__rr_sroa_ret_x <- 15.0") && o2.contains("(22.5)");
    let fully_folded = o2.contains("return((37.5))");
    assert!(
        (o2.contains(scalar_rebound_x) && o2.contains(scalar_rebound_y)
            || folded_sroa_fields
            || fully_folded)
            && !o2.contains("list(")
            && !o2.contains("Sym_35("),
        "optimized trait chain should fold through cross-call scalar fields without record allocation in the hot path:\n{o2}"
    );
    assert!(
        !o2.contains("$translate") && !o2.contains("$scale"),
        "optimized trait chain must keep static dispatch before SROA shape check:\n{o2}"
    );
}

#[test]
pub(crate) fn generic_trait_sroa_survives_unrelated_store_index_in_same_function() {
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
  let scratch = c(0.0, 0.0)
  let initial_pos: Vec2 = {x: 10.0, y: 15.0}
  let velocity: Vec2 = {x: 2.0, y: -3.0}
  let final_state = simulate_rebound(initial_pos, velocity, 1.5)
  scratch[1.0] = final_state.x
  final_state.y + scratch[1.0]
}

print(main())
"#;

    let (output, _) = compile_with_config(
        "generic_trait_sroa_with_store_index.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("StoreIndex in the same function should not disable trait-chain SROA");

    assert!(
        output.contains("final_state__rr_sroa_ret_x")
            && output.contains("final_state__rr_sroa_ret_y")
            && !output.contains("Sym_35("),
        "trait record result should stay scalarized despite an unrelated vector store:\n{output}"
    );
    assert!(
        !output.contains("FieldEnergy.energy")
            && !output.contains("$translate")
            && !output.contains("$scale"),
        "trait dispatch must remain static before store-index/SROA shape checks:\n{output}"
    );
    assert!(
        !output.contains("final_state <- list(") && !output.contains("final_state <- (list("),
        "final_state should not be materialized as a list before field projection:\n{output}"
    );
}

#[test]
pub(crate) fn generic_trait_bound_requires_concrete_impl_at_call_site() {
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
pub(crate) fn generic_trait_method_requires_declared_where_bound() {
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
pub(crate) fn generic_type_parameter_inference_rejects_conflicting_concrete_types() {
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
pub(crate) fn explicit_turbofish_monomorphizes_generic_function_and_runs() {
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
pub(crate) fn generic_return_type_inference_uses_annotated_let_type_and_runs() {
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
pub(crate) fn generic_impl_block_instantiates_for_concrete_receiver_and_runs() {
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
pub(crate) fn generic_impl_block_enforces_where_bound_at_dispatch_site() {
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
pub(crate) fn imported_trait_and_impl_are_visible_to_entry_module_and_run() {
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
pub(crate) fn imported_private_trait_metadata_is_not_visible_to_entry_module() {
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
