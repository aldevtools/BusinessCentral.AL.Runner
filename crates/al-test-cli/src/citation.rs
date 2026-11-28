//! Corpus citations by name (ADR-065 amendment 2): the token, the corpus objects it names at one pin, its check and
//! its rewrite at a pin move. The one owner of "what a citation means"; `citation_scan.rs` reads the files.
//!
//! The token is `corpus:` (pinned) or `corpus-pr<N>:` (pending: corpus pull request N, not pinned yet), then an
//! optional object kind and `:` (default `codeunit`), the object name in double quotes, an optional `/` and a member
//! (a `[Test]` procedure, another procedure or a trigger; plain or in double quotes), and an optional `#` and a cell
//! name as the AL string literal of the test (`#'two second empty'`). An object name in angle brackets (`"<Name>"`)
//! is a placeholder and is not checked.
//!
//! A pinned citation resolves when the pinned corpus has exactly one object of the kind and name (AL compares names
//! without case), the object has the member, and the member's body holds the cell's literal. A `[Test]` member of a
//! test codeunit is the case key (codeunit id, test name; ADR-065 amendment 1). A pending citation resolves when an
//! entry of the assumption registry names its corpus pull request. At a pin move (`rewrite`), a citation resolved at
//! the old pin goes to the new pin by the object's id: a test through the rename map (`rename_map.rs`), any other
//! member and an object-level citation by its kind and id. Entry points: `find`, `Corpus`, `resolve`, `rewrite`.

use crate::oracle::CaseId;
use crate::rename_map::RenameMap;
use al_testrun::discover::objects::{self, ObjectDecl};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The first word of every token (written once, so this file's own code holds no token).
pub const PREFIX: &str = "corpus";

/// A citation's status: in the pinned corpus, or in an open corpus pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Pinned,
    Pending(u32),
}

/// One citation token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub status: Status,
    /// Lower case; `codeunit` when the token names no kind.
    pub kind: String,
    pub object: String,
    pub member: Option<String>,
    /// The cell as the AL string literal, with its quotes (`'it''s'`).
    pub cell: Option<String>,
}

impl fmt::Display for Citation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            Status::Pinned => write!(f, "{PREFIX}:")?,
            Status::Pending(n) => write!(f, "{PREFIX}-pr{n}:")?,
        }
        if self.kind != "codeunit" {
            write!(f, "{}:", self.kind)?;
        }
        write!(f, "\"{}\"", self.object)?;
        if let Some(m) = &self.member {
            if identifier(m).is_some_and(|(_, n)| n == m.len()) {
                write!(f, "/{m}")?
            } else {
                write!(f, "/\"{m}\"")?
            }
        }
        if let Some(c) = &self.cell {
            write!(f, "#{c}")?;
        }
        Ok(())
    }
}

/// A citation found in a text: its byte range and the token, or why the token is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub at: std::ops::Range<usize>,
    pub citation: Result<Citation, String>,
}

/// Every citation token in `text`. `corpus:` starts a token only before `"` or `<kind>:"`, `corpus-pr` only before a
/// digit or `:`.
/// A `File.al:line` citation is no token (the old form, still accepted until step 3 of the corpus-detach plan).
pub fn find(text: &str) -> Vec<Found> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(off) = text[from..].find(PREFIX) {
        let at = from + off;
        from = at + PREFIX.len();
        if at > 0 && (b[at - 1].is_ascii_alphanumeric() || matches!(b[at - 1], b'_' | b'-' | b'.' | b'/')) {
            continue;
        }
        let rest = &text[from..];
        let (status, after) = if let Some(r) = rest
            .strip_prefix("-pr")
            .filter(|r| r.starts_with(|c: char| c.is_ascii_digit() || c == ':'))
        {
            let n = r.bytes().take_while(u8::is_ascii_digit).count();
            match (r[..n].parse::<u32>(), r[n..].strip_prefix(':')) {
                (Ok(pr), Some(after)) => (Status::Pending(pr), after),
                _ => {
                    out.push(Found {
                        at: at..from + 3 + n,
                        citation: Err("`corpus-pr` needs a pull request number and `:`".into()),
                    });
                    continue;
                }
            }
        } else if let Some(r) = rest.strip_prefix(':') {
            let w = r.bytes().take_while(u8::is_ascii_lowercase).count();
            if !(r.starts_with('"') || (w > 0 && r[w..].starts_with(":\""))) {
                continue;
            }
            (Status::Pinned, r)
        } else {
            continue;
        };
        let start = text.len() - after.len();
        let (len, citation) = body(after, status);
        from = start + len;
        match citation {
            Ok(None) => {}
            Ok(Some(c)) => out.push(Found {
                at: at..from,
                citation: Ok(c),
            }),
            Err(e) => out.push(Found {
                at: at..from,
                citation: Err(e),
            }),
        }
    }
    out
}

