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

use al_deps::artifact::Cdn;
use al_deps::service_tier::ServiceTier;
use std::path::PathBuf;
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
