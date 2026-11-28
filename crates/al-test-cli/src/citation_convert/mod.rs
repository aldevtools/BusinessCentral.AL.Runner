//! `--convert-citations PATH`: rewrites old corpus citations (`File.al:line`, `old.rs`) under PATH into citation tokens
//! by name (ADR-065 amendment 2, step 3 of the corpus-detach plan). Each citation is read at the corpus pin of the
//! commit that wrote its line (`git blame`): the file at that pin, the line mapped to what it names there (`place.rs`),
//! then carried to the checked-out pin by object id and the rename map (`citation::carry`) and checked there
//! (`citation::resolve`). What does not map is listed for a hand fix and left as it is; the run is deterministic and
//! can be repeated (a converted line holds no old citation). The module goes once every crate is converted.
//! Entry points: `convert`, `summary`.

mod old;
mod place;

use crate::citation::{self, Citation, Corpus, Resolved, Status};
use crate::rename_map::RenameMap;
use place::{File, Target};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What became of one old citation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// One `[Test]` procedure.
    Test,
    /// One cell of a test.
    Cell,
    /// A procedure or trigger that is not a test.
    Member,
    /// An object: a line outside its members, or a range over most of a codeunit's tests.
    Object,
    /// The file header.
    Header,
    /// A file without lines.
    File,
    /// Several targets (a range over several members, or a line list).
    Range,
    /// Left for a hand fix; the row says why.
    HandFix,
    /// Not a corpus file at any pin (a fixture, a Microsoft source), left.
    NotCorpus,
    /// A must-not-compile case: no token yet (amendment 2), left.
    MustNotCompile,
    /// In Rust code, not a comment (a string literal), left.
    InCode,
    /// A bare `:line` with no file citation near it, left.
    Orphan,
}

impl Class {
    pub const ALL: [Class; 12] = [
        Class::Test,
        Class::Cell,
        Class::Member,
        Class::Object,
        Class::Header,
        Class::File,
        Class::Range,
        Class::HandFix,
        Class::NotCorpus,
        Class::MustNotCompile,
        Class::InCode,
        Class::Orphan,
    ];
    fn name(self) -> &'static str {
        match self {
            Class::Test => "test",
            Class::Cell => "cell",
            Class::Member => "member",
            Class::Object => "object",
            Class::Header => "header",
            Class::File => "file",
            Class::Range => "range",
            Class::HandFix => "hand-fix",
            Class::NotCorpus => "not-corpus",
            Class::MustNotCompile => "mnc",
            Class::InCode => "in-code",
            Class::Orphan => "orphan",
        }
    }
}

/// One old citation and what it became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Relative to the repository root.
    pub path: PathBuf,
    pub line: usize,
    pub old: String,
    /// The text that replaces it; `None` when it stays.
    pub new: Option<String>,
    pub class: Class,
    /// Why it stays (hand fix), or the pin it was read at.
    pub note: String,
}

/// The git side of a conversion: who wrote a line, which corpus pin a commit had, and the corpus files at a pin.
struct Repo<'a> {
    root: &'a Path,
    corpus: &'a Path,
    /// Every corpus pin of the history, oldest first.
    pins: Vec<String>,
    /// The checked-out corpus commit.
    current: String,
    listings: std::sync::Mutex<BTreeMap<String, std::sync::Arc<Vec<String>>>>,
}

/// Where one cited file is read: the pin and its path there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct At {
    pin: String,
    path: String,
}

