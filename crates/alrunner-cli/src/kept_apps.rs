//! What a warm server (`al-runner serve`) keeps between requests while the files it came from are unchanged, and the
//! one rule of "unchanged" ([`Stamp`], [`stamps`], [`tree_at`]).
//!
//! The apps under test (al-runner issue 8205): on a kept daemon reading an unchanged app again, its sources, its
//! translation files and the layout files its code names, was most of `setupMs` (`sourcesMs`, 83%). [`KeptApps`]
//! keeps each app as `apps::load` read it and reads it again when anything under its folder, or a file outside it that
//! the load read (`AppInput::outside_reads`), changed, came or went. Anything under the folder counts, also what a run
//! writes there (an output file, a local cache): the next request then reads the app again, which costs time and
//! never a result. A kept app is the app a fresh read gives. An app is kept only where its stamps carry a change time
//! (unix today, [`unchanged`]); elsewhere it is read on every request, and its files are not walked
//! (`KeptApps::change_times`).
//!
//! What a warm server keeps between requests while the files it came from are unchanged, and the one
//! rule of "unchanged" (`Stamp`: length and modification time, on unix also device, inode and change
//! time; a stamp taken less than 2 s after the file changed is racy and equals none, as in git's
//! index; `stamps`, `tree_at`, which follows a linked folder once). `KeptApps` keeps each app under
//! test as `apps::load` read it, only when its stamps carry a change time (`unchanged`; elsewhere the
//! app is read on every request and its files are not walked, `change_times`), with its test
//! codeunits (`AppInput::test_codeunits`, found once per read), and reads it again when a file under
//! its folder, the record next to it (`<path>.json`) or a file outside it the load read
//! (`AppInput::outside_reads`, from al-testrun `runtime::load_manifest_reading`: resource folders and
//! the logo) changed, came or went (al-runner issue 8205). Entry points: `load`, `KeptApps::load`,
//! `stamps`.

use crate::apps::{self, AppInput};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// A file whose modification or change time is less than this before the moment it is stamped is not trusted
/// ([`Stamp`]): a write in the same tick of the file system's clock may follow, at the same length.
const RACY: Duration = Duration::from_secs(2);

/// What a file's metadata says about its content: its length and modification time, and on unix its device, inode and
/// change time (no user tool sets a change time back, as `cp -p`, `rsync -t`, `tar x` or `touch -r` set a modification
/// time back; a save that writes a new file and renames it gets a new inode). A stamp taken less than [`RACY`] after the
/// file changed is "racy", as in git's index: a later write in the same clock tick, at the same length, would leave
/// every field as it is, so a racy stamp equals no stamp and the file is read again the next time.
#[derive(Debug, Clone)]
pub struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    /// Device, inode, change time (seconds, nanoseconds); `None` off unix.
    unix: Option<(u64, u64, i64, i64)>,
    racy: bool,
}

impl PartialEq for Stamp {
    fn eq(&self, other: &Stamp) -> bool {
        !self.racy && !other.racy && (self.len, self.modified, self.unix) == (other.len, other.modified, other.unix)
    }
}

/// The stamp of each file (`None`: it is not there).
pub type Stamps = Vec<(PathBuf, Option<Stamp>)>;

/// The stamp of `p` (following a link) at the moment `now`.
fn stamp(p: &Path, now: SystemTime) -> Option<Stamp> {
    let m = std::fs::metadata(p).ok()?;
    let modified = m.modified().ok();
    #[cfg(unix)]
    let unix = {
        use std::os::unix::fs::MetadataExt;
        Some((m.dev(), m.ino(), m.ctime(), m.ctime_nsec()))
    };
    #[cfg(not(unix))]
    let unix = None;
    let changed = unix.and_then(|(_, _, s, n)| {
        let since_epoch = Duration::new(u64::try_from(s).ok()?, u32::try_from(n).ok()?);
        SystemTime::UNIX_EPOCH.checked_add(since_epoch)
    });
    let trusted_before = now.checked_sub(RACY).unwrap_or(SystemTime::UNIX_EPOCH);
    // A time that cannot be read is no proof of an old file either.
    let racy = [modified, changed]
        .into_iter()
        .take(if cfg!(unix) { 2 } else { 1 })
        .any(|t| t.is_none_or(|t| t >= trusted_before));
    Some(Stamp {
        len: m.len(),
        modified,
        unix,
        racy,
    })
}

/// Each folder of `search` and its entries, and each file of `read`: another package in a folder, or a package changed,
/// changes them (the dependency apps a warm server keeps, `run.rs` `KeptDependencies`).
pub fn stamps(search: &[PathBuf], read: &[PathBuf]) -> Stamps {
    stamps_at(search, read, SystemTime::now())
}

