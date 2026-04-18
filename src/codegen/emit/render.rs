use super::*;

pub(super) fn build_named_arg_list(
    this: &RBackend,
    args: &[usize],
    names: &[Option<String>],
    values: &[Value],
    params: &[String],
) -> String {
    let mut out = String::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let value = this.resolve_preferred_scalar_call_arg_expr(*a, values, params);
        if let Some(Some(name)) = names.get(i) {
            out.push_str(name);
            out.push_str(" = ");
            out.push_str(&value);
        } else {
            out.push_str(&value);
        }
    }
    out
}

pub(super) fn build_plain_arg_list(
    this: &RBackend,
    args: &[usize],
    values: &[Value],
    params: &[String],
) -> String {
    let mut out = String::new();
    for (idx, arg) in args.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&this.resolve_preferred_scalar_call_arg_expr(*arg, values, params));
    }
    out
}

pub(super) fn intrinsic_helper(op: IntrinsicOp) -> &'static str {
    match op {
        IntrinsicOp::VecAddF64 => "rr_intrinsic_vec_add_f64",
        IntrinsicOp::VecSubF64 => "rr_intrinsic_vec_sub_f64",
        IntrinsicOp::VecMulF64 => "rr_intrinsic_vec_mul_f64",
        IntrinsicOp::VecDivF64 => "rr_intrinsic_vec_div_f64",
        IntrinsicOp::VecAbsF64 => "rr_intrinsic_vec_abs_f64",
        IntrinsicOp::VecLogF64 => "rr_intrinsic_vec_log_f64",
        IntrinsicOp::VecSqrtF64 => "rr_intrinsic_vec_sqrt_f64",
        IntrinsicOp::VecPmaxF64 => "rr_intrinsic_vec_pmax_f64",
        IntrinsicOp::VecPminF64 => "rr_intrinsic_vec_pmin_f64",
        IntrinsicOp::VecSumF64 => "rr_intrinsic_vec_sum_f64",
        IntrinsicOp::VecMeanF64 => "rr_intrinsic_vec_mean_f64",
    }
}

pub(super) fn binary_op_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        BinOp::MatMul => "%*%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&",
        BinOp::Or => "|",
    }
}

pub(super) fn unary_op_str(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
        UnaryOp::Formula => "~",
    }
}

pub(super) fn emit_lit(this: &RBackend, lit: &Lit) -> String {
    match lit {
        Lit::Int(i) => format!("{}L", i),
        Lit::Float(f) => emit_float_lit(this, *f),
        Lit::Str(s) => format!("\"{}\"", s),
        Lit::Bool(true) => "TRUE".to_string(),
        Lit::Bool(false) => "FALSE".to_string(),
        Lit::Null => "NULL".to_string(),
        Lit::Na => "NA".to_string(),
    }
}

pub(super) fn emit_lit_with_value(this: &RBackend, lit: &Lit, value: &Value) -> String {
    match lit {
        Lit::Float(f)
            if value.value_ty.prim == PrimTy::Double
                || matches!(value.value_term, TypeTerm::Double) =>
        {
            emit_float_lit(this, *f)
        }
        _ => emit_lit(this, lit),
    }
}

pub(super) fn emit_float_lit(_this: &RBackend, value: f64) -> String {
    let mut rendered = value.to_string();
    if value.is_finite() && !rendered.contains(['.', 'e', 'E']) {
        rendered.push_str(".0");
    }
    rendered
}

pub(super) fn emit_mark(this: &mut RBackend, span: Span, label: Option<&str>) {
    if span.start_line == 0 {
        return;
    }
    this.write_indent();
    let _ = label;
    this.write(&format!(
        "rr_mark({}L, {}L);",
        span.start_line, span.start_col
    ));
    this.newline();
}
