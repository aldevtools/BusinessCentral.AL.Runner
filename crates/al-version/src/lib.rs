//! BC versions and the run target (ADR-015).
//!
//! A leaf crate: no dependencies, no OS access (ADR-006). Everything that
//! depends on the Business Central version is a function of a [`Target`].

mod choose;
mod gates;
pub use choose::{AppVersions, Chosen, TargetError, choose_target};
pub use gates::{
    Evidence, FeatureKeyRows, Gate, feature_key_rows, has as gate_on, platform_table_scopes_known,
    test_runner_root_scope,
};

use std::collections::BTreeSet;
use std::fmt;

/// The oldest BC major the runtime emulates (ADR-015: nothing older than 27).
pub const MIN_BC_MAJOR: u16 = 27;

/// Runtime major minus BC major (ADR-015 2.0): BC 27 is runtime 16.0.
const RUNTIME_OFFSET: u16 = 11;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VersionError {
    Empty,
    BadPart(String),
    TooManyParts(String),
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionError::Empty => write!(f, "empty version"),
            VersionError::BadPart(s) => write!(f, "`{s}` is not a version number"),
            VersionError::TooManyParts(s) => write!(f, "`{s}` has too many parts"),
        }
    }
}

impl std::error::Error for VersionError {}

/// A Business Central server version. Compared by `(major, minor)`; the build
/// string is for reports only.
#[derive(Debug, Clone)]
pub struct BcVersion {
    pub major: u16,
    pub minor: u16,
    pub build: Option<String>,
}

/// An app runtime version (app.json `runtime`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Runtime {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub bc: BcVersion,
    pub runtime: Runtime,
    /// The `runtime` each app under test declares in its app.json, by app id (lower case), where it is below
    /// `runtime`: the builtins such an app may call are those of its own runtime (the AL compiler gives AL0666 for
    /// the others; al-runner issue 5573). Empty: every app sees `runtime`. Dependency apps are never listed.
    pub app_runtimes: Vec<(String, Runtime)>,
}

impl BcVersion {
    pub fn new(major: u16, minor: u16) -> Self {
        BcVersion {
            major,
            minor,
            build: None,
        }
    }

    /// `29`, `29.0`, `27.5` or a full build such as `29.0.54011.55816`.
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        let parts = numbers(s, 4)?;
        let build = (parts.len() > 2).then(|| s.trim().to_string());
        Ok(BcVersion {
            major: parts[0],
            minor: parts.get(1).copied().unwrap_or(0),
            build,
        })
    }

    /// True when `prefix` (`27`, `27.5`) is a whole-segment prefix of this
    /// version: `27.5` matches `27.5.46862.1` but not `27.50`.
    pub fn matches_prefix(&self, prefix: &str) -> bool {
        let Ok(want) = numbers(prefix, 4) else {
            return false;
        };
        let have: Vec<u16> = match &self.build {
            Some(b) => match numbers(b, 4) {
                Ok(v) => v,
                Err(_) => return false,
            },
            None => vec![self.major, self.minor],
        };
        want.len() <= have.len() && have[..want.len()] == want[..]
    }
}

impl PartialEq for BcVersion {
    fn eq(&self, other: &Self) -> bool {
        (self.major, self.minor) == (other.major, other.minor)
    }
}
impl Eq for BcVersion {}
impl PartialOrd for BcVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BcVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor).cmp(&(other.major, other.minor))
    }
}

impl fmt::Display for BcVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.build {
            Some(b) => write!(f, "{b}"),
            None => write!(f, "{}.{}", self.major, self.minor),
        }
    }
}

impl Runtime {
    /// The runtime that ships with a BC major (BC 27 is 16.0).
    pub fn for_bc(major: u16) -> Runtime {
        Runtime {
            major: major.saturating_sub(RUNTIME_OFFSET),
            minor: 0,
        }
    }

    /// The newest runtime the real service tier of a BC major publishes. It is higher than `for_bc`
    /// for the majors the corpus has probed: BC 27.5 accepts up to 16.1 and BC 28.5 up to 17.1
    /// (corpus run 39110861115, refusals in run 39108859837); BC 29.0 accepts 18.0 (same runs).
    /// A major not probed yet gets `for_bc`.
    pub fn max_for_bc(major: u16) -> Runtime {
        match major {
            27 => Runtime { major: 16, minor: 1 },
            28 => Runtime { major: 17, minor: 1 },
            _ => Runtime::for_bc(major),
        }
    }

    /// The lowest runtime the runtime supports (BC 27's).
    pub fn floor() -> Runtime {
        Runtime::for_bc(MIN_BC_MAJOR)
    }

    pub fn parse(s: &str) -> Result<Runtime, VersionError> {
        let parts = numbers(s, 2)?;
        Ok(Runtime {
            major: parts[0],
            minor: parts.get(1).copied().unwrap_or(0),
        })
    }
}

impl fmt::Display for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Target {
    /// The target for a BC version with the runtime that ships with it.
    pub fn for_bc(bc: BcVersion) -> Target {
        let runtime = Runtime::for_bc(bc.major);
        Target {
            bc,
            runtime,
            app_runtimes: Vec::new(),
        }
    }

