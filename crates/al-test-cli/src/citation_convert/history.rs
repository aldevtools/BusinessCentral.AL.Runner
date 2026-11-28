//! The git side of `--convert-citations`: the corpus pin a citing line was written at (the first commit whose added
//! line writes the citation's key, `git log -p --follow`, followed into the file a commit moved the line from; `git
//! blame` past the path-rewrite commits when no commit does), the corpus files at a pin, and the next pin that changed
//! a cited file, followed across a move. Entry points: `Repo::open`, `Repo::first_pins`, `Repo::blame`,
//! `Repo::locate`, `Repo::next_change`.

use super::Class;
use crate::citation_old::{Key, Written};
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
    /// The commits that only rewrote citation paths (full ids): their lines were written before them.
    rewrites: BTreeSet<String>,
    /// Per runner commit: the lines it removed, with their files.
    removed: Mutex<BTreeMap<String, Arc<Removed>>>,
}

/// The lines one runner commit removed: each with its file.
type Removed = Vec<(String, String)>;

/// Where the first commit that wrote a citation's key was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum First {
    /// The corpus pin of that commit.
    Pin(String),
    /// No commit wrote it: the line is not committed yet, or the file is not tracked.
    Unwritten,
    /// The commit moved the line from another file, where no earlier commit can be found that wrote it: why.
    Lost(String),
}

/// The next pin that changed a cited file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Next {
    /// It stays the same up to the checked-out pin.
    Same,
    /// It differs at this pin and path.
    Changed(At),
    /// It leaves its path after this pin, with no file of the same content or name to follow.
    Lost(String),
}

/// The commit `git blame` names for a citing line, and its corpus pin.
#[derive(Debug, Clone)]
pub(super) struct Blamed {
    pub commit: Option<String>,
    pub pin: String,
}

/// How many times a citing line is followed into the file a commit moved it from.
const MOVES: usize = 4;

/// Where one cited file is read: the pin and its path there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct At {
    pub pin: String,
    pub path: String,
}