/// The token after its `corpus:` or `corpus-pr<N>:`: the bytes it takes and the citation (`None` for a placeholder).
fn body(s: &str, status: Status) -> (usize, Result<Option<Citation>, String>) {
    let mut i = 0;
    let mut kind = "codeunit".to_string();
    let w = s.bytes().take_while(u8::is_ascii_lowercase).count();
    if w > 0 && s[w..].starts_with(':') {
        kind = s[..w].to_string();
        if !objects::is_object_kind(&kind) {
            return (w + 1, Err(format!("`{kind}` is not an object kind")));
        }
        i = w + 1;
    }
    let Some((object, n)) = quoted(&s[i..]) else {
        return (i, Err("the object name needs double quotes on one line".into()));
    };
    i += n;
    if object.starts_with('<') && object.ends_with('>') {
        return (i, Ok(None));
    }
    let mut member = None;
    if s[i..].starts_with('/') {
        let Some((m, n)) = quoted(&s[i + 1..]).or_else(|| identifier(&s[i + 1..])) else {
            return (i + 1, Err("`/` needs a member name".into()));
        };
        member = Some(m.to_string());
        i += 1 + n;
    }
    let mut cell = None;
    if s[i..].starts_with('#') {
        let Some(n) = objects::literal_len(&s[i + 1..]) else {
            return (i + 1, Err("`#` needs the cell as an AL string literal ('...')".into()));
        };
        cell = Some(s[i + 1..i + 1 + n].to_string());
        i += 1 + n;
    }
    let object = object.to_string();
    (
        i,
        Ok(Some(Citation {
            status,
            kind,
            object,
            member,
            cell,
        })),
    )
}

/// A name in double quotes at the start of `s`, on one line and not empty: the name and the bytes with the quotes.
fn quoted(s: &str) -> Option<(&str, usize)> {
    let r = s.strip_prefix('"')?;
    let end = r.find(['"', '\n'])?;
    (r.as_bytes()[end] == b'"' && end > 0).then(|| (&r[..end], end + 2))
}

/// A plain AL identifier at the start of `s`.
fn identifier(s: &str) -> Option<(&str, usize)> {
    let n = s
        .bytes()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == b'_')
        .count();
    (n > 0 && !s.as_bytes()[0].is_ascii_digit()).then(|| (&s[..n], n))
}

/// One object of the corpus at a pin.
#[derive(Debug, Clone)]
struct Object {
    app: String,
    /// Index into `Corpus::texts`.
    file: usize,
    decl: ObjectDecl,
}

/// The objects of the corpus at one pin, from the source files of every app.
#[derive(Debug, Default)]
pub struct Corpus {
    texts: Vec<String>,
    objects: Vec<Object>,
    by_name: BTreeMap<(String, String), Vec<usize>>,
    by_id: BTreeMap<(String, u32), usize>,
}

/// What a resolved citation names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// A `[Test]` procedure of a test codeunit: the case key.
    Case(CaseId),
    /// An object, or a member that is not a test.
    Object { kind: String, id: Option<u32> },
    /// A pending citation whose corpus pull request an assumption entry names.
    Pending,
}

