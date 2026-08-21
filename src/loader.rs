use std::collections::{BTreeMap, HashMap, HashSet, btree_map::Entry};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use unicase::UniCase;

use crate::application::{Application, ApplicationId, SCHEMA_VERSION};
use crate::catalog::Catalog;
use crate::diagnostic::{Diagnostic, Severity};
use crate::error::Error;
use crate::platform::{Platform, PlatformNameStatus, ProcessName};
use crate::region::RegionSet;
use crate::text::validate_display_name;

#[derive(Debug, Deserialize)]
struct RawRuleFile {
    version: u32,
    #[serde(flatten)]
    applications: BTreeMap<String, RawApplication>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawApplication {
    display_name: Option<String>,
    regions: RawRegions,
    windows: Option<Vec<String>>,
    android: Option<Vec<String>>,
    linux: Option<Vec<String>>,
    macos: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawRegions {
    Any(String),
    Only(Vec<String>),
    Except(RawRegionExceptions),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRegionExceptions {
    except: Vec<String>,
}

/// Result of loading every source file, including warnings and recoverable errors.
#[derive(Debug)]
pub struct ValidationOutcome {
    catalog: Option<Catalog>,
    diagnostics: Vec<Diagnostic>,
}

impl ValidationOutcome {
    fn diagnostic_count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == severity)
            .count()
    }

    /// Validated catalog, available only when there are no error diagnostics.
    #[must_use]
    pub const fn catalog(&self) -> Option<&Catalog> {
        self.catalog.as_ref()
    }

    /// All diagnostics in deterministic source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Number of validation errors.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostic_count(Severity::Error)
    }

    /// Number of non-fatal warnings.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.diagnostic_count(Severity::Warning)
    }

    /// Consume the outcome and return the catalog or a summarized validation error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationFailed`] when one or more error diagnostics exist.
    pub fn into_catalog(self) -> Result<Catalog, Error> {
        let errors = self.error_count();
        self.catalog.ok_or(Error::ValidationFailed { errors })
    }
}

/// Load and validate all direct `*.toml` children of a rules directory.
#[must_use]
pub fn load_catalog(rules_dir: &Path, require_non_empty: bool) -> ValidationOutcome {
    let mut diagnostics = Vec::new();
    let files = discover_rule_files(rules_dir, &mut diagnostics);
    let mut applications = Vec::new();
    let mut seen_ids: BTreeMap<ApplicationId, PathBuf> = BTreeMap::new();

    for file in files {
        load_rule_file(&file, &mut applications, &mut seen_ids, &mut diagnostics);
    }
    applications.sort_by(|left, right| left.id().cmp(right.id()));

    validate_process_collisions(&applications, &mut diagnostics);

    let mut has_errors = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    if require_non_empty && !has_errors && applications.is_empty() {
        diagnostics.push(Diagnostic::error(
            "empty-catalog",
            rules_dir,
            "rules directory must contain at least one valid application",
        ));
        has_errors = true;
    }

    ValidationOutcome {
        catalog: (!has_errors).then_some(Catalog { applications }),
        diagnostics,
    }
}

fn discover_rule_files(rules_dir: &Path, diagnostics: &mut Vec<Diagnostic>) -> Vec<PathBuf> {
    let entries = match fs::read_dir(rules_dir) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "rules-dir-unreadable",
                rules_dir,
                format!("cannot read rules directory: {error}"),
            ));
            return Vec::new();
        }
    };

    let mut files = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path
                    .extension()
                    .is_some_and(|extension| extension == "toml")
                {
                    files.push(path);
                }
            }
            Err(error) => diagnostics.push(Diagnostic::error(
                "rules-dir-entry-unreadable",
                rules_dir,
                format!("cannot inspect a rules directory entry: {error}"),
            )),
        }
    }
    files.sort();
    files
}

