//! Which Business Central build supplies the Microsoft apps (ADR-013 decision 2, "Layer A").
//!
//! Mirrors AL Runner's `BcArtifacts.SelectArtifactVersionDir` and its default-version rules
//! (`BusinessCentral.AL.Runner`, `AlRunner/Infrastructure/BcArtifacts.cs:292`, `Program.cs`
//! lines 1068-1230), with the adaptation ADR-013 names: there is no engine, so the default is
//! the major of the app.json floor. Everything here is a pure function over values: no file,
//! no network (ADR-006 spirit; this crate may touch the OS, this module does not).
//!
//! Entry points: `BuildId`, `parse_index`, `BuildQuery`, `Origin`.

use std::cmp::Ordering;
use std::fmt;

/// A published build such as `27.5.46862.56249`. Ordered by its four numbers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildId {
    parts: [u32; 4],
    text: String,
}

impl BuildId {
    /// Exactly four dot-separated numbers.
    pub fn parse(s: &str) -> Result<BuildId, String> {
        let s = s.trim();
        let pieces: Vec<&str> = s.split('.').collect();
        if pieces.len() != 4 {
            return Err(format!("`{s}` is not a build (expected major.minor.build.revision)"));
        }
        let mut parts = [0u32; 4];
        for (slot, p) in parts.iter_mut().zip(&pieces) {
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("`{s}` is not a build: `{p}` is not a number"));
            }
            *slot = p
                .parse()
                .map_err(|_| format!("`{s}` is not a build: `{p}` is too large"))?;
        }
        Ok(BuildId {
            parts,
            text: parts.map(|p| p.to_string()).join("."),
        })
    }

    pub fn major(&self) -> u32 {
        self.parts[0]
    }

    pub fn minor(&self) -> u32 {
        self.parts[1]
    }

    /// True when `prefix` (`27`, `27.5`, `27.5.46862`) is a whole-segment prefix of this build:
    /// `27.5` matches `27.5.46862.1` but not `27.50.1.1`. A prefix that is not numbers matches nothing.
    pub fn matches_prefix(&self, prefix: &str) -> bool {
        let want: Vec<&str> = prefix.trim().split('.').collect();
        if want.len() > 4
            || want
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return false;
        }
        want.iter()
            .zip(&self.parts)
            .all(|(w, have)| w.parse::<u32>().is_ok_and(|w| w == *have))
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl PartialOrd for BuildId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BuildId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parts.cmp(&other.parts)
    }
}

impl fmt::Display for BuildId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// The builds listed by the artifact index (`indexes/w1.json`): `[{"Version": "...", "CreationTime": "..."}]`.
/// An entry that is not a four-part build is skipped; the file is Microsoft's, not ours.
pub fn parse_index(json: &str) -> Result<Vec<BuildId>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("artifact index: {e}"))?;
    let list = v.as_array().ok_or("artifact index: expected a JSON array")?;
    let mut out: Vec<BuildId> = list
        .iter()
        .filter_map(|e| e["Version"].as_str())
        .filter_map(|s| BuildId::parse(s).ok())
        .collect();
    out.sort();
    out.dedup();
    Ok(out)
}

/// What the user asked for, and the floor the default must respect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildQuery {
    /// `--bc-version` as typed (an exact build or a prefix), or the BC major of the target.
    pub prefix: String,
    /// `--bc-version` was given: it wins, even below the floor (with a warning).
    pub explicit: bool,
    /// The higher of `application` and `platform` in app.json, as (major, minor).
    pub floor: Option<(u32, u32)>,
    /// The build the tests themselves come from (a Microsoft test archive or package names it).
    /// Without `--bc-version`, the Microsoft apps, the platform `System.app` and the demo data
    /// come from this build when it is cached, on disk or still published; otherwise the usual
    /// rules pick and the selection warns.
    pub preferred: Option<BuildId>,
}

