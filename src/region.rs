use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

const CANONICAL_REGION_IDS: &[&str] = &[
    "ad",
    "ae",
    "af",
    "ag",
    "ai",
    "al",
    "am",
    "ao",
    "aq",
    "ar",
    "as",
    "at",
    "au",
    "aw",
    "ax",
    "az",
    "ba",
    "bb",
    "bd",
    "be",
    "bf",
    "bg",
    "bh",
    "bi",
    "bj",
    "bl",
    "bm",
    "bn",
    "bo",
    "bq",
    "br",
    "bs",
    "bt",
    "bv",
    "bw",
    "by",
    "bz",
    "ca",
    "cc",
    "cd",
    "cf",
    "cg",
    "ch",
    "ci",
    "ck",
    "cl",
    "cm",
    "cn-mainland",
    "co",
    "cr",
    "cu",
    "cv",
    "cw",
    "cx",
    "cy",
    "cz",
    "de",
    "dj",
    "dk",
    "dm",
    "do",
    "dz",
    "ec",
    "ee",
    "eg",
    "eh",
    "er",
    "es",
    "et",
    "fi",
    "fj",
    "fk",
    "fm",
    "fo",
    "fr",
    "ga",
    "gb",
    "gd",
    "ge",
    "gf",
    "gg",
    "gh",
    "gi",
    "gl",
    "gm",
    "gn",
    "gp",
    "gq",
    "gr",
    "gs",
    "gt",
    "gu",
    "gw",
    "gy",
    "hk",
    "hm",
    "hn",
    "hr",
    "ht",
    "hu",
    "id",
    "ie",
    "il",
    "im",
    "in",
    "io",
    "iq",
    "ir",
    "is",
    "it",
    "je",
    "jm",
    "jo",
    "jp",
    "ke",
    "kg",
    "kh",
    "ki",
    "km",
    "kn",
    "kp",
    "kr",
    "kw",
    "ky",
    "kz",
    "la",
    "lb",
    "lc",
    "li",
    "lk",
    "lr",
    "ls",
    "lt",
    "lu",
    "lv",
    "ly",
    "ma",
    "mc",
    "md",
    "me",
    "mf",
    "mg",
    "mh",
    "mk",
    "ml",
    "mm",
    "mn",
    "mo",
    "mp",
    "mq",
    "mr",
    "ms",
    "mt",
    "mu",
    "mv",
    "mw",
    "mx",
    "my",
    "mz",
    "na",
    "nc",
    "ne",
    "nf",
    "ng",
    "ni",
    "nl",
    "no",
    "np",
    "nr",
    "nu",
    "nz",
    "om",
    "pa",
    "pe",
    "pf",
    "pg",
    "ph",
    "pk",
    "pl",
    "pm",
    "pn",
    "pr",
    "ps",
    "pt",
    "pw",
    "py",
    "qa",
    "re",
    "ro",
    "rs",
    "ru",
    "rw",
    "sa",
    "sb",
    "sc",
    "sd",
    "se",
    "sg",
    "sh",
    "si",
    "sj",
    "sk",
    "sl",
    "sm",
    "sn",
    "so",
    "sr",
    "ss",
    "st",
    "sv",
    "sx",
    "sy",
    "sz",
    "tc",
    "td",
    "tf",
    "tg",
    "th",
    "tj",
    "tk",
    "tl",
    "tm",
    "tn",
    "to",
    "tr",
    "tt",
    "tv",
    "tw",
    "tz",
    "ua",
    "ug",
    "um",
    "us",
    "uy",
    "uz",
    "va",
    "vc",
    "ve",
    "vg",
    "vi",
    "vn",
    "vu",
    "wf",
    "ws",
    "ye",
    "yt",
    "za",
    "zm",
    "zw",
];

/// A validated canonical egress-region identifier.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Region(String);

impl Region {
    /// Parse one canonical region identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is not present in the canonical registry.
    pub fn parse(value: &str) -> Result<Self, String> {
        CANONICAL_REGION_IDS
            .binary_search(&value)
            .map(|_| Self(value.to_owned()))
            .map_err(|_| format!("`{value}` is not a recognized canonical region ID"))
    }

    /// Canonical source and artifact label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Region {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl fmt::Display for Region {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A validated application's egress-region availability.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RegionSet(RegionSetKind);

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum RegionSetKind {
    Any,
    Only(BTreeSet<Region>),
    Except(BTreeSet<Region>),
}

impl RegionSet {
    /// Construct an unrestricted region set.
    #[must_use]
    pub const fn any() -> Self {
        Self(RegionSetKind::Any)
    }

    /// Construct availability limited to a non-empty set of regions.
    ///
    /// # Errors
    ///
    /// Returns an error when the iterator contains no regions.
    pub fn only(regions: impl IntoIterator<Item = Region>) -> Result<Self, String> {
        Self::non_empty(regions, RegionSetKind::Only, "a region list")
    }