impl Corpus {
    /// From `(app folder, file text)` of every source file of the pin. One codeunit id in two apps is an error (a case
    /// key is the codeunit id and the test name; ADR-065 amendment 1).
    pub fn from_sources(files: impl IntoIterator<Item = (String, String)>) -> Result<Corpus, String> {
        let files: Vec<(String, String)> = files.into_iter().collect();
        // Scanning is the bulk of the work and the files are independent.
        let decls: Vec<Vec<ObjectDecl>> = files.par_iter().map(|(_, text)| objects::objects(text)).collect();
        let mut c = Corpus::default();
        for ((app, text), decls) in files.into_iter().zip(decls) {
            let file = c.texts.len();
            for decl in decls {
                let at = c.objects.len();
                if let Some(id) = decl.id {
                    match c.by_id.get(&(decl.kind.clone(), id)) {
                        Some(&had) if decl.kind == "codeunit" && c.objects[had].app != app => {
                            return Err(format!(
                                "codeunit {id} is in two corpus apps, {} and {app}; a case key is the codeunit id and \
                                 the test name, so codeunit ids must be unique across the corpus apps",
                                c.objects[had].app
                            ));
                        }
                        Some(_) => {}
                        None => {
                            c.by_id.insert((decl.kind.clone(), id), at);
                        }
                    }
                }
                c.by_name
                    .entry((decl.kind.clone(), decl.name.to_lowercase()))
                    .or_default()
                    .push(at);
                c.objects.push(Object {
                    app: app.clone(),
                    file,
                    decl,
                });
            }
            c.texts.push(text);
        }
        Ok(c)
    }

    /// The case `key` is a `[Test]` procedure of a test codeunit at this pin.
    pub fn has_case(&self, key: &CaseId) -> bool {
        self.by_id
            .get(&("codeunit".to_string(), key.codeunit))
            .is_some_and(|&o| {
                let d = &self.objects[o].decl;
                d.test_codeunit
                    && d.members
                        .iter()
                        .any(|m| m.test && m.name.eq_ignore_ascii_case(&key.name))
            })
    }

    /// The `[Test]` procedures of codeunit `id` at this pin.
    fn tests_of(&self, id: u32) -> Vec<String> {
        self.by_id
            .get(&("codeunit".to_string(), id))
            .map(|&o| {
                let d = &self.objects[o].decl;
                d.members.iter().filter(|m| m.test).map(|m| m.name.clone()).collect()
            })
            .unwrap_or_default()
    }

    /// The name of the object of `kind` and `id` at this pin.
    fn name_of(&self, kind: &str, id: u32) -> Result<String, String> {
        self.by_id
            .get(&(kind.to_string(), id))
            .map(|&o| self.objects[o].decl.name.clone())
            .ok_or_else(|| format!("no {kind} {id} at the new pin"))
    }
}

/// Whether `c` resolves at `corpus`; `pending_prs` are the corpus pull requests the assumption registry names.
pub fn resolve(c: &Citation, corpus: &Corpus, pending_prs: &BTreeSet<u32>) -> Result<Resolved, String> {
    if let Status::Pending(n) = c.status {
        return match pending_prs.contains(&n) {
            true => Ok(Resolved::Pending),
            false => Err(format!(
                "no assumption entry names corpus PR {n}; a pending citation cites the open probe of an entry"
            )),
        };
    }
    let (o, members) = lookup(c, corpus)?;
    match (o.decl.test_codeunit, o.decl.id, members.iter().find(|x| x.test)) {
        (true, Some(codeunit), Some(t)) => Ok(Resolved::Case(CaseId {
            codeunit,
            name: t.name.clone(),
        })),
        _ => Ok(Resolved::Object {
            kind: c.kind.clone(),
            id: o.decl.id,
        }),
    }
}

/// `c` as it must be written: its object and member in the corpus's spelling, printed by `Display` (no `codeunit:`,
/// a plain member unquoted), so a search for the printed token finds every citation of it. `None` for a pending
/// citation; an error when it does not resolve.
pub fn canonical(c: &Citation, corpus: &Corpus) -> Result<Option<Citation>, String> {
    if c.status != Status::Pinned {
        return Ok(None);
    }
    let (o, members) = lookup(c, corpus)?;
    Ok(Some(Citation {
        object: o.decl.name.clone(),
        member: members.first().map(|m| m.name.clone()),
        ..c.clone()
    }))
}

