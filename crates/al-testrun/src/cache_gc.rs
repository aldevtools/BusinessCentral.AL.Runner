//! Cleanup of the compile cache folder (ADR-022, "Cleanup of old builds").
//!
//! Every entry file name starts with the tag of the al-runner build that wrote it (`al_cache::build_tag`), and a
//! cache hit sets the file's modification time to now. So the cleanup needs `stat` only, never a file read:
//!
//! - an entry of another build (or from before the tag existed) goes after 7 days unused: several builds run on one
//!   machine, and deleting at once would make them evict each other;
//! - any entry unused for 30 days goes;
//! - a temporary file of a write that did not finish goes after 1 day.
//!
//! A reader that finds a file gone treats it as a miss, so a race with a delete costs a rebuild, nothing else.
//!
//! Cleanup of the compile cache folder (ADR-022 amendment): other-build entries after 7 days, any
//! entry after 30, old temp files after 1 day; automatic once a day within 200 ms (`al-runner test`),
//! by hand with `al-runner cache prune|info`. Stat only, never reads an entry. Entry points: `prune`,
//! `auto_prune`, `info`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Days an entry of another build may stay unused.
pub const OTHER_BUILD_DAYS: u64 = 7;
/// Days any entry may stay unused.
pub const UNUSED_DAYS: u64 = 30;
/// Days a temporary file may stay.
pub const TEMP_DAYS: u64 = 1;
/// Hours between two automatic cleanups of one folder.
pub const INTERVAL_HOURS: u64 = 24;
/// How long the automatic cleanup may run; the rest waits for the next run.
pub const AUTO_BUDGET: Duration = Duration::from_millis(200);
/// Set (not empty, not `0`) to turn the automatic cleanup off.
pub const NO_PRUNE_ENV: &str = "ALRUNNER_NO_CACHE_PRUNE";
/// The file whose modification time says when the last automatic cleanup ran.
pub const STAMP: &str = ".last-prune";

/// Why a file goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    OtherBuild,
    Unused,
    Temp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    pub path: PathBuf,
    pub bytes: u64,
    pub why: Why,
}

/// What a cleanup did, or with `dry_run` would do.
#[derive(Debug, Default)]
pub struct Report {
    pub removed: Vec<Removed>,
    /// False when the time budget ended the walk early.
    pub complete: bool,
}

impl Report {
    pub fn count(&self) -> usize {
        self.removed.len()
    }
    pub fn bytes(&self) -> u64 {
        self.removed.iter().map(|r| r.bytes).sum()
    }
    pub fn count_of(&self, why: Why) -> (usize, u64) {
        self.removed
            .iter()
            .filter(|r| r.why == why)
            .fold((0, 0), |(n, b), r| (n + 1, b + r.bytes))
    }
}

pub struct Options {
    /// The tag of this build (`al_cache::build_tag`).
    pub build: String,
    pub dry_run: bool,
    /// Remove entries of other builds whatever their age.
    pub all_other_builds: bool,
    pub budget: Option<Duration>,
    pub now: SystemTime,
}

impl Options {
    pub fn new(build: String) -> Options {
        Options {
            build,
            dry_run: false,
            all_other_builds: false,
            budget: None,
            now: SystemTime::now(),
        }
    }
}

/// The tag of the running build.
pub fn this_build() -> String {
    al_cache::build_tag(&crate::compile_cache::runtime_build())
}

enum Kind<'a> {
    /// An entry file; the build tag is `None` for a name from before the tag existed.
    Entry(Option<&'a str>),
    Temp,
    Other,
}

fn classify(name: &str) -> Kind<'_> {
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    if name.starts_with('.') && name.contains(".tmp-") || ext.starts_with("tmp") {
        Kind::Temp
    } else if matches!(ext, "alcc" | "alci") || ext == crate::baseline_cache::EXTENSION {
        Kind::Entry(al_cache::tag_of_file_name(name))
    } else {
        Kind::Other
    }
}

fn age(now: SystemTime, modified: SystemTime) -> Duration {
    now.duration_since(modified).unwrap_or_default()
}

fn days(d: u64) -> Duration {
    Duration::from_secs(d * 24 * 3600)
}