impl<'a> Repo<'a> {
    pub fn open(root: &'a Path, corpus: &'a Path, rewrites: &[&str]) -> Result<Repo<'a>, String> {
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
        let rewrites = rewrites
            .iter()
            .map(|r| {
                git_lines(root, &["rev-parse", "--verify", &format!("{r}^{{commit}}")])?
                    .pop()
                    .ok_or_else(|| format!("path-rewrite commit {r} is not in the repository"))
            })
            .collect::<Result<_, String>>()?;
        Ok(Repo {
            root,
            corpus,
            pins,
            current,
            listings: Default::default(),
            commit_pins: Default::default(),
            rewrites,
            removed: Default::default(),
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

    /// Per key of `keys`: where the first commit (oldest first, across renames of `path`) whose added line writes it
    /// was found, else the commit of `blamed` (the same index). A path-rewrite commit writes none. A commit that also
    /// removes the key from another file moved the line: the first commit is looked up in that file's history before it
    /// (review r1 of PR 9454, H1).
    pub fn first_pins(&self, path: &Path, keys: &[Key], blamed: &[Option<String>]) -> Result<Vec<First>, String> {
        let file = path.to_string_lossy();
        let wanted: Vec<&Key> = keys.iter().collect();
        let found = self.first_commits(&file, None, &wanted)?;
        let mut out = Vec::with_capacity(keys.len());
        for ((k, f), b) in keys.iter().zip(found).zip(blamed) {
            out.push(match (f, b) {
                (Some((commit, added_to)), _) => self.followed(k, commit, added_to, 0)?,
                (None, Some(commit)) => self.followed(k, commit.clone(), file.to_string(), 0)?,
                (None, None) => First::Unwritten,
            });
        }
        Ok(out)
    }

    /// The first pin of key `k`, which `commit` added to file `added_to`: its pin, or where the line was moved from.
    fn followed(&self, k: &Key, commit: String, added_to: String, moves: usize) -> Result<First, String> {
        let removed = self.removed(&commit)?;
        let from = removed
            .iter()
            .find(|(f, l)| *f != added_to && k.written_in(&Written::of(l)))
            .map(|(f, _)| f.clone());
        let Some(from) = from else {
            return Ok(match self.pin(&commit)? {
                Some(pin) => First::Pin(pin),
                None => First::Unwritten,
            });
        };
        let lost = |why: &str| First::Lost(format!("{commit:.10} moved the citing line from {from}, {why}"));
        if moves == MOVES {
            return Ok(lost("one move too many to follow"));
        }
        match self.first_commits(&from, Some(&commit), &[k])?.pop().flatten() {
            Some((c, f)) => self.followed(k, c, f, moves + 1),
            None => Ok(lost("where no earlier commit wrote it")),
        }
    }

    /// Per key: the first commit (oldest first, across renames of `file`, before `until` when given) whose added line
    /// writes it, and the file it was added to there. Path-rewrite commits are skipped.
    fn first_commits(
        &self,
        file: &str,
        until: Option<&str>,
        keys: &[&Key],
    ) -> Result<Vec<Option<(String, String)>>, String> {
        let mut found: Vec<Option<(String, String)>> = vec![None; keys.len()];
        if keys.is_empty() {
            return Ok(found);
        }
        let before = until.map(|u| format!("{u}^"));
        let mut args = vec!["log", "-p", "--follow", "--reverse", "--no-color", "--format=%x01%H"];
        args.extend(before.as_deref());
        args.extend(["--", file]);
        let o = crate::base_oracle::git(self.root, &args)?;
        if !o.status.success() {
            return Ok(found);
        }
        let text = String::from_utf8_lossy(&o.stdout);
        let (mut commit, mut to, mut skip, mut left) = ("", "", false, keys.len());
        for l in text.lines() {
            if let Some(sha) = l.strip_prefix('\u{1}') {
                commit = sha;
                skip = self.rewrites.contains(sha);
                continue;
            }
            if let Some(f) = l.strip_prefix("+++ b/") {
                to = f;
                continue;
            }
            let Some(added) = l.strip_prefix('+').filter(|_| !skip && !l.starts_with("+++ ")) else {
                continue;
            };
            let w = Written::of(added);
            for (k, slot) in keys.iter().zip(found.iter_mut()) {
                if slot.is_none() && k.written_in(&w) {
                    *slot = Some((commit.to_string(), to.to_string()));
                    left -= 1;
                }
            }
            if left == 0 {
                break;
            }
        }
        Ok(found)
    }

    /// The lines runner commit `commit` removed, with their files (cached).
    fn removed(&self, commit: &str) -> Result<Arc<Removed>, String> {
        if let Some(r) = self.removed.lock().unwrap_or_else(|e| e.into_inner()).get(commit) {
            return Ok(r.clone());
        }
        let text = git_lines(self.root, &["show", "--no-color", "--format=", "-p", commit])?;
        let mut out = Vec::new();
        let mut from = String::new();
        for l in &text {
            if let Some(f) = l.strip_prefix("--- ") {
                from = f.strip_prefix("a/").unwrap_or(f).to_string();
            } else if let Some(r) = l.strip_prefix('-') {
                out.push((from.clone(), r.to_string()));
            }
        }
        let out = Arc::new(out);
        self.removed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(commit.to_string(), out.clone());
        Ok(out)
    }

    /// The commit that last changed each of `lines` of `path` (`git blame` of the working tree, past the path-rewrite
    /// commits) and its corpus pin; a line not committed yet, or a file git does not track, has no commit and reads the
    /// checked-out pin.
    pub fn blame(&self, path: &Path, lines: &BTreeSet<usize>) -> Result<BTreeMap<usize, Blamed>, String> {
        let mut out: BTreeMap<usize, Blamed> = lines
            .iter()
            .map(|&l| {
                let b = Blamed {
                    commit: None,
                    pin: self.current.clone(),
                };
                (l, b)
            })
            .collect();
        if lines.is_empty() {
            return Ok(out);
        }
        let mut args = vec!["blame".to_string(), "--porcelain".to_string()];
        for r in &self.rewrites {
            args.extend(["--ignore-rev".to_string(), r.clone()]);
        }
        args.extend(lines.iter().map(|l| format!("-L{l},{l}")));
        args.extend(["--".to_string(), path.to_string_lossy().into_owned()]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let o = crate::base_oracle::git(self.root, &args)?;
        if !o.status.success() {
            let file = path.to_string_lossy();
            if git_lines(self.root, &["ls-files", "--", &file])?.is_empty() {
                return Ok(out); // untracked: the checked-out pin
            }
            return Err(format!("git blame {file}: {}", String::from_utf8_lossy(&o.stderr)));
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
                let commit = Some(sha.to_string());
                out.insert(fin, Blamed { commit, pin });
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

    /// Where the file `cited` (as written) is read for `pin`: at `pin` by its path, else by its file name when that is
    /// unique there (a folder renamed before the line was written). When no pin up to `pin` has it, the line was written
    /// ahead of its pin bump: at the first later pin that has it. A file that left before `pin` is a hand fix (the pin
    /// cannot be read, review r1 of PR 9454, M1); no pin with it, not a corpus file.
    pub fn locate(&self, pin: &str, cited: &str) -> Result<At, Class> {
        let at = self.pins.iter().position(|p| p == pin).unwrap_or(self.pins.len());
        let earlier = self.pins[..at].iter().map(String::as_str);
        let (found, p) = match self.find(pin, cited) {
            Some(f) => (f, pin),
            None if earlier.clone().any(|p| self.find(p, cited).is_some()) => return Err(Class::HandFix),
            None => match self.later(pin).find_map(|p| Some((self.find(p, cited)?, p))) {
                Some(hit) => hit,
                None => return Err(Class::NotCorpus),
            },
        };
        match found? {
            path if path.starts_with("tests/must-not-compile/") => Err(Class::MustNotCompile),
            path => Ok(At {
                pin: p.to_string(),
                path,
            }),
        }
    }

    /// The one file at `pin` that `cited` names: by its path, else by its file name; [`Class::HandFix`] when it names
    /// several, `None` when none.
    fn find(&self, pin: &str, cited: &str) -> Option<Result<String, Class>> {
        let rel = cited.trim_start_matches("corpus/").trim_start_matches("tests/");
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let (exact, suffix) = (format!("tests/{rel}"), format!("/{rel}"));
        let files = self.listing(pin);
        let by_path: Vec<&String> = files.keys().filter(|f| **f == exact || f.ends_with(&suffix)).collect();
        let hits = if by_path.is_empty() {
            files.keys().filter(|f| f.rsplit('/').next() == Some(name)).collect()
        } else {
            by_path
        };
        match hits.as_slice() {
            [] => None,
            [f] => Some(Ok(f.to_string())),
            _ => Some(Err(Class::HandFix)),
        }
    }

    /// The first later pin at which the file of `at` differs. A file that leaves its path is followed to the one file
    /// of the same content, else of the same name, at that pin (review r1 of PR 9454, H1).
    pub fn next_change(&self, at: &At) -> Next {
        let Some(blob) = self.listing(&at.pin).get(&at.path).cloned() else {
            return Next::Lost(at.pin.clone());
        };
        let (mut path, mut last) = (at.path.clone(), at.pin.as_str());
        let name = at.path.rsplit('/').next().unwrap_or_default().to_string();
        for p in self.later(&at.pin) {
            let files = self.listing(p);
            if !files.contains_key(&path) {
                let same: Vec<&String> = files.iter().filter(|(_, b)| **b == blob).map(|(f, _)| f).collect();
                let named: Vec<&String> = files.keys().filter(|f| f.rsplit('/').next() == Some(&name)).collect();
                match (same.as_slice(), named.as_slice()) {
                    ([f], _) | ([], [f]) => path = f.to_string(),
                    _ => return Next::Lost(last.to_string()),
                }
            }
            if files[&path] != blob {
                return Next::Changed(At {
                    pin: p.to_string(),
                    path,
                });
            }
            last = p;
        }
        Next::Same
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
