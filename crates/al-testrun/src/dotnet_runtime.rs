//! Which `Microsoft.NETCore.App` the .NET sidecar runs on (ADR-014 section 2; al-runner issues 6924 and 5590).
//!
//! The sidecar is built once, for `net8.0`, and runs on the runtime major of the BC service tier in use: BC loads its
//! service-tier assemblies into that runtime, and an assembly built for a newer major does not load into an older one.
//! When the run has a service tier (a fetched one, `al_deps::service_tier`, or an extracted artifact), the major is the
//! one its runtime configuration names ([`ServiceRuntime`], `al_deps::service_runtime`): BC 27 and 28 builds of the
//! same minor run on .NET 8 or on .NET 10 (the measured list is in `al_deps::service_runtime`). Without a service tier
//! the BC major's table below decides, as measured on these builds (the `TargetFrameworkAttribute` of each assembly,
//! and the `tfm` of `Microsoft.Dynamics.Nav.Server.runtimeconfig.json` where the artifact has it):
//! - BC 27.5 (27.5.46862.48827) and 28.1 (28.1.49838.50621): `Microsoft.Dynamics.Nav.Server.dll`,
//!   `Microsoft.Dynamics.Nav.OpenXml.dll` and `System.Drawing.Common.dll` target `.NETCoreApp,Version=v8.0`; the
//!   runtimeconfig says `"tfm": "net8.0"`, `Microsoft.NETCore.App 8.0.0`.
//! - BC 29.0 (29.0.54011.56307): `Microsoft.Dynamics.Nav.OpenXml.dll` and `System.Drawing.Common.dll` target
//!   `.NETCoreApp,Version=v10.0` and reference `System.Runtime 10.0.0.0`. On .NET 8 the type load of
//!   `Microsoft.Dynamics.Nav.OpenXml.Spreadsheet.WorkbookWriter` fails with "Could not load file or assembly
//!   'System.Runtime, Version=10.0.0.0'"; on .NET 10 it loads.
//!
//! A newer BC major than the table knows runs on the newest major the table names: the sidecar then says which
//! assembly it could not load, and the table gets its row.
//!
//! The process is started as `dotnet exec --fx-version <version> AlDotNetSidecar.dll`, so the version is the one
//! chosen here. `DOTNET_ROLL_FORWARD=Major`, used before, never left .NET 8 when .NET 8 was installed: it rolls forward
//! only when the requested major is missing, which is why `--dotnet-allow-roll-forward` changed nothing on a machine
//! with .NET 8 and 10.
//!
//! Which `Microsoft.NETCore.App` the sidecar runs on: the major the runtime configuration of the
//! run's service tier names (`ServiceRuntime`: a fetched tier or an extracted service folder; an
//! unreadable one refuses every DotNet call), else the BC major's table (8 for BC 27-28, 10 for BC 29),
//! and the newest installed patch of it, or a newer major with `--dotnet-allow-roll-forward`.
//! Entry points: `ServiceRuntime`, `need`, `start_note` (asked once at the start of an offline run), `runtime_major_for_bc`,
//! `pick_runtime`.

use al_deps::artifact::Cdn;
use al_deps::service_tier::ServiceTier;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The runtime major the sidecar needs, and whose service tier needs it (`BC 29`, `BC build 27.5.54899.58349`), for the
/// refusal text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    pub major: u32,
    pub of: Option<String>,
}

/// The function of a [`ServiceRuntime`]: the need, or why it cannot be known.
pub type NeedFn = dyn Fn() -> Result<Need, String> + Send + Sync;

/// The service tier the run's DotNet types load from, asked which runtime it runs on when the sidecar is first resolved
/// (`crate::dotnet::SidecarConfig::service_runtime`). An error refuses every DotNet call with its text.
#[derive(Clone)]
pub struct ServiceRuntime(pub Arc<NeedFn>);

impl std::fmt::Debug for ServiceRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ServiceRuntime")
    }
}

impl ServiceRuntime {
    /// The fetched service tier of a build: its runtime configuration, from the cache or the platform artifact.
    pub fn of_tier<C: Cdn + Send + Sync + 'static>(tier: Arc<ServiceTier<C>>) -> ServiceRuntime
    where
        C::Src: Send,
    {
        ServiceRuntime(Arc::new(move || {
            let of = format!("BC build {}", tier.build());
            al_deps::service_runtime::major_of_tier(&tier)
                .map(|major| Need {
                    major,
                    of: Some(of.clone()),
                })
                .map_err(|e| format!("the .NET runtime of {of}'s service tier is not known: {e}"))
        }))
    }

