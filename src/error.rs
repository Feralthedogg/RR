use crate::utils::Span;
use std::env;
use std::io::IsTerminal;

pub type RR<T> = Result<T, RRException>;

#[derive(Debug, Clone)]
pub enum RRCode {
    E0001,   // Unexpected Token
    E1001,   // Undefined Variable
    E1002,   // Type Mismatch
    E1003,   // Definite Assignment Violation
    E1010,   // Type Hint Conflict
    E1011,   // Call Signature Type Mismatch
    E1012,   // Unresolved Type In Strict Mode
    E1030,   // Parallel Safety Proof Failed (required mode)
    E1031,   // Parallel Backend Load/Call Failure (required mode)
    E1032,   // Non-deterministic Parallel Reduction Rejected
    E2001,   // Bound Check Failure
    E2007,   // Index out of bounds (logical)
    E3001,   // Unsupported Feature
    E9999,   // Internal Error (legacy)
    ICE9001, // Internal Compiler Error
}

impl RRCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::E0001 => "E0001",
            Self::E1001 => "E1001",
            Self::E1002 => "E1002",
            Self::E1003 => "E1003",
            Self::E1010 => "E1010",
            Self::E1011 => "E1011",
            Self::E1012 => "E1012",
            Self::E1030 => "E1030",
            Self::E1031 => "E1031",
            Self::E1032 => "E1032",
            Self::E2001 => "E2001",
            Self::E2007 => "E2007",
            Self::E3001 => "E3001",
            Self::E9999 => "E9999",
            Self::ICE9001 => "ICE9001",
        }
    }
}

