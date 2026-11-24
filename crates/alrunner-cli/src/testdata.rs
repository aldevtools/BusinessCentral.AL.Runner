//! Which demo backup a run restores (ADR-020, owner decision of 2026-10-14: test data is on by
//! default). Order: the backup of the build the run uses, from the extracted artifacts on disk,
//! then from the artifact cache, then downloaded (about 520 MB, once); only then a backup of
//! another build of the same BC version, with a note that says which. Nothing found is not an
//! error for a default run: test data is off and the run says why.
//!
//! Which demo backup a run restores: disk, cache, download, other build; or why test data is off
//! (ADR-020). Entry points: `locate`, `Where`, `Located`.

use al_bcdb_std::default_backup_in_country;
use al_deps::build::BuildId;
use al_deps::demo::cached_backup;
use std::path::{Path, PathBuf};

/// Where a backup may come from, and how to download one.
pub struct Where<'a, D: FnMut(&BuildId) -> Result<PathBuf, String>> {
    /// `~/.bcartifacts.cache/sandbox`, when it exists.
    pub local_root: Option<&'a Path>,
    /// The al-runner artifact cache root (`--cache-dir`, `$ALRUNNER_CACHE`, the default).
    pub cache_root: &'a Path,
    /// Downloads the backup of a build into the cache. `None` for `--no-download`.
    pub download: Option<D>,
    /// `--country` was given: only that country's backup answers, never another country's.
    pub explicit_country: bool,
}

/// The answer: a backup and the notes to print, or no backup and the reason.
#[derive(Debug, PartialEq)]
pub struct Located {
    pub path: Option<PathBuf>,
    /// One line each: where the data came from when that is not the build the run uses, and what
    /// failed on the way.
    pub notes: Vec<String>,
    /// Why there is no backup (set exactly when `path` is `None`).
    pub off: Option<String>,
}