impl BuildQuery {
    /// The query for a run. `requested` is `--bc-version`. Without it the BC major of the
    /// target is used (ADR-013 decision 2, step 2; ADR-015 3.2 has already raised a floor
    /// below 27 to 27).
    pub fn new(requested: Option<&str>, target_major: u16, floor: Option<(u32, u32)>) -> BuildQuery {
        match requested {
            Some(p) => BuildQuery {
                prefix: p.trim().to_string(),
                explicit: true,
                floor,
                preferred: None,
            },
            None => BuildQuery {
                prefix: target_major.to_string(),
                explicit: false,
                floor,
                preferred: None,
            },
        }
    }

    /// Sets the build of the tests (see [`BuildQuery::preferred`]).
    pub fn with_preferred(mut self, build: Option<BuildId>) -> BuildQuery {
        self.preferred = build;
        self
    }

    /// The one whole build this query names: `--bc-version` written as a build, else the
    /// build of the tests when nothing was requested.
    pub fn exact(&self) -> Option<BuildId> {
        BuildId::parse(&self.prefix)
            .ok()
            .or_else(|| self.preferred.clone().filter(|_| !self.explicit))
    }

    fn admits(&self, b: &BuildId) -> bool {
        if !b.matches_prefix(&self.prefix) {
            return false;
        }
        // The default never selects below the floor; an explicit request may (it warns).
        self.explicit || self.floor.is_none_or(|(maj, min)| (b.major(), b.minor()) >= (maj, min))
    }