impl std::str::FromStr for RRCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "E0001" => Ok(Self::E0001),
            "E1001" => Ok(Self::E1001),
            "E1002" => Ok(Self::E1002),
            "E1003" => Ok(Self::E1003),
            "E1010" => Ok(Self::E1010),
            "E1011" => Ok(Self::E1011),
            "E1012" => Ok(Self::E1012),
            "E1030" => Ok(Self::E1030),
            "E1031" => Ok(Self::E1031),
            "E1032" => Ok(Self::E1032),
            "E2001" => Ok(Self::E2001),
            "E2007" => Ok(Self::E2007),
            "E3001" => Ok(Self::E3001),
            "E9999" => Ok(Self::E9999),
            "ICE9001" => Ok(Self::ICE9001),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stage {
    Lex,
    Parse,
    Lower,
    Mir,
    Opt,
    Codegen,
    Runtime,
    Runner,
    Ice,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLabelKind {
    Primary,
    Origin,
    Constraint,
    Use,
}

impl DiagnosticLabelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Origin => "origin",
            Self::Constraint => "constraint",
            Self::Use => "use",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticLabel {
    pub kind: DiagnosticLabelKind,
    pub span: Span,
    pub message: Box<str>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticFix {
    pub message: Box<str>,
    pub span: Option<Span>,
    pub replacement: Option<Box<str>>,
}

#[derive(Debug, Clone)]
pub struct RRException(Box<RRExceptionData>);

#[derive(Debug, Clone)]
pub struct RRExceptionData {
    pub module: &'static str,
    pub message: Box<str>,
    pub code: RRCode,
    pub stage: Stage,
    pub span: Option<Span>,
    pub stacktrace: Box<Vec<Frame>>,
    pub notes: Box<Vec<String>>,
    pub helps: Box<Vec<String>>,
    pub labels: Box<Vec<DiagnosticLabel>>,
    pub fixes: Box<Vec<DiagnosticFix>>,
    pub related: Box<Vec<RRException>>,
}

#[derive(Debug, Clone)]
pub struct InternalCompilerError {
    pub stage: Stage,
    pub message: String,
    pub span: Option<Span>,
    pub notes: Vec<String>,
}

impl InternalCompilerError {
    pub fn new(stage: Stage, msg: impl Into<String>) -> Self {
        Self {
            stage,
            message: msg.into(),
            span: None,
            notes: Vec::new(),
        }
    }

    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn into_exception(self) -> RRException {
        self.into()
    }
}

impl From<InternalCompilerError> for RRException {
    fn from(err: InternalCompilerError) -> Self {
        let mut out = RRException::new("RR.InternalError", RRCode::ICE9001, err.stage, err.message);
        if let Some(span) = err.span {
            out = out.at(span);
        }
        for note in err.notes {
            out = out.note(note);
        }
        out
    }
}

impl RRException {
    pub fn new(module: &'static str, code: RRCode, stage: Stage, msg: impl Into<String>) -> Self {
        Self(Box::new(RRExceptionData {
            module,
            message: msg.into().into_boxed_str(),
            code,
            stage,
            span: None,
            stacktrace: Box::new(Vec::new()),
            notes: Box::new(Vec::new()),
            helps: Box::new(Vec::new()),
            labels: Box::new(Vec::new()),
            fixes: Box::new(Vec::new()),
            related: Box::new(Vec::new()),
        }))
    }

    pub fn aggregate(
        module: &'static str,
        code: RRCode,
        stage: Stage,
        msg: impl Into<String>,
        related: Vec<RRException>,
    ) -> Self {
        let mut out = Self::new(module, code, stage, msg);
        out.related = Box::new(related);
        out
    }

    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn push_frame(mut self, name: impl Into<String>, span: Option<Span>) -> Self {
        self.stacktrace.push(Frame {
            name: name.into(),
            span,
        });
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    pub fn label(
        mut self,
        kind: DiagnosticLabelKind,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        self.labels.push(DiagnosticLabel {
            kind,
            span,
            message: message.into().into_boxed_str(),
        });
        self
    }

    pub fn fix(mut self, message: impl Into<String>) -> Self {
        self.fixes.push(DiagnosticFix {
            message: message.into().into_boxed_str(),
            span: None,
            replacement: None,
        });
        self
    }

    pub fn replace(
        mut self,
        span: Span,
        replacement: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.fixes.push(DiagnosticFix {
            message: message.into().into_boxed_str(),
            span: Some(span),
            replacement: Some(replacement.into().into_boxed_str()),
        });
        self
    }

    pub fn internal(stage: Stage, msg: impl Into<String>) -> Self {
        InternalCompilerError::new(stage, msg).into_exception()
    }

    pub fn display(&self, source: Option<&str>, file: Option<&str>) {
        let color = color_enabled_stdout();
        let palette = palette_for_module(self.module);
        let code_color = palette_for_rr_code(self.code.as_str(), palette.code);
        let file = file.unwrap_or("RR");
        let at = if let Some(span) = self.span {
            format!("{}:{}:{}", file, span.start_line, span.start_col)
        } else {
            file.to_string()
        };
        println!(
            "{}",
            style(
                color,
                palette.header,
                &format!("** ({}) {}", self.module, self.message),
            )
        );
        println!(
            "{}",
            style(
                color,
                code_color,
                &format!("    error[{}]: {}", self.code.as_str(), self.message),
            )
        );
        println!(
            "{}",
            style(
                color,
                palette.at,
                &format!("    at {} ({})", at, self.stage_name()),
            )
        );

        if !self.related.is_empty() {
            println!(
                "{}",
                style(
                    color,
                    "1;93",
                    &format!("    found {} error(s)", self.related.len()),
                )
            );
            for (i, child) in self.related.iter().enumerate() {
                println!(
                    "{}",
                    style(
                        color,
                        "1;93",
                        &format!("    [{}] ------------------------------", i + 1),
                    )
                );
                child.display(source, Some(file));
            }
            return;
        }

        if let Some(src) = source {
            if let Some(span) = self.span {
                self.show_snippet(src, span, color, Some("primary"), palette.caret);
            }
            for label in self.labels.iter() {
                let label_color = match label.kind {
                    DiagnosticLabelKind::Primary => palette.caret,
                    DiagnosticLabelKind::Origin => "1;94",
                    DiagnosticLabelKind::Constraint => "1;38;5;208",
                    DiagnosticLabelKind::Use => "1;92",
                };
                println!(
                    "{}",
                    style(
                        color,
                        label_color,
                        &format!(
                            "    {}: {}:{}:{}: {}",
                            label.kind.as_str(),
                            file,
                            label.span.start_line,
                            label.span.start_col,
                            label.message
                        ),
                    )
                );
                self.show_snippet(
                    src,
                    label.span,
                    color,
                    Some(label.kind.as_str()),
                    label_color,
                );
            }
        }

        if !self.stacktrace.is_empty() {
            println!("{}", style(color, "1;95", "    stacktrace:"));
            for frame in self.stacktrace.iter().rev() {
                if let Some(span) = frame.span {
                    println!(
                        "{}",
                        style(
                            color,
                            "2",
                            &format!(
                                "      (rr) {} at {}:{}:{}",
                                frame.name, file, span.start_line, span.start_col
                            ),
                        )
                    );
                } else {
                    println!(
                        "{}",
                        style(color, "2", &format!("      (rr) {}", frame.name))
                    );
                }
            }
        }
        for n in self.notes.iter() {
            if n.to_ascii_lowercase().contains("r ") || n.to_ascii_lowercase().contains("r-") {
                println!(
                    "{}",
                    style(color, palette.note_r, &format!("note (R): {}", n))
                );
            } else {
                println!("{}", style(color, palette.hint, &format!("hint: {}", n)));
            }
        }
        for h in self.helps.iter() {
            println!("{}", style(color, "1;92", &format!("help: {}", h)));
        }
        for fix in self.fixes.iter() {
            let mut line = format!("fix: {}", fix.message);
            if let Some(span) = fix.span {
                line.push_str(&format!(
                    " @ {}:{}:{}",
                    file, span.start_line, span.start_col
                ));
            }
            if let Some(replacement) = &fix.replacement {
                line.push_str(&format!(" => `{}`", replacement));
            }
            println!("{}", style(color, "1;92", &line));
        }
    }

    fn stage_name(&self) -> &'static str {
        match self.stage {
            Stage::Lex => "Lex",
            Stage::Parse => "Parse",
            Stage::Lower => "Lower",
            Stage::Mir => "MIR",
            Stage::Opt => "Opt",
            Stage::Codegen => "Codegen",
            Stage::Runtime => "Runtime",
            Stage::Runner => "Runner",
            Stage::Ice => "ICE",
        }
    }

    fn show_snippet(
        &self,
        source: &str,
        span: Span,
        color: bool,
        label: Option<&str>,
        caret_color: &str,
    ) {
        let lines: Vec<&str> = source.lines().collect();
        if span.start_line > 0 && span.start_line as usize <= lines.len() {
            let line_idx = (span.start_line - 1) as usize;
            let line = lines[line_idx];
            println!(
                "{}",
                style(color, "2", &format!("{:>4} | {}", span.start_line, line))
            );
            let indent = " ".repeat(span.start_col as usize + 6);
            let caret = match label {
                Some(label) => format!("{}^ [{}]", indent, label),
                None => format!("{}^", indent),
            };
            println!("{}", style(color, caret_color, &caret));
        }
    }
}

impl std::ops::Deref for RRException {
    type Target = RRExceptionData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RRException {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct ErrorPalette {
    header: &'static str,
    code: &'static str,
    at: &'static str,
    caret: &'static str,
    note_r: &'static str,
    hint: &'static str,
}

fn palette_for_module(module: &str) -> ErrorPalette {
    if module.contains("Warning") {
        ErrorPalette {
            header: "1;38;5;208",
            code: "1;38;5;208",
            at: "1;96",
            caret: "1;38;5;208",
            note_r: "1;94",
            hint: "1;92",
        }
    } else if module.contains("ParseError") || module.contains("LexError") {
        ErrorPalette {
            header: "1;95",
            code: "1;35",
            at: "1;96",
            caret: "1;95",
            note_r: "1;94",
            hint: "1;92",
        }
    } else if module.contains("TypeError") || module.contains("SemanticError") {
        ErrorPalette {
            header: "1;93",
            code: "1;33",
            at: "1;96",
            caret: "1;93",
            note_r: "1;94",
            hint: "1;92",
        }
    } else if module.contains("OptError") {
        ErrorPalette {
            header: "1;96",
            code: "1;36",
            at: "1;96",
            caret: "1;96",
            note_r: "1;94",
            hint: "1;92",
        }
    } else if module.contains("CodegenError") {
        ErrorPalette {
            header: "1;94",
            code: "1;34",
            at: "1;96",
            caret: "1;94",
            note_r: "1;94",
            hint: "1;92",
        }
    } else if module.contains("RunnerError") {
        ErrorPalette {
            header: "1;35",
            code: "1;35",
            at: "1;96",
            caret: "1;35",
            note_r: "1;94",
            hint: "1;92",
        }
    } else if module.contains("RuntimeError")
        || module.contains("BoundsError")
        || module.contains("ValueError")
    {
        ErrorPalette {
            header: "1;91",
            code: "1;31",
            at: "1;96",
            caret: "1;91",
            note_r: "1;94",
            hint: "1;92",
        }
    } else if module.contains("InternalError") || module.contains("ICE") {
        ErrorPalette {
            header: "1;97;41",
            code: "1;97;41",
            at: "1;96",
            caret: "1;97;41",
            note_r: "1;94",
            hint: "1;92",
        }
    } else {
        ErrorPalette {
            header: "1;91",
            code: "1;93",
            at: "1;96",
            caret: "1;91",
            note_r: "1;94",
            hint: "1;92",
        }
    }
}

fn palette_for_rr_code<'a>(code: &'a str, fallback: &'a str) -> &'a str {
    if code.starts_with("ICE") || code == "E9999" {
        "1;97;41"
    } else if code.starts_with("E0") {
        "1;35"
    } else if code.starts_with("E1") {
        "1;33"
    } else if code.starts_with("E2") {
        "1;31"
    } else if code.starts_with("E3") {
        "1;38;5;208"
    } else {
        fallback
    }
}

fn color_enabled_stdout() -> bool {
    let no_color = env::var_os("NO_COLOR").is_some();
    let force_color = env::var_os("RR_FORCE_COLOR").is_some();
    let is_tty = std::io::stdout().is_terminal();
    (force_color || is_tty) && !no_color
}

fn style(color: bool, code: &str, text: &str) -> String {
    if color {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}

#[macro_export]
macro_rules! bail {
    ($mod:expr, $code:expr, $stage:expr, $($arg:tt)*) => {
        return Err($crate::error::RRException::new($mod, $code, $stage, format!($($arg)*)))
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $mod:expr, $code:expr, $stage:expr, $($arg:tt)*) => {
        if !($cond) {
            return Err($crate::error::RRException::new($mod, $code, $stage, format!($($arg)*)))
        }
    };
}

#[macro_export]
macro_rules! bail_at {
    ($span:expr, $mod:expr, $code:expr, $stage:expr, $($arg:tt)*) => {
        return Err($crate::error::RRException::new($mod, $code, $stage, format!($($arg)*)).at($span))
    };
}

#[macro_export]
macro_rules! ensure_at {
    ($cond:expr, $span:expr, $mod:expr, $code:expr, $stage:expr, $($arg:tt)*) => {
        if !($cond) {
            return Err($crate::error::RRException::new($mod, $code, $stage, format!($($arg)*)).at($span))
        }
    };
}

pub trait RRCtx<T> {
    fn ctx(self, name: &'static str, span: Option<Span>) -> RR<T>;
}

impl<T> RRCtx<T> for RR<T> {
    fn ctx(self, name: &'static str, span: Option<Span>) -> RR<T> {
        self.map_err(|e| e.push_frame(name, span))
    }
}
