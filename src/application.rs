use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::platform::{Platform, ProcessName};
use crate::region::RegionSet;

/// Current source format version.
pub const SCHEMA_VERSION: u32 = 1;

/// Stable, validated application identifier.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ApplicationId(String);

impl ApplicationId {
    /// Parse one stable application identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` does not use the canonical lowercase slug syntax.
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            return Err("application ID must not be empty".to_owned());
        }
        let bytes = value.as_bytes();
        if !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        {
            return Err(
                "application ID must contain only ASCII lowercase letters, digits, and hyphens"
                    .to_owned(),
            );
        }
        if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
            return Err("application ID must start with a lowercase letter or digit".to_owned());
        }
        if !bytes[bytes.len() - 1].is_ascii_lowercase() && !bytes[bytes.len() - 1].is_ascii_digit()
        {
            return Err("application ID must end with a lowercase letter or digit".to_owned());
        }
        if value.contains("--") {
            return Err("application ID must not contain consecutive hyphens".to_owned());
        }
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for ApplicationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A fully validated application aggregate.
#[derive(Debug, Clone)]
pub struct Application {
    pub(crate) id: ApplicationId,
    pub(crate) display_name: Option<String>,
    pub(crate) category: String,
    pub(crate) source: PathBuf,
    pub(crate) regions: RegionSet,
    pub(crate) processes: BTreeMap<Platform, Vec<ProcessName>>,
}

impl Application {
    /// Stable application ID.
    #[must_use]
    pub const fn id(&self) -> &ApplicationId {
        &self.id
    }

    /// Optional human-facing display name.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Organizational source category derived from the TOML filename.
    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Source file containing this application.
    #[must_use]
    pub fn source(&self) -> &Path {
        &self.source
    }

    /// Validated egress region set.
    #[must_use]
    pub const fn regions(&self) -> &RegionSet {
        &self.regions
    }

    /// Exact names for one concrete platform.
    #[must_use]
    pub fn processes(&self, platform: Platform) -> &[ProcessName] {
        self.processes.get(&platform).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_identifiers_at_construction() {
        assert!(ApplicationId::parse("valid-id").is_ok());
        assert!(ApplicationId::parse("Invalid").is_err());
    }
}
