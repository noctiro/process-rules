use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Operational errors returned by the library and CLI.
#[derive(Debug, Error)]
pub enum Error {
    /// Source validation failed. Individual diagnostics have already been emitted.
    #[error("source validation failed with {errors} error(s)")]
    ValidationFailed {
        /// Number of error diagnostics.
        errors: usize,
    },

    /// A collection identifier could not be parsed.
    #[error(
        "invalid collection `{0}`; expected any, only-<region>, contain-<region>, or not-contain-<region>"
    )]
    InvalidCollection(String),

    /// A syntactically valid collection is absent from the loaded catalog.
    #[error("collection `{0}` is not available because its region does not occur in the catalog")]
    UnavailableCollection(String),

    /// No rule matched an explain query.
    #[error("no process rule matched `{process}` on {platform}")]
    NotFound {
        /// Queried process name.
        process: String,
        /// Queried platform.
        platform: String,
    },

    /// An unsafe or unsupported output target was supplied.
    #[error("invalid output path `{path}`: {reason}")]
    InvalidOutput {
        /// Rejected output path.
        path: PathBuf,
        /// Rejection reason.
        reason: String,
    },

    /// A filesystem operation failed.
    #[error("failed to {operation} `{path}`: {source}")]
    Io {
        /// Operation being attempted.
        operation: &'static str,
        /// Affected path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// JSON manifest or diagnostic serialization failed.
    #[error("failed to serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub(crate) fn invalid_output(path: &Path, reason: impl Into<String>) -> Self {
        Self::InvalidOutput {
            path: path.to_path_buf(),
            reason: reason.into(),
        }
    }

    pub(crate) fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }

    /// Stable process exit code for this error category.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::ValidationFailed { .. } => 3,
            Self::InvalidCollection(_) | Self::UnavailableCollection(_) | Self::NotFound { .. } => {
                4
            }
            Self::InvalidOutput { .. } | Self::Io { .. } | Self::Json(_) => 1,
        }
    }
}