/// The one object of `c`'s kind and name at `corpus`, and its members of `c`'s member name (none for an object-level
/// citation), checked: a cell needs a member whose observations name it (`objects::cells`).
fn lookup<'a>(c: &Citation, corpus: &'a Corpus) -> Result<(&'a Object, Vec<&'a objects::MemberDecl>), String> {
    let (kind, object) = (&c.kind, &c.object);
    let held = corpus.by_name.get(&(kind.clone(), object.to_lowercase()));
    let o = match held.map(Vec::as_slice) {
        Some([o]) => &corpus.objects[*o],
        Some(many) => return Err(format!("{} objects of kind {kind} are named \"{object}\"", many.len())),
        None => return Err(format!("no {kind} is named \"{object}\" in the corpus")),
    };
    let Some(m) = &c.member else {
        if c.cell.is_some() {
            return Err("a cell needs a member: name the test that holds it".into());
        }
        return Ok((o, Vec::new()));
    };
    let members: Vec<_> = o
        .decl
        .members
        .iter()
        .filter(|x| x.name.eq_ignore_ascii_case(m))
        .collect();
    if members.is_empty() {
        return Err(format!("{kind} \"{object}\" has no procedure or trigger {m}"));
    }
    if let Some(cell) = &c.cell {
        let text = &corpus.texts[o.file];
        if !members
            .iter()
            .any(|x| objects::cells(text, x.span.clone()).contains(&cell.as_str()))
        {
            return Err(format!("{m} of {kind} \"{object}\" has no cell {cell}"));
        }
    }
    Ok((o, members))
}

/// `text` with every pinned citation that resolves at `old` rewritten to the same object, member and cell at `new`:
/// a test through `map`, anything else by its kind and id. A citation that does not resolve at `old`, a split test and
/// a removed one stay as they are and get a note (the check then names them).
pub fn rewrite(text: &str, old: &Corpus, new: &Corpus, map: &RenameMap) -> (String, Vec<String>) {
    let (mut out, mut notes, mut last) = (String::with_capacity(text.len()), Vec::new(), 0);
    for f in find(text) {
        let Ok(c) = &f.citation else { continue };
        let to = match moved(c, old, new, map) {
            Ok(Some(to)) => to,
            Ok(None) => continue,
            Err(e) => {
                notes.push(format!("{c}: {e}"));
                continue;
            }
        };
        out.push_str(&text[last..f.at.start]);
        out.push_str(&to.to_string());
        last = f.at.end;
    }
    out.push_str(&text[last..]);
    (out, notes)
}

/// `c` at the new pin; `None` when it stays as it is (pending, a kind without an id, nothing moved).
fn moved(c: &Citation, old: &Corpus, new: &Corpus, map: &RenameMap) -> Result<Option<Citation>, String> {
    if c.status != Status::Pinned {
        return Ok(None);
    }
    let r = resolve(c, old, &BTreeSet::new()).map_err(|e| format!("does not resolve at the old pin: {e}"))?;
    let tests = match &r {
        Resolved::Object { kind, id: Some(id) } if kind == "codeunit" => old.tests_of(*id),
        _ => Vec::new(),
    };
    carry(c, r, &tests, new, map)
}

