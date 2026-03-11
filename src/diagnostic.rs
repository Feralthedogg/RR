use crate::error::{DiagnosticLabelKind, RR, RRCode, RRException, Stage};
use crate::utils::Span;

pub struct DiagnosticBuilder {
    inner: RRException,
}

impl DiagnosticBuilder {
    pub fn new(
        module: &'static str,
        code: RRCode,
        stage: Stage,
        message: impl Into<String>,
    ) -> Self {
        Self {
            inner: RRException::new(module, code, stage, message),
        }
    }

    pub fn at(mut self, span: Span) -> Self {
        self.inner = self.inner.at(span);
        self
    }

    pub fn primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .label(DiagnosticLabelKind::Primary, span, message);
        self
    }

    pub fn origin(mut self, span: Span, message: impl Into<String>) -> Self {
        self.inner = self.inner.label(DiagnosticLabelKind::Origin, span, message);
        self
    }

    pub fn constraint(mut self, span: Span, message: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .label(DiagnosticLabelKind::Constraint, span, message);
        self
    }

    pub fn use_site(mut self, span: Span, message: impl Into<String>) -> Self {
        self.inner = self.inner.label(DiagnosticLabelKind::Use, span, message);
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.inner = self.inner.note(note);
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.inner = self.inner.help(help);
        self
    }

    pub fn fix(mut self, message: impl Into<String>) -> Self {
        self.inner = self.inner.fix(message);
        self
    }

    pub fn replace(
        mut self,
        span: Span,
        replacement: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.inner = self.inner.replace(span, replacement, message);
        self
    }

    pub fn build(self) -> RRException {
        self.inner
    }
}

pub fn finish_diagnostics(
    module: &'static str,
    code: RRCode,
    stage: Stage,
    summary: impl Into<String>,
    mut errors: Vec<RRException>,
) -> RR<()> {
    normalize_diagnostics(&mut errors);
    if errors.is_empty() {
        return Ok(());
    }
    if errors.len() == 1 {
        return Err(errors.remove(0));
    }
    Err(RRException::aggregate(module, code, stage, summary, errors))
}

fn stage_rank(stage: &Stage) -> u8 {
    match stage {
        Stage::Lex => 0,
        Stage::Parse => 1,
        Stage::Lower => 2,
        Stage::Mir => 3,
        Stage::Opt => 4,
        Stage::Codegen => 5,
        Stage::Runtime => 6,
        Stage::Runner => 7,
        Stage::Ice => 8,
    }
}

fn diagnostic_sort_key(err: &RRException) -> (u8, u32, u32, &'static str, u8, String) {
    let (has_span, line, col) = match err.span {
        Some(span) => (0, span.start_line, span.start_col),
        None => (1, u32::MAX, u32::MAX),
    };
    (
        has_span,
        line,
        col,
        err.module,
        stage_rank(&err.stage),
        format!("{}:{}", err.code.as_str(), err.message),
    )
}

fn normalize_diagnostics(errors: &mut Vec<RRException>) {
    errors.sort_by_key(diagnostic_sort_key);
    errors.dedup_by(|a, b| diagnostic_sort_key(a) == diagnostic_sort_key(b));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_diagnostics_sorts_and_dedups() {
        let a = RRException::new("RR.RuntimeError", RRCode::E2001, Stage::Mir, "b")
            .at(Span::new(0, 0, 3, 1, 3, 2));
        let a_dup = RRException::new("RR.RuntimeError", RRCode::E2001, Stage::Mir, "b")
            .at(Span::new(0, 0, 3, 1, 3, 2));
        let b = RRException::new("RR.RuntimeError", RRCode::E2007, Stage::Mir, "a")
            .at(Span::new(0, 0, 1, 1, 1, 2));
        let err = finish_diagnostics(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "summary",
            vec![a, a_dup, b],
        )
        .expect_err("expected aggregated diagnostics");
        assert_eq!(err.related.len(), 2);
        assert_eq!(err.related[0].message.as_ref(), "a");
        assert_eq!(err.related[1].message.as_ref(), "b");
    }
}