    /// The service folder of an extracted artifact, the folder the sidecar loads the service-tier assemblies from.
    pub fn of_dir(dir: PathBuf) -> ServiceRuntime {
        ServiceRuntime(Arc::new(move || {
            let of = format!("the service folder {}", dir.display());
            match al_deps::service_runtime::major_in_dir(&dir) {
                Some(Ok(major)) => Ok(Need { major, of: Some(of) }),
                Some(Err(e)) => Err(format!("the .NET runtime of {of} is not known: {e}")),
                None => Err(format!(
                    "the .NET runtime of {of} is not known: it has no {}",
                    al_deps::service_runtime::SERVER_RUNTIME_CONFIG
                )),
            }
        }))
    }
}

/// The runtime when the sidecar loads the service-tier assemblies from the extracted folder `dir`. The folder answers when
/// it is the run build's own folder (`own_dir`) or the run has no build (`run_tier` `None`). The folder of another build of
/// the major (`crate::dotnet::default_probe_dir`'s fall back) answers only when its major equals the one the run build's
/// own service tier names, and then as that tier; else every DotNet call is refused, naming both.
pub fn of_extracted(dir: PathBuf, own_dir: Option<&Path>, run_tier: Option<ServiceRuntime>) -> ServiceRuntime {
    let is_own = own_dir == Some(dir.as_path());
    let folder = ServiceRuntime::of_dir(dir);
    if is_own {
        return folder;
    }
    match run_tier {
        Some(run) => ServiceRuntime(Arc::new(move || {
            let (f, r) = ((folder.0)()?, (run.0)()?);
            if f.major == r.major {
                return Ok(r);
            }
            Err(format!(
                "the assemblies come from {}, of another build of the major, which runs on .NET {}; {}'s service tier runs on .NET {}",
                f.of.unwrap_or_default(),
                f.major,
                r.of.unwrap_or_default(),
                r.major
            ))
        })),
        None => folder,
    }
}

/// Run start (issue 8977): with downloading off, asks the run's service tier once which runtime it runs on, before any
/// test, from the cache and the disk only; the answer names the build and the missing file here, once, instead of first
/// at a DotNet call. With downloading on it asks nothing and makes no request: the tier is asked at the first DotNet call.
/// `Some`: the line to print, when every DotNet call of the run will be refused with that reason.
pub fn start_note(service: Option<&ServiceRuntime>, download: bool) -> Option<String> {
    if download {
        return None;
    }
    let why = (service?.0)().err()?;
    Some(format!("warning: every DotNet call of this run will be refused: {why}"))
}

/// What the sidecar needs: the service tier's own answer when the run has one, else the BC major's table.
pub fn need(service: Option<&ServiceRuntime>, bc_major: Option<u32>) -> Result<Need, String> {
    match service {
        Some(s) => (s.0)(),
        None => Ok(Need {
            major: runtime_major_for_bc(bc_major),
            of: bc_major.map(|m| format!("BC {m}")),
        }),
    }
}

/// The `Microsoft.NETCore.App` major the service tier of BC `bc_major` runs on when the run has no service tier to ask
/// (`al_version::dotnet_major_for_bc`, which holds the measured table); `None` (no BC version known, such as a run with
/// no platform package) gives 8, the major of the sidecar's build.
pub fn runtime_major_for_bc(bc_major: Option<u32>) -> u32 {
    bc_major.map_or(8, |m| {
        al_version::dotnet_major_for_bc(u16::try_from(m).unwrap_or(u16::MAX))
    })
}

fn major(v: &str) -> Option<u32> {
    v.split('.').next()?.parse().ok()
}

/// A version's numeric parts, for ordering (`10.0.13` after `10.0.8`); a part that is not a number orders first.
fn parts(v: &str) -> Vec<u64> {
    v.split(['.', '-']).map(|p| p.parse().unwrap_or(0)).collect()
}

/// The runtime the sidecar runs on, picked from the installed `versions` (`dotnet --list-runtimes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeChoice {
    /// The version passed to `--fx-version`.
    pub version: String,
    /// The major the BC version needs.
    pub needed: u32,
    /// Whether `version` is of a newer major than `needed` (`--dotnet-allow-roll-forward`).
    pub rolled_forward: bool,
}