/// `c`, which names `r` at an old pin, at the pin of `new`: a test through `map`, a codeunit (or its member) by its
/// id unless the map moves its tests (`old_tests`, its tests at the old pin) to another codeunit, any other object by
/// its kind and id. `None` when it stays as it is (pending, a kind without an id, nothing moved).
pub fn carry(
    c: &Citation,
    r: Resolved,
    old_tests: &[String],
    new: &Corpus,
    map: &RenameMap,
) -> Result<Option<Citation>, String> {
    let mut to = c.clone();
    match r {
        Resolved::Case(key) => {
            let keys: Vec<CaseId> = map.translate(&key, &|k| new.has_case(k))?.keys.into_iter().collect();
            match keys.as_slice() {
                [k] => {
                    to.object = new.name_of("codeunit", k.codeunit)?;
                    if *k != key {
                        to.member = Some(k.name.clone());
                    }
                }
                [] => return Err("the test was removed at the new pin; cite another test or drop the citation".into()),
                many => {
                    let names: Vec<String> = many.iter().map(|k| format!("{}/{}", k.codeunit, k.name)).collect();
                    return Err(format!(
                        "the test was split into {}; cite the new tests by hand",
                        names.join(", ")
                    ));
                }
            }
        }
        Resolved::Object { kind, id: Some(id) } if kind == "codeunit" => {
            to.object = new.name_of("codeunit", follow_codeunit(id, old_tests, new, map)?)?
        }
        Resolved::Object { kind, id: Some(id) } => to.object = new.name_of(&kind, id)?,
        Resolved::Object { id: None, .. } | Resolved::Pending => return Ok(None),
    }
    Ok((to != *c).then_some(to))
}