fn stamps_at(search: &[PathBuf], read: &[PathBuf], now: SystemTime) -> Stamps {
    let mut out: Stamps = Vec::new();
    for dir in search {
        out.push((dir.clone(), stamp(dir, now)));
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
            entries.sort();
            out.extend(entries.into_iter().map(|p| {
                let s = stamp(&p, now);
                (p, s)
            }));
        }
    }
    out.extend(read.iter().map(|p| (p.clone(), stamp(p, now))));
    out
}

/// `root` and, when it is a folder, every folder and file under it, in path order, stamped at the moment `now`: a file
/// changed, added or removed changes them. A linked folder is followed, as the loaders read through it, once: a link back
/// to a folder already walked is stamped and not entered again.
fn tree_at(root: &Path, now: SystemTime) -> Stamps {
    let mut out: Stamps = vec![(root.to_path_buf(), stamp(root, now))];
    let mut walked: BTreeSet<PathBuf> = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        if !walked.insert(std::fs::canonicalize(&d).unwrap_or_else(|_| d.clone())) {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for p in rd.filter_map(|e| e.ok().map(|e| e.path())) {
            if p.is_dir() {
                stack.push(p.clone());
            }
            let s = stamp(&p, now);
            out.push((p, s));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The files an app's load read: its folder (or package file) as a [`tree_at`], the record next to it (`<path>.json`, the
/// build of its variables, `al_deps::load`), and the files and folders outside it that the load read.
fn app_stamps(path: &Path, outside: &[PathBuf], now: SystemTime) -> Stamps {
    let mut record = path.as_os_str().to_owned();
    record.push(".json");
    let mut out = tree_at(path, now);
    out.extend(stamps_at(&[], &[PathBuf::from(record)], now));
    for p in outside {
        out.extend(tree_at(p, now));
    }
    out
}

/// Whether an app kept with the stamps `kept` is unchanged, its files now stamped `current`: equal stamps that all carry
/// a change time. Without one (off unix today) a same-length edit with the old modification time leaves the stamp as
/// it was, so such an app is read on every request; the dependency stamps ([`stamps`]) are compared without it.
fn unchanged(kept: &Stamps, current: &Stamps) -> bool {
    kept == current && kept.iter().all(|(_, s)| s.as_ref().is_none_or(|s| s.unix.is_some()))
}

/// One app kept: the path and build it was read for, the stamps of what the load read, the app.
struct Kept {
    path: PathBuf,
    build: Option<String>,
    stamps: Stamps,
    input: AppInput,
}

/// The apps under test a warm server read for its last request ([`crate::run::WarmCtx::apps`]).
#[derive(Default)]
pub struct KeptApps {
    apps: Vec<Kept>,
    /// Whether this platform's stamps carry a change time, decided once from a real stamp of an app folder (`None`
    /// until one was stamped). Without one no app is kept ([`unchanged`]), so its files are not walked either.
    change_times: Option<bool>,
    /// How many apps were read from their files (tests).
    pub(crate) reads: usize,
    /// How many apps had their files walked and stamped (tests).
    walks: usize,
}

impl KeptApps {
    /// The apps `paths` (`build`: as `apps::load`): each kept one whose files are unchanged, the others read again.
    /// Only these apps are kept afterwards.
    pub fn load(&mut self, paths: &[PathBuf], build: Option<&str>) -> Result<Vec<AppInput>, String> {
        self.load_at(paths, build, SystemTime::now())
    }

    /// [`KeptApps::load`] with the files stamped at the moment `now`.
    fn load_at(&mut self, paths: &[PathBuf], build: Option<&str>, now: SystemTime) -> Result<Vec<AppInput>, String> {
        // Whether a stamp has a change time is a property of the platform's metadata (`stamp`), not of one file, so one
        // real stamp decides it for every file; a probe per request would cost a metadata call per app and say the same.
        if self.change_times.is_none() {
            self.change_times = paths.iter().find_map(|p| stamp(p, now)).map(|s| s.unix.is_some());
        }
        if self.change_times == Some(false) {
            self.apps.clear();
            let inputs = paths
                .iter()
                .map(|p| apps::load(p, build))
                .collect::<Result<Vec<_>, _>>()?;
            self.reads += inputs.len();
            inputs.iter().for_each(|i| _ = i.test_codeunits());
            return Ok(inputs);
        }
        let mut kept = Vec::with_capacity(paths.len());
        for path in paths {
            self.walks += 1;
            let known = (self.apps.iter())
                .position(|k| k.path == *path && k.build.as_deref() == build)
                .map(|i| self.apps.swap_remove(i));
            let unchanged = known.filter(|k| unchanged(&k.stamps, &app_stamps(path, &k.input.outside_reads, now)));
            kept.push(match unchanged {
                Some(k) => k,
                None => {
                    // Stamped before the read: a file that changes while it is read is read again next time.
                    let before = tree_at(path, now);
                    let input = apps::load(path, build)?;
                    self.reads += 1;
                    let mut stamps = app_stamps(path, &input.outside_reads, now);
                    if stamps[..before.len().min(stamps.len())] != before[..] {
                        stamps.clear();
                    }
                    Kept {
                        path: path.clone(),
                        build: build.map(str::to_string),
                        stamps,
                        input,
                    }
                }
            });
        }
        self.apps = kept;
        // The test codeunits are found once per read and kept with the app (`AppInput::test_codeunits`).
        for k in &self.apps {
            k.input.test_codeunits();
        }
        Ok(self.apps.iter().map(|k| k.input.clone()).collect())
    }
}

/// The apps `paths`: kept by `warm` when given ([`KeptApps::load`]), else read.
pub fn load(paths: &[PathBuf], build: Option<&str>, warm: Option<&mut KeptApps>) -> Result<Vec<AppInput>, String> {
    match warm {
        Some(k) => k.load(paths, build),
        None => paths.iter().map(|p| apps::load(p, build)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, text: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    /// An app folder `app` with a codeunit, a translation file, and outside it a resource folder `../shared` that holds a
    /// layout its code names.
    fn workspace(root: &Path) -> PathBuf {
        let app = root.join("app");
        write(
            &app.join("app.json"),
            r#"{"id":"82050000-0000-0000-0000-000000000001","name":"Kept","publisher":"P","version":"1.0.0.0","resourceFolders":["../shared"],"dependencies":[]}"#,
        );
        write(
            &app.join("src/R.al"),
            "report 50100 \"R\" { rendering { layout(L) { Type = RDLC; LayoutFile = '../shared/L.rdlc'; } } }\n",
        );
        write(&app.join("src/C.al"), "codeunit 50101 \"C\" { }\n");
        write(&root.join("shared/L.rdlc"), "<Report/>");
        write(
            &app.join("Translations/Kept.da-DK.xlf"),
            r#"<?xml version="1.0" encoding="utf-8"?><xliff version="1.2"><file datatype="xml" source-language="en-US" target-language="da-DK" original="Kept"><body><group id="body"></group></body></file></xliff>"#,
        );
        write(&root.join("shared/r.txt"), "one");
        app
    }

    /// The app as a fresh read gives it, its test codeunits found.
    fn fresh(app: &Path) -> String {
        let input = apps::load(app, None).unwrap();
        input.test_codeunits();
        format!("{input:?}")
    }

    /// A moment well after the files were written: their stamps are not racy then.
    fn later() -> SystemTime {
        SystemTime::now() + Duration::from_secs(60)
    }

    fn once(k: &mut KeptApps, app: &Path) -> String {
        format!("{:?}", k.load_at(&[app.to_path_buf()], None, later()).unwrap()[0])
    }

    // Stamps without a change time never keep an app (`unchanged`), so where they have none (off unix today) a request
    // reads the apps and neither walks their files nor keeps them.
    #[test]
    fn without_change_times_the_apps_are_read_and_neither_walked_nor_kept() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps {
            change_times: Some(false),
            ..KeptApps::default()
        };
        assert_eq!(once(&mut k, &app), fresh(&app));
        assert_eq!(once(&mut k, &app), fresh(&app));
        assert_eq!(k.reads, 2, "read on every request");
        assert_eq!(k.walks, 0, "no files walked");
        assert!(k.apps.is_empty(), "nothing kept");
    }

    // Whether stamps carry a change time is decided from the first app folder stamped: on unix they do.
    #[test]
    fn the_change_time_is_decided_from_a_real_stamp() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps::default();
        once(&mut k, &app);
        assert_eq!(k.change_times, Some(cfg!(unix)));
    }

    // Kept only with stamps that carry a change time (`unchanged`): unix today.
    #[cfg(unix)]
    #[test]
    fn an_unchanged_app_is_kept_and_is_what_a_fresh_read_gives() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps::default();
        let first = once(&mut k, &app);
        let second = once(&mut k, &app);
        assert_eq!(k.reads, 1, "the second request reads nothing");
        assert_eq!(second, first);
        assert_eq!(second, fresh(&app));
    }

    #[test]
    fn a_changed_added_or_removed_file_under_the_folder_is_read_again() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps::default();
        once(&mut k, &app);
        let changes: [&dyn Fn(); 4] = [
            &|| {
                write(
                    &app.join("src/C.al"),
                    "codeunit 50101 \"C\" { procedure P() begin end; }\n",
                )
            },
            &|| write(&app.join("src/D.al"), "codeunit 50102 \"D\" { }\n"),
            &|| std::fs::remove_file(app.join("src/D.al")).unwrap(),
            &|| {
                write(
                    &app.join("Translations/Kept.da-DK.xlf"),
                    r#"<?xml version="1.0" encoding="utf-8"?><xliff version="1.2"><file datatype="xml" source-language="en-US" target-language="da-DK" original="Kept"><body><group id="body"><trans-unit id="Codeunit 1 - Method 2"><source>a</source><target>b</target></trans-unit></group></body></file></xliff>"#,
                )
            },
        ];
        for (n, change) in changes.iter().enumerate() {
            change();
            assert_eq!(once(&mut k, &app), fresh(&app), "change {n}");
            assert_eq!(k.reads, n + 2, "change {n} is read");
        }
    }

    #[test]
    fn a_changed_file_outside_the_folder_that_the_load_read_is_read_again() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps::default();
        once(&mut k, &app);
        write(&d.path().join("shared/r.txt"), "two!");
        assert_eq!(once(&mut k, &app), fresh(&app));
        assert_eq!(k.reads, 2, "the resource outside the folder");
        write(&d.path().join("shared/L.rdlc"), "<Report />");
        assert_eq!(once(&mut k, &app), fresh(&app));
        // The layout is never read as a layout (al-testrun `layout_file_at`: a path outside the folder), but it is a file
        // of the resource folder `../shared`, so the app is read again.
        assert_eq!(
            k.reads, 3,
            "the layout outside the folder, a file of its resource folder"
        );
        write(&d.path().join("app.json"), r#"{"build":"28.5.54151.58194"}"#);
        assert_eq!(once(&mut k, &app), fresh(&app));
        assert_eq!(k.reads, 4, "the record next to the folder");
    }

    #[test]
    fn another_build_or_another_set_of_apps_reads_again_and_keeps_only_those() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps::default();
        once(&mut k, &app);
        k.load(std::slice::from_ref(&app), Some("28.5.54151.58194")).unwrap();
        assert_eq!(k.reads, 2);
        k.load(&[], None).unwrap();
        assert!(k.apps.is_empty());
    }

    // A copy or restore that sets the old modification time back (`cp -p`, `rsync -t`, `tar x`, `touch -r`), or a write
    // in the same clock tick: same length, same modification time, other content.
    #[cfg(unix)]
    #[test]
    fn a_same_size_edit_with_the_old_modification_time_is_read_again() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps::default();
        once(&mut k, &app);
        let file = app.join("src/C.al");
        let old = std::fs::metadata(&file).unwrap().modified().unwrap();
        write(&file, "codeunit 50109 \"C\" { }\n");
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert_eq!(std::fs::metadata(&file).unwrap().modified().unwrap(), old);
        assert_eq!(once(&mut k, &app), fresh(&app));
        assert_eq!(k.reads, 2, "the edit is read");
    }

    // Off unix a stamp has only the length and the modification time, which a same-length edit with the old modification
    // time leaves as they were (`a_same_size_edit_with_the_old_modification_time_is_read_again`): such stamps never keep an
    // app, which is then read on every request.
    #[test]
    fn stamps_without_a_change_time_never_keep_the_app() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let stamps = app_stamps(&app, &[], later());
        if cfg!(unix) {
            assert!(unchanged(&stamps, &stamps), "stamps with a change time keep the app");
        }
        let mut bare = stamps;
        for s in bare.iter_mut().filter_map(|(_, s)| s.as_mut()) {
            s.unix = None;
        }
        assert!(!unchanged(&bare, &bare), "stamps without a change time do not");
    }

    // Stamps taken right after the files changed are not trusted: the next request reads the app again, and once the
    // files are older than the margin, the app is kept.
    // Kept only with stamps that carry a change time (`unchanged`): unix today.
    #[cfg(unix)]
    #[test]
    fn files_stamped_right_after_they_changed_are_read_again_the_next_time() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        let mut k = KeptApps::default();
        let now = SystemTime::now();
        k.load_at(std::slice::from_ref(&app), None, now).unwrap();
        k.load_at(std::slice::from_ref(&app), None, now).unwrap();
        assert_eq!(k.reads, 2, "racy stamps are not trusted");
        let later = later();
        k.load_at(std::slice::from_ref(&app), None, later).unwrap();
        k.load_at(std::slice::from_ref(&app), None, later).unwrap();
        assert_eq!(k.reads, 3, "kept once the files are older than the margin");
    }

    // A link back to a folder already walked is stamped, not walked again.
    #[cfg(unix)]
    #[test]
    fn a_link_loop_under_the_folder_ends() {
        let d = tempfile::tempdir().unwrap();
        let app = workspace(d.path());
        std::os::unix::fs::symlink(&app, app.join("src/loop")).unwrap();
        let stamps = tree_at(&app, later());
        assert!(stamps.iter().any(|(p, _)| p.ends_with("src/loop")));
        assert!(!stamps.iter().any(|(p, _)| p.ends_with("src/loop/src")));
    }
}