fn load_rule_file(
    file: &Path,
    applications: &mut Vec<Application>,
    seen_ids: &mut BTreeMap<ApplicationId, PathBuf>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(category) = file.file_stem().and_then(|stem| stem.to_str()) else {
        diagnostics.push(Diagnostic::error(
            "invalid-category",
            file,
            "rule filename must be valid UTF-8",
        ));
        return;
    };
    if let Err(message) = ApplicationId::parse(category) {
        diagnostics.push(Diagnostic::error(
            "invalid-category",
            file,
            format!("invalid category filename: {message}"),
        ));
        return;
    }

    let source = match fs::read_to_string(file) {
        Ok(source) => source,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "rule-file-unreadable",
                file,
                format!("cannot read rule file as UTF-8: {error}"),
            ));
            return;
        }
    };

    let raw: RawRuleFile = match toml::from_str(&source) {
        Ok(raw) => raw,
        Err(error) => {
            let mut diagnostic = Diagnostic::error("invalid-toml", file, error.message());
            if let Some(span) = error.span() {
                let (line, column) = byte_offset_position(&source, span.start);
                diagnostic = diagnostic.position(line, column);
            }
            diagnostics.push(diagnostic);
            return;
        }
    };

    if raw.version != SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "unsupported-version",
            file,
            format!(
                "source version {} is unsupported; expected {SCHEMA_VERSION}",
                raw.version
            ),
        ));
    }

    for (raw_id, raw_application) in raw.applications {
        let id = match ApplicationId::parse(&raw_id) {
            Ok(id) => id,
            Err(message) => {
                diagnostics.push(
                    Diagnostic::error("invalid-application-id", file, message).application(raw_id),
                );
                continue;
            }
        };

        let duplicate = match seen_ids.entry(id.clone()) {
            Entry::Occupied(entry) => {
                diagnostics.push(
                    Diagnostic::error(
                        "duplicate-application-id",
                        file,
                        format!(
                            "application ID `{id}` was already defined in `{}`",
                            entry.get().display()
                        ),
                    )
                    .application(id.to_string()),
                );
                true
            }
            Entry::Vacant(entry) => {
                entry.insert(file.to_path_buf());
                false
            }
        };

        let application = validate_application(id, category, file, raw_application, diagnostics);
        if !duplicate && let Some(application) = application {
            applications.push(application);
        }
    }
}

