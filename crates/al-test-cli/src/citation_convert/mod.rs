//! `--convert-citations PATH`: rewrites old corpus citations (`File.al:line`, `citation_old.rs`) under PATH into citation tokens
//! by name (ADR-065 amendment 2, step 3 of the corpus-detach plan). Each citation is read at the corpus pin of the
//! commit that wrote it (`history.rs`): the file at that pin, the line mapped to what it names there (`place.rs`),
//! then carried to the checked-out pin by object id and the rename map (`citation::carry`) and checked there
//! (`citation::resolve`). What does not map is listed for a hand fix and left as it is; the run is deterministic and
//! can be repeated (a converted line holds no old citation). The module goes once every crate is converted; the old-form
//! parser `citation_old.rs` stays for the check that none is left.
//! Entry points: `convert`, `summary`.

mod history;
mod place;

use crate::citation::{self, Citation, Corpus, Resolved, Status};
use crate::citation_old as old;
use crate::rename_map::RenameMap;
use history::{At, First, Next, Repo};
use old::line_of;
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
    pub fn name(self) -> &'static str {
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

/// The runner commits that rewrote the paths of old citations and nothing else (`tools/rewrite_paths.py`, the corpus
/// restructure of 2026-11-04): a citing line they changed was written before them, at an older pin.
pub const PATH_REWRITES: &[&str] = &[
    "3e173ef2a50dc88abb732f891cf3bdae61a214d5",
    "00aa3c950d0c65764c94eadc8529aa07e658f849",
    "e30c77a65b245e1b887cda70af257a77d3809e7b",
];

/// Converts the old citations of the files under `paths` (relative to `root`; files or folders). `corpus` is the
/// corpus checkout (the pin citations are carried to), `map` its rename map, `rewrites` the commits that only rewrote
/// citation paths ([`PATH_REWRITES`]). Writes the files unless `dry_run`.
pub fn convert(
    root: &Path,
    corpus: &Path,
    map: &RenameMap,
    paths: &[PathBuf],
    rewrites: &[&str],
    dry_run: bool,
) -> Result<Vec<Row>, String> {
    let repo = Repo::open(root, corpus, rewrites)?;
    let new = crate::citation_scan::corpus_at_checkout(corpus)?;
    let files = citing_files(root, paths)?;
    let found: Vec<Found> = files
        .into_par_iter()
        .map(|(path, text)| {
            let cites = old::find(&text, old::Form::of(&path));
            let pins = cite_pins(&repo, &path, &text, &cites)?;
            Ok(Found {
                path,
                text,
                cites,
                pins,
            })
        })
        .collect::<Result<_, String>>()?;
    // Where each cited file is read (at both pins of each citation, and at the next pin that changed it), then those
    // texts in one batch.
    let mut located: BTreeMap<(String, String), Result<At, Class>> = BTreeMap::new();
    for f in &found {
        for (c, p) in f.cites.iter().zip(&f.pins) {
            let Some(cited) = cited_file(c, &f.cites) else { continue };
            for pin in [p.first.as_ref().ok(), Some(&p.blame)].into_iter().flatten() {
                located
                    .entry((pin.clone(), cited.to_string()))
                    .or_insert_with(|| repo.locate(pin, cited));
            }
        }
    }
    let mut wanted: BTreeSet<At> = located.values().filter_map(|r| r.as_ref().ok().cloned()).collect();
    let next: BTreeMap<At, Next> = wanted.iter().map(|a| (a.clone(), repo.next_change(a))).collect();
    wanted.extend(next.values().filter_map(|n| match n {
        Next::Changed(a) => Some(a.clone()),
        _ => None,
    }));
    let wanted: Vec<At> = wanted.into_iter().collect();
    let requests: Vec<String> = wanted.iter().map(|a| format!("{}:{}", a.pin, a.path)).collect();
    let texts = crate::citation_scan::cat_files(corpus, &requests)?;
    let sources: BTreeMap<At, String> = wanted.into_iter().zip(texts).collect();
    let parsed: BTreeMap<&At, File> = sources.iter().map(|(a, t)| (a, File::new(t))).collect();
    let ctx = Ctx {
        new: &new,
        map,
        located: &located,
        next: &next,
        parsed: &parsed,
    };
    let mut rows = Vec::new();
    for f in &found {
        let (out, file_rows) = ctx.file(f);
        if !dry_run && out != f.text {
            std::fs::write(root.join(&f.path), &out).map_err(|e| format!("{}: {e}", f.path.display()))?;
        }
        rows.extend(file_rows);
    }
    Ok(rows)
}