/// Removes what the rules in the module comment select. Best effort: a file that is gone or cannot be removed is
/// skipped.
pub fn prune(dir: &Path, opt: &Options) -> Report {
    let started = Instant::now();
    let mut report = Report {
        removed: Vec::new(),
        complete: true,
    };
    let Ok(read) = std::fs::read_dir(dir) else {
        return report;
    };
    for e in read.filter_map(|e| e.ok()) {
        if opt.budget.is_some_and(|b| started.elapsed() >= b) {
            report.complete = false;
            break;
        }
        let name = e.file_name();
        let name = name.to_string_lossy();
        let (kind, is_other_build) = match classify(&name) {
            Kind::Other => continue,
            Kind::Temp => (Why::Temp, false),
            Kind::Entry(tag) => (Why::Unused, tag != Some(opt.build.as_str())),
        };
        let Ok(meta) = e.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let Ok(modified) = meta.modified() else { continue };
        let age = age(opt.now, modified);
        let why = match kind {
            Why::Temp => (age > days(TEMP_DAYS)).then_some(Why::Temp),
            _ if age > days(UNUSED_DAYS) => Some(Why::Unused),
            _ if is_other_build && (opt.all_other_builds || age > days(OTHER_BUILD_DAYS)) => Some(Why::OtherBuild),
            _ => None,
        };
        let Some(why) = why else { continue };
        let path = e.path();
        // A file that is gone (another process was faster) or cannot be removed is not counted.
        if opt.dry_run || std::fs::remove_file(&path).is_ok() {
            report.removed.push(Removed {
                path,
                bytes: meta.len(),
                why,
            });
        }
    }
    report
}

/// What a folder holds.
#[derive(Debug, Default)]
pub struct Info {
    pub entries: usize,
    pub bytes: u64,
    /// Entries and bytes of this build.
    pub this_build: (usize, u64),
    /// Entries and bytes per other build tag; `"unknown"` holds names from before the tag.
    pub other_builds: BTreeMap<String, (usize, u64)>,
    pub temp: (usize, u64),
}

pub fn info(dir: &Path, build: &str) -> Info {
    let mut i = Info::default();
    let Ok(read) = std::fs::read_dir(dir) else {
        return i;
    };
    for e in read.filter_map(|e| e.ok()) {
        let name = e.file_name();
        let name = name.to_string_lossy();
        let len = e.metadata().map(|m| m.len()).unwrap_or(0);
        match classify(&name) {
            Kind::Other => {}
            Kind::Temp => {
                i.temp.0 += 1;
                i.temp.1 += len;
            }
            Kind::Entry(tag) => {
                i.entries += 1;
                i.bytes += len;
                let slot = if tag == Some(build) {
                    &mut i.this_build
                } else {
                    i.other_builds.entry(tag.unwrap_or("unknown").to_string()).or_default()
                };
                slot.0 += 1;
                slot.1 += len;
            }
        }
    }
    i
}