/// `builds` are the versions of the Microsoft apps the run loaded, best first; `downloadable` are
/// the ones that can be a published build (the build of the feed's apps, `--bc-version`, the
/// tests' own build). `major` and `minor` are the BC version of the target.
pub fn locate<D: FnMut(&BuildId) -> Result<PathBuf, String>>(
    builds: &[String],
    downloadable: &[BuildId],
    major: u16,
    minor: Option<u16>,
    w: Where<'_, D>,
    country: &str,
) -> Located {
    let mut notes = Vec::new();
    let in_local = |b: &[String]| {
        let root = w.local_root?;
        default_backup_in_country(root, b, major, minor, w.explicit_country.then_some(country))
            .ok()
            .map(|p| {
                let name = p
                    .ancestors()
                    .nth(2)
                    .and_then(|d| d.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (p, name)
            })
    };
    // 1. The build the run uses, on disk.
    let wanted: Vec<&String> = builds.iter().collect();
    if let Some((p, name)) = in_local(builds) {
        if wanted.iter().any(|b| **b == name) {
            return Located {
                path: Some(p),
                notes,
                off: None,
            };
        }
    }
    // 2. The same build in the artifact cache.
    for b in downloadable {
        let p = cached_backup(w.cache_root, b, country);
        if p.is_file() {
            return Located {
                path: Some(p),
                notes,
                off: None,
            };
        }
    }
    // 3. Download it for the first build that can be published.
    let mut download = w.download;
    let mut failure = None;
    match (&mut download, downloadable.first()) {
        (Some(get), Some(b)) => match get(b) {
            Ok(p) => {
                return Located {
                    path: Some(p),
                    notes,
                    off: None,
                };
            }
            Err(e) => failure = Some(format!("the demo data of build {b} could not be downloaded: {e}")),
        },
        (None, Some(b)) => {
            failure = Some(format!(
                "the demo data of build {b} is not cached and --no-download is set"
            ))
        }
        _ => {}
    }
    notes.extend(failure.clone());
    // 4. Another build of the same BC version.
    if let Some((p, name)) = in_local(builds) {
        notes.push(format!(
            "test data is from build {name}, not the build of the Microsoft apps ({})",
            builds.first().map_or("unknown", String::as_str)
        ));
        return Located {
            path: Some(p),
            notes,
            off: None,
        };
    }
    let off = match failure {
        Some(f) => f,
        None => format!(
            "no demo data for BC {major} in {} or in the artifact cache",
            w.local_root
                .map_or("~/.bcartifacts.cache/sandbox".to_string(), |r| r.display().to_string())
        ),
    };
    Located {
        path: None,
        notes: Vec::new(),
        off: Some(off),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn b(s: &str) -> BuildId {
        BuildId::parse(s).unwrap()
    }

    fn local_with(root: &Path, build: &str) {
        let dir = root.join(build).join("w1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("BusinessCentral-W1.bak"), b"x").unwrap();
    }

    type Never = fn(&BuildId) -> Result<PathBuf, String>;

    fn no_download() -> Option<Never> {
        None
    }

    #[test]
    fn the_build_of_the_run_on_disk_is_used_without_a_download() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("sandbox");
        local_with(&local, "27.5.46862.48827");
        local_with(&local, "27.5.46862.56249");
        let called = Cell::new(false);
        let got = locate(
            &["27.5.46862.48827".into()],
            &[b("27.5.46862.48827")],
            27,
            Some(5),
            Where {
                local_root: Some(&local),
                cache_root: &dir.path().join("cache"),
                download: Some(|_: &BuildId| {
                    called.set(true);
                    Err("no".into())
                }),
                explicit_country: false,
            },
            "w1",
        );
        assert!(got.path.unwrap().to_string_lossy().contains("27.5.46862.48827"));
        assert!(got.notes.is_empty() && got.off.is_none() && !called.get());
    }

    #[test]
    fn the_artifact_cache_comes_next() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let want = cached_backup(&cache, &b("27.5.46862.48827"), "w1");
        std::fs::create_dir_all(want.parent().unwrap()).unwrap();
        std::fs::write(&want, b"x").unwrap();
        let got = locate(
            &["27.5.46862.48827".into()],
            &[b("27.5.46862.48827")],
            27,
            Some(5),
            Where {
                local_root: None,
                cache_root: &cache,
                download: no_download(),
                explicit_country: false,
            },
            "w1",
        );
        assert_eq!(got.path, Some(want));
    }

    #[test]
    fn a_missing_backup_is_downloaded_for_the_build_of_the_run() {
        let dir = tempfile::tempdir().unwrap();
        let asked = std::cell::RefCell::new(Vec::new());
        let got = locate(
            &["27.5.46862.48827".into()],
            &[b("27.5.46862.48827")],
            27,
            Some(5),
            Where {
                local_root: None,
                cache_root: dir.path(),
                download: Some(|build: &BuildId| {
                    asked.borrow_mut().push(build.to_string());
                    Ok(PathBuf::from("/cache/x.bak"))
                }),
                explicit_country: false,
            },
            "w1",
        );
        assert_eq!(got.path, Some(PathBuf::from("/cache/x.bak")));
        assert_eq!(*asked.borrow(), ["27.5.46862.48827"]);
    }

    #[test]
    fn a_failed_download_falls_back_to_another_build_and_names_both() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("sandbox");
        local_with(&local, "27.5.46862.48827");
        let got = locate(
            &["27.5.46862.56249".into()],
            &[b("27.5.46862.56249")],
            27,
            Some(5),
            Where {
                local_root: Some(&local),
                cache_root: dir.path(),
                download: Some(|_: &BuildId| Err::<PathBuf, _>("offline".into())),
                explicit_country: false,
            },
            "w1",
        );
        assert!(got.path.is_some() && got.off.is_none());
        let notes = got.notes.join("\n");
        assert!(
            notes.contains("offline") && notes.contains("27.5.46862.56249"),
            "{notes}"
        );
        assert!(notes.contains("from build 27.5.46862.48827"), "{notes}");
    }

    #[test]
    fn nothing_anywhere_turns_test_data_off_with_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        let got = locate(
            &["27.5.46862.56249".into()],
            &[b("27.5.46862.56249")],
            27,
            Some(5),
            Where {
                local_root: None,
                cache_root: dir.path(),
                download: Some(|_: &BuildId| Err::<PathBuf, _>("offline".into())),
                explicit_country: false,
            },
            "w1",
        );
        assert_eq!(got.path, None);
        assert!(got.off.unwrap().contains("offline"));
        let got = locate(
            &["27.5.46862.56249".into()],
            &[b("27.5.46862.56249")],
            27,
            Some(5),
            Where {
                local_root: None,
                cache_root: dir.path(),
                download: no_download(),
                explicit_country: false,
            },
            "w1",
        );
        assert!(got.off.unwrap().contains("--no-download"));
    }

    #[test]
    fn a_run_with_no_published_build_and_no_backup_says_where_it_looked() {
        let dir = tempfile::tempdir().unwrap();
        let got = locate(
            &[],
            &[],
            27,
            None,
            Where {
                local_root: None,
                cache_root: dir.path(),
                download: no_download(),
                explicit_country: false,
            },
            "w1",
        );
        assert!(got.off.unwrap().contains("no demo data for BC 27"));
    }

    fn local_country(root: &Path, build: &str, country: &str) {
        let dir = root.join(build).join(country);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("BusinessCentral-{}.bak", country.to_uppercase())),
            b"x",
        )
        .unwrap();
    }

    #[test]
    fn an_explicit_country_uses_only_its_own_backup() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("sandbox");
        local_with(&local, "28.5.1.1");
        let asked = std::cell::RefCell::new(Vec::new());
        // Only w1 is on disk: `us` must not be answered with it. It is downloaded for the country.
        let got = locate(
            &["28.5.1.1".into()],
            &[b("28.5.1.1")],
            28,
            Some(5),
            Where {
                local_root: Some(&local),
                cache_root: &dir.path().join("cache"),
                download: Some(|build: &BuildId| {
                    asked.borrow_mut().push(build.to_string());
                    Err::<PathBuf, _>("offline".into())
                }),
                explicit_country: true,
            },
            "us",
        );
        assert!(got.path.is_none(), "{got:?}");
        assert_eq!(*asked.borrow(), ["28.5.1.1"]);
        // With the country's folder on disk it is found there.
        local_country(&local, "28.5.1.1", "us");
        let got = locate(
            &["28.5.1.1".into()],
            &[b("28.5.1.1")],
            28,
            Some(5),
            Where {
                local_root: Some(&local),
                cache_root: &dir.path().join("cache"),
                download: no_download(),
                explicit_country: true,
            },
            "us",
        );
        assert!(got.path.unwrap().ends_with("28.5.1.1/us/BusinessCentral-US.bak"));
    }

    #[test]
    fn without_the_flag_the_old_fallback_to_another_country_stays() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("sandbox");
        local_country(&local, "28.5.1.1", "us");
        let got = locate(
            &["28.5.1.1".into()],
            &[b("28.5.1.1")],
            28,
            Some(5),
            Where {
                local_root: Some(&local),
                cache_root: &dir.path().join("cache"),
                download: no_download(),
                explicit_country: false,
            },
            "w1",
        );
        assert!(got.path.unwrap().ends_with("us/BusinessCentral-US.bak"));
    }
}
