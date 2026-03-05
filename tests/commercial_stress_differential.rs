use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct RunResult {
    status: i32,
    stdout: String,
    stderr: String,
}

#[derive(Clone, Copy, Debug)]
struct Params {
    n: i32,
    k: i32,
    a: i32,
    b: i32,
    c: i32,
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 32) as u32
    }

    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi - lo + 1) as u32;
        lo + (self.next_u32() % span) as i32
    }
}

fn unique_dir(root: &Path, name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.join(format!("{}_{}_{}", name, std::process::id(), ts))
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty() {
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

fn run_rscript(path: &str, script: &Path) -> RunResult {
    let output = Command::new(path)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    RunResult {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn compile_rr(rr_bin: &Path, rr_path: &Path, out_path: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_path)
        .arg("-o")
        .arg(out_path)
        .arg("--no-runtime")
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_path.display(),
        level
    );
}

fn rr_source(p: Params) -> String {
    format!(
        r#"
fn kernel(n, k, a, b, c) {{
  let x = seq_len(n);
  let y = seq_len(n);
  let s = 0L;
  for (i in 1L..length(x)) {{
    if ((((x[i] * a) + b) - c) > k) {{
      y[i] = (((x[i] * a) + b) - c) - k;
    }} else {{
      y[i] = (((x[i] * a) + b) - c) + k;
    }}
    s = s + y[i];
  }}
  return s;
}}

fn mix(n, k, a, b, c) {{
  let i = 1L;
  let acc = 0L;
  while (i <= n) {{
    acc = acc + (((i * a) + b) - c);
    i = i + 1L;
  }}
  let ys = kernel(n, k, a, b, c);
  return acc + ys;
}}

print(mix({}L, {}L, {}L, {}L, {}L));
"#,
        p.n, p.k, p.a, p.b, p.c
    )
}

fn reference_r_source(p: Params) -> String {
    format!(
        r#"
kernel <- function(n, k, a, b, c) {{
  x <- seq_len(n)
  y <- seq_len(n)
  s <- 0L
  for (i in 1:length(x)) {{
    if ((((x[i] * a) + b) - c) > k) {{
      y[i] <- (((x[i] * a) + b) - c) - k
    }} else {{
      y[i] <- (((x[i] * a) + b) - c) + k
    }}
    s <- s + y[i]
  }}
  s
}}

mix <- function(n, k, a, b, c) {{
  i <- 1L
  acc <- 0L
  while (i <= n) {{
    acc <- acc + (((i * a) + b) - c)
    i <- i + 1L
  }}
  ys <- kernel(n, k, a, b, c)
  acc + ys
}}

print(mix({}L, {}L, {}L, {}L, {}L))
"#,
        p.n, p.k, p.a, p.b, p.c
    )
}

#[test]
fn stress_differential_semantics_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stress differential tests: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("commercial_stress_differential");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let mut rng = Lcg::new(0xC0FFEE_BADC0DE);
    let mut cases = Vec::new();
    for _ in 0..24 {
        cases.push(Params {
            n: rng.range_i32(5, 40),
            k: rng.range_i32(1, 12),
            a: rng.range_i32(1, 7),
            b: rng.range_i32(-6, 6),
            c: rng.range_i32(0, 5),
        });
    }

    for (idx, p) in cases.iter().enumerate() {
        let rr_path = proj_dir.join(format!("case_{idx:02}.rr"));
        let ref_path = proj_dir.join(format!("case_{idx:02}_ref.R"));
        fs::write(&rr_path, rr_source(*p)).expect("failed to write rr case");
        fs::write(&ref_path, reference_r_source(*p)).expect("failed to write reference case");

        let reference = run_rscript(&rscript, &ref_path);
        assert_eq!(
            reference.status, 0,
            "reference R failed for case {idx}: {p:?}\nstdout:\n{}\nstderr:\n{}",
            reference.stdout, reference.stderr
        );

        for (flag, tag) in [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")] {
            let out_path = proj_dir.join(format!("case_{idx:02}_{tag}.R"));
            compile_rr(&rr_bin, &rr_path, &out_path, flag);
            let compiled = run_rscript(&rscript, &out_path);

            assert_eq!(
                reference.status, compiled.status,
                "status mismatch case {idx} ({flag}) params={p:?}"
            );
            assert_eq!(
                normalize(&reference.stdout),
                normalize(&compiled.stdout),
                "stdout mismatch case {idx} ({flag}) params={p:?}\nref:\n{}\nrr:\n{}",
                reference.stdout,
                compiled.stdout
            );
            assert_eq!(
                normalize(&reference.stderr),
                normalize(&compiled.stderr),
                "stderr mismatch case {idx} ({flag}) params={p:?}\nref:\n{}\nrr:\n{}",
                reference.stderr,
                compiled.stderr
            );
        }
    }
}
