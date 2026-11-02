//! The OS variables the .NET sidecar keeps from the host environment (ADR-014 section 9 clears the rest).
//!
//! On Windows the .NET runtime reads `SystemRoot` to find the system directory. Without it CryptoAPI
//! (`RSACryptoServiceProvider`), Winsock and X509 fail with "environment is incorrect"/missing-provider
//! errors (reporter measurement: `SystemRoot` alone restores them; runner issue 6793). `windir` is the
//! legacy alias of the same path and `SystemDrive` is the drive of it; both are read by the same
//! Windows APIs, so they follow. All three are fixed per machine, so runs stay repeatable.
//! Nothing user-specific (PATH, USERNAME, TEMP, ...) passes, and the in-memory file system rule is not
//! affected: the sidecar's HOME, TMPDIR and working directory stay the per-run empty folders.

use std::ffi::OsString;

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