    pub fn with_runtime(self, runtime: Runtime) -> Target {
        Target { runtime, ..self }
    }
}

/// The preprocessor symbols the corpus CI defines for a BC major, cumulative:
/// `BC27PLUS` through `BC<major>PLUS` (ADR-015 3.9). Only the corpus uses this
/// convention; a third-party app gets only its own `preprocessorSymbols`.
pub fn corpus_symbols(major: u16) -> BTreeSet<String> {
    (MIN_BC_MAJOR..=major.max(MIN_BC_MAJOR))
        .map(|n| format!("BC{n}PLUS"))
        .collect()
}

/// Dot-separated decimal parts, at most `max` of them.
fn numbers(s: &str, max: usize) -> Result<Vec<u16>, VersionError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(VersionError::Empty);
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() > max {
        return Err(VersionError::TooManyParts(s.to_string()));
    }
    // A build counter can exceed u16 (`29.0.54011.55816`); only major and minor must fit.
    parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return Err(VersionError::BadPart(p.to_string()));
            }
            match p.parse::<u16>() {
                Ok(n) => Ok(n),
                Err(_) if i >= 2 => Ok(u16::MAX),
                Err(_) => Err(VersionError::BadPart(p.to_string())),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bc(s: &str) -> BcVersion {
        BcVersion::parse(s).unwrap()
    }

    #[test]
    fn runtime_is_bc_major_minus_eleven() {
        assert_eq!(Runtime::for_bc(27), Runtime { major: 16, minor: 0 });
        assert_eq!(Runtime::for_bc(28), Runtime { major: 17, minor: 0 });
        assert_eq!(Runtime::for_bc(29), Runtime { major: 18, minor: 0 });
        assert_eq!(Runtime::floor(), Runtime::for_bc(27));
    }

    #[test]
    fn parses_major_minor_and_full_build() {
        assert_eq!((bc("29").major, bc("29").minor), (29, 0));
        assert_eq!((bc("27.5").major, bc("27.5").minor), (27, 5));
        let full = bc("29.0.54011.55816");
        assert_eq!((full.major, full.minor), (29, 0));
        assert_eq!(full.build.as_deref(), Some("29.0.54011.55816"));
        assert_eq!(bc("27.5").build, None);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(BcVersion::parse(""), Err(VersionError::Empty));
        assert!(matches!(BcVersion::parse("x.1"), Err(VersionError::BadPart(_))));
        assert!(matches!(BcVersion::parse("27..1"), Err(VersionError::BadPart(_))));
        assert!(matches!(
            BcVersion::parse("1.2.3.4.5"),
            Err(VersionError::TooManyParts(_))
        ));
    }

    #[test]
    fn compares_by_major_and_minor_only() {
        assert!(bc("28.5") > bc("28.0"));
        assert!(bc("29") > bc("28.5"));
        assert_eq!(bc("29.0.1.2"), bc("29.0"));
        assert!(bc("27.10") > bc("27.5"));
    }

    #[test]
    fn prefix_matches_whole_segments() {
        let v = bc("27.5.46862.53931");
        assert!(v.matches_prefix("27"));
        assert!(v.matches_prefix("27.5"));
        assert!(v.matches_prefix("27.5.46862"));
        assert!(!v.matches_prefix("27.50"));
        assert!(!v.matches_prefix("2"));
        assert!(!v.matches_prefix("28"));
        assert!(bc("27.50").matches_prefix("27.50"));
    }

    #[test]
    fn parses_runtime() {
        assert_eq!(Runtime::parse("16.0"), Ok(Runtime { major: 16, minor: 0 }));
        assert_eq!(Runtime::parse("17"), Ok(Runtime { major: 17, minor: 0 }));
        assert!(Runtime::parse("a.b").is_err());
        assert!(Runtime::parse("1.2.3").is_err());
        assert!(Runtime { major: 17, minor: 0 } > Runtime { major: 16, minor: 9 });
    }

    #[test]
    fn target_takes_the_runtime_of_its_bc_major() {
        let t = Target::for_bc(bc("29.0"));
        assert_eq!(t.runtime, Runtime { major: 18, minor: 0 });
        let t = t.with_runtime(Runtime { major: 16, minor: 0 });
        assert_eq!(t.bc.major, 29);
        assert_eq!(t.runtime.major, 16);
    }

    #[test]
    fn corpus_symbols_are_cumulative_from_27() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<BTreeSet<_>>();
        assert_eq!(corpus_symbols(27), s(&["BC27PLUS"]));
        assert_eq!(corpus_symbols(28), s(&["BC27PLUS", "BC28PLUS"]));
        assert_eq!(corpus_symbols(29), s(&["BC27PLUS", "BC28PLUS", "BC29PLUS"]));
        assert!(corpus_symbols(30).contains("BC30PLUS"));
    }

    #[test]
    fn corpus_symbols_below_the_floor_still_define_the_floor_symbol() {
        // The runtime never runs below BC 27; a lower major gets BC 27's set.
        assert_eq!(corpus_symbols(26), corpus_symbols(27));
    }
}