/// One citing file: its old citations and the pins each is read at.
struct Found {
    path: PathBuf,
    text: String,
    cites: Vec<old::OldCite>,
    pins: Vec<Pins>,
}

/// The corpus pins one old citation is read at.
struct Pins {
    /// The pin of the first commit whose added line writes the citation's key, else of the blamed commit, either followed
    /// across a move from another file (`Repo::first_pins`); else `blame`. Why that commit cannot be told.
    first: Result<String, String>,
    /// The pin of the commit that last changed the citing line.
    blame: String,
}

/// The pins of each citation of a file.
fn cite_pins(repo: &Repo, path: &Path, text: &str, cites: &[old::OldCite]) -> Result<Vec<Pins>, String> {
    let lines: Vec<usize> = cites.iter().map(|c| line_of(text, c.at.start)).collect();
    let blamed = repo.blame(path, &lines.iter().copied().collect())?;
    let blamed: Vec<_> = lines.iter().map(|l| blamed[l].clone()).collect();
    let keys: Vec<old::Key> = (0..cites.len()).map(|k| old::Key::of(text, cites, k)).collect();
    let commits: Vec<Option<String>> = blamed.iter().map(|b| b.commit.clone()).collect();
    let first = repo.first_pins(path, &keys, &commits)?;
    Ok(first
        .into_iter()
        .zip(blamed)
        .map(|(f, b)| Pins {
            first: match f {
                First::Pin(p) => Ok(p),
                First::Unwritten => Ok(b.pin.clone()),
                First::Lost(why) => Err(why),
            },
            blame: b.pin,
        })
        .collect())
}

/// The pins besides its first that a citation is read at, each once and none its first: its blame pin, and for a bare
/// continuation (keyed by its whole line, so a prose edit moves its first pin) its parent citation's first and blame pins
/// (review r2 of PR 9454, follow-up).
fn other_pins<'p>(pins: &'p Pins, parent: Option<&'p Pins>) -> Vec<(&'static str, &'p str)> {
    let first = pins.first.as_deref().ok();
    let candidates = [
        ("the blame pin", Some(pins.blame.as_str())),
        (
            "the parent citation's first pin",
            parent.and_then(|p| p.first.as_deref().ok()),
        ),
        ("the parent citation's blame pin", parent.map(|p| p.blame.as_str())),
    ];
    let mut out: Vec<(&'static str, &'p str)> = Vec::new();
    for (what, pin) in candidates {
        if let Some(pin) = pin
            && Some(pin) != first
            && !out.iter().any(|(_, p)| *p == pin)
        {
            out.push((what, pin));
        }
    }
    out
}

/// The file a citation in prose names: its own, or for a bare continuation its parent's; `None` in code or orphaned.
fn cited_file<'c>(c: &old::OldCite, cites: &'c [old::OldCite]) -> Option<&'c str> {
    let f = if c.bare {
        &cites[c.parent?]
    } else {
        cites.iter().find(|x| x.at == c.at)?
    };
    (c.in_comment && f.in_comment).then_some(f.path.as_str())
}

/// An old citation's replacement `new` after the prose `before`, without repeating the prose: `corpus File.al:3` gives
/// the token, not the word and the token, and `header of File.al:1` drops the token's own `the file header of`.
/// Returns the bytes at the end of `before` to drop, and the text.
fn spliced<'n>(before: &str, new: &'n str) -> (usize, &'n str) {
    let lower = before.to_ascii_lowercase();
    let word = format!("{} ", citation::PREFIX);
    match new.strip_prefix("the file header of ") {
        Some(token) if lower.ends_with("header of ") => (0, token),
        _ if lower.ends_with(&word) && new.strip_prefix(citation::PREFIX).is_some_and(|r| r.starts_with(':')) => {
            (word.len(), new)
        }
        _ => (0, new),
    }
}

