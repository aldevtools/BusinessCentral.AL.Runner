//! The git side of `--convert-citations`: the corpus pin a citing line was written at (the first commit whose added
//! line holds the citation text, `git log -p --follow`; `git blame` when no commit does), the corpus files at a pin,
//! and the next pin that changed a cited file. Entry points: `Repo::open`, `Repo::pickaxe_pins`, `Repo::blame_pins`,
//! `Repo::locate`, `Repo::next_change`.

use super::Class;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The runner repository and its corpus checkout.
pub(super) struct Repo<'a> {
    root: &'a Path,
    corpus: &'a Path,
    /// Every corpus pin of the history, oldest first.
    pins: Vec<String>,
    /// The checked-out corpus commit.
    pub current: String,
    /// Per pin: the files under `tests/` and their blob ids.
    listings: Mutex<BTreeMap<String, Arc<BTreeMap<String, String>>>>,
    /// Per runner commit: its corpus pin.
    commit_pins: Mutex<BTreeMap<String, Option<String>>>,
}

/// Where one cited file is read: the pin and its path there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct At {
    pub pin: String,
    pub path: String,
}

impl<'a> Repo<'a> {
    pub fn open(root: &'a Path, corpus: &'a Path) -> Result<Repo<'a>, String> {
        let commits = git_lines(root, &["rev-list", "--topo-order", "--reverse", "HEAD", "--", "corpus"])?;
        let mut pins: Vec<String> = Vec::new();
        for c in commits {
            if let Some(p) = pin_of(root, &c)?
                && !pins.contains(&p)
            {
                pins.push(p);
            }
        }
        let current = git_lines(corpus, &["rev-parse", "HEAD"])?
            .pop()
            .ok_or("the corpus checkout has no HEAD")?;
        Ok(Repo {
            root,
            corpus,
            pins,
            current,
            listings: Default::default(),
            commit_pins: Default::default(),
        })
    }

    /// The corpus pin of runner commit `commit` (cached); `None` before the corpus was a submodule.
    fn pin(&self, commit: &str) -> Result<Option<String>, String> {
        if let Some(p) = self.commit_pins.lock().unwrap_or_else(|e| e.into_inner()).get(commit) {
            return Ok(p.clone());
        }
        let p = pin_of(self.root, commit)?;
        self.commit_pins
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(commit.to_string(), p.clone());
        Ok(p)
    }

    /// Per text of `keys`: the corpus pin of the first commit (oldest first, across renames of `path`) whose added
    /// line holds it. A text no commit added (the line was rewritten since, the file is not tracked) has no entry.
    pub fn pickaxe_pins(&self, path: &Path, keys: &BTreeSet<String>) -> Result<BTreeMap<String, String>, String> {
        let mut found: BTreeMap<String, String> = BTreeMap::new();
        if keys.is_empty() {
            return Ok(found);
        }
        let file = path.to_string_lossy();
        let o = crate::base_oracle::git(
            self.root,
            &[
                "log",
                "-p",
                "--follow",
                "--reverse",
                "--no-color",
                "--format=%x01%H",
                "--",
                &file,
            ],
        )?;
        if !o.status.success() {
            return Ok(found);
        }
        let text = String::from_utf8_lossy(&o.stdout);
        let mut commit = "";
        for l in text.lines() {
            if let Some(sha) = l.strip_prefix('\u{1}') {
                commit = sha;
                continue;
            }
            let Some(added) = l.strip_prefix('+').filter(|_| !l.starts_with("+++ ")) else {
                continue;
            };
            for k in keys {
                if !found.contains_key(k)
                    && added.contains(k.as_str())
                    && let Some(pin) = self.pin(commit)?
                {
                    found.insert(k.clone(), pin);
                }
            }
            if found.len() == keys.len() {
                break;
            }
        }
        Ok(found)
    }

    /// The corpus pin of the commit that last changed each of `lines` of `path` (`git blame` of the working tree); a
    /// line not committed yet, or a file git does not track, reads the checked-out pin.
    pub fn blame_pins(&self, path: &Path, lines: &BTreeSet<usize>) -> Result<BTreeMap<usize, String>, String> {
        let mut out: BTreeMap<usize, String> = lines.iter().map(|&l| (l, self.current.clone())).collect();
        if lines.is_empty() {
            return Ok(out);
        }
        let mut args = vec!["blame".to_string(), "--porcelain".to_string()];
        args.extend(lines.iter().map(|l| format!("-L{l},{l}")));
        args.extend(["--".to_string(), path.to_string_lossy().into_owned()]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let o = crate::base_oracle::git(self.root, &args)?;
        if !o.status.success() {
            return Ok(out); // untracked: the checked-out pin
        }
        for l in String::from_utf8_lossy(&o.stdout).lines() {
            let mut w = l.split(' ');
            let (Some(sha), Some(_), Some(fin)) = (w.next(), w.next(), w.next()) else {
                continue;
            };
            let Ok(fin) = fin.parse::<usize>() else { continue };
            if sha.len() != 40 || !sha.bytes().all(|c| c.is_ascii_hexdigit()) || sha.bytes().all(|c| c == b'0') {
                continue;
            }
            if let Some(pin) = self.pin(sha)? {
                out.insert(fin, pin);
            }
        }
        Ok(out)
    }

    /// The corpus files under `tests/` at `pin`, with their blob ids.
    fn listing(&self, pin: &str) -> Arc<BTreeMap<String, String>> {
        let mut l = self.listings.lock().unwrap_or_else(|e| e.into_inner());
        l.entry(pin.to_string())
            .or_insert_with(|| {
                let rows = git_lines(self.corpus, &["ls-tree", "-r", pin, "--", "tests"]).unwrap_or_default();
                // `<mode> blob <sha>\t<path>`
                Arc::new(
                    rows.iter()
                        .filter_map(|r| {
                            let (meta, path) = r.split_once('\t')?;
                            Some((path.to_string(), meta.rsplit(' ').next()?.to_string()))
                        })
                        .collect(),
                )
            })
            .clone()
    }

    /// The pins after `pin`, oldest first, ending with the checked-out one.
    fn later(&self, pin: &str) -> impl Iterator<Item = &str> {
        let at = self
            .pins
            .iter()
            .position(|p| p == pin)
            .map_or(self.pins.len(), |k| k + 1);
        self.pins[at..]
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(self.current.as_str()))
    }

    /// Where the file `cited` (as written) is read for a citation written at `pin`: at `pin` by its path, else by its
    /// file name when that is unique there (a folder renamed before the line was written); else at the first later pin
    /// that has it, else the last earlier one.
    pub fn locate(&self, pin: &str, cited: &str) -> Result<At, Class> {
        let rel = cited.trim_start_matches("corpus/").trim_start_matches("tests/");
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let at = self.pins.iter().position(|p| p == pin).unwrap_or(self.pins.len());
        let earlier = self.pins[..at].iter().rev().map(String::as_str);
        let order = std::iter::once(pin).chain(self.later(pin)).chain(earlier);
        let (exact, suffix) = (format!("tests/{rel}"), format!("/{rel}"));
        let mut seen = BTreeSet::new();
        for p in order {
            if !seen.insert(p) {
                continue;
            }
            let files = self.listing(p);
            let by_path: Vec<&String> = files.keys().filter(|f| **f == exact || f.ends_with(&suffix)).collect();
            let hits = if by_path.is_empty() {
                files.keys().filter(|f| f.rsplit('/').next() == Some(name)).collect()
            } else {
                by_path
            };
            match hits.as_slice() {
                [] => continue,
                [f] if f.starts_with("tests/must-not-compile/") => return Err(Class::MustNotCompile),
                [f] => {
                    return Ok(At {
                        pin: p.to_string(),
                        path: f.to_string(),
                    });
                }
                _ => return Err(Class::HandFix),
            }
        }
        Err(Class::NotCorpus)
    }

    /// The first later pin at which the file of `at` differs, at the same path; `None` when it stays the same up to the
    /// checked-out pin or leaves that path.
    pub fn next_change(&self, at: &At) -> Option<At> {
        let blob = self.listing(&at.pin).get(&at.path)?.clone();
        for p in self.later(&at.pin) {
            match self.listing(p).get(&at.path) {
                Some(b) if *b == blob => continue,
                Some(_) => {
                    return Some(At {
                        pin: p.to_string(),
                        path: at.path.clone(),
                    });
                }
                None => return None,
            }
        }
        None
    }
}

/// The corpus pin of runner commit `commit`; `None` before the corpus was a submodule.
fn pin_of(root: &Path, commit: &str) -> Result<Option<String>, String> {
    let o = crate::base_oracle::git(root, &["rev-parse", "--verify", "-q", &format!("{commit}:corpus")])?;
    Ok(o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string()))
}

/// The output lines of `git -C dir args`; an error when git fails.
fn git_lines(dir: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let o = crate::base_oracle::git(dir, args)?;
    if !o.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect())
}