    /// The newest of `builds` that satisfies the query.
    pub fn pick<'a>(&self, builds: &'a [BuildId]) -> Option<&'a BuildId> {
        builds.iter().filter(|b| self.admits(b)).max()
    }

    /// The warning for an explicit request that is below the app's floor.
    pub fn floor_warning(&self, chosen: &BuildId) -> Option<String> {
        let (maj, min) = self.floor?;
        (self.explicit && (chosen.major(), chosen.minor()) < (maj, min)).then(|| {
            format!(
                "--bc-version {} selects build {chosen}, below the app's floor {maj}.{min} (application/platform in app.json); running anyway",
                self.prefix
            )
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Already in the artifact cache.
    Cache,
    /// Newest published build, to be downloaded.
    Published,
    /// An extracted artifact on disk (BcContainerHelper's `~/.bcartifacts.cache`).
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub build: BuildId,
    pub origin: Origin,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// Nothing matches. Names the request, the builds that were available and the cache directory;
    /// there is no silent fallback to another major.
    NoMatch {
        request: String,
        available: Vec<String>,
        cache_dir: String,
    },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let BuildError::NoMatch {
            request,
            available,
            cache_dir,
        } = self;
        let shown = if available.is_empty() {
            "none".to_string()
        } else if available.len() > 8 {
            format!(
                "{}, ... ({} in all)",
                available[available.len() - 8..].join(", "),
                available.len()
            )
        } else {
            available.join(", ")
        };
        write!(
            f,
            "no BC build matches {request}; builds available: {shown}; artifact cache: {cache_dir}"
        )
    }
}

impl std::error::Error for BuildError {}

/// Chooses the build: the newest matching build in the cache; else the newest matching published
/// build (`published` is `None` when downloading is off or the index was not asked for).
/// A match in the cache is used even when a newer one is published (ADR-013, step 1 and 2:
/// "the newest cached build is used, and if none is cached, ... downloaded").
pub fn select_build(
    q: &BuildQuery,
    cached: &[BuildId],
    published: Option<&[BuildId]>,
    cache_dir: &str,
) -> Result<Selection, BuildError> {
    let preferred = q.preferred.as_ref().filter(|_| !q.explicit);
    let (build, origin) = if let Some(b) = preferred.filter(|p| cached.contains(p)) {
        (b.clone(), Origin::Cache)
    } else if let Some(b) = preferred.filter(|p| published.is_some_and(|ps| ps.contains(p))) {
        (b.clone(), Origin::Published)
    } else if let Some(b) = q.pick(cached) {
        (b.clone(), Origin::Cache)
    } else if let Some(b) = published.and_then(|p| q.pick(p)) {
        (b.clone(), Origin::Published)
    } else {
        let mut available: Vec<BuildId> = cached.iter().chain(published.unwrap_or(&[])).cloned().collect();
        available.sort();
        available.dedup();
        let request = match (q.explicit, q.floor) {
            (true, _) => format!("`--bc-version {}`", q.prefix),
            (false, Some((maj, min))) => format!("BC {} at or above the app's floor {maj}.{min}", q.prefix),
            (false, None) => format!("BC {}", q.prefix),
        };
        return Err(BuildError::NoMatch {
            request,
            // Only the builds of the requested major are useful in the message.
            available: available
                .iter()
                .filter(|b| b.matches_prefix(q.prefix.split('.').next().unwrap_or("")))
                .map(|b| b.to_string())
                .collect(),
            cache_dir: cache_dir.to_string(),
        });
    };
    let mut warnings: Vec<String> = q.floor_warning(&build).into_iter().collect();
    if let Some(p) = q.preferred.as_ref().filter(|p| !q.explicit && **p != build) {
        warnings.push(format!(
            "the tests are build {p}, which is neither cached nor published; the Microsoft apps come from build {build}"
        ));
    }
    Ok(Selection {
        build,
        origin,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> BuildId {
        BuildId::parse(s).unwrap()
    }
    fn bs(v: &[&str]) -> Vec<BuildId> {
        v.iter().map(|s| b(s)).collect()
    }
    const DIR: &str = "/cache/artifacts";

    #[test]
    fn builds_compare_by_their_four_numbers_not_as_text() {
        assert!(b("27.5.46862.9") < b("27.5.46862.10"));
        assert!(b("27.5.46862.56249") < b("28.0.1.1"));
        assert!(b("27.10.1.1") > b("27.9.99999.99999"));
        assert_eq!(b("27.5.46862.56249").to_string(), "27.5.46862.56249");
        assert!(BuildId::parse("27.5").is_err());
        assert!(BuildId::parse("27.5.x.1").is_err());
        assert!(BuildId::parse("27.5.1.2.3").is_err());
    }

    #[test]
    fn a_prefix_matches_whole_segments_only() {
        let x = b("27.5.46862.56249");
        assert!(x.matches_prefix("27"));
        assert!(x.matches_prefix("27.5"));
        assert!(x.matches_prefix("27.5.46862.56249"));
        assert!(!x.matches_prefix("27.50"));
        assert!(!x.matches_prefix("2"));
        assert!(!x.matches_prefix("27.5.4686"));
        assert!(!x.matches_prefix("abc"));
        assert!(!x.matches_prefix(""));
    }

    #[test]
    fn the_index_lists_builds_oldest_first_and_skips_junk() {
        let json = r#"[{"Version":"28.0.1.2","CreationTime":"x"},{"Version":"27.5.46862.56249"},{"Version":"junk"},{"Other":1},{"Version":"27.5.46862.56249"}]"#;
        assert_eq!(parse_index(json).unwrap(), bs(&["27.5.46862.56249", "28.0.1.2"]));
        assert!(parse_index("{").is_err());
        assert!(parse_index("{}").is_err());
    }

    #[test]
    fn an_explicit_prefix_takes_the_newest_cached_match() {
        let cached = bs(&["27.4.1.1", "27.5.46862.48827", "27.5.46862.53931", "28.1.1.1"]);
        let q = BuildQuery::new(Some("27.5"), 27, None);
        let s = select_build(&q, &cached, None, DIR).unwrap();
        assert_eq!(
            (s.build.to_string().as_str(), s.origin),
            ("27.5.46862.53931", Origin::Cache)
        );
        assert!(s.warnings.is_empty());
    }

    #[test]
    fn a_cached_match_beats_a_newer_published_one() {
        // AL Runner asks the cache first; a stray newer build is not downloaded over it.
        let cached = bs(&["27.5.46862.48827"]);
        let published = bs(&["27.5.46862.48827", "27.5.46862.56249"]);
        let q = BuildQuery::new(Some("27.5"), 27, None);
        let s = select_build(&q, &cached, Some(&published), DIR).unwrap();
        assert_eq!(
            (s.build.to_string().as_str(), s.origin),
            ("27.5.46862.48827", Origin::Cache)
        );
    }

    #[test]
    fn nothing_cached_downloads_the_newest_published_match() {
        let published = bs(&["27.4.1.1", "27.5.46862.53931", "27.5.46862.56249", "28.0.1.1"]);
        let q = BuildQuery::new(Some("27.5"), 27, None);
        let s = select_build(&q, &[], Some(&published), DIR).unwrap();
        assert_eq!(
            (s.build.to_string().as_str(), s.origin),
            ("27.5.46862.56249", Origin::Published)
        );
    }

    #[test]
    fn a_prefix_27_5_does_not_match_27_50() {
        let published = bs(&["27.50.1.1", "27.5.46862.1"]);
        let q = BuildQuery::new(Some("27.5"), 27, None);
        assert_eq!(
            select_build(&q, &[], Some(&published), DIR).unwrap().build,
            b("27.5.46862.1")
        );
    }

    #[test]
    fn the_default_is_the_newest_build_of_the_target_major() {
        let cached = bs(&["27.0.38460.53934", "27.5.46862.53931", "28.1.49838.50621"]);
        let q = BuildQuery::new(None, 27, Some((27, 0)));
        assert_eq!(
            select_build(&q, &cached, None, DIR).unwrap().build,
            b("27.5.46862.53931")
        );
        let q = BuildQuery::new(None, 28, Some((28, 0)));
        assert_eq!(
            select_build(&q, &cached, None, DIR).unwrap().build,
            b("28.1.49838.50621")
        );
    }

    #[test]
    fn a_stray_build_of_another_major_never_becomes_the_default() {
        // Only a 28 is cached and the app wants 27: download a 27, do not use the 28.
        let cached = bs(&["28.1.49838.50621"]);
        let published = bs(&["27.5.46862.56249", "28.1.49838.50621"]);
        let q = BuildQuery::new(None, 27, Some((27, 0)));
        let s = select_build(&q, &cached, Some(&published), DIR).unwrap();
        assert_eq!(
            (s.build.to_string().as_str(), s.origin),
            ("27.5.46862.56249", Origin::Published)
        );
    }

    #[test]
    fn the_default_never_selects_below_the_floor() {
        // Floor 27.5: the cached 27.3 is skipped even though it is the only cached 27.
        let cached = bs(&["27.3.44313.53909"]);
        let published = bs(&["27.3.44313.53909", "27.5.46862.56249"]);
        let q = BuildQuery::new(None, 27, Some((27, 5)));
        let s = select_build(&q, &cached, Some(&published), DIR).unwrap();
        assert_eq!(s.build, b("27.5.46862.56249"));
        assert_eq!(s.origin, Origin::Published);
    }

    #[test]
    fn an_explicit_request_below_the_floor_runs_with_a_warning() {
        let published = bs(&["27.3.44313.53909", "27.5.46862.56249"]);
        let q = BuildQuery::new(Some("27.3"), 27, Some((27, 5)));
        let s = select_build(&q, &[], Some(&published), DIR).unwrap();
        assert_eq!(s.build, b("27.3.44313.53909"));
        assert_eq!(s.warnings.len(), 1);
        assert!(
            s.warnings[0].contains("27.3.44313.53909") && s.warnings[0].contains("27.5"),
            "{:?}",
            s.warnings
        );
    }

    #[test]
    fn no_match_is_an_error_naming_request_builds_and_cache_with_no_fallback() {
        let cached = bs(&["27.5.46862.48827", "28.1.49838.50621"]);
        let published = bs(&["27.5.46862.56249"]);
        let q = BuildQuery::new(Some("29"), 29, None);
        let e = select_build(&q, &cached, Some(&published), DIR)
            .unwrap_err()
            .to_string();
        assert!(e.contains("--bc-version 29") && e.contains(DIR), "{e}");
        // A default below the floor with only older builds is also an error, not a quiet downgrade.
        let q = BuildQuery::new(None, 27, Some((27, 9)));
        let e = select_build(&q, &cached, Some(&published), DIR)
            .unwrap_err()
            .to_string();
        assert!(e.contains("floor 27.9") && e.contains("27.5.46862.56249"), "{e}");
    }

    #[test]
    fn without_an_index_only_the_cache_is_consulted() {
        let q = BuildQuery::new(Some("27.5"), 27, None);
        let e = select_build(&q, &bs(&["28.1.1.1"]), None, DIR).unwrap_err().to_string();
        assert!(
            e.contains("builds available: none") || e.contains("builds available: "),
            "{e}"
        );
    }
    #[test]
    fn the_build_of_the_tests_wins_when_it_is_listed() {
        let cached = bs(&["27.5.46862.48827"]);
        let published = bs(&["27.5.46862.48827", "27.5.46862.56249", "28.1.1.1"]);
        let q = BuildQuery::new(None, 27, None).with_preferred(Some(b("27.5.46862.48827")));
        let s = select_build(&q, &[], Some(&published), DIR).unwrap();
        assert_eq!(s.build.to_string(), "27.5.46862.48827");
        assert!(s.warnings.is_empty(), "{:?}", s.warnings);
        let s = select_build(&q, &cached, Some(&published), DIR).unwrap();
        assert_eq!(
            (s.build.to_string().as_str(), s.origin),
            ("27.5.46862.48827", Origin::Cache)
        );
        assert_eq!(q.exact(), Some(b("27.5.46862.48827")));
    }

    #[test]
    fn the_build_of_the_tests_beats_another_cached_build() {
        let cached = bs(&["27.5.46862.56249"]);
        let published = bs(&["27.5.46862.48827", "27.5.46862.56249"]);
        let q = BuildQuery::new(None, 27, None).with_preferred(Some(b("27.5.46862.48827")));
        let s = select_build(&q, &cached, Some(&published), DIR).unwrap();
        assert_eq!(
            (s.build.to_string().as_str(), s.origin),
            ("27.5.46862.48827", Origin::Published)
        );
    }

    #[test]
    fn a_build_of_the_tests_that_is_gone_falls_back_to_the_newest_and_says_so() {
        let published = bs(&["27.5.46862.56249", "28.1.1.1"]);
        let q = BuildQuery::new(None, 27, None).with_preferred(Some(b("27.5.46862.48827")));
        let s = select_build(&q, &[], Some(&published), DIR).unwrap();
        assert_eq!(s.build.to_string(), "27.5.46862.56249");
        assert!(
            s.warnings
                .iter()
                .any(|w| w.contains("27.5.46862.48827") && w.contains("27.5.46862.56249")),
            "{:?}",
            s.warnings
        );
    }

    #[test]
    fn an_explicit_request_beats_the_build_of_the_tests() {
        let published = bs(&["27.4.1.1", "27.5.46862.48827", "27.5.46862.56249"]);
        let q = BuildQuery::new(Some("27.4"), 27, None).with_preferred(Some(b("27.5.46862.48827")));
        let s = select_build(&q, &[], Some(&published), DIR).unwrap();
        assert_eq!(s.build.to_string(), "27.4.1.1");
        assert!(s.warnings.is_empty());
        assert_eq!(q.exact(), None);
    }
}