/// The quoted names of a citing line: AL literals as written (`'cell'`; an apostrophe inside a word opens or closes
/// none) and the text in backticks without its double quotes (`` `"First Test"` `` gives `First Test`).
fn quotes(line: &str) -> Vec<String> {
    let b = line.as_bytes();
    let word = |i: Option<usize>| i.and_then(|i| b.get(i)).is_some_and(u8::is_ascii_alphanumeric);
    let marks: Vec<usize> = (0..b.len())
        .filter(|&i| b[i] == b'\'' && !(word(i.checked_sub(1)) && word(Some(i + 1))))
        .collect();
    let mut out: Vec<String> = Vec::new();
    let mut k = 0;
    while k + 1 < marks.len() {
        let (open, close) = (marks[k], marks[k + 1]);
        if word(open.checked_sub(1)) || word(Some(close + 1)) {
            k += 1;
            continue;
        }
        if close > open + 1 {
            out.push(line[open..=close].to_string());
        }
        k += 2;
    }
    // The text between a pair of backticks: every odd part, when a backtick closes it.
    let parts: Vec<&str> = line.split('`').collect();
    for q in (1..parts.len().saturating_sub(1))
        .step_by(2)
        .map(|k| parts[k].trim_matches('"'))
    {
        if !q.is_empty() {
            out.push(q.to_string());
        }
    }
    out
}

/// Whether the quoted names of a citing line hold cell literal `cell` (`'x'`), as an AL literal or in backticks.
fn quoted(quotes: &[String], cell: &str) -> bool {
    quotes
        .iter()
        .any(|q| q == cell || q.as_str() == cell.trim_matches('\''))
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
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
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

/// What one old citation became: its tokens, class and the file and pin read; or the class and note it is left with.
type Outcome = Result<(String, Class, String), (Class, String)>;

/// What one file's conversion reads.
struct Ctx<'a> {
    new: &'a Corpus,
    map: &'a RenameMap,
    located: &'a BTreeMap<(String, String), Result<At, Class>>,
    /// Per located file: the next pin that changed it.
    next: &'a BTreeMap<At, Next>,
    parsed: &'a BTreeMap<&'a At, File<'a>>,
}