/// Converts the old citations of the files under `paths` (relative to `root`; files or folders). `corpus` is the
/// corpus checkout (the pin citations are carried to), `map` its rename map. Writes the files unless `dry_run`.
pub fn convert(
    root: &Path,
    corpus: &Path,
    map: &RenameMap,
    paths: &[PathBuf],
    dry_run: bool,
) -> Result<Vec<Row>, String> {
    let repo = Repo::open(root, corpus)?;
    let new = crate::citation_scan::corpus_at_checkout(corpus)?;
    let files = citing_files(root, paths)?;
    // Per file: its old citations and the pin of each citing line.
    let found: Vec<(PathBuf, String, Vec<old::OldCite>, BTreeMap<usize, String>)> = files
        .into_par_iter()
        .map(|(path, text)| {
            let cites = old::find(&text, path.extension().is_some_and(|e| e == "rs"));
            let lines: BTreeSet<usize> = cites.iter().map(|c| line_of(&text, c.at.start)).collect();
            let pins = repo.pins_of_lines(&path, &lines)?;
            Ok((path, text, cites, pins))
        })
        .collect::<Result<_, String>>()?;
    // Where each cited file is read, then those texts in one batch.
    let mut wanted: BTreeSet<At> = BTreeSet::new();
    let mut located: BTreeMap<(String, String), Result<At, Class>> = BTreeMap::new();
    for (path, text, cites, pins) in &found {
        for c in cites.iter().filter(|c| !c.bare && c.in_comment) {
            let pin = &pins[&line_of(text, c.at.start)];
            let key = (pin.clone(), c.path.clone());
            if !located.contains_key(&key) {
                let at = repo.locate(pin, &c.path);
                if let Ok(at) = &at {
                    wanted.insert(at.clone());
                }
                located.insert(key, at);
            }
        }
        let _ = path;
    }
    let wanted: Vec<At> = wanted.into_iter().collect();
    let requests: Vec<String> = wanted.iter().map(|a| format!("{}:{}", a.pin, a.path)).collect();
    let texts = crate::citation_scan::cat_files(corpus, &requests)?;
    let sources: BTreeMap<At, String> = wanted.into_iter().zip(texts).collect();
    let parsed: BTreeMap<&At, File> = sources.iter().map(|(a, t)| (a, File::new(t))).collect();
    let ctx = Ctx {
        repo: &repo,
        new: &new,
        map,
        located: &located,
        parsed: &parsed,
    };
    let mut rows = Vec::new();
    for (path, text, cites, pins) in &found {
        let (out, file_rows) = ctx.file(path, text, cites, pins);
        if !dry_run && out != *text {
            std::fs::write(root.join(path), &out).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        rows.extend(file_rows);
    }
    Ok(rows)
}

/// The 1-based line of byte `at`.
fn line_of(text: &str, at: usize) -> usize {
    text[..at].matches('\n').count() + 1
}

/// The UTF-8 files under `paths` that may hold an old citation (`.al` in the text), generated files skipped.
fn citing_files(root: &Path, paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>, String> {
    let mut files = Vec::new();
    for p in paths {
        let full = root.join(p);
        if full.is_dir() {
            crate::citation_scan::walk(&full, &mut files)?;
        } else if full.is_file() {
            files.push(full);
        } else {
            return Err(format!("{}: no such file or folder", p.display()));
        }
    }
    files.sort();
    files.dedup();
    let mut out = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let generated = text
            .lines()
            .take(5)
            .any(|l| l.contains("@generated") || l.contains("Generated by"));
        if text.contains(".al") && !generated {
            out.push((f.strip_prefix(root).unwrap_or(&f).to_path_buf(), text));
        }
    }
    Ok(out)
}

impl<'a> Repo<'a> {
    fn open(root: &'a Path, corpus: &'a Path) -> Result<Repo<'a>, String> {
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
        })
    }

    /// The corpus pin of the commit that wrote each of `lines` of `path` (`git blame` of the working tree); a line not
    /// committed yet, or a file git does not track, reads the checked-out pin.
    fn pins_of_lines(&self, path: &Path, lines: &BTreeSet<usize>) -> Result<BTreeMap<usize, String>, String> {
        let mut out: BTreeMap<usize, String> = lines.iter().map(|&l| (l, self.current.clone())).collect();
        if lines.is_empty() {
            return Ok(out);
        }
        let mut args = vec!["blame".to_string(), "--porcelain".to_string()];
        for l in lines {
            args.push(format!("-L{l},{l}"));
        }
        args.extend(["--".to_string(), path.to_string_lossy().into_owned()]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let o = crate::base_oracle::git(self.root, &args)?;
        if !o.status.success() {
            return Ok(out); // untracked: the checked-out pin
        }
        let mut commit_pins: BTreeMap<String, Option<String>> = BTreeMap::new();
        for l in String::from_utf8_lossy(&o.stdout).lines() {
            let mut w = l.split(' ');
            let (Some(sha), Some(_), Some(fin)) = (w.next(), w.next(), w.next()) else { continue };
            let Ok(fin) = fin.parse::<usize>() else { continue };
            if sha.len() != 40 || !sha.bytes().all(|c| c.is_ascii_hexdigit()) || sha.bytes().all(|c| c == b'0') {
                continue;
            }
            if !commit_pins.contains_key(sha) {
                commit_pins.insert(sha.to_string(), pin_of(self.root, sha)?);
            }
            if let Some(Some(pin)) = commit_pins.get(sha) {
                out.insert(fin, pin.clone());
            }
        }
        Ok(out)
    }

    /// The corpus files under `tests/` at `pin`.
    fn listing(&self, pin: &str) -> std::sync::Arc<Vec<String>> {
        let mut l = self.listings.lock().unwrap_or_else(|e| e.into_inner());
        l.entry(pin.to_string())
            .or_insert_with(|| {
                std::sync::Arc::new(git_lines(self.corpus, &["ls-tree", "-r", "--name-only", pin, "--", "tests"]).unwrap_or_default())
            })
            .clone()
    }

    /// Where the file `cited` (as written) is read for a citation written at `pin`: at `pin` by its path, else by its
    /// file name when that is unique there (a folder renamed before the line was written); else at the first later pin
    /// that has it, else the last earlier one.
    fn locate(&self, pin: &str, cited: &str) -> Result<At, Class> {
        let rel = cited.trim_start_matches("corpus/").trim_start_matches("tests/");
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let at = self.pins.iter().position(|p| p == pin).unwrap_or(self.pins.len());
        let later = self.pins.iter().skip(at + 1).chain(std::iter::once(&self.current));
        let order = std::iter::once(pin).chain(later.map(String::as_str)).chain(self.pins[..at.min(self.pins.len())].iter().rev().map(String::as_str));
        let mut seen = BTreeSet::new();
        for p in order {
            if !seen.insert(p) {
                continue;
            }
            let files = self.listing(p);
            let by_path: Vec<&String> = files.iter().filter(|f| f.as_str() == format!("tests/{rel}") || f.ends_with(&format!("/{rel}"))).collect();
            let hits = if by_path.is_empty() {
                files.iter().filter(|f| f.rsplit('/').next() == Some(name)).collect()
            } else {
                by_path
            };
            match hits.as_slice() {
                [] => continue,
                [f] if f.starts_with("tests/must-not-compile/") => return Err(Class::MustNotCompile),
                [f] => return Ok(At { pin: p.to_string(), path: f.to_string() }),
                _ => return Err(Class::HandFix),
            }
        }
        Err(Class::NotCorpus)
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
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&o.stderr)));
    }
    Ok(String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect())
}