    /// Construct availability in every region except a non-empty set.
    ///
    /// # Errors
    ///
    /// Returns an error when the iterator contains no regions.
    pub fn except(regions: impl IntoIterator<Item = Region>) -> Result<Self, String> {
        Self::non_empty(regions, RegionSetKind::Except, "an exception list")
    }

    fn non_empty(
        regions: impl IntoIterator<Item = Region>,
        kind: impl FnOnce(BTreeSet<Region>) -> RegionSetKind,
        name: &str,
    ) -> Result<Self, String> {
        let regions = regions.into_iter().collect::<BTreeSet<_>>();
        if regions.is_empty() {
            return Err(format!("{name} must not be empty"));
        }
        Ok(Self(kind(regions)))
    }

    pub(crate) fn parse_only(values: &[String]) -> Result<Self, String> {
        Self::parse_values(values).map(|regions| Self(RegionSetKind::Only(regions)))
    }

    pub(crate) fn parse_except(values: &[String]) -> Result<Self, String> {
        Self::parse_values(values).map(|regions| Self(RegionSetKind::Except(regions)))
    }

    fn parse_values(values: &[String]) -> Result<BTreeSet<Region>, String> {
        if values.is_empty() {
            return Err("regions must contain at least one value".to_owned());
        }

        let mut concrete = BTreeSet::new();
        for value in values {
            let region = Region::parse(value)?;
            if !concrete.insert(region) {
                return Err(format!("duplicate region `{value}`"));
            }
        }
        Ok(concrete)
    }

    /// Whether this region set is unrestricted.
    #[must_use]
    pub const fn is_any(&self) -> bool {
        matches!(self.0, RegionSetKind::Any)
    }

    /// Whether this set contains exactly one specified concrete region.
    #[must_use]
    pub fn is_only(&self, region: &Region) -> bool {
        matches!(&self.0, RegionSetKind::Only(regions) if regions.len() == 1 && regions.contains(region))
    }

    /// Whether this restricted set permits a region.
    /// Unrestricted `any` sets remain a separate collection.
    #[must_use]
    pub fn contains(&self, region: &Region) -> bool {
        match &self.0 {
            RegionSetKind::Any => false,
            RegionSetKind::Only(regions) => regions.contains(region),
            RegionSetKind::Except(regions) => !regions.contains(region),
        }
    }

    /// Whether this restricted set rejects a region.
    /// Unrestricted `any` sets do not count as excluding a region.
    #[must_use]
    pub fn excludes(&self, region: &Region) -> bool {
        match &self.0 {
            RegionSetKind::Any => false,
            RegionSetKind::Only(regions) => !regions.contains(region),
            RegionSetKind::Except(regions) => regions.contains(region),
        }
    }

    /// Iterate over explicitly mentioned regions; unrestricted sets yield no items.
    pub fn mentioned_regions(&self) -> impl Iterator<Item = &Region> {
        match &self.0 {
            RegionSetKind::Any => None,
            RegionSetKind::Only(regions) | RegionSetKind::Except(regions) => Some(regions),
        }
        .into_iter()
        .flatten()
    }
}

impl fmt::Display for RegionSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            RegionSetKind::Any => formatter.write_str("any"),
            RegionSetKind::Only(regions) => write_regions(formatter, regions),
            RegionSetKind::Except(regions) => {
                formatter.write_str("except ")?;
                write_regions(formatter, regions)
            }
        }
    }
}

fn write_regions(formatter: &mut fmt::Formatter<'_>, regions: &BTreeSet<Region>) -> fmt::Result {
    for (index, region) in regions.iter().enumerate() {
        if index > 0 {
            formatter.write_str(", ")?;
        }
        write!(formatter, "{region}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestResult;

    #[test]
    fn canonical_registry_is_sorted_and_unique() -> TestResult {
        assert_eq!(CANONICAL_REGION_IDS.len(), 249);
        assert!(
            CANONICAL_REGION_IDS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(Region::parse("cn").is_err());
        assert!(Region::parse("CN").is_err());
        assert_eq!(Region::parse("cn-mainland")?.as_str(), "cn-mainland");
        assert!(Region::parse("jp").is_ok());
        Ok(())
    }

    #[test]
    fn restricted_region_sets_are_non_empty() -> TestResult {
        assert!(RegionSet::only([]).is_err());
        assert!(RegionSet::except([]).is_err());
        assert!(RegionSet::only([Region::parse("jp")?]).is_ok());
        assert!(RegionSet::except([Region::parse("jp")?]).is_ok());
        Ok(())
    }

    #[test]
    fn exclusion_sets_invert_region_membership() -> TestResult {
        let cn = Region::parse("cn-mainland")?;
        let jp = Region::parse("jp")?;
        let regions = RegionSet::except([cn.clone()])?;

        assert!(!regions.contains(&cn));
        assert!(regions.excludes(&cn));
        assert!(regions.contains(&jp));
        assert!(!regions.excludes(&jp));
        assert_eq!(regions.to_string(), "except cn-mainland");
        Ok(())
    }
}
