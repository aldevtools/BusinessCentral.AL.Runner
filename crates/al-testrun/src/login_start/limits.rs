//! The caps of an engine's login starts (al-runner issue 8211): how many starts it keeps and how many bytes they hold
//! together, set per run (`RunOptions::login_replay_limits`, from the environment the front ends read next to
//! `ALRUNNER_LOGIN_REPLAY`). Unset, the caps are those of ADR-073 Phase 1: 8 starts and 64 MB. The cap on one start
//! (`login_start::MAX_START_BYTES`) and on the refused keys (`login_start::MAX_REFUSED`) stay fixed.
//!
//! This crate reads no environment (ADR-006): [`LoginReplayLimits::read`] takes the reader the front end passes.
//!
//! The byte cap is a hard limit on what the kept starts hold, so one check at the start keeps them inside a run's
//! memory limit (ADR-073 amendment 2026-11-26, al-runner issue 8211): [`LoginReplayLimits::check_memory_limit`]. The
//! memory pool counts the room the cap leaves the starts before it admits a thread (`Pool::reserve`,
//! `runtime/corpus.rs` `share_with_pool`); no start is dropped for memory.

use std::ffi::{OsStr, OsString};

/// The most starts an engine keeps when the run sets no other cap (ADR-073 Phase 1).
pub(crate) const DEFAULT_STARTS: usize = 8;
/// The most bytes (`LoginImage::kept_bytes`) the starts of an engine hold together when the run sets no other cap.
pub(crate) const DEFAULT_BYTES: u64 = 64 << 20;

/// The caps of the login starts an engine keeps, the least recently used out (`login_start::Starts::record`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginReplayLimits {
    /// The most starts the engine keeps.
    pub max_starts: usize,
    /// The most bytes (`LoginImage::kept_bytes`) its starts hold together.
    pub max_bytes: u64,
}

impl Default for LoginReplayLimits {
    fn default() -> Self {
        LoginReplayLimits {
            max_starts: DEFAULT_STARTS,
            max_bytes: DEFAULT_BYTES,
        }
    }
}

impl LoginReplayLimits {
    /// The variable that sets [`LoginReplayLimits::max_starts`], a count.
    pub const MAX_IMAGES_VAR: &'static str = "ALRUNNER_LOGIN_REPLAY_MAX_IMAGES";
    /// The variable that sets [`LoginReplayLimits::max_bytes`], in megabytes (MiB).
    pub const MAX_MB_VAR: &'static str = "ALRUNNER_LOGIN_REPLAY_MAX_MB";

    /// The caps the environment sets, read through `var` (the front ends pass `std::env::var_os`). An unset variable
    /// keeps its default; a value that is not a whole number from 1 up (or does not fit) is an error that names the
    /// variable, never a silent default.
    pub fn read(var: impl Fn(&str) -> Option<OsString>) -> Result<Self, String> {
        let defaults = Self::default();
        let max_starts = match var(Self::MAX_IMAGES_VAR) {
            None => defaults.max_starts,
            Some(v) => usize::try_from(whole(Self::MAX_IMAGES_VAR, &v, usize::MAX as u64)?).unwrap_or(usize::MAX),
        };
        let max_bytes = match var(Self::MAX_MB_VAR) {
            None => defaults.max_bytes,
            Some(v) => whole(Self::MAX_MB_VAR, &v, u64::MAX >> 20)? << 20,
        };
        Ok(LoginReplayLimits { max_starts, max_bytes })
    }

    /// Stops a run that keeps starts when a memory limit is set (`limit`, bytes: `--max-memory` or serve's `maxMemory`)
    /// and the byte cap is not below it, with an error that names both values (al-runner issue 8211, the maintainer's
    /// start check). Without a set limit there is nothing to compare with.
    pub fn check_memory_limit(&self, limit: Option<u64>) -> Result<(), String> {
        match limit {
            Some(limit) if self.max_bytes >= limit => Err(format!(
                "{} is {}, not below the memory limit of {} (--max-memory or maxMemory): set it below the limit, or \
                 raise the limit",
                Self::MAX_MB_VAR,
                size(self.max_bytes),
                size(limit)
            )),
            _ => Ok(()),
        }
    }
}

/// `bytes` as the error texts show a size: whole MB when it is one, else bytes.
fn size(bytes: u64) -> String {
    match bytes % (1 << 20) {
        0 => format!("{} MB", bytes >> 20),
        _ => format!("{bytes} bytes"),
    }
}

