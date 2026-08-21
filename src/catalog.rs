use std::collections::{BTreeSet, HashSet};

use unicase::UniCase;

use crate::application::Application;
use crate::collection::{Collection, RegionRelation};
use crate::platform::{Platform, PlatformSelection, ProcessName};
use crate::region::RegionSet;

/// Validated aggregate of all source files.
#[derive(Debug, Clone)]
pub struct Catalog {
    pub(crate) applications: Vec<Application>,
}

/// One matching record returned by an explain query.
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Application ID.
    pub application: String,
    /// Optional display name.
    pub display_name: Option<String>,
    /// Organizational source category.
    pub category: String,
    /// Concrete platform where the match was found.
    pub platform: Platform,
    /// Case-preserving source process name.
    pub process: String,
    /// Validated egress-region availability.
    pub regions: RegionSet,
    /// All generated collection IDs containing this rule.
    pub collections: Vec<String>,
}

impl Catalog {
    /// Number of source applications.
    #[must_use]
    pub const fn application_count(&self) -> usize {
        self.applications.len()
    }
    /// All available generated collections in stable order.
    #[must_use]
    pub fn collections(&self) -> Vec<Collection> {
        let regions = self
            .applications
            .iter()
            .flat_map(|application| application.regions.mentioned_regions())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut collections = Vec::with_capacity(1 + regions.len() * RegionRelation::ALL.len());
        collections.push(Collection::Any);
        for region in regions {
            for relation in RegionRelation::ALL {
                collections.push(Collection::Regional {
                    region: region.clone(),
                    relation,
                });
            }
        }
        collections
    }

    /// Select, case-insensitively deduplicate, and sort exact process names.
    ///
    /// Returns `None` when the collection is not part of this catalog's output matrix.
    #[must_use]
    pub fn select(
        &self,
        platform: PlatformSelection,
        collection: &Collection,
    ) -> Option<Vec<ProcessName>> {
        let available = match collection {
            Collection::Any => true,
            Collection::Regional { region, .. } => self.applications.iter().any(|application| {
                application
                    .regions()
                    .mentioned_regions()
                    .any(|item| item == region)
            }),
        };
        available.then(|| self.select_available(platform, collection))
    }

    pub(crate) fn select_available(
        &self,
        platform: PlatformSelection,
        collection: &Collection,
    ) -> Vec<ProcessName> {
        let mut seen = HashSet::new();
        let mut selected = Vec::new();
        for application in &self.applications {
            if !collection.matches(application.regions()) {
                continue;
            }
            for concrete_platform in platform.platforms() {
                for process in application.processes(*concrete_platform) {
                    if seen.insert(UniCase::new(process.as_str())) {
                        selected.push(process.clone());
                    }
                }
            }
        }
        selected.sort_by(|left, right| {
            left.as_str()
                .to_lowercase()
                .cmp(&right.as_str().to_lowercase())
                .then_with(|| left.cmp(right))
        });
        selected
    }

