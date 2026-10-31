//! Runs the corpus's `[Test]` codeunits and reports them against the oracle.

pub use al_testrun::{baseline, deps, dotnet, explain, host, isolation, runtime, target};

/// mimalloc as the global allocator (feature `mimalloc`, on by default). It sits in the library, not in `main.rs`, so
/// every binary that links this crate uses it: the CLI, and the integration tests and examples that compile the corpus
/// in their own process (the cold-compile budget in `perf_budget`, `perf_vm`, `phases`). With the system allocator the
/// cold compile took 20.4 G instructions; with mimalloc 18.7 G.
#[cfg(all(feature = "mimalloc", not(target_family = "wasm")))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod args;
pub mod awaiting;
pub mod behavior_coverage;
pub mod config;
pub mod corpus;
pub mod coverage;
pub mod drift;
pub mod insight;
pub mod language_catalog;
pub mod mnc;
pub mod oracle;
pub mod output;
pub mod pin;
pub mod platform_columns;
pub mod report;
pub mod runner;
pub mod select;
pub mod work;

/// What tells two builds of this binary apart: its size and when it was written, as
/// `<bytes> bytes, written <UTC time>`. A rebuild (after a golden file such as
/// `al-builtins/src/pinned_columns.rs` was regenerated) changes it; two runs reported as "the same
/// build" must print the same line. `None` when the executable cannot be read.
pub fn binary_identity() -> Option<String> {
    let meta = std::fs::metadata(std::env::current_exe().ok()?).ok()?;
    let written = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(format!("{} bytes, written {}", meta.len(), utc_time(written.as_secs())))
}

/// `YYYY-MM-DD HH:MM:SS UTC` of a Unix time (days to a civil date: H. Hinnant's algorithm).
fn utc_time(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

#[cfg(test)]
mod binary_identity_tests {
    #[test]
    fn utc_time_formats_a_unix_time() {
        assert_eq!(super::utc_time(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(super::utc_time(951_782_400), "2000-02-29 00:00:00 UTC");
        assert_eq!(super::utc_time(1_792_506_896), "2026-10-20 14:34:56 UTC");
    }

    #[test]
    fn the_identity_names_size_and_write_time() {
        let id = super::binary_identity().unwrap();
        assert!(id.contains(" bytes, written ") && id.ends_with(" UTC"), "{id}");
    }
}