fn validate_application(
    id: ApplicationId,
    category: &str,
    file: &Path,
    raw: RawApplication,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Application> {
    let RawApplication {
        display_name,
        regions,
        windows,
        android,
        linux,
        macos,
    } = raw;
    let platforms = [
        (Platform::Windows, windows),
        (Platform::Android, android),
        (Platform::Linux, linux),
        (Platform::Macos, macos),
    ];

    let display_name_valid = match display_name
        .as_deref()
        .map_or(Ok(()), validate_display_name)
    {
        Ok(()) => true,
        Err(message) => {
            diagnostics.push(
                Diagnostic::error("invalid-display-name", file, message)
                    .application(id.to_string())
                    .field("display_name"),
            );
            false
        }
    };

    let region_set = match validate_regions(regions) {
        Ok(region_set) => Some(region_set),
        Err(message) => {
            diagnostics.push(
                Diagnostic::error("invalid-regions", file, message)
                    .application(id.to_string())
                    .field("regions"),
            );
            None
        }
    };

    let has_process_name = platforms
        .iter()
        .any(|(_, names)| names.as_ref().is_some_and(|names| !names.is_empty()));
    if !has_process_name {
        diagnostics.push(
            Diagnostic::error(
                "missing-process-name",
                file,
                "application must define at least one process name on one platform",
            )
            .application(id.to_string()),
        );
    }

    let mut processes = BTreeMap::new();
    let mut process_names_valid = true;
    for (platform, raw_names) in platforms {
        process_names_valid &=
            validate_platform_names(file, &id, platform, raw_names, diagnostics, &mut processes);
    }

    let regions = region_set?;
    if !display_name_valid || !has_process_name || !process_names_valid {
        return None;
    }

    Some(Application {
        id,
        display_name,
        category: category.to_owned(),
        source: file.to_path_buf(),
        regions,
        processes,
    })
}

fn validate_regions(raw: RawRegions) -> Result<RegionSet, String> {
    match raw {
        RawRegions::Any(value) if value == "any" => Ok(RegionSet::any()),
        RawRegions::Any(value) => Err(format!("regions string must be `any`, found `{value}`")),
        RawRegions::Only(values) => RegionSet::parse_only(&values),
        RawRegions::Except(raw) => RegionSet::parse_except(&raw.except),
    }
}

fn validate_platform_names(
    file: &Path,
    id: &ApplicationId,
    platform: Platform,
    raw_names: Option<Vec<String>>,
    diagnostics: &mut Vec<Diagnostic>,
    processes: &mut BTreeMap<Platform, Vec<ProcessName>>,
) -> bool {
    let Some(raw_names) = raw_names else {
        return true;
    };
    if raw_names.is_empty() {
        diagnostics.push(
            Diagnostic::error(
                "empty-platform-list",
                file,
                format!("{platform} must be omitted instead of set to an empty array"),
            )
            .application(id.to_string())
            .field(platform.as_str()),
        );
        return false;
    }

    let mut names = Vec::with_capacity(raw_names.len());
    let mut valid = true;
    for raw_name in raw_names {
        let name = match ProcessName::parse(&raw_name) {
            Ok(name) => name,
            Err(message) => {
                diagnostics.push(
                    Diagnostic::error("invalid-process-name", file, message)
                        .application(id.to_string())
                        .field(platform.as_str()),
                );
                valid = false;
                continue;
            }
        };

        match platform.check_name(&name) {
            PlatformNameStatus::Valid => names.push(name),
            PlatformNameStatus::Warning { code, message } => {
                diagnostics.push(
                    Diagnostic::warning(code, file, message)
                        .application(id.to_string())
                        .field(platform.as_str()),
                );
                names.push(name);
            }
            PlatformNameStatus::Error { code, message } => {
                diagnostics.push(
                    Diagnostic::error(code, file, message)
                        .application(id.to_string())
                        .field(platform.as_str()),
                );
                valid = false;
            }
        }
    }
    if !names.is_empty() {
        processes.insert(platform, names);
    }
    valid
}

fn validate_process_collisions(applications: &[Application], diagnostics: &mut Vec<Diagnostic>) {
    let mut per_platform: HashMap<(Platform, UniCase<&str>), (&ApplicationId, &ProcessName)> =
        HashMap::new();
    let mut combined: HashMap<UniCase<&str>, (&RegionSet, Platform, &ApplicationId, &ProcessName)> =
        HashMap::new();
    let mut reported_combined = HashSet::new();

    for application in applications {
        for platform in Platform::ALL {
            for process in application.processes(platform) {
                let folded = UniCase::new(process.as_str());
                let platform_key = (platform, folded);
                if let Some(&(first_app, first_name)) = per_platform.get(&platform_key) {
                    diagnostics.push(
                        Diagnostic::error(
                            "duplicate-process-name",
                            application.source(),
                            format!(
                                "`{process}` duplicates `{first_name}` from application `{first_app}` on {platform}; matching is case-insensitive"
                            ),
                        )
                        .application(application.id().to_string())
                        .field(platform.as_str()),
                    );
                } else {
                    per_platform.insert(platform_key, (application.id(), process));
                }

                if let Some(&(first_regions, first_platform, first_app, first_name)) =
                    combined.get(&folded)
                {
                    if first_regions != application.regions() && reported_combined.insert(folded) {
                        diagnostics.push(
                            Diagnostic::error(
                                "conflicting-all-platform-regions",
                                application.source(),
                                format!(
                                    "`{process}` on {platform} conflicts with `{first_name}` on {first_platform} from application `{first_app}`; the all-platform artifact cannot assign one case-insensitive name to two region sets"
                                ),
                            )
                            .application(application.id().to_string())
                            .field(platform.as_str()),
                        );
                    }
                } else {
                    combined.insert(
                        folded,
                        (application.regions(), platform, application.id(), process),
                    );
                }
            }
        }
    }
}

fn byte_offset_position(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source.as_bytes()[..offset.min(source.len())];
    let mut line = 1;
    let mut column_start = 0;
    for (position, byte) in prefix.iter().enumerate() {
        if *byte == b'\n' {
            line += 1;
            column_start = position + 1;
        }
    }
    let column = String::from_utf8_lossy(&prefix[column_start..])
        .chars()
        .count()
        + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::TestResult;

    #[test]
    fn rejects_case_insensitive_platform_collisions() -> TestResult {
        let directory = tempdir()?;
        fs::write(
            directory.path().join("other.toml"),
            r#"version = 1

[first]
regions = ["jp"]
windows = ["Agent.exe"]

[second]
regions = ["jp"]
windows = ["agent.EXE"]
"#,
        )?;

        let outcome = load_catalog(directory.path(), false);
        assert!(outcome.catalog().is_none());
        assert!(
            outcome
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "duplicate-process-name")
        );
        Ok(())
    }

    #[test]
    fn reports_multiple_schema_invariants_in_one_pass() -> TestResult {
        let directory = tempdir()?;
        fs::write(
            directory.path().join("broken.toml"),
            r#"version = 1

[broken]
display_name = "first\nsecond"
regions = { except = [] }
windows = []
android = ["not-a-package"]
linux = ["directory/program"]
"#,
        )?;

        let outcome = load_catalog(directory.path(), false);
        let codes = outcome
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("invalid-display-name"));
        assert!(codes.contains("invalid-regions"));
        assert!(codes.contains("empty-platform-list"));
        assert!(codes.contains("invalid-android-package"));
        assert!(codes.contains("invalid-process-name"));
        assert!(outcome.catalog().is_none());
        Ok(())
    }

    #[test]
    fn duplicate_application_ids_are_global_across_categories() -> TestResult {
        let directory = tempdir()?;
        for category in ["first", "second"] {
            fs::write(
                directory.path().join(format!("{category}.toml")),
                "version = 1\n\n[duplicate]\nregions = [\"jp\"]\nlinux = [\"demo\"]\n",
            )?;
        }

        let outcome = load_catalog(directory.path(), false);
        assert!(
            outcome
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "duplicate-application-id")
        );
        Ok(())
    }

    #[test]
    fn windows_suffix_warning_does_not_block_generation() -> TestResult {
        let directory = tempdir()?;
        fs::write(
            directory.path().join("other.toml"),
            "version = 1\n\n[helper]\nregions = \"any\"\nwindows = [\"helper\"]\n",
        )?;

        let outcome = load_catalog(directory.path(), false);
        assert_eq!(outcome.error_count(), 0);
        assert_eq!(outcome.warning_count(), 1);
        assert!(outcome.catalog().is_some());
        Ok(())
    }
}
