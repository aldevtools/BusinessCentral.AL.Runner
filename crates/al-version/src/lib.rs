//! BC versions and the run target (ADR-015).
//!
//! A leaf crate: no dependencies, no OS access (ADR-006). Everything that
//! depends on the Business Central version is a function of a [`Target`].
//!
//! `BcVersion::resolved_to` makes the target the build the Microsoft apps came from (minor and build
//! string; both CLIs call it, al-runner issue 8209). Entry points: `VersionError`, `BcVersion`,
//! `Runtime`, `Target`.

mod awaited;
mod choose;
mod gates;
mod symbols;
pub use awaited::{
    BulkWrite, bulk_commit_ends_cached_row, http_probe_1424_measured, journal_temp_part_reads_after_refresh,
    temporary_table_blob_line_breaks_measured, visible_names_row_field_reads_yes, web_service_session_measured,
};
pub use choose::{AppVersions, Chosen, TargetError, choose_target};
pub use gates::{
    Evidence, FeatureKeyRows, Gate, dev_endpoint_answers_unhandled_ui, feature_key_refusal, feature_key_rows,
    getfield_finds_extension_fields, has as gate_on, other_record_option_member_names_confirmed,
    platform_table_scopes_known, test_runner_root_scope,
};
pub use symbols::{ReleasedMinors, corpus_symbols};

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

    /// The run's version once the Microsoft apps were found: `build` is that build (`29.1.57326.58275`). The minor follows
    /// it, so a major-only `--bc-version 29` on a 29.1 build is the target `29.1` names (al-runner issue 8209). A build of
    /// another major is an error: the run would answer from the lists of a major the apps do not come from (al-runner
    /// issue 8243).
    pub fn resolved_to(&mut self, build: &str) -> Result<(), String> {
        if let Ok(b) = BcVersion::parse(build) {
            if b.major != self.major {
                return Err(format!(
                    "the Microsoft apps come from build {build} (BC {}), but the run targets BC {}; name the build to run on with --bc-version",
                    b.major, self.major
                ));
            }
            self.minor = b.minor;
        }
        self.build = Some(build.to_string());
        Ok(())
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

/// The `Microsoft.NETCore.App` major the service tier of BC `major` runs on, measured on the platform artifacts
/// (`TargetFrameworkAttribute` of the service-tier assemblies; `"tfm"` of `Microsoft.Dynamics.Nav.Server.runtimeconfig.json`
/// where the artifact has it): 27.5.46862.48827 and 28.1.49838.50621 target `net8.0`; 29.0.54011.56307's
/// `Microsoft.Dynamics.Nav.OpenXml.dll` and `System.Drawing.Common.dll` target `.NETCoreApp,Version=v10.0` and reference
/// `System.Runtime 10.0.0.0` (al-runner issues 6924 and 5590). A newer major than the table knows gets the newest major
/// it names: the .NET helper then says which assembly did not load, and the table gets its row. Used only by a run with no
/// service tier to ask: BC 27 and 28 builds published from 2026-11-17 run on .NET 10, which their service tier's runtime
/// configuration says (`al_deps::service_runtime`).
pub fn dotnet_major_for_bc(major: u16) -> u32 {
    match major {
        ..=28 => 8,
        _ => 10,
    }
}

/// The builds whose service-tier runtime configuration was measured (`al_deps::service_runtime`, 2026-11-17), with the
/// `Microsoft.NETCore.App` major their runtime configuration names. A test of `al-test-cli` requires every `bc_build` of `oracle/oracle.toml` here, with the major the corpus sidecar's table
/// gives its BC major (the corpus sidecar asks no service tier).
pub const MEASURED_SERVICE_RUNTIMES: &[(&str, u32)] = &[
    ("26.5.38752.58303", 8),
    ("27.5.46862.56249", 8),
    ("27.5.46862.58057", 8),
    ("27.5.46862.58294", 8),
    ("28.5.54151.56267", 8),
    ("28.5.54151.58194", 8),
    ("27.0.54905.58351", 10),
    ("27.5.54899.58349", 10),
    ("28.5.54976.58348", 10),
    ("29.0.54011.56262", 10),
    ("29.0.54011.58356", 10),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn bc(s: &str) -> BcVersion {
        BcVersion::parse(s).unwrap()
    }

    // al-runner issue 8209: `--bc-version 29`, `29.1` and the full build pick the same 29.1 build, so the target is that
    // build however it was named. With no platform build known the Feature Key rows measured on 29.0 are refused in all
    // three (the 29.1 list follows the platform build, `feature_key_rows`).
    #[test]
    fn the_target_follows_the_build_the_microsoft_apps_come_from() {
        let build = "29.1.57326.58275";
        for asked in ["29", "29.1", build] {
            let mut t = Target::for_bc(bc(asked));
            t.bc.resolved_to(build).unwrap();
            assert_eq!(
                (t.bc.major, t.bc.minor, t.bc.to_string()),
                (29, 1, build.to_string()),
                "{asked}"
            );
            assert_eq!(feature_key_rows(Some(&t), None), None, "{asked}");
        }
        // Neighbor: a 29.0 build keeps main's 29.0 rows.
        let mut t = Target::for_bc(bc("29"));
        t.bc.resolved_to("29.0.54011.56262").unwrap();
        assert_eq!(feature_key_rows(Some(&t), None), Some(FeatureKeyRows::Bc29));
    }

    // al-runner issue 8243: Microsoft apps of another major than the target's stop the run with both versions named, so
    // no reader of the minor answers from the list of a major the apps do not come from.
    #[test]
    fn a_build_of_another_major_is_an_error_that_names_both() {
        let mut t = Target::for_bc(bc("28"));
        let e = t.bc.resolved_to("27.5.46862.58057").unwrap_err();
        assert!(e.contains("27.5.46862.58057") && e.contains("BC 28"), "{e}");
        // Neighbor: a build of the same major still sets the minor.
        let mut t = Target::for_bc(bc("28"));
        t.bc.resolved_to("28.5.54151.58194").unwrap();
        assert_eq!((t.bc.major, t.bc.minor), (28, 5));
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
    fn bc_27_and_28_run_on_net8_and_bc_29_on_net10() {
        assert_eq!(dotnet_major_for_bc(27), 8);
        assert_eq!(dotnet_major_for_bc(28), 8);
        assert_eq!(dotnet_major_for_bc(29), 10);
        assert_eq!(dotnet_major_for_bc(30), 10);
    }
}