/// The id of codeunit `id` at the new pin. When the map moves none of its tests to another codeunit, or some of its
/// tests (`tests`, at the old pin) are still at `id`, the same id. Otherwise the one codeunit its moved tests go to;
/// an error when they go to several or nowhere. A renumbered codeunit whose old id another codeunit takes is so
/// followed by the map, not by the id (review r1 of PR 9432, M4).
fn follow_codeunit(id: u32, tests: &[String], new: &Corpus, map: &RenameMap) -> Result<u32, String> {
    if !map.moves_codeunit(id) {
        return Ok(id);
    }
    let mut to: BTreeSet<u32> = BTreeSet::new();
    for t in tests {
        let key = CaseId {
            codeunit: id,
            name: t.clone(),
        };
        to.extend(
            map.translate(&key, &|k| new.has_case(k))?
                .keys
                .iter()
                .map(|k| k.codeunit),
        );
    }
    if to.contains(&id) {
        return Ok(id);
    }
    match to.into_iter().collect::<Vec<_>>().as_slice() {
        [one] => Ok(*one),
        _ => Err(format!(
            "the rename map moves tests of codeunit {id} to other codeunits, not all to one; cite it by hand"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A citation text: the token prefix is built, so the repository check does not read these test inputs.
    fn cite(rest: &str) -> String {
        format!("{PREFIX}{rest}")
    }
    fn src(id: u32, name: &str, tests: &[&str]) -> String {
        let procs: String = tests
            .iter()
            .map(|t| format!("    [Test]\n    procedure {t}()\n    begin\n        Observer.Expect('cell of {t}', 'x');\n    end;\n"))
            .collect();
        format!(
            "codeunit {id} \"{name}\"\n{{\n    Subtype = Test;\n{procs}    local procedure Helper()\n    begin\n    end;\n}}\n"
        )
    }
    fn corpus(files: &[(&str, String)]) -> Corpus {
        Corpus::from_sources(files.iter().map(|(a, t)| (a.to_string(), t.clone()))).unwrap()
    }
    fn one(text: &str) -> Citation {
        let f = find(text);
        assert_eq!(f.len(), 1, "{f:?}");
        f[0].citation.clone().unwrap()
    }

    #[test]
    fn the_token_forms_parse_and_print_back() {
        for t in [
            "\"LT Cites\"/First#'it''s'",
            "\"LT Cites\"/\"Space Test\"",
            "\"LT Cites\"",
            "table:\"LT Table\"/OnValidate",
        ] {
            let text = cite(&format!(":{t}"));
            assert_eq!(one(&format!("see ({text}).")).to_string(), text);
        }
        let p = one(&cite("-pr1650:\"LT New\"/A"));
        assert_eq!(p.status, Status::Pending(1650));
        assert_eq!((p.kind.as_str(), p.member.as_deref()), ("codeunit", Some("A")));
    }

    #[test]
    fn the_old_form_and_other_corpus_words_are_no_token() {
        assert!(find("pinned (record/X/TFoo.al:12-14), corpus: 42 tests, `corpus/tests`").is_empty());
        assert!(find(&cite(":\"<Codeunit Name>\"/<Test>")).is_empty());
    }

    #[test]
    fn a_malformed_token_is_an_error() {
        for bad in [":table:\"X", ":nokind:\"X\"", "-pr:\"X\"", ":\"X\"/A#cell"] {
            let f = find(&cite(bad));
            assert!(f.len() == 1 && f[0].citation.is_err(), "{bad}: {f:?}");
        }
    }

    #[test]
    fn a_pinned_citation_resolves_to_its_case_its_member_or_its_object() {
        let c = corpus(&[("app", src(50100, "LT Cites", &["First"]))]);
        let none = BTreeSet::new();
        let r = |t: &str| resolve(&one(&cite(t)), &c, &none);
        let key = CaseId {
            codeunit: 50100,
            name: "First".into(),
        };
        assert_eq!(r(":\"lt cites\"/first#'cell of First'"), Ok(Resolved::Case(key)));
        let obj = Resolved::Object {
            kind: "codeunit".into(),
            id: Some(50100),
        };
        assert_eq!(r(":\"LT Cites\"/Helper"), Ok(obj.clone()));
        assert_eq!(r(":\"LT Cites\""), Ok(obj));
    }

    #[test]
    fn a_citation_to_a_missing_test_object_or_cell_fails() {
        let c = corpus(&[("app", src(50100, "LT Cites", &["First"]))]);
        let none = BTreeSet::new();
        let r = |t: &str| resolve(&one(&cite(t)), &c, &none).unwrap_err();
        assert!(
            r(":\"LT Cites\"/Gone").contains("has no procedure or trigger Gone"),
            "{}",
            r(":\"LT Cites\"/Gone")
        );
        assert!(r(":\"LT Missing\"/First").contains("no codeunit is named \"LT Missing\""));
        assert!(r(":\"LT Cites\"/First#'other'").contains("has no cell 'other'"));
        assert!(r(":\"LT Cites\"#'cell of First'").contains("a cell needs a member"));
    }

    #[test]
    fn a_pending_citation_needs_a_pull_request_the_registry_names() {
        let c = Corpus::default();
        let open: BTreeSet<u32> = [1650].into();
        assert_eq!(
            resolve(&one(&cite("-pr1650:\"LT New\"/A")), &c, &open),
            Ok(Resolved::Pending)
        );
        let e = resolve(&one(&cite("-pr1651:\"LT New\"/A")), &c, &open).unwrap_err();
        assert!(e.contains("no assumption entry names corpus PR 1651"), "{e}");
    }

    #[test]
    fn one_codeunit_id_in_two_apps_is_an_error() {
        let s = src(50100, "LT Cites", &[]);
        let e = Corpus::from_sources([("a".to_string(), s.clone()), ("b".to_string(), s)]).unwrap_err();
        assert!(e.contains("codeunit 50100 is in two corpus apps, a and b"), "{e}");
    }

    #[test]
    fn a_rename_keeps_the_id_and_a_renumber_follows_the_map() {
        let old = corpus(&[("app", src(50100, "LT Old", &["A", "B"]))]);
        // At the new pin 50100 is renamed and its test B moved to 50200 by a map line.
        let new = corpus(&[
            ("app", src(50100, "LT New", &["A"])),
            ("app", src(50200, "LT Moved", &["B2"])),
        ]);
        let map = RenameMap::parse("old_key\tnew_key\napp/50100/B\tapp/50200/B2\n").unwrap();
        let text = format!(
            "a {} b {} c {}",
            cite(":\"LT Old\"/A#'cell of A'"),
            cite(":\"LT Old\"/B"),
            cite(":\"LT Old\"")
        );
        let (out, notes) = rewrite(&text, &old, &new, &map);
        assert_eq!(
            out,
            format!(
                "a {} b {} c {}",
                cite(":\"LT New\"/A#'cell of A'"),
                cite(":\"LT Moved\"/B2"),
                cite(":\"LT New\"")
            )
        );
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn a_split_or_removed_test_is_left_with_a_note() {
        let old = corpus(&[("app", src(50100, "LT Old", &["A", "B"]))]);
        let new = corpus(&[("app", src(50100, "LT Old", &["A1", "A2"]))]);
        let map = RenameMap::parse("old_key\tnew_key\napp/50100/A\tapp/50100/A1\napp/50100/A\tapp/50100/A2\n").unwrap();
        let text = format!("{} {}", cite(":\"LT Old\"/A"), cite(":\"LT Old\"/B"));
        let (out, notes) = rewrite(&text, &old, &new, &map);
        assert_eq!(out, text);
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(notes[0].contains("split") && notes[1].contains("removed"), "{notes:?}");
    }

    // Review r1 of PR 9432, M4: a renumbered codeunit whose old id another codeunit takes is followed by the map.
    #[test]
    fn an_object_citation_follows_a_renumber_not_a_reused_id() {
        let old = corpus(&[("app", src(50100, "LT Old", &["A", "B"]))]);
        let new = corpus(&[
            ("app", src(50100, "LT Unrelated", &["Z"])),
            ("app", src(50200, "LT Old", &["A", "B"])),
        ]);
        let map = RenameMap::parse("old_key\tnew_key\napp/50100/A\tapp/50200/A\napp/50100/B\tapp/50200/B\n").unwrap();
        let text = format!("{} {}", cite(":\"LT Old\""), cite(":\"LT Old\"/Helper"));
        let (out, notes) = rewrite(&text, &old, &new, &map);
        assert_eq!(out, text, "{notes:?}");
        assert!(notes.is_empty(), "{notes:?}");
        // Its tests split over two codeunits: no codeunit to follow, a note.
        let split = corpus(&[
            ("app", src(50100, "LT Unrelated", &["Z"])),
            ("app", src(50200, "LT Old", &["A"])),
            ("app", src(50300, "LT Other", &["B"])),
        ]);
        let map = RenameMap::parse("old_key\tnew_key\napp/50100/A\tapp/50200/A\napp/50100/B\tapp/50300/B\n").unwrap();
        let (out, notes) = rewrite(&cite(":\"LT Old\""), &old, &split, &map);
        assert_eq!(out, cite(":\"LT Old\""));
        assert!(
            notes.len() == 1 && notes[0].contains("moves tests of codeunit 50100"),
            "{notes:?}"
        );
    }

    // Review r1 of PR 9432, M2 and L3: a field's trigger is no member; an expected value is no cell.
    #[test]
    fn a_field_trigger_or_an_expected_value_does_not_resolve() {
        let table =
            "table 50101 \"LT T\"\n{\n    fields { field(1; A; Integer) { trigger OnValidate() begin end; } }\n}\n";
        let c = corpus(&[("app", src(50100, "LT Cites", &["First"])), ("app", table.to_string())]);
        let none = BTreeSet::new();
        assert!(resolve(&one(&cite(":table:\"LT T\"/OnValidate")), &c, &none).is_err());
        assert!(resolve(&one(&cite(":\"LT Cites\"/First#'x'")), &c, &none).is_err());
    }

    // Review r1 of PR 9432, L2: a token is written as it prints, with the corpus spelling of its names.
    #[test]
    fn a_citation_not_in_its_canonical_form_is_named() {
        let c = corpus(&[("app", src(50100, "LT Cites", &["First"]))]);
        for bad in [
            ":\"lt cites\"/First",
            ":codeunit:\"LT Cites\"",
            ":\"LT Cites\"/\"First\"",
            ":\"LT Cites\"/first",
        ] {
            let k = canonical(&one(&cite(bad)), &c).unwrap().unwrap();
            assert_ne!(k.to_string(), cite(bad), "{bad}");
        }
        let good = cite(":\"LT Cites\"/First#'cell of First'");
        assert_eq!(canonical(&one(&good), &c).unwrap().unwrap().to_string(), good);
    }
}