/// The automatic cleanup: at most once in `INTERVAL_HOURS` per folder, within `AUTO_BUDGET`, never when
/// `ALRUNNER_NO_CACHE_PRUNE` is set. `None` when it did not run. A cleanup that ran out of time leaves the stamp
/// alone, so the next run goes on.
pub fn auto_prune(dir: &Path, build: &str) -> Option<Report> {
    if std::env::var(NO_PRUNE_ENV).is_ok_and(|v| !v.is_empty() && v != "0") {
        return None;
    }
    let stamp = dir.join(STAMP);
    if !dir.is_dir() {
        return None;
    }
    let fresh = std::fs::metadata(&stamp)
        .and_then(|m| m.modified())
        .ok()
        .is_some_and(|t| age(SystemTime::now(), t) < Duration::from_secs(INTERVAL_HOURS * 3600));
    if fresh {
        return None;
    }
    let mut opt = Options::new(build.to_string());
    opt.budget = Some(AUTO_BUDGET);
    let report = prune(dir, &opt);
    if report.complete {
        let _ = std::fs::write(&stamp, b"");
        let _ = std::fs::File::options()
            .write(true)
            .open(&stamp)
            .and_then(|f| f.set_modified(SystemTime::now()));
    }
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    const ME: &str = "aaaaaaaaaaaaaaaa";
    const OLD: &str = "bbbbbbbbbbbbbbbb";

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::var_os("AL_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-scratch"));
        let d = root.join(format!("cache_gc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A file of `len` bytes last used `age_days` days ago.
    fn put(dir: &Path, name: &str, len: usize, age_days: u64) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, vec![7u8; len]).unwrap();
        let t = SystemTime::now() - days(age_days) - Duration::from_secs(60);
        File::options().write(true).open(&p).unwrap().set_modified(t).unwrap();
        p
    }

    fn opt() -> Options {
        Options::new(ME.into())
    }

    #[test]
    fn another_build_goes_after_seven_days_and_not_before() {
        let d = scratch("other");
        let young = put(&d, &format!("{OLD}-deps-1.alcc"), 10, 6);
        let old = put(&d, &format!("{OLD}-deps-2.alcc"), 20, 8);
        let mine = put(&d, &format!("{ME}-deps-3.alcc"), 30, 8);
        let r = prune(&d, &opt());
        assert_eq!((r.count(), r.bytes()), (1, 20));
        assert!(young.exists() && mine.exists() && !old.exists());
        assert_eq!(r.removed[0].why, Why::OtherBuild);
    }

    #[test]
    fn a_name_from_before_the_tag_counts_as_another_build() {
        let d = scratch("untagged");
        let old = put(&d, "deps-0123.alcc", 5, 8);
        let old_baseline = put(&d, &format!("{}.albr", "c".repeat(64)), 5, 8);
        let young = put(&d, "index-0123.alci", 5, 1);
        let r = prune(&d, &opt());
        assert_eq!(r.count(), 2);
        assert!(!old.exists() && !old_baseline.exists() && young.exists());
    }

    #[test]
    fn any_entry_goes_after_thirty_days() {
        let d = scratch("unused");
        let mine_old = put(&d, &format!("{ME}-index-1.alci"), 4, 31);
        let mine_ok = put(&d, &format!("{ME}-{}.albr", "d".repeat(64)), 4, 29);
        let r = prune(&d, &opt());
        assert_eq!(r.count(), 1);
        assert_eq!(r.removed[0].why, Why::Unused);
        assert!(!mine_old.exists() && mine_ok.exists());
    }

    #[test]
    fn all_other_builds_ignores_the_age_but_keeps_this_build() {
        let d = scratch("all");
        let other = put(&d, &format!("{OLD}-deps-1.alcc"), 10, 0);
        let mine = put(&d, &format!("{ME}-deps-2.alcc"), 10, 0);
        let mut o = opt();
        o.all_other_builds = true;
        let r = prune(&d, &o);
        assert_eq!(r.count(), 1);
        assert!(!other.exists() && mine.exists());
    }

    #[test]
    fn temporary_files_go_after_one_day() {
        let d = scratch("temp");
        let old = put(&d, &format!(".{ME}-deps-1.alcc.tmp-42-0"), 3, 2);
        let fresh = put(&d, &format!(".{ME}-deps-2.alcc.tmp-42-1"), 3, 0);
        let r = prune(&d, &opt());
        assert_eq!(r.count_of(Why::Temp), (1, 3));
        assert!(!old.exists() && fresh.exists());
    }

    #[test]
    fn other_files_and_the_stamp_stay() {
        let d = scratch("foreign");
        let a = put(&d, "notes.txt", 3, 100);
        let b = put(&d, STAMP, 0, 100);
        assert_eq!(prune(&d, &opt()).count(), 0);
        assert!(a.exists() && b.exists());
    }

    #[test]
    fn a_dry_run_removes_nothing_and_reports_the_same() {
        let d = scratch("dry");
        let f = put(&d, &format!("{OLD}-deps-1.alcc"), 10, 40);
        let mut o = opt();
        o.dry_run = true;
        let r = prune(&d, &o);
        assert_eq!((r.count(), r.bytes()), (1, 10));
        assert!(f.exists());
    }

    #[test]
    fn a_missing_folder_is_not_an_error() {
        let d = scratch("missing").join("nope");
        assert_eq!(prune(&d, &opt()).count(), 0);
        assert!(auto_prune(&d, ME).is_none());
    }

    #[test]
    fn a_spent_budget_stops_early_and_says_so() {
        let d = scratch("budget");
        for i in 0..5 {
            put(&d, &format!("{OLD}-deps-{i}.alcc"), 1, 40);
        }
        let mut o = opt();
        o.budget = Some(Duration::ZERO);
        let r = prune(&d, &o);
        assert!(!r.complete);
        assert!(r.count() < 5);
        let full = prune(&d, &opt());
        assert!(full.complete);
        assert_eq!(r.count() + full.count(), 5);
    }

    #[test]
    fn info_counts_by_build() {
        let d = scratch("info");
        put(&d, &format!("{ME}-deps-1.alcc"), 10, 0);
        put(&d, &format!("{OLD}-deps-2.alcc"), 20, 0);
        put(&d, &format!("{OLD}-index-3.alci"), 5, 0);
        put(&d, "deps-9.alcc", 7, 0);
        put(&d, &format!(".{ME}-x.alcc.tmp-1-1"), 2, 0);
        let i = info(&d, ME);
        assert_eq!((i.entries, i.bytes), (4, 42));
        assert_eq!(i.this_build, (1, 10));
        assert_eq!(i.other_builds[OLD], (2, 25));
        assert_eq!(i.other_builds["unknown"], (1, 7));
        assert_eq!(i.temp, (1, 2));
    }

    #[test]
    fn the_automatic_cleanup_runs_once_a_day() {
        let d = scratch("auto");
        let f = put(&d, &format!("{OLD}-deps-1.alcc"), 10, 40);
        assert!(auto_prune(&d, ME).is_some(), "first run: no stamp");
        assert!(!f.exists());
        let g = put(&d, &format!("{OLD}-deps-2.alcc"), 10, 40);
        assert!(auto_prune(&d, ME).is_none(), "second run within a day");
        assert!(g.exists());
        // The stamp is a day old: it runs again.
        let t = SystemTime::now() - days(2);
        File::options()
            .write(true)
            .open(d.join(STAMP))
            .unwrap()
            .set_modified(t)
            .unwrap();
        assert!(auto_prune(&d, ME).is_some());
        assert!(!g.exists());
    }
}
