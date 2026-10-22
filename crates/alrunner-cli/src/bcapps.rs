//! Where Microsoft's recipe files come from: a sparse checkout of the BCApps repository (MIT), kept in the cache.
//!
//! BCApps has one branch per BC release, `releases/<major>.<minor>`. The recipe needs three parts of it: the disabled
//! tests (`src/DisabledTests`), the project settings (`build/projects`) and the test app lists (`build/scripts`). They
//! are fetched with `git` (a shallow, blob-filtered, sparse clone) into `<cache>/bcapps/<major>.<minor>/` once and read
//! from there afterwards. `--bcapps DIR` names a checkout instead; `--no-download` uses the cache and never fetches.
//!
//! Only `releases/29.0` and later hold the BaseApp test recipe (the `Test Apps W1` project, the test app lists and the
//! disabled tests); `releases/27.x` and `releases/28.x` do not (measured 2026-10-18). A release without the recipe uses
//! the oldest release that has it, [`FIRST_WITH_RECIPE`], and the run says so. Disabled tests are matched by codeunit
//! id and method name, so a test that 29.0 disables and 27.5 does not have is simply not there.
//! Nothing from BCApps is committed to this repository (hard rule 7).
//!
//! This file does the OS work (git, folders) that the core crates must not do (ADR-006).

use std::path::{Path, PathBuf};
use std::process::Command;

/// The GitHub address of BCApps. `ALRUNNER_BCAPPS_URL` replaces it (a mirror, or a local repository in tests).
pub const DEFAULT_URL: &str = "https://github.com/microsoft/BCApps";

/// The parts of the repository the recipe reads, relative to its root.
pub const SPARSE_PATHS: [&str; 3] = ["src/DisabledTests", "build/projects", "build/scripts"];

/// The oldest BCApps release branch that has the BaseApp test recipe.
pub const FIRST_WITH_RECIPE: &str = "releases/29.0";

/// Written last into a finished cache folder, so a fetch that stopped halfway is not mistaken for one.
const COMPLETE: &str = ".alrunner-complete";

/// The branch of a BC release: `releases/27.5`.
pub fn branch(major: u32, minor: u32) -> String {
    format!("releases/{major}.{minor}")
}

/// The cache folder of a release.
pub fn cache_dir(cache_root: &Path, major: u32, minor: u32) -> PathBuf {
    cache_root.join("bcapps").join(format!("{major}.{minor}"))
}

/// Whether a cache folder holds a finished fetch.
pub fn is_complete(dir: &Path) -> bool {
    dir.join(COMPLETE).is_file()
}

/// Where the recipe files of this run are, and how they were found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub dir: PathBuf,
    /// One line for `--verbose`.
    pub how: String,
}

/// Fetches `branch` of the repository at `url` into the empty folder `dest`. Replaceable in tests.
pub type Fetch<'a> = &'a dyn Fn(&str, &str, &Path) -> Result<(), String>;

/// Whether a fetched folder holds the recipe (the project settings the lanes are read from).
pub fn has_recipe(dir: &Path) -> bool {
    dir.join("build/projects/Test Apps W1/.AL-Go/settings.json").is_file()
}

/// The recipe files for a BC release.
///
/// 1. `explicit` (`--bcapps DIR`) when given: used as it is.
/// 2. The cache folder of the release, when finished.
/// 3. A fetch into the cache, unless `offline`: the release's own branch, or [`FIRST_WITH_RECIPE`] when that branch has
///    no recipe (`recipe_present` says).
#[allow(clippy::too_many_arguments)]
pub fn resolve(
    explicit: Option<&Path>,
    major: u32,
    minor: u32,
    cache_root: &Path,
    offline: bool,
    url: &str,
    fetch: Fetch<'_>,
    recipe_present: &dyn Fn(&Path) -> bool,
) -> Result<Source, String> {
    if let Some(dir) = explicit {
        if !dir.is_dir() {
            return Err(format!("--bcapps {}: not a folder", dir.display()));
        }
        return Ok(Source {
            dir: dir.to_path_buf(),
            how: format!("recipe files: {} (--bcapps)", dir.display()),
        });
    }
    let dir = cache_dir(cache_root, major, minor);
    if is_complete(&dir) {
        let marker = std::fs::read_to_string(dir.join(COMPLETE)).unwrap_or_default();
        return Ok(Source {
            how: format!("recipe files: {} (cache; {})", dir.display(), marker.trim()),
            dir,
        });
    }
    let own = branch(major, minor);
    if offline {
        return Err(format!(
            "the recipe files for {major}.{minor} are not in the cache ({}) and --no-download forbids fetching them; run once without --no-download, or name a BCApps checkout with --bcapps DIR",
            dir.display()
        ));
    }
    // Fetch beside the final folder and move it into place, so the cache never holds half a fetch.
    let parent = dir.parent().ok_or("bad cache folder")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let tmp = parent.join(format!(".fetch-{major}.{minor}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let attempt = (|| -> Result<String, String> {
        fetch(url, &own, &tmp).map_err(|e| format!("fetching {own} of {url}: {e}"))?;
        let mut used = format!("fetched {own} of {url}");
        if !recipe_present(&tmp) {
            let _ = std::fs::remove_dir_all(&tmp);
            fetch(url, FIRST_WITH_RECIPE, &tmp).map_err(|e| format!("fetching {FIRST_WITH_RECIPE} of {url}: {e}"))?;
            if !recipe_present(&tmp) {
                return Err(format!(
                    "{FIRST_WITH_RECIPE} of {url} has no BaseApp test recipe either"
                ));
            }
            used = format!(
                "{own} has no BaseApp test recipe; fetched {FIRST_WITH_RECIPE} of {url}, the oldest release that has one"
            );
        }
        std::fs::write(tmp.join(COMPLETE), format!("{used}\n")).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::rename(&tmp, &dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(used)
    })();
    match attempt {
        Ok(used) => Ok(Source {
            how: format!("recipe files: {} ({used})", dir.display()),
            dir,
        }),
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err(e)
        }
    }
}

