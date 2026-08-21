use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Severity of a source-data diagnostic.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Invalid data that prevents generation.
    Error,
    /// Suspicious but supported data.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => formatter.write_str("error"),
            Self::Warning => formatter.write_str("warning"),
        }
    }
}

/// A stable, structured validation diagnostic.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    /// Error or warning severity.
    pub severity: Severity,
    /// Stable machine-readable diagnostic code.
    pub code: &'static str,
    /// Source file associated with the diagnostic.
    pub file: PathBuf,
    /// Application ID, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    /// Field name, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// One-based source line, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// One-based source column, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Human-readable description with a corrective action where possible.
    pub message: String,
}

impl Diagnostic {
    fn new(
        severity: Severity,
        code: &'static str,
        file: &Path,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            file: file.to_path_buf(),
            application: None,
            field: None,
            line: None,
            column: None,
            message: message.into(),
        }
    }

    pub(crate) fn error(code: &'static str, file: &Path, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, file, message)
    }

    pub(crate) fn warning(code: &'static str, file: &Path, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, file, message)
    }

    pub(crate) fn application(mut self, application: impl Into<String>) -> Self {
        self.application = Some(application.into());
        self
    }

    pub(crate) fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub(crate) const fn position(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.file.display())?;
        if let Some(line) = self.line {
            write!(formatter, ":{line}")?;
            if let Some(column) = self.column {
                write!(formatter, ":{column}")?;
            }
        }
        write!(
            formatter,
            ": {}[{}]: {}",
            self.severity, self.code, self.message
        )?;
        if self.application.is_some() || self.field.is_some() {
            formatter.write_str(" (")?;
            if let Some(application) = &self.application {
                write!(formatter, "application={application}")?;
            }
            if let Some(field) = &self.field {
                if self.application.is_some() {
                    formatter.write_str(", ")?;
                }
                write!(formatter, "field={field}")?;
            }
            formatter.write_str(")")?;
        }
        Ok(())
    }
}
