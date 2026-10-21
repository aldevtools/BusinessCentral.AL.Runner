//! How the target of a run is chosen (ADR-015 3.2, 3.8).
//!
//! Pure: callers read `--bc-version` and app.json and pass the values in.

use crate::{BcVersion, MIN_BC_MAJOR, Runtime, Target, VersionError};
use std::fmt;

/// The version fields of an app.json. Each is optional, as in the manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppVersions {
    /// `runtime`, for example `16.0`.
    pub runtime: Option<Runtime>,
    /// `platform`, for example `27.0.0.0`.
    pub platform: Option<BcVersion>,
    /// `application`, for example `27.0.0.0`.
    pub application: Option<BcVersion>,
}

fn some(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

impl AppVersions {
    /// Parses the three manifest strings. An empty string counts as absent.
    pub fn parse(
        runtime: Option<&str>,
        platform: Option<&str>,
        application: Option<&str>,
    ) -> Result<AppVersions, VersionError> {
        // A manifest version is a floor, not a build: drop the build string.
        let floor = |s: Option<&str>| -> Result<Option<BcVersion>, VersionError> {
            some(s)
                .map(|s| {
                    BcVersion::parse(s).map(|mut v| {
                        v.build = None;
                        v
                    })
                })
                .transpose()
        };
        Ok(AppVersions {
            runtime: some(runtime).map(Runtime::parse).transpose()?,
            platform: floor(platform)?,
            application: floor(application)?,
        })
    }

    /// The higher of `application` and `platform` (ADR-013 Layer A step 2).
    pub fn floor(&self) -> Option<&BcVersion> {
        match (&self.application, &self.platform) {
            (Some(a), Some(p)) => Some(a.max(p)),
            (a, p) => a.as_ref().or(p.as_ref()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetError {
    /// `--bc-version` names a BC major below 27.
    BcTooOld(BcVersion),
    /// The app's runtime is above the runtime that ships with the target.
    RuntimeAboveTarget { runtime: Runtime, target: BcVersion },
}

impl fmt::Display for TargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetError::BcTooOld(v) => {
                write!(f, "BC {v} is not supported: BC {MIN_BC_MAJOR} and later only")
            }
            TargetError::RuntimeAboveTarget { runtime, target } => write!(
                f,
                "the app's runtime {runtime} is newer than BC {target} supports (runtime {})",
                Runtime::max_for_bc(target.major)
            ),
        }
    }
}

impl std::error::Error for TargetError {}

/// The target and the notes to print for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
    pub target: Target,
    pub notes: Vec<String>,
}

/// Chooses the target of a run.
///
/// 1. `requested` (`--bc-version`) wins. Below BC 27 it is an error.
/// 2. Otherwise the major of the app's floor. Below 27 it becomes BC 27, with a note.
/// 3. Otherwise `fallback` (the caller's newest verified version).
///
/// The app's `runtime` is used when present, else the runtime of the BC
/// major. Above the target's runtime is an error; below 16.0 it becomes 16.0,
/// with a note.
pub fn choose_target(
    requested: Option<&BcVersion>,
    app: &AppVersions,
    fallback: &BcVersion,
) -> Result<Chosen, TargetError> {
    let mut notes = Vec::new();
    let bc = match (requested, app.floor()) {
        (Some(v), _) if v.major < MIN_BC_MAJOR => return Err(TargetError::BcTooOld(v.clone())),
        (Some(v), _) => v.clone(),
        (None, Some(floor)) if floor.major < MIN_BC_MAJOR => {
            notes.push(format!(
                "the app's floor is BC {}.{}; BC {MIN_BC_MAJOR} is the oldest supported, so it runs as BC {MIN_BC_MAJOR}",
                floor.major, floor.minor
            ));
            BcVersion::new(MIN_BC_MAJOR, 0)
        }
        (None, Some(floor)) => BcVersion::new(floor.major, 0),
        (None, None) => fallback.clone(),
    };
    let mut target = Target::for_bc(bc);
    if let Some(runtime) = app.runtime {
        if runtime > Runtime::max_for_bc(target.bc.major) {
            return Err(TargetError::RuntimeAboveTarget {
                runtime,
                target: target.bc,
            });
        }
        if runtime < Runtime::floor() {
            notes.push(format!(
                "the app's runtime is {runtime}, older than {}, the oldest AL Runner knows the members of: a call the docs do not list for runtime {runtime} is not run",
                Runtime::floor()
            ));
            target.runtime = Runtime::floor();
        } else {
            target.runtime = runtime;
        }
    }
    Ok(Chosen { target, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bc(s: &str) -> BcVersion {
        BcVersion::parse(s).unwrap()
    }
    fn rt(s: &str) -> Runtime {
        Runtime::parse(s).unwrap()
    }
    fn app(runtime: Option<&str>, platform: Option<&str>, application: Option<&str>) -> AppVersions {
        AppVersions::parse(runtime, platform, application).unwrap()
    }

    #[test]
    fn manifest_versions_parse_and_drop_the_build() {
        let a = app(Some("16.0"), Some("27.0.0.0"), Some("28.1.0.0"));
        assert_eq!(a.runtime, Some(rt("16.0")));
        assert_eq!(a.platform.as_ref().unwrap().build, None);
        assert_eq!(a.floor(), Some(&bc("28.1")));
        assert_eq!(app(None, None, None).floor(), None);
        assert_eq!(app(Some(""), Some(""), None), AppVersions::default());
        assert!(AppVersions::parse(Some("x"), None, None).is_err());
    }

    #[test]
    fn requested_version_wins_over_the_floor() {
        let a = app(Some("16.0"), Some("27.0.0.0"), Some("27.0.0.0"));
        let c = choose_target(Some(&bc("29")), &a, &bc("27.5")).unwrap();
        assert_eq!((c.target.bc.major, c.target.runtime), (29, rt("16.0")));
        assert!(c.notes.is_empty());
    }

    #[test]
    fn floor_major_is_used_without_a_request() {
        let a = app(None, Some("28.0.0.0"), Some("27.0.0.0"));
        let c = choose_target(None, &a, &bc("29")).unwrap();
        assert_eq!(c.target.bc.major, 28);
        assert_eq!(c.target.runtime, rt("17.0"), "no app runtime: the BC major's");
    }

    #[test]
    fn no_request_and_no_floor_uses_the_fallback() {
        let c = choose_target(None, &AppVersions::default(), &bc("29")).unwrap();
        assert_eq!(c.target.bc, bc("29"));
        assert_eq!(c.target.runtime, rt("18.0"));
    }

    #[test]
    fn a_floor_below_27_runs_as_bc_27_with_a_note() {
        let a = app(None, None, Some("24.0.0.0"));
        let c = choose_target(None, &a, &bc("29")).unwrap();
        assert_eq!(c.target.bc.major, 27);
        assert_eq!(c.target.runtime, rt("16.0"));
        assert_eq!(c.notes.len(), 1);
        assert!(
            c.notes[0].contains("24") && c.notes[0].contains("BC 27"),
            "{:?}",
            c.notes
        );
    }

    #[test]
    fn a_requested_version_below_27_is_refused() {
        let e = choose_target(Some(&bc("26")), &AppVersions::default(), &bc("29")).unwrap_err();
        assert_eq!(e, TargetError::BcTooOld(bc("26")));
        assert!(e.to_string().contains("27 and later only"));
    }

    #[test]
    fn a_runtime_below_16_runs_as_16_with_a_note() {
        let a = app(Some("12.0"), None, None);
        let c = choose_target(Some(&bc("27.5")), &a, &bc("29")).unwrap();
        assert_eq!(c.target.runtime, rt("16.0"));
        assert_eq!(c.notes.len(), 1);
        assert!(c.notes[0].contains("12.0"), "{:?}", c.notes);
    }

    #[test]
    fn a_runtime_above_the_target_is_refused() {
        let a = app(Some("17.0"), None, None);
        let e = choose_target(Some(&bc("27.5")), &a, &bc("29")).unwrap_err();
        assert_eq!(
            e,
            TargetError::RuntimeAboveTarget {
                runtime: rt("17.0"),
                target: bc("27.5")
            }
        );
        // The same runtime on BC 28 is fine.
        assert!(choose_target(Some(&bc("28")), &a, &bc("29")).is_ok());
    }

    #[test]
    fn the_build_of_a_requested_version_is_kept() {
        let c = choose_target(Some(&bc("29.0.54011.55816")), &AppVersions::default(), &bc("27")).unwrap();
        assert_eq!(c.target.bc.build.as_deref(), Some("29.0.54011.55816"));
    }

    /// Corpus runs 39108859837 and 39110861115: the real service tier publishes runtime 16.1 on BC 27.5
    /// and 17.1 on BC 28.5, and refuses 18.0 there ("must be set to '16.1' or earlier", "'17.1' or earlier");
    /// BC 29.0 publishes 18.0.
    #[test]
    fn the_newest_runtime_each_major_publishes_is_accepted() {
        for (bc_v, ok, refused) in [
            ("27.5", "16.1", "16.2"),
            ("28.5", "17.1", "17.2"),
            ("29.0", "18.0", "18.1"),
        ] {
            let c = choose_target(Some(&bc(bc_v)), &app(Some(ok), None, None), &bc("29")).unwrap();
            assert_eq!(c.target.runtime, rt(ok), "BC {bc_v}");
            let e = choose_target(Some(&bc(bc_v)), &app(Some(refused), None, None), &bc("29")).unwrap_err();
            assert!(matches!(e, TargetError::RuntimeAboveTarget { .. }), "BC {bc_v}: {e}");
        }
        assert!(choose_target(Some(&bc("27.5")), &app(Some("18.0"), None, None), &bc("29")).is_err());
        assert!(choose_target(Some(&bc("28.5")), &app(Some("18.0"), None, None), &bc("29")).is_err());
    }
}
