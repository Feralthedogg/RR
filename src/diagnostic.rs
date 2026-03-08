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
    if errors.is_empty() {
        return Ok(());
    }
    if errors.len() == 1 {
        return Err(errors.remove(0));
    }
    Err(RRException::aggregate(module, code, stage, summary, errors))
}
