#[derive(Clone, Debug)]
pub struct GeneratedErrorCase {
    #[allow(dead_code)]
    pub family: &'static str,
    pub name: String,
    pub rr_src: String,
    pub expected_module: &'static str,
    pub expected_message_fragment: String,
    pub expected_help_fragment: Option<String>,
    pub strict_let: bool,
}

#[derive(Clone, Copy, Debug)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn pick<'a>(&mut self, values: &'a [&'a str]) -> &'a str {
        let idx = (self.next_u32() as usize) % values.len();
        values[idx]
    }

    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi - lo + 1) as u32;
        lo + (self.next_u32() % span) as i32
    }
}

pub fn generate_cases(seed: u64, count: usize) -> Vec<GeneratedErrorCase> {
    let mut rng = Lcg::new(seed);
    let mut cases = Vec::with_capacity(count);
    for idx in 0..count {
        let family = idx % 5;
        let case = match family {
            0 => undefined_variable_case(idx, &mut rng),
            1 => undefined_function_case(idx, &mut rng),
            2 => arity_mismatch_case(idx, &mut rng),
            3 => strict_let_case(idx, &mut rng),
            _ => parse_missing_brace_case(idx, &mut rng),
        };
        cases.push(case);
    }
    cases
}

#[allow(dead_code)]
pub fn suite_summary(cases: &[GeneratedErrorCase]) -> String {
    let mut families = Vec::new();
    for case in cases {
        if !families.contains(&case.family) {
            families.push(case.family);
        }
    }
    families.join(",")
}

fn swap_adjacent(name: &str, salt: u32) -> String {
    let mut chars: Vec<char> = name.chars().collect();
    if chars.len() < 2 {
        return format!("{name}x");
    }
    let start = salt as usize % (chars.len() - 1);
    for offset in 0..(chars.len() - 1) {
        let idx = (start + offset) % (chars.len() - 1);
        if chars[idx] == chars[idx + 1] {
            continue;
        }
        chars.swap(idx, idx + 1);
        return chars.into_iter().collect();
    }
    format!("{name}x")
}

fn undefined_variable_case(idx: usize, rng: &mut Lcg) -> GeneratedErrorCase {
    let bindings = ["total", "result", "buffer", "sample", "matrix", "score"];
    let binding = rng.pick(&bindings);
    let typo = swap_adjacent(binding, rng.next_u32());
    let init = rng.range_i32(1, 9);
    let name = format!("undefined_variable_{idx:02}");
    let rr_src = format!(
        r#"
fn main() {{
  let {binding} = {init}L
  return {typo} + {binding}
}}
main()
"#
    );
    GeneratedErrorCase {
        family: "undefined_variable",
        name,
        rr_src,
        expected_module: "RR.SemanticError",
        expected_message_fragment: format!("undefined variable '{typo}'"),
        expected_help_fragment: Some(binding.to_string()),
        strict_let: false,
    }
}

fn undefined_function_case(idx: usize, rng: &mut Lcg) -> GeneratedErrorCase {
    let builtins = ["print", "length"];
    let builtin = rng.pick(&builtins);
    let typo = swap_adjacent(builtin, rng.next_u32());
    let arg = rng.range_i32(1, 9);
    let name = format!("undefined_function_{idx:02}");
    let rr_src = format!(
        r#"
fn main() {{
  return {typo}({arg}L)
}}
main()
"#
    );
    GeneratedErrorCase {
        family: "undefined_function",
        name,
        rr_src,
        expected_module: "RR.SemanticError",
        expected_message_fragment: format!("undefined function '{typo}'"),
        expected_help_fragment: Some(builtin.to_string()),
        strict_let: false,
    }
}

fn arity_mismatch_case(idx: usize, rng: &mut Lcg) -> GeneratedErrorCase {
    let fn_names = ["combine", "foldit", "blend", "project", "mixup"];
    let fn_name = rng.pick(&fn_names);
    let arity = rng.range_i32(2, 4) as usize;
    let got = arity - 1;
    let params: Vec<String> = (0..arity).map(|n| format!("v{}", n + 1)).collect();
    let args: Vec<String> = (0..got).map(|n| format!("{}L", n + 1)).collect();
    let mut expr = String::new();
    for (idx, param) in params.iter().enumerate() {
        if idx > 0 {
            expr.push_str(" + ");
        }
        expr.push_str(param);
    }
    let name = format!("arity_mismatch_{idx:02}");
    let rr_src = format!(
        r#"
fn {fn_name}({params}) {{
  return {expr}
}}

fn main() {{
  return {fn_name}({args})
}}
main()
"#,
        params = params.join(", "),
        args = args.join(", ")
    );
    GeneratedErrorCase {
        family: "arity_mismatch",
        name,
        rr_src,
        expected_module: "RR.SemanticError",
        expected_message_fragment: format!(
            "function '{fn_name}' expects {arity} argument(s), got {got}"
        ),
        expected_help_fragment: None,
        strict_let: false,
    }
}

fn strict_let_case(idx: usize, rng: &mut Lcg) -> GeneratedErrorCase {
    let bindings = ["total", "result", "buffer", "sample", "matrix", "score"];
    let binding = rng.pick(&bindings);
    let typo = swap_adjacent(binding, rng.next_u32());
    let init = rng.range_i32(1, 9);
    let assigned = rng.range_i32(2, 18);
    let name = format!("strict_let_{idx:02}");
    let rr_src = format!(
        r#"
fn main() {{
  let {binding} = {init}L
  {typo} <- {assigned}L
  return {binding}
}}
main()
"#
    );
    GeneratedErrorCase {
        family: "strict_let",
        name,
        rr_src,
        expected_module: "RR.SemanticError",
        expected_message_fragment: format!("assignment to undeclared variable '{typo}'"),
        expected_help_fragment: Some(binding.to_string()),
        strict_let: true,
    }
}

fn parse_missing_brace_case(idx: usize, rng: &mut Lcg) -> GeneratedErrorCase {
    let bindings = ["total", "result", "buffer", "sample", "matrix", "score"];
    let binding = rng.pick(&bindings);
    let init = rng.range_i32(1, 9);
    let name = format!("parse_missing_brace_{idx:02}");
    let rr_src = format!(
        r#"
fn main() {{
  let {binding} = {init}L
  return {binding}
"#
    );
    GeneratedErrorCase {
        family: "parse_missing_brace",
        name,
        rr_src,
        expected_module: "RR.ParseError",
        expected_message_fragment: "Expected RBrace, got EOF".to_string(),
        expected_help_fragment: None,
        strict_let: false,
    }
}