/// Picks the newest installed version of the needed major. When there is none and `allow_roll_forward` is set, the
/// newest version of the lowest installed major above it. Otherwise the reason, as the refusal of every DotNet call.
pub fn pick_runtime(versions: &[String], need: &Need, allow_roll_forward: bool) -> Result<RuntimeChoice, String> {
    let needed = need.major;
    let newest_of = |m: u32| {
        versions
            .iter()
            .filter(|v| major(v) == Some(m))
            .max_by_key(|v| parts(v))
            .cloned()
    };
    if let Some(version) = newest_of(needed) {
        return Ok(RuntimeChoice {
            version,
            needed,
            rolled_forward: false,
        });
    }
    let higher = versions.iter().filter_map(|v| major(v)).filter(|m| *m > needed).min();
    if allow_roll_forward {
        if let Some(version) = higher.and_then(newest_of) {
            return Ok(RuntimeChoice {
                version,
                needed,
                rolled_forward: true,
            });
        }
    }
    let found = if versions.is_empty() {
        "no Microsoft.NETCore.App runtime".to_string()
    } else {
        format!("found {}", versions.join(", "))
    };
    let why = match &need.of {
        Some(of) => format!(" ({of}'s service tier runs on .NET {needed})"),
        None => String::new(),
    };
    let hint = if higher.is_some() {
        " (--dotnet-allow-roll-forward runs it on a newer major)"
    } else {
        ""
    };
    Err(format!(
        "needs the .NET {needed} runtime{why}; {found}; install Microsoft.NETCore.App {needed}.x{hint}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vs(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn of_bc(major: u32, bc: Option<u32>) -> Need {
        Need {
            major,
            of: bc.map(|m| format!("BC {m}")),
        }
    }

    /// An extracted service folder whose runtime configuration names `netcore`.
    fn folder(root: &Path, build: &str, netcore: &str) -> PathBuf {
        let dir = root.join(build).join("Service");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = format!(
            r#"{{"runtimeOptions":{{"frameworks":[{{"name":"Microsoft.NETCore.App","version":"{netcore}"}}]}}}}"#
        );
        std::fs::write(dir.join(al_deps::service_runtime::SERVER_RUNTIME_CONFIG), cfg).unwrap();
        dir
    }

    /// The run build's own service tier, answering `major` (or an error when `None`).
    fn run_tier(build: &str, major: Option<u32>) -> ServiceRuntime {
        let of = format!("BC build {build}");
        ServiceRuntime(Arc::new(move || {
            major
                .map(|major| Need {
                    major,
                    of: Some(of.clone()),
                })
                .ok_or_else(|| format!("the .NET runtime of {of}'s service tier is not known: not cached"))
        }))
    }

    #[test]
    fn another_builds_folder_on_another_major_is_refused_naming_both_builds() {
        let t = tempfile::tempdir().unwrap();
        let dir = folder(t.path(), "27.5.54899.58349", "10.0.0");
        let rt = of_extracted(dir, None, Some(run_tier("27.5.46862.58057", Some(8))));
        let why = need(Some(&rt), Some(27)).unwrap_err();
        assert!(
            why.contains("27.5.54899.58349") && why.contains("BC build 27.5.46862.58057") && why.contains(".NET 10"),
            "{why}"
        );
        // The mirror shape: the run build on .NET 10, the other build's folder on .NET 8.
        let dir = folder(t.path(), "27.5.46862.56249", "8.0.0");
        let rt = of_extracted(dir, None, Some(run_tier("27.5.54899.58349", Some(10))));
        assert!(need(Some(&rt), Some(27)).is_err());
    }

    #[test]
    fn another_builds_folder_on_the_same_major_answers_with_the_run_builds_tier() {
        let t = tempfile::tempdir().unwrap();
        let dir = folder(t.path(), "27.5.46862.56249", "8.0.0");
        let rt = of_extracted(dir, None, Some(run_tier("27.5.46862.58057", Some(8))));
        assert_eq!(
            need(Some(&rt), Some(27)),
            Ok(Need {
                major: 8,
                of: Some("BC build 27.5.46862.58057".into())
            })
        );
        // An unknown run build's tier refuses too: the other build's folder alone does not decide.
        let dir = folder(t.path(), "27.5.46862.56250", "8.0.0");
        let rt = of_extracted(dir, None, Some(run_tier("27.5.46862.58057", None)));
        assert!(need(Some(&rt), Some(27)).unwrap_err().contains("not cached"));
    }

    #[test]
    fn the_run_builds_own_folder_or_a_run_without_a_build_takes_the_folders_answer() {
        let t = tempfile::tempdir().unwrap();
        let dir = folder(t.path(), "27.5.54899.58349", "10.0.0");
        // The run build's own tier is not asked (it would refuse).
        let own = of_extracted(dir.clone(), Some(&dir), Some(run_tier("27.5.54899.58349", None)));
        assert_eq!(need(Some(&own), Some(27)).map(|n| n.major), Ok(10));
        let no_build = of_extracted(dir, None, None);
        assert_eq!(need(Some(&no_build), Some(27)).map(|n| n.major), Ok(10));
    }

    /// Issue 8977: offline, a fetched tier whose runtime configuration the cache lacks is named once at the start of the
    /// run, with the build and the file's place in the cache, not first at a DotNet call.
    #[test]
    fn the_start_note_names_the_build_and_the_missing_file_of_an_offline_tier() {
        let cache = tempfile::tempdir().unwrap();
        let build = al_deps::build::BuildId::parse("29.0.54011.55816").unwrap();
        let tier = Arc::new(ServiceTier::new(
            al_deps::artifact::HttpCdn::new("http://127.0.0.1:9"),
            cache.path(),
            build.clone(),
            false,
        ));
        let file = al_deps::service_tier::service_dir(cache.path(), &build)
            .join(al_deps::service_runtime::SERVER_RUNTIME_CONFIG);
        let note = start_note(Some(&ServiceRuntime::of_tier(tier)), false).unwrap();
        assert!(
            note.starts_with("warning: every DotNet call of this run will be refused: ")
                && note.contains("BC build 29.0.54011.55816")
                && note.contains(&file.display().to_string())
                && note.contains("downloading is off"),
            "{note}"
        );
        // A tier that answers, and a run without one, have no note.
        let t = tempfile::tempdir().unwrap();
        let dir = folder(t.path(), "29.0.54011.55816", "10.0.0");
        assert_eq!(start_note(Some(&ServiceRuntime::of_dir(dir)), false), None);
        assert_eq!(start_note(None, false), None);
    }

    /// Issue 8977, review round 1: with downloading on, the start of a run does not ask the service tier at all, so it
    /// makes no request; the tier is asked at the first DotNet call, as before the start note.
    #[test]
    fn with_downloading_on_the_start_note_does_not_ask_the_service_tier() {
        use std::sync::atomic::{AtomicU32, Ordering};
        let calls = Arc::new(AtomicU32::new(0));
        let counted = calls.clone();
        let rt = ServiceRuntime(Arc::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
            Err("asked".into())
        }));
        assert_eq!(start_note(Some(&rt), true), None);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        // An online tier whose feed cannot be reached: no note, and nothing is written into its folder.
        let cache = tempfile::tempdir().unwrap();
        let build = al_deps::build::BuildId::parse("29.0.54011.55816").unwrap();
        let tier = Arc::new(ServiceTier::new(
            al_deps::artifact::HttpCdn::new("http://127.0.0.1:9"),
            cache.path(),
            build.clone(),
            true,
        ));
        assert_eq!(start_note(Some(&ServiceRuntime::of_tier(tier)), true), None);
        assert!(!al_deps::service_tier::service_dir(cache.path(), &build).exists());
    }

    #[test]
    fn bc_27_and_28_run_on_net8_and_bc_29_on_net10() {
        assert_eq!(runtime_major_for_bc(None), 8);
        assert_eq!(runtime_major_for_bc(Some(27)), 8);
        assert_eq!(runtime_major_for_bc(Some(28)), 8);
        assert_eq!(runtime_major_for_bc(Some(29)), 10);
        assert_eq!(runtime_major_for_bc(Some(30)), 10);
    }

    #[test]
    fn the_newest_patch_of_the_needed_major_is_picked_even_when_8_is_installed() {
        let installed = vs(&["8.0.32", "9.0.15", "10.0.8", "10.0.13"]);
        let c = pick_runtime(&installed, &of_bc(10, Some(29)), false).unwrap();
        assert_eq!(c.version, "10.0.13");
        assert!(!c.rolled_forward);
        assert_eq!(
            pick_runtime(&installed, &of_bc(8, Some(28)), false).unwrap().version,
            "8.0.32"
        );
    }

    #[test]
    fn a_missing_major_is_refused_with_the_bc_version_and_what_was_found() {
        let why = pick_runtime(&vs(&["8.0.32"]), &of_bc(10, Some(29)), false).unwrap_err();
        assert_eq!(
            why,
            "needs the .NET 10 runtime (BC 29's service tier runs on .NET 10); found 8.0.32; install Microsoft.NETCore.App 10.x"
        );
        let why = pick_runtime(&[], &of_bc(8, None), false).unwrap_err();
        assert_eq!(
            why,
            "needs the .NET 8 runtime; no Microsoft.NETCore.App runtime; install Microsoft.NETCore.App 8.x"
        );
    }

    #[test]
    fn roll_forward_takes_the_lowest_higher_major_only_when_the_needed_one_is_missing() {
        let installed = vs(&["9.0.6", "9.0.22", "10.0.14"]);
        let why = pick_runtime(&installed, &of_bc(8, None), false).unwrap_err();
        assert!(why.contains("--dotnet-allow-roll-forward"), "{why}");
        let c = pick_runtime(&installed, &of_bc(8, None), true).unwrap();
        assert_eq!((c.version.as_str(), c.rolled_forward), ("9.0.22", true));
        // With the needed major installed, the flag changes nothing.
        let c = pick_runtime(&vs(&["8.0.33", "10.0.14"]), &of_bc(8, Some(28)), true).unwrap();
        assert_eq!((c.version.as_str(), c.rolled_forward), ("8.0.33", false));
    }
}
