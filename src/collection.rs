use std::fmt;
use std::str::FromStr;

use crate::region::{Region, RegionSet};

/// How a concrete region is related to an application's complete region set.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RegionRelation {
    /// The application has exactly this one region.
    Only,
    /// The application contains this region, possibly alongside others.
    Contains,
    /// The application has concrete regions and does not contain this region.
    Excludes,
}

impl RegionRelation {
    /// All regional relations in stable output order.
    pub const ALL: [Self; 3] = [Self::Only, Self::Contains, Self::Excludes];

    const fn slug(self) -> &'static str {
        match self {
            Self::Only => "only",
            Self::Contains => "contain",
            Self::Excludes => "not-contain",
        }
    }

    fn matches(self, regions: &RegionSet, region: &Region) -> bool {
        match self {
            Self::Only => regions.is_only(region),
            Self::Contains => regions.contains(region),
            Self::Excludes => regions.excludes(region),
        }
    }
}

/// A generated view over application region sets.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Collection {
    /// Applications whose region set is unrestricted.
    Any,
    /// One relation applied uniformly to any concrete region.
    Regional {
        /// Region used by the predicate.
        region: Region,
        /// Relationship required between the application region set and `region`.
        relation: RegionRelation,
    },
}

impl Collection {
    /// Construct the exact-single-region collection for `region`.
    #[must_use]
    pub const fn only(region: Region) -> Self {
        Self::Regional {
            region,
            relation: RegionRelation::Only,
        }
    }

    /// Construct the collection containing `region`.
    #[must_use]
    pub const fn containing(region: Region) -> Self {
        Self::Regional {
            region,
            relation: RegionRelation::Contains,
        }
    }

    /// Construct the concrete-region collection excluding `region`.
    #[must_use]
    pub const fn excluding(region: Region) -> Self {
        Self::Regional {
            region,
            relation: RegionRelation::Excludes,
        }
    }

    /// Stable artifact and CLI identifier.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::Any => "any".to_owned(),
            Self::Regional { region, relation } => {
                format!("{}-{region}", relation.slug())
            }
        }
    }

    /// Whether an application region set belongs to this collection.
    #[must_use]
    pub fn matches(&self, regions: &RegionSet) -> bool {
        match self {
            Self::Any => regions.is_any(),
            Self::Regional { region, relation } => relation.matches(regions, region),
        }
    }
}

impl fmt::Display for Collection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.name())
    }
}

impl FromStr for Collection {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "any" {
            return Ok(Self::Any);
        }

        let (relation, region_slug) = if let Some(region) = value.strip_prefix("only-") {
            (RegionRelation::Only, region)
        } else if let Some(region) = value.strip_prefix("contain-") {
            (RegionRelation::Contains, region)
        } else if let Some(region) = value.strip_prefix("not-contain-") {
            (RegionRelation::Excludes, region)
        } else {
            return Err(
                "expected any, only-<region>, contain-<region>, or not-contain-<region>".to_owned(),
            );
        };

        Region::parse(region_slug).map(|region| Self::Regional { region, relation })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestResult;

    fn regions(values: &[&str]) -> TestResult<RegionSet> {
        let values = values
            .iter()
            .map(|value| Region::parse(value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(RegionSet::only(values)?)
    }

    fn except(values: &[&str]) -> TestResult<RegionSet> {
        let values = values
            .iter()
            .map(|value| Region::parse(value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(RegionSet::except(values)?)
    }

    fn region(value: &str) -> TestResult<Region> {
        Ok(value.parse()?)
    }

    #[test]
    fn every_region_uses_the_same_three_relations() -> TestResult {
        let only_cn = regions(&["cn-mainland"])?;
        let only_jp = regions(&["jp"])?;
        let mixed = regions(&["cn-mainland", "jp"])?;
        let except_cn = except(&["cn-mainland"])?;
        let any = RegionSet::any();
        let cn = region("cn-mainland")?;
        let jp = region("jp")?;

        assert!(Collection::only(cn.clone()).matches(&only_cn));
        assert!(Collection::containing(cn.clone()).matches(&only_cn));
        assert!(Collection::containing(cn.clone()).matches(&mixed));
        assert!(Collection::excluding(cn.clone()).matches(&only_jp));

        assert!(Collection::only(jp.clone()).matches(&only_jp));
        assert!(Collection::containing(jp.clone()).matches(&mixed));
        assert!(Collection::excluding(jp).matches(&only_cn));

        assert!(Collection::excluding(cn.clone()).matches(&except_cn));
        assert!(Collection::containing(region("jp")?).matches(&except_cn));
        assert!(!Collection::only(cn).matches(&except_cn));

        assert!(!Collection::excluding(region("us")?).matches(&any));
        assert!(Collection::Any.matches(&any));
        Ok(())
    }

    #[test]
    fn parses_generic_region_collections() -> TestResult {
        assert_eq!(
            "only-cn-mainland".parse::<Collection>(),
            Ok(Collection::only(region("cn-mainland")?))
        );
        assert_eq!(
            "contain-jp".parse::<Collection>(),
            Ok(Collection::containing(region("jp")?))
        );
        assert_eq!(
            "not-contain-us".parse::<Collection>(),
            Ok(Collection::excluding(region("us")?))
        );
        assert!("only-cn".parse::<Collection>().is_err());
        Ok(())
    }
}
