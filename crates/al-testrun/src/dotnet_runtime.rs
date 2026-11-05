//! Which `Microsoft.NETCore.App` the .NET sidecar runs on (ADR-014 section 2; al-runner issues 6924 and 5590).
//!
//! The sidecar is built once, for `net8.0`, and runs on the runtime major of the BC service tier in use: BC loads its
//! service-tier assemblies into that runtime, and an assembly built for a newer major does not load into an older one.
//! Measured on the service tiers of the platform artifacts (the `TargetFrameworkAttribute` of each assembly, and the
//! `tfm` of `Microsoft.Dynamics.Nav.Server.runtimeconfig.json` where the artifact has it):
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

/// The `Microsoft.NETCore.App` major the service tier of BC `bc_major` runs on; `None` (no BC version known, such as
/// a run with no platform package) gives 8, the major of the sidecar's build.
pub fn runtime_major_for_bc(bc_major: Option<u32>) -> u32 {
    match bc_major {
        None => 8,
        Some(m) if m <= 28 => 8,
        // BC 29 and newer: see the module comment.
        Some(_) => 10,
    }
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

/// Picks the newest installed version of the `needed` major. When there is none and `allow_roll_forward` is set, the
/// newest version of the lowest installed major above it. Otherwise the reason, as the refusal of every DotNet call.
pub fn pick_runtime(
    versions: &[String],
    needed: u32,
    bc_major: Option<u32>,
    allow_roll_forward: bool,
) -> Result<RuntimeChoice, String> {
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
    let why = match bc_major {
        Some(bc) => format!(" (BC {bc}'s service tier runs on .NET {needed})"),
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
        let c = pick_runtime(&installed, 10, Some(29), false).unwrap();
        assert_eq!(c.version, "10.0.13");
        assert!(!c.rolled_forward);
        assert_eq!(pick_runtime(&installed, 8, Some(28), false).unwrap().version, "8.0.32");
    }

    #[test]
    fn a_missing_major_is_refused_with_the_bc_version_and_what_was_found() {
        let why = pick_runtime(&vs(&["8.0.32"]), 10, Some(29), false).unwrap_err();
        assert_eq!(
            why,
            "needs the .NET 10 runtime (BC 29's service tier runs on .NET 10); found 8.0.32; install Microsoft.NETCore.App 10.x"
        );
        let why = pick_runtime(&[], 8, None, false).unwrap_err();
        assert_eq!(
            why,
            "needs the .NET 8 runtime; no Microsoft.NETCore.App runtime; install Microsoft.NETCore.App 8.x"
        );
    }

    #[test]
    fn roll_forward_takes_the_lowest_higher_major_only_when_the_needed_one_is_missing() {
        let installed = vs(&["9.0.6", "9.0.22", "10.0.14"]);
        let why = pick_runtime(&installed, 8, None, false).unwrap_err();
        assert!(why.contains("--dotnet-allow-roll-forward"), "{why}");
        let c = pick_runtime(&installed, 8, None, true).unwrap();
        assert_eq!((c.version.as_str(), c.rolled_forward), ("9.0.22", true));
        // With the needed major installed, the flag changes nothing.
        let c = pick_runtime(&vs(&["8.0.33", "10.0.14"]), 8, Some(28), true).unwrap();
        assert_eq!((c.version.as_str(), c.rolled_forward), ("8.0.33", false));
    }
}