    /// Explain every case-insensitive source match for a process query.
    #[must_use]
    pub fn explain(&self, platform: PlatformSelection, process: &str) -> Vec<Explanation> {
        let query = UniCase::new(process);
        let available_collections = self.collections();
        let mut explanations = Vec::new();
        for application in &self.applications {
            let collections = available_collections
                .iter()
                .filter(|collection| collection.matches(application.regions()))
                .map(Collection::name)
                .collect::<Vec<_>>();
            for concrete_platform in platform.platforms() {
                for candidate in application.processes(*concrete_platform) {
                    if UniCase::new(candidate.as_str()) == query {
                        explanations.push(Explanation {
                            application: application.id().to_string(),
                            display_name: application.display_name().map(str::to_owned),
                            category: application.category().to_owned(),
                            platform: *concrete_platform,
                            process: candidate.to_string(),
                            regions: application.regions().clone(),
                            collections: collections.clone(),
                        });
                    }
                }
            }
        }
        explanations
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::{PlatformSelection, Region, TestResult, load_catalog};

    fn region(value: &str) -> TestResult<Region> {
        Ok(value.parse()?)
    }

    fn names(values: &[ProcessName]) -> Vec<&str> {
        values.iter().map(ProcessName::as_str).collect()
    }

    #[test]
    fn loads_and_selects_region_collections() -> TestResult {
        let directory = tempdir()?;
        fs::write(
            directory.path().join("messaging.toml"),
            r#"version = 1

[china-only]
regions = ["cn-mainland"]
windows = ["China.exe"]

[mixed]
regions = ["cn-mainland", "jp"]
linux = ["mixed"]

[outside-china]
regions = { except = ["cn-mainland"] }
windows = ["Outside.exe"]

[global]
regions = "any"
android = ["org.example.global"]
"#,
        )?;

        let outcome = load_catalog(directory.path(), false);
        assert_eq!(outcome.error_count(), 0, "{:?}", outcome.diagnostics());
        let catalog = outcome.catalog().ok_or("expected a valid catalog")?;
        assert_eq!(
            catalog
                .collections()
                .iter()
                .map(Collection::name)
                .collect::<Vec<_>>(),
            vec![
                "any",
                "only-cn-mainland",
                "contain-cn-mainland",
                "not-contain-cn-mainland",
                "only-jp",
                "contain-jp",
                "not-contain-jp",
            ]
        );

        let selected = catalog
            .select(
                PlatformSelection::All,
                &Collection::only(region("cn-mainland")?),
            )
            .ok_or("expected only-cn-mainland to be available")?;
        assert_eq!(names(&selected), vec!["China.exe"]);

        let selected = catalog
            .select(
                PlatformSelection::All,
                &Collection::excluding(region("cn-mainland")?),
            )
            .ok_or("expected not-contain-cn-mainland to be available")?;
        assert_eq!(names(&selected), vec!["Outside.exe"]);

        let selected = catalog
            .select(
                PlatformSelection::All,
                &Collection::containing(region("jp")?),
            )
            .ok_or("expected contain-jp to be available")?;
        assert_eq!(names(&selected), vec!["mixed", "Outside.exe"]);

        let selected = catalog
            .select(
                PlatformSelection::All,
                &Collection::containing(region("cn-mainland")?),
            )
            .ok_or("expected contain-cn-mainland to be available")?;
        assert_eq!(names(&selected), vec!["China.exe", "mixed"]);

        let selected = catalog
            .select(
                PlatformSelection::All,
                &Collection::excluding(region("jp")?),
            )
            .ok_or("expected not-contain-jp to be available")?;
        assert_eq!(names(&selected), vec!["China.exe"]);

        let selected = catalog
            .select(PlatformSelection::All, &Collection::Any)
            .ok_or("expected any to be available")?;
        assert_eq!(names(&selected), vec!["org.example.global"]);
        assert!(
            catalog
                .select(PlatformSelection::All, &Collection::only(region("us")?))
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn same_name_on_different_platforms_is_deduplicated_when_regions_agree() -> TestResult {
        let directory = tempdir()?;
        fs::write(
            directory.path().join("other.toml"),
            r#"version = 1

[one]
regions = ["jp"]
windows = ["Shared.exe"]

[two]
regions = ["jp"]
linux = ["shared.EXE"]
"#,
        )?;

        let outcome = load_catalog(directory.path(), false);
        assert_eq!(outcome.error_count(), 0, "{:?}", outcome.diagnostics());
        let selected = outcome
            .catalog()
            .ok_or("expected a valid catalog")?
            .select(
                PlatformSelection::All,
                &Collection::containing(region("jp")?),
            )
            .ok_or("expected contain-jp to be available")?;
        assert_eq!(names(&selected), vec!["Shared.exe"]);
        Ok(())
    }
}