/// What one file's conversion reads.
struct Ctx<'a> {
    repo: &'a Repo<'a>,
    new: &'a Corpus,
    map: &'a RenameMap,
    located: &'a BTreeMap<(String, String), Result<At, Class>>,
    parsed: &'a BTreeMap<&'a At, File<'a>>,
}

impl Ctx<'_> {
    /// The converted text of one file and a row per old citation. A file citation and its bare continuations convert
    /// together or stay together.
    fn file(
        &self,
        path: &Path,
        text: &str,
        cites: &[old::OldCite],
        pins: &BTreeMap<usize, String>,
    ) -> (String, Vec<Row>) {
        let line = |c: &old::OldCite| line_of(text, c.at.start);
        // Each citation's file: its own for a file citation, its parent's for a bare one.
        let mut results: Vec<Result<(String, Class), (Class, String)>> = Vec::with_capacity(cites.len());
        for c in cites {
            let r = if !c.in_comment {
                Err((Class::InCode, String::new()))
            } else {
                let file = if c.bare { c.parent.map(|p| &cites[p]) } else { Some(c) };
                match file {
                    None => Err((Class::Orphan, String::new())),
                    Some(f) if !f.in_comment => Err((Class::Orphan, String::new())),
                    Some(f) => self.one(&pins[&line(c)], &f.path, &c.lines, c.bare),
                }
            };
            results.push(r);
        }
        // A group (a file citation and its continuations) with a hand fix stays whole.
        let mut group_fails = vec![false; cites.len()];
        for (k, c) in cites.iter().enumerate() {
            let g = if c.bare { c.parent } else { Some(k) };
            if let (Some(g), Err((Class::HandFix, _))) = (g, &results[k]) {
                group_fails[g] = true;
            }
        }
        let (mut out, mut rows, mut last) = (String::with_capacity(text.len()), Vec::new(), 0);
        for (k, c) in cites.iter().enumerate() {
            let g = if c.bare { c.parent } else { Some(k) };
            let held = g.is_some_and(|g| group_fails[g]);
            let old = text[c.at.clone()].to_string();
            let mut row = Row {
                path: path.to_path_buf(),
                line: line(c),
                old,
                new: None,
                class: Class::HandFix,
                note: String::new(),
            };
            match &results[k] {
                Ok((new, class)) if !held => {
                    out.push_str(&text[last..c.at.start]);
                    out.push_str(new);
                    last = c.at.end;
                    row.new = Some(new.clone());
                    row.class = *class;
                }
                Ok(_) => row.note = "another citation of its group needs a hand fix".into(),
                Err((class, note)) => {
                    row.class = *class;
                    row.note = note.clone();
                }
            }
            rows.push(row);
        }
        out.push_str(&text[last..]);
        (out, rows)
    }

    /// The tokens for `lines` of the file cited as `cited` in a line written at `pin`.
    fn one(&self, pin: &str, cited: &str, lines: &[(u32, u32)], bare: bool) -> Result<(String, Class), (Class, String)> {
        let at = match &self.located[&(pin.to_string(), cited.to_string())] {
            Ok(at) => at,
            Err(Class::HandFix) => return Err((Class::HandFix, format!("`{cited}` names several corpus files"))),
            Err(class) => return Err((*class, String::new())),
        };
        let file = &self.parsed[at];
        let targets = if lines.is_empty() {
            match file.main_object() {
                Some(o) => vec![Target::Object(o)],
                None => return Err((Class::HandFix, format!("{} at {} has no object", at.path, short(&at.pin)))),
            }
        } else {
            file.targets(lines)
                .map_err(|e| (Class::HandFix, format!("{} at {}: {e}", at.path, short(&at.pin))))?
        };
        let mut tokens: Vec<String> = Vec::new();
        let mut class = Class::Range;
        for t in &targets {
            let (token, c) = self.token(file, t).map_err(|e| (Class::HandFix, format!("{} at {}: {e}", at.path, short(&at.pin))))?;
            class = c;
            if !tokens.contains(&token) {
                tokens.push(token);
            }
        }
        if lines.is_empty() && !bare {
            class = Class::File;
        } else if tokens.len() > 1 {
            class = Class::Range;
        }
        let _ = self.repo;
        Ok((tokens.join(", "), class))
    }

    /// The token of one target at the checked-out pin: the citation at the target's pin, carried and checked.
    fn token(&self, file: &File, t: &Target) -> Result<(String, Class), String> {
        let (o, member, cell, class) = match t {
            Target::Header(o) => (*o, None, None, Class::Header),
            Target::Object(o) => (*o, None, None, Class::Object),
            Target::Member { object, member, cell } => {
                let m = &file.objects[*object].members[*member];
                let class = match (m.test, cell) {
                    (true, Some(_)) => Class::Cell,
                    (true, None) => Class::Test,
                    _ => Class::Member,
                };
                (*object, Some(m), cell.clone(), class)
            }
        };
        let obj = &file.objects[o];
        let c = Citation {
            status: Status::Pinned,
            kind: obj.kind.clone(),
            object: obj.name.clone(),
            member: member.map(|m| m.name.clone()),
            cell,
        };
        let r = match (member, obj.id) {
            (Some(m), Some(id)) if m.test && obj.test_codeunit => Resolved::Case(crate::oracle::CaseId {
                codeunit: id,
                name: m.name.clone(),
            }),
            _ => Resolved::Object {
                kind: obj.kind.clone(),
                id: obj.id,
            },
        };
        let carried = citation::carry(&c, r, self.new, self.map)?.unwrap_or(c);
        let checked = match citation::resolve(&carried, self.new, &BTreeSet::new()) {
            Ok(_) => carried,
            // The cell's literal changed since the pin: cite the test.
            Err(_) if carried.cell.is_some() => {
                let test = Citation { cell: None, ..carried };
                citation::resolve(&test, self.new, &BTreeSet::new())?;
                return Ok((test.to_string(), Class::Test));
            }
            Err(e) => return Err(e),
        };
        let token = checked.to_string();
        Ok(match class {
            Class::Header => (format!("the file header of {token}"), class),
            _ => (token, class),
        })
    }
}

