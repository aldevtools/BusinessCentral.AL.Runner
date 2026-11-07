//! The OS variables the .NET sidecar keeps from the host environment (ADR-014 section 9 clears the rest).
//!
//! On Windows the .NET runtime reads `SystemRoot` to find the system directory. Without it CryptoAPI
//! (`RSACryptoServiceProvider`), Winsock and X509 fail with "environment is incorrect"/missing-provider
//! errors (reporter measurement: `SystemRoot` alone restores them; runner issue 6793). `windir` is the
//! legacy alias of the same path and `SystemDrive` is the drive of it; both are read by the same
//! Windows APIs, so they follow. All three are fixed per machine, so runs stay repeatable.
//! Nothing user-specific (PATH, USERNAME, the host's TEMP, ...) passes. The sidecar's HOME, its temporary folder
//! (TMP, TEMP and TMPDIR, so no platform falls back to a shared folder such as `C:\Windows`; issue 6954) and its
//! working directory are per-process empty folders, removed with the process. Only files .NET opens inside a member
//! AL called (ADR-014 section 8 item 6, OpenXml's own temporary copy of a workbook) land there; AL's own file calls
//! and .NET file calls AL makes stay in the in-memory file system (ADR-056).

use std::ffi::OsString;
use std::path::Path;

/// The variables every sidecar process gets, whatever the host holds: its own `home` and `tmp` folders (per process,
/// removed with it), UTC, and the runtime settings. Set after the environment is cleared (ADR-014 section 9).
pub fn fixed(home: &Path, tmp: &Path) -> Vec<(&'static str, OsString)> {
    let tmp = tmp.as_os_str().to_os_string();
    vec![
        ("HOME", home.as_os_str().to_os_string()),
        // Windows `Path.GetTempPath` reads TMP, then TEMP, and falls back to the Windows folder (issue 6954).
        ("TMP", tmp.clone()),
        ("TEMP", tmp.clone()),
        ("TMPDIR", tmp),
        ("TZ", "UTC".into()),
        ("DOTNET_NOLOGO", "1".into()),
        ("DOTNET_CLI_TELEMETRY_OPTOUT", "1".into()),
        ("DOTNET_TieredPGO", "0".into()),
        ("DOTNET_TC_QuickJitForLoops", "1".into()),
        ("DOTNET_gcServer", "0".into()),
    ]
}

/// Variables passed through on Windows. Windows variable names are case-insensitive, so the lookup is too.
pub const WINDOWS_PASSTHROUGH: &[&str] = &["SystemRoot", "windir", "SystemDrive"];

/// The `(name, value)` pairs to set on the sidecar for `names`, looked up with `get` (the host environment).
/// Names `get` does not know are skipped.
pub fn passthrough(names: &[&'static str], get: impl Fn(&str) -> Option<OsString>) -> Vec<(&'static str, OsString)> {
    names
        .iter()
        .filter_map(|n| get(n).filter(|v| !v.is_empty()).map(|v| (*n, v)))
        .collect()
}

/// What the sidecar keeps from the real environment on this platform.
pub fn host_passthrough() -> Vec<(&'static str, OsString)> {
    if cfg!(windows) {
        passthrough(WINDOWS_PASSTHROUGH, |n| std::env::var_os(n))
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_known_names_that_are_set() {
        let env = |n: &str| match n {
            "SystemRoot" => Some(OsString::from(r"C:\Windows")),
            "windir" => Some(OsString::from(r"C:\Windows")),
            "PATH" => Some(OsString::from("x")),
            "SystemDrive" => Some(OsString::new()),
            _ => None,
        };
        let got = passthrough(WINDOWS_PASSTHROUGH, env);
        let names: Vec<_> = got.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["SystemRoot", "windir"]);
    }

    /// Issue 6954: on Windows `Path.GetTempPath` reads `TMP`, then `TEMP`, then falls back to the Windows folder
    /// (`C:\\Windows\\tmpXXX.tmp`, "Access to the path ... is denied" inside OpenXml's `WorkbookReader.Open`). All
    /// three names must point at the process's own temporary folder, so no platform falls back to a shared one.
    #[test]
    fn every_temp_variable_names_the_process_temp_folder() {
        let (home, tmp) = (Path::new("/w/home"), Path::new("/w/tmp"));
        let env = fixed(home, tmp);
        for name in ["TMP", "TEMP", "TMPDIR"] {
            let got = env.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone());
            assert_eq!(got, Some(OsString::from("/w/tmp")), "{name}");
        }
        assert!(env.iter().any(|(n, v)| *n == "HOME" && v == "/w/home"));
    }

    #[test]
    fn system_root_is_always_on_the_windows_list() {
        assert!(WINDOWS_PASSTHROUGH.contains(&"SystemRoot"));
        assert!(!WINDOWS_PASSTHROUGH.contains(&"PATH"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_host_passes_system_root() {
        let got = host_passthrough();
        assert!(got.iter().any(|(n, v)| *n == "SystemRoot" && !v.is_empty()));
    }

    #[cfg(not(windows))]
    #[test]
    fn other_hosts_pass_nothing() {
        assert!(host_passthrough().is_empty());
    }
}
