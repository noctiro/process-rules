use std::fmt;
use std::str::FromStr;

use crate::text::contains_control_or_line_separator;

/// Platform whose process identifier syntax is represented by a source field.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Platform {
    /// Microsoft Windows executable basename.
    Windows,
    /// Android application package name.
    Android,
    /// Linux process basename.
    Linux,
    /// macOS executable basename.
    Macos,
}

impl Platform {
    /// All supported concrete platforms in stable output order.
    pub const ALL: [Self; 4] = [Self::Windows, Self::Android, Self::Linux, Self::Macos];

    /// Canonical CLI, source-field, and path name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Android => "android",
            Self::Linux => "linux",
            Self::Macos => "macos",
        }
    }

    pub(super) fn check_name(self, name: &ProcessName) -> PlatformNameStatus {
        match self {
            Self::Android if !is_valid_android_package(name.as_str()) => {
                PlatformNameStatus::Error {
                    code: "invalid-android-package",
                    message: format!("`{name}` is not a valid Android package name"),
                }
            }
            Self::Windows if !name.as_str().to_ascii_lowercase().ends_with(".exe") => {
                PlatformNameStatus::Warning {
                    code: "windows-missing-exe",
                    message: format!("Windows process `{name}` does not end in `.exe`"),
                }
            }
            Self::Windows | Self::Android | Self::Linux | Self::Macos => PlatformNameStatus::Valid,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|platform| platform.as_str() == value)
            .ok_or_else(|| "expected windows, android, linux, or macos".to_owned())
    }
}

/// A concrete platform or a combined cross-platform view.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlatformSelection {
    /// One supported operating system.
    Platform(Platform),
    /// Union of all supported platforms.
    All,
}

impl PlatformSelection {
    /// Every supported platform selection in stable output order.
    pub const ALL: [Self; 5] = [
        Self::Platform(Platform::Windows),
        Self::Platform(Platform::Android),
        Self::Platform(Platform::Linux),
        Self::Platform(Platform::Macos),
        Self::All,
    ];

    /// Canonical CLI and artifact path name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Platform(platform) => platform.as_str(),
            Self::All => "all",
        }
    }

    pub(crate) const fn platforms(self) -> &'static [Platform] {
        match self {
            Self::Platform(Platform::Windows) => &[Platform::Windows],
            Self::Platform(Platform::Android) => &[Platform::Android],
            Self::Platform(Platform::Linux) => &[Platform::Linux],
            Self::Platform(Platform::Macos) => &[Platform::Macos],
            Self::All => &Platform::ALL,
        }
    }
}

impl fmt::Display for PlatformSelection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PlatformSelection {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "all" {
            Ok(Self::All)
        } else {
            value
                .parse()
                .map(Self::Platform)
                .map_err(|_| "expected windows, android, linux, macos, or all".to_owned())
        }
    }
}

/// Exact process name or Android package name accepted by the source schema.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProcessName(String);

impl ProcessName {
    /// Parse one exact process basename or Android package name.
    ///
    /// # Errors
    ///
    /// Returns an error when the value cannot be represented safely by a Mihomo
    /// classical `PROCESS-NAME` rule.
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            return Err("process name must not be empty".to_owned());
        }
        if value.trim() != value {
            return Err("process name must not start or end with whitespace".to_owned());
        }
        if value.contains(',') {
            return Err("process name must not contain a comma because Mihomo uses commas as field separators".to_owned());
        }
        if value.contains('/') || value.contains('\\') {
            return Err("process name must be a basename, not a path".to_owned());
        }
        if contains_control_or_line_separator(value) {
            return Err(
                "process name must not contain control characters or line separators".to_owned(),
            );
        }
        Ok(Self(value.to_owned()))
    }

    /// Borrow the original, case-preserving process name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProcessName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub enum PlatformNameStatus {
    Valid,
    Warning { code: &'static str, message: String },
    Error { code: &'static str, message: String },
}

fn is_valid_android_package(value: &str) -> bool {
    let mut count = 0;
    for segment in value.split('.') {
        count += 1;
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !first.is_ascii_alphanumeric() {
            return false;
        }
        if !chars.all(|character| character.is_ascii_alphanumeric() || character == '_') {
            return false;
        }
    }
    count >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_process_names() {
        assert!(ProcessName::parse("directory/program").is_err());
        assert!(ProcessName::parse("first\u{2028}second").is_err());
        assert!(ProcessName::parse("valid process").is_ok());
    }

    #[test]
    fn validates_android_package_segments() {
        assert!(is_valid_android_package("com.example.app_2"));
        assert!(is_valid_android_package("com.115.android"));
        assert!(!is_valid_android_package("example"));
        assert!(!is_valid_android_package("com.example-app"));
    }
}