/// `value` of variable `name` as a whole number from 1 to `max`.
fn whole(name: &str, value: &OsStr, max: u64) -> Result<u64, String> {
    value
        .to_str()
        .and_then(|t| t.parse::<u64>().ok())
        .filter(|n| (1..=max).contains(n))
        .ok_or_else(|| {
            format!(
                "{name} takes a whole number from 1 to {max}, not `{}`",
                value.to_string_lossy()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::LoginReplayLimits;
    use std::ffi::OsString;

    const IMAGES: &str = LoginReplayLimits::MAX_IMAGES_VAR;
    const MB: &str = LoginReplayLimits::MAX_MB_VAR;

    /// The caps an environment of `vars` sets.
    fn read(vars: &[(&str, &str)]) -> Result<LoginReplayLimits, String> {
        LoginReplayLimits::read(|name| vars.iter().find(|(n, _)| *n == name).map(|(_, v)| OsString::from(v)))
    }

    #[test]
    fn the_variables_are_named_next_to_the_replay_switch() {
        assert_eq!(IMAGES, "ALRUNNER_LOGIN_REPLAY_MAX_IMAGES");
        assert_eq!(MB, "ALRUNNER_LOGIN_REPLAY_MAX_MB");
    }

    #[test]
    fn unset_variables_keep_the_defaults_and_set_ones_set_their_cap() {
        assert_eq!(read(&[]), Ok(LoginReplayLimits::default()));
        let both = read(&[(IMAGES, "32"), (MB, "512")]).unwrap();
        assert_eq!((both.max_starts, both.max_bytes), (32, 512 << 20));
        let images = read(&[(IMAGES, "128")]).unwrap();
        assert_eq!((images.max_starts, images.max_bytes), (128, 64 << 20));
        let mb = read(&[(MB, "1")]).unwrap();
        assert_eq!((mb.max_starts, mb.max_bytes), (8, 1 << 20));
        // The largest megabytes whose bytes still fit.
        assert_eq!(
            read(&[(MB, "17592186044415")]).unwrap().max_bytes,
            (u64::MAX >> 20) << 20
        );
    }

    #[test]
    fn a_bad_value_is_an_error_that_names_its_variable() {
        for name in [IMAGES, MB] {
            for bad in ["", "abc", "0", "-1", "+", "1.5", " 8", "8MB", "99999999999999999999"] {
                let err = read(&[(name, bad)]).expect_err(&format!("{name}={bad}"));
                assert!(
                    err.starts_with(&format!("{name} takes a whole number from 1 to ")),
                    "{err}"
                );
                assert!(err.ends_with(&format!("not `{bad}`")), "{err}");
            }
        }
        // Megabytes whose bytes do not fit in 64 bits.
        let err = read(&[(MB, "17592186044416")]).unwrap_err();
        assert_eq!(
            err,
            "ALRUNNER_LOGIN_REPLAY_MAX_MB takes a whole number from 1 to 17592186044415, not `17592186044416`"
        );
        // A good value of the other variable does not hide a bad one.
        let err = read(&[(IMAGES, "16"), (MB, "zero")]).unwrap_err();
        assert!(err.starts_with(MB), "{err}");
    }

    #[test]
    fn a_byte_cap_not_below_a_set_memory_limit_is_an_error_that_names_both_values() {
        let caps = |mb: &str| read(&[(MB, mb)]).unwrap();
        assert_eq!(
            caps("64").check_memory_limit(Some(64 << 20)).unwrap_err(),
            "ALRUNNER_LOGIN_REPLAY_MAX_MB is 64 MB, not below the memory limit of 64 MB (--max-memory or maxMemory): \
             set it below the limit, or raise the limit"
        );
        let err = caps("512").check_memory_limit(Some(300 << 20)).unwrap_err();
        assert!(err.contains("is 512 MB, not below the memory limit of 300 MB"), "{err}");
        // A limit that is not a whole number of MB shows its bytes.
        let err = caps("1").check_memory_limit(Some(1000)).unwrap_err();
        assert!(
            err.contains("is 1 MB, not below the memory limit of 1000 bytes"),
            "{err}"
        );
        // Below the limit, or no limit set: the run goes on.
        assert_eq!(caps("63").check_memory_limit(Some(64 << 20)), Ok(()));
        assert_eq!(LoginReplayLimits::default().check_memory_limit(Some(6 << 30)), Ok(()));
        assert_eq!(caps("4096").check_memory_limit(None), Ok(()));
    }
}