impl Ctx<'_> {
    /// The converted text of one file and a row per old citation. A file citation and its bare continuations convert
    /// together or stay together.
    fn file(&self, f: &Found) -> (String, Vec<Row>) {
        let (text, cites) = (f.text.as_str(), f.cites.as_slice());
        let line = |c: &old::OldCite| line_of(text, c.at.start);
        let results: Vec<Outcome> = cites
            .iter()
            .zip(&f.pins)
            .map(|(c, pins)| match cited_file(c, cites) {
                _ if !c.in_comment => Err((Class::InCode, String::new())),
                _ if c.doubt.is_some() => Err((Class::HandFix, c.doubt.unwrap_or_default().into())),
                None => Err((Class::Orphan, String::new())),
                Some(cited) => {
                    let parent = c.bare.then(|| c.parent.map(|p| &f.pins[p])).flatten();
                    self.checked(&other_pins(pins, parent), pins, cited, c, &quotes(c.line_in(text)))
                }
            })
            .collect();
        // A group (a file citation and its continuations) with a hand fix stays whole.
        let group = |k: usize| if cites[k].bare { cites[k].parent } else { Some(k) };
        let mut group_fails = vec![false; cites.len()];
        for (k, r) in results.iter().enumerate() {
            if let (Some(g), Err((Class::HandFix, _))) = (group(k), r) {
                group_fails[g] = true;
            }
        }
        let (mut out, mut rows, mut last) = (String::with_capacity(text.len()), Vec::new(), 0);
        for (k, c) in cites.iter().enumerate() {
            let held = group(k).is_some_and(|g| group_fails[g]);
            let mut row = Row {
                path: f.path.clone(),
                line: line(c),
                old: text[c.at.clone()].to_string(),
                new: None,
                class: Class::HandFix,
                note: String::new(),
            };
            match &results[k] {
                Ok((new, class, read_at)) if !held => {
                    let (drop, new) = spliced(&text[last..c.at.start], new);
                    out.push_str(&text[last..c.at.start - drop]);
                    out.push_str(new);
                    last = c.at.end;
                    row.new = Some(new.to_string());
                    row.class = *class;
                    row.note = read_at.clone();
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

    /// The located file `cited` at `pin`.
    fn at(&self, pin: &str, cited: &str) -> Result<&At, (Class, String)> {
        match &self.located[&(pin.to_string(), cited.to_string())] {
            Ok(at) => Ok(at),
            Err(Class::HandFix) => Err((
                Class::HandFix,
                format!("pin {} has no one corpus file `{cited}`", short(pin)),
            )),
            Err(class) => Err((*class, String::new())),
        }
    }

    /// The tokens of citation `c` read at its first pin, when the same lines name the same at each of `others` (the blame
    /// pin, review r1 of PR 9432, step-3 note 2; a bare continuation's parent pins, [`other_pins`]) and at the next pin
    /// that changed the file, with the file and pin read; a hand fix otherwise, also when one of those pins cannot be read
    /// (review r1 of PR 9454, M1). A member cited by name is read at the first pin only: a name does not drift with the
    /// lines, and a later rename is what `citation::carry` follows.
    fn checked(
        &self,
        others: &[(&str, &str)],
        pins: &Pins,
        cited: &str,
        c: &old::OldCite,
        quotes: &[String],
    ) -> Outcome {
        let first = pins.first.as_ref().map_err(|why| (Class::HandFix, why.clone()))?;
        let at = self.at(first, cited)?;
        let (token, class) = self.one(at, c, quotes)?;
        if c.member.is_some() {
            return Ok((token, class, format!("read at {} at {}", at.path, short(&at.pin))));
        }
        let mut read: Vec<(&str, &At)> = Vec::new();
        for &(what, pin) in others {
            match self.at(pin, cited) {
                Ok(o) => read.push((what, o)),
                Err((_, why)) => {
                    let why = if why.is_empty() {
                        format!("it has no `{cited}`")
                    } else {
                        why
                    };
                    return Err((Class::HandFix, format!("{what} ({}) cannot be read: {why}", short(pin))));
                }
            }
        }
        match &self.next[at] {
            Next::Same => {}
            Next::Changed(n) => read.push(("the next pin that changed the file", n)),
            Next::Lost(last) => {
                return Err((
                    Class::HandFix,
                    format!("{} cannot be followed after {}", at.path, short(last)),
                ));
            }
        }
        for (what, o) in read {
            let t = match self.one(o, c, quotes) {
                Ok((t, _)) => t,
                Err((_, e)) => format!("nothing ({e})"),
            };
            if t != token {
                return Err((
                    Class::HandFix,
                    format!(
                        "{what} ({}) reads {t}, the first pin ({}) {token}",
                        short(&o.pin),
                        short(&at.pin)
                    ),
                ));
            }
        }
        Ok((token, class, format!("read at {} at {}", at.path, short(&at.pin))))
    }

    /// The tokens for what citation `c` names in the file at `at` (its lines, its member, or the file); a hand fix when
    /// the citing text quotes a name the file has outside the lines.
    fn one(&self, at: &At, c: &old::OldCite, quotes: &[String]) -> Result<(String, Class), (Class, String)> {
        let hand = |e: String| (Class::HandFix, format!("{} at {}: {e}", at.path, short(&at.pin)));
        let file = &self.parsed[at];
        let mut targets = if let Some(name) = &c.member {
            vec![file.named(name).map_err(hand)?]
        } else if c.lines.is_empty() {
            vec![Target::Object(
                file.main_object().ok_or_else(|| hand("no object".into()))?,
            )]
        } else {
            file.targets(&c.lines).map_err(hand)?
        };
        if let Some(q) = file.quoted_elsewhere(&targets, quotes) {
            return Err(hand(format!(
                "the citing text quotes {q}, which the file has outside the cited lines"
            )));
        }
        // A cell only when the citing text quotes its literal: a line off by one names the next cell of the same test.
        for t in &mut targets {
            if let Target::Member {
                cell: cell @ Some(_), ..
            } = t
                && !cell.as_deref().is_some_and(|c| quoted(quotes, c))
            {
                *cell = None;
            }
        }
        let mut tokens: Vec<String> = Vec::new();
        let mut class = Class::Range;
        for t in &targets {
            let (token, c) = self.token(file, t).map_err(hand)?;
            class = c;
            if !tokens.contains(&token) {
                tokens.push(token);
            }
        }
        if c.lines.is_empty() && c.member.is_none() && !c.bare {
            class = Class::File;
        } else if tokens.len() > 1 {
            class = Class::Range;
        }
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
        // A codeunit's tests at the pin: the rename map moves a codeunit citation only when it moves all of them.
        let old_tests: Vec<String> = match &r {
            Resolved::Object { kind, .. } if kind == "codeunit" => {
                obj.members.iter().filter(|m| m.test).map(|m| m.name.clone()).collect()
            }
            _ => Vec::new(),
        };
        let carried = citation::carry(&c, r, &old_tests, self.new, self.map)?.unwrap_or(c);
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
        let mut parts = r
            .path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned());
        let area = match (parts.next(), parts.next()) {
            (Some(a), Some(b)) if a == "crates" => format!("{a}/{b}"),
            (Some(a), _) => a,
            _ => String::new(),
        };
        *by.entry(area).or_default().entry(r.class).or_default() += 1;
    }
    let mut out = format!(
        "| area | {} | total |\n|---|{}---|\n",
        Class::ALL.map(Class::name).join(" | "),
        "---|".repeat(Class::ALL.len())
    );
    let mut totals: BTreeMap<Class, usize> = BTreeMap::new();
    for counts in by.values() {
        for (c, n) in counts {
            *totals.entry(*c).or_insert(0) += n;
        }
    }
    for (area, counts) in by.iter().chain(std::iter::once((&"total".to_string(), &totals))) {
        let cells: Vec<String> = Class::ALL
            .iter()
            .map(|c| counts.get(c).copied().unwrap_or(0).to_string())
            .collect();
        let sum: usize = counts.values().sum();
        out.push_str(&format!("| {area} | {} | {sum} |\n", cells.join(" | ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review r2 of PR 9454 (follow-up): a bare continuation is keyed by its whole line, so a prose edit moves its first
    /// pin; it is also read at its parent citation's pins, and a hand fix when they disagree.
    #[test]
    fn a_bare_continuation_is_read_at_its_parents_pins_too() {
        let pins = |first: &str, blame: &str| Pins {
            first: Ok(first.into()),
            blame: blame.into(),
        };
        let own = pins("b", "c");
        let parent = pins("a", "c");
        assert_eq!(
            other_pins(&own, Some(&parent)),
            [("the blame pin", "c"), ("the parent citation's first pin", "a")]
        );
        // A file citation, or pins that are all the first: nothing else to read.
        assert_eq!(other_pins(&own, None), [("the blame pin", "c")]);
        assert!(other_pins(&pins("a", "a"), Some(&pins("a", "a"))).is_empty());
        // A parent whose first pin cannot be told holds the group anyway; its blame pin is still read.
        let lost = Pins {
            first: Err("x".into()),
            blame: "d".into(),
        };
        assert_eq!(
            other_pins(&pins("a", "a"), Some(&lost)),
            [("the parent citation's blame pin", "d")]
        );
    }

    #[test]
    fn the_prose_in_front_of_a_citation_is_not_repeated() {
        assert_eq!(spliced("rows of corpus ", "corpus:\"X\""), (7, "corpus:\"X\""));
        assert_eq!(
            spliced("(header of ", "the file header of corpus:\"X\""),
            (0, "corpus:\"X\"")
        );
        assert_eq!(
            spliced("(see ", "the file header of corpus:\"X\""),
            (0, "the file header of corpus:\"X\"")
        );
        assert_eq!(spliced("(see ", "corpus:\"X\""), (0, "corpus:\"X\""));
    }

    #[test]
    fn a_citing_line_quotes_cells_and_names() {
        assert_eq!(
            quotes("// T.al:9 checks 'one cell' in `\"First Test\"`, not `Helper"),
            ["'one cell'", "First Test"]
        );
        assert!(quotes("// T.al:9, nothing quoted").is_empty());
        // An apostrophe inside a word opens no literal.
        assert_eq!(
            quotes("// the test's cell 'one' and the row's 'two'."),
            ["'one'", "'two'"]
        );
    }
}