/// A commit's short form for notes.
fn short(sha: &str) -> &str {
    &sha[..sha.len().min(10)]
}

/// A table of the rows: per crate (the first two path parts) a count per class, and the totals.
pub fn summary(rows: &[Row]) -> String {
    let mut by: BTreeMap<String, BTreeMap<Class, usize>> = BTreeMap::new();
    for r in rows {
        let mut parts = r.path.components().map(|c| c.as_os_str().to_string_lossy().into_owned());
        let area = match (parts.next(), parts.next()) {
            (Some(a), Some(b)) if a == "crates" => format!("{a}/{b}"),
            (Some(a), _) => a,
            _ => String::new(),
        };
        *by.entry(area).or_default().entry(r.class).or_default() += 1;
    }
    let mut out = format!("| area | {} | total |\n|---|{}---|\n", Class::ALL.map(Class::name).join(" | "), "---|".repeat(Class::ALL.len()));
    let mut totals: BTreeMap<Class, usize> = BTreeMap::new();
    for counts in by.values() {
        for (c, n) in counts {
            *totals.entry(*c).or_insert(0) += n;
        }
    }
    for (area, counts) in by.iter().chain(std::iter::once((&"total".to_string(), &totals))) {
        let cells: Vec<String> = Class::ALL.iter().map(|c| counts.get(c).copied().unwrap_or(0).to_string()).collect();
        let sum: usize = counts.values().sum();
        out.push_str(&format!("| {area} | {} | {sum} |\n", cells.join(" | ")));
    }
    out
}