/// The real fetch: a shallow clone with no file contents, then only the recipe folders checked out. The `.git` folder is
/// removed afterwards; the cache holds the files only.
pub fn git_fetch(url: &str, branch: &str, dest: &Path) -> Result<(), String> {
    git_fetch_parts(url, branch, dest, &SPARSE_PATHS)
}

/// [`git_fetch`] of other parts of the repository (the DemoTool's source and files, `demotool.rs`).
pub fn git_fetch_parts(url: &str, branch: &str, dest: &Path, parts: &[&str]) -> Result<(), String> {
    let run = |args: &[&str], cwd: Option<&Path>| -> Result<(), String> {
        let mut c = Command::new("git");
        c.args(args).env("GIT_TERMINAL_PROMPT", "0");
        if let Some(d) = cwd {
            c.current_dir(d);
        }
        let out = c.output().map_err(|e| format!("cannot run git: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "git {}: {}",
                args.first().copied().unwrap_or(""),
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    };
    let dest_text = dest.to_string_lossy();
    run(
        &[
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            "--branch",
            branch,
            url,
            &dest_text,
        ],
        None,
    )?;
    let mut args = vec!["sparse-checkout", "set"];
    args.extend(parts);
    run(&args, Some(dest))?;
    std::fs::remove_dir_all(dest.join(".git")).map_err(|e| format!("{}: {e}", dest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn fake_fetch(dest: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dest.join("src/DisabledTests")).map_err(|e| e.to_string())?;
        std::fs::write(dest.join("src/DisabledTests/x.json"), "[]").map_err(|e| e.to_string())
    }

    #[test]
    fn the_branch_and_the_cache_folder_are_named_after_the_release() {
        assert_eq!(branch(27, 5), "releases/27.5");
        assert_eq!(cache_dir(Path::new("/c"), 28, 1), PathBuf::from("/c/bcapps/28.1"));
    }

    #[test]
    fn a_named_folder_is_used_as_it_is_and_never_fetched() {
        let root = tempfile::tempdir().unwrap();
        let mine = tempfile::tempdir().unwrap();
        let calls = Cell::new(0);
        let s = resolve(
            Some(mine.path()),
            27,
            5,
            root.path(),
            false,
            "u",
            &|_, _, _| {
                calls.set(calls.get() + 1);
                Ok(())
            },
            &|_| true,
        )
        .unwrap();
        assert_eq!(s.dir, mine.path());
        assert_eq!(calls.get(), 0);
        assert!(
            resolve(
                Some(&root.path().join("nope")),
                27,
                5,
                root.path(),
                false,
                "u",
                &|_, _, _| Ok(()),
                &|_| true
            )
            .unwrap_err()
            .contains("not a folder")
        );
    }

    #[test]
    fn the_first_run_fetches_into_the_cache_and_the_next_reads_it() {
        let root = tempfile::tempdir().unwrap();
        let calls = Cell::new(0);
        let fetch = |url: &str, branch: &str, dest: &Path| {
            calls.set(calls.get() + 1);
            assert_eq!((url, branch), ("U", "releases/27.5"));
            fake_fetch(dest)
        };
        let first = resolve(None, 27, 5, root.path(), false, "U", &fetch, &|_| true).unwrap();
        assert_eq!(first.dir, root.path().join("bcapps/27.5"));
        assert!(first.dir.join("src/DisabledTests/x.json").is_file());
        assert!(is_complete(&first.dir));
        assert!(first.how.contains("fetched"));
        let second = resolve(None, 27, 5, root.path(), false, "U", &fetch, &|_| true).unwrap();
        assert_eq!(second.dir, first.dir);
        assert!(second.how.contains("cache"));
        assert_eq!(calls.get(), 1);
        // No half-fetched folder is left behind.
        let left: Vec<_> = std::fs::read_dir(root.path().join("bcapps")).unwrap().collect();
        assert_eq!(left.len(), 1);
    }

    #[test]
    fn offline_uses_the_cache_and_refuses_to_fetch() {
        let root = tempfile::tempdir().unwrap();
        let err = resolve(
            None,
            27,
            5,
            root.path(),
            true,
            "U",
            &|_, _, _| panic!("fetched"),
            &|_| true,
        )
        .unwrap_err();
        assert!(err.contains("--no-download") && err.contains("--bcapps"), "{err}");
        // A folder without the finished marker is not a cache.
        let dir = cache_dir(root.path(), 27, 5);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(resolve(None, 27, 5, root.path(), true, "U", &|_, _, _| panic!(), &|_| true).is_err());
        std::fs::write(dir.join(COMPLETE), "").unwrap();
        assert!(resolve(None, 27, 5, root.path(), true, "U", &|_, _, _| panic!(), &|_| true).is_ok());
    }

    #[test]
    fn a_failed_fetch_leaves_no_cache_folder_and_names_the_branch() {
        let root = tempfile::tempdir().unwrap();
        let err = resolve(
            None,
            99,
            9,
            root.path(),
            false,
            "U",
            &|_, _, dest| {
                fake_fetch(dest)?;
                Err("remote branch not found".into())
            },
            &|_| true,
        )
        .unwrap_err();
        assert!(
            err.contains("releases/99.9") && err.contains("remote branch not found"),
            "{err}"
        );
        assert!(!cache_dir(root.path(), 99, 9).exists());
        let left: Vec<_> = std::fs::read_dir(root.path().join("bcapps")).unwrap().collect();
        assert!(left.is_empty());
    }

    #[test]
    fn a_release_without_the_recipe_uses_the_oldest_release_that_has_it_and_says_so() {
        let root = tempfile::tempdir().unwrap();
        let asked = std::cell::RefCell::new(Vec::new());
        let fetch = |_: &str, branch: &str, dest: &Path| {
            asked.borrow_mut().push(branch.to_string());
            fake_fetch(dest)?;
            // Only the fallback branch has the project settings.
            if branch == FIRST_WITH_RECIPE {
                std::fs::write(dest.join("marker-29"), "").map_err(|e| e.to_string())?;
            }
            Ok(())
        };
        let present = |d: &Path| d.join("marker-29").is_file();
        let s = resolve(None, 27, 5, root.path(), false, "U", &fetch, &present).unwrap();
        assert_eq!(*asked.borrow(), vec!["releases/27.5", "releases/29.0"]);
        assert_eq!(s.dir, root.path().join("bcapps/27.5"));
        assert!(s.dir.join("marker-29").is_file());
        assert!(
            s.how.contains("releases/27.5 has no BaseApp test recipe") && s.how.contains("releases/29.0"),
            "{}",
            s.how
        );
        // The cache keeps the reason.
        let again = resolve(None, 27, 5, root.path(), true, "U", &|_, _, _| panic!(), &present).unwrap();
        assert!(again.how.contains("no BaseApp test recipe"), "{}", again.how);
        // A fallback without the recipe is an error, and nothing stays in the cache.
        let none = resolve(None, 28, 1, root.path(), false, "U", &fetch, &|_| false).unwrap_err();
        assert!(none.contains("has no BaseApp test recipe either"), "{none}");
        assert!(!cache_dir(root.path(), 28, 1).exists());
    }

    /// The real `git` fetch, against a local repository with a release branch.
    #[test]
    fn git_fetch_takes_only_the_recipe_folders_of_the_branch() {
        let git = |dir: &Path, args: &[&str]| {
            let ok = Command::new("git")
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.com",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .current_dir(dir)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            assert!(ok, "git {args:?}");
        };
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let repo = tempfile::tempdir().unwrap();
        git(repo.path(), &["init", "-q", "-b", "main"]);
        for (p, text) in [
            ("src/DisabledTests/Tests-A/a.json", "[]"),
            ("build/projects/P/s.json", "{}"),
            ("build/scripts/TestConfiguration.json", "{}"),
            ("src/Apps/big.al", "not wanted"),
            ("LICENSE", "MIT"),
        ] {
            let f = repo.path().join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, text).unwrap();
        }
        git(repo.path(), &["add", "."]);
        git(repo.path(), &["commit", "-q", "-m", "x"]);
        git(repo.path(), &["branch", "releases/27.5"]);
        let out = tempfile::tempdir().unwrap();
        let dest = out.path().join("checkout");
        let url = format!("file://{}", repo.path().display());
        git_fetch(&url, "releases/27.5", &dest).unwrap();
        assert!(dest.join("src/DisabledTests/Tests-A/a.json").is_file());
        assert!(dest.join("build/projects/P/s.json").is_file());
        assert!(dest.join("build/scripts/TestConfiguration.json").is_file());
        assert!(dest.join("LICENSE").is_file());
        assert!(!dest.join("src/Apps").exists());
        assert!(!dest.join(".git").exists());
        let err = git_fetch(&url, "releases/1.0", &out.path().join("other")).unwrap_err();
        assert!(err.contains("git"), "{err}");
    }
}
