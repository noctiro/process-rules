//! Core library for loading, validating, selecting, and exporting process rules.

mod application;
mod catalog;
mod collection;
mod diagnostic;
mod error;
mod loader;
mod output;
mod platform;
mod region;
mod text;

pub use application::{Application, ApplicationId, SCHEMA_VERSION};
pub use catalog::{Catalog, Explanation};
pub use collection::{Collection, RegionRelation};
pub use diagnostic::{Diagnostic, Severity};
pub use error::Error;
pub use loader::{ValidationOutcome, load_catalog};
pub use output::{ArtifactManifest, BuildManifest, build_all, render_mihomo, write_atomic_file};
pub use platform::{Platform, PlatformSelection, ProcessName};
pub use region::{Region, RegionSet};

#[cfg(test)]
pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
