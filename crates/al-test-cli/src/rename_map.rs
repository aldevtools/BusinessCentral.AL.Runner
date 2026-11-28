//! The corpus rename map (`corpus/tests/rename-map.tsv`) and what a corpus pin move does to the cases of one BC major
//! (ADR-065 amendment 1, owner decision 2026-11-28). The one owner of "translate case keys through the rename map".
//!
//! A case key is the codeunit id and the test name (`CaseId`). Corpus codeunit ids are unique across the corpus apps, so
//! the app folder is not part of the key: the app part of a map key is ignored, an app move needs no map line, and two
//! apps that hold the same codeunit id are an error (`app_of`).
//!
//! A line is read by its first two columns and only for a base key that is gone, as the corpus's facts check reads it
//! (`.github/scripts/facts.py`). Unlike facts.py, the runner follows chains and calls a key gone when no head oracle of
//! the major has it (facts.py: when the corpus sources lack it). A base key that a head oracle still has keeps its key;
//! a gone key goes to its new keys, following a chain through new keys that are gone too (a bump can span several
//! corpus pull requests). A split sends one key to many, a merge many keys to one. A gone key without a line is a
//! removed case (ADR-065 section 3); a gone key whose only line is a self line is a dead target, as is a new key no
//! head oracle has; a must-not-compile key (`mnc/<stem>`) is in no list and is skipped.
//!
//! `carry` gives, per list of the major, the base cases in head keys: the moved list lines, and the new keys whose old key
//! passed on the old pin, which must pass on the new one. Pure; `base_oracle.rs` reads the files. Entry points:
//! `RenameMap`, `carry`, `bump_lines`, `rewrite_list`. A citation by name follows the map through `citation.rs`.

use crate::oracle::CaseId;
use std::collections::{BTreeMap, BTreeSet};

/// The lines of the rename map, old key to its new keys, apps dropped.
#[derive(Debug, Default, Clone)]
pub struct RenameMap {
    next: BTreeMap<CaseId, BTreeSet<CaseId>>,
}

/// Where a base key went at head.
#[derive(Debug, PartialEq, Eq)]
pub struct Translation {
    /// The head keys: the key itself when a head oracle has it, its new keys when the map moved it, none when it was
    /// removed without a line.
    pub keys: BTreeSet<CaseId>,
    /// New keys the map names that no head oracle has and the map does not move on (a bad map line), and a gone key
    /// whose only line is a self line.
    pub dead: BTreeSet<CaseId>,
}

/// `<app folder>/<codeunit id>/<test name>` as a case key; `None` for a must-not-compile key `mnc/<stem>`.
fn key(text: &str) -> Result<Option<CaseId>, String> {
    if text.starts_with("mnc/") {
        return Ok(None);
    }
    let mut parts = text.splitn(3, '/');
    let (Some(_app), Some(id), Some(name)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(format!("`{text}` is not <app folder>/<codeunit id>/<test name>"));
    };
    let codeunit = id
        .parse()
        .map_err(|_| format!("`{text}`: `{id}` is not a codeunit id"))?;
    if name.is_empty() {
        return Err(format!("`{text}` has no test name"));
    }
    Ok(Some(CaseId {
        codeunit,
        name: name.to_string(),
    }))
}

impl RenameMap {
    /// The map file's text: `old_key<TAB>new_key` lines (further columns ignored), `#` comments and the header line
    /// `old_key<TAB>new_key` skipped.
    pub fn parse(text: &str) -> Result<RenameMap, String> {
        let mut map = RenameMap::default();
        for (n, line) in text.lines().enumerate() {
            let at = |e: String| format!("tests/rename-map.tsv line {}: {e}", n + 1);
            if line.trim().is_empty() || line.starts_with('#') || line.starts_with("old_key\tnew_key") {
                continue;
            }
            // The first two columns, as facts.py `load_map` reads them; further columns are not part of a key.
            let mut cols = line.split('\t');
            let (Some(old), Some(new)) = (cols.next(), cols.next()) else {
                return Err(at(format!("not old_key<TAB>new_key: `{line}`")));
            };
            match (key(old).map_err(at)?, key(new).map_err(at)?) {
                (Some(old), Some(new)) => {
                    map.next.entry(old).or_default().insert(new);
                }
                (None, None) => {}
                _ => {
                    return Err(at(format!(
                        "a test key and a must-not-compile key on one line: `{line}`"
                    )));
                }
            }
        }
        Ok(map)
    }

    /// Where `key` went: itself when `at_head` has it, else its new keys through the map. A line that moves a key to
    /// itself (an app move written as a line) is followed nowhere: a gone key whose only line is such a line is dead.
    pub fn translate(&self, key: &CaseId, at_head: &dyn Fn(&CaseId) -> bool) -> Result<Translation, String> {
        let mut t = Translation {
            keys: BTreeSet::new(),
            dead: BTreeSet::new(),
        };
        self.follow(key, at_head, &mut Vec::new(), &mut t)?;
        Ok(t)
    }

    /// Whether a line moves a test of codeunit `id` to another codeunit (a renumber, or a move of some of its tests).
    pub fn moves_codeunit(&self, id: u32) -> bool {
        self.next
            .iter()
            .any(|(old, new)| old.codeunit == id && new.iter().any(|n| n.codeunit != id))
    }

    fn follow(
        &self,
        key: &CaseId,
        at_head: &dyn Fn(&CaseId) -> bool,
        path: &mut Vec<CaseId>,
        t: &mut Translation,
    ) -> Result<(), String> {
        if at_head(key) {
            t.keys.insert(key.clone());
            return Ok(());
        }
        let next: Vec<&CaseId> = self.next.get(key).into_iter().flatten().filter(|n| *n != key).collect();
        if next.is_empty() {
            // Reached through the map, or named by a self line only: a key the map sends nowhere. A base key without
            // a line is a removed case.
            if !path.is_empty() || self.next.contains_key(key) {
                t.dead.insert(key.clone());
            }
            return Ok(());
        }
        if path.contains(key) {
            return Err(format!(
                "tests/rename-map.tsv has a cycle through {} {}",
                key.codeunit, key.name
            ));
        }
        path.push(key.clone());
        for n in next {
            self.follow(n, at_head, path, t)?;
        }
        path.pop();
        Ok(())
    }
}

/// One not-passing list of a BC major at the base revision (the old pin).
pub struct BaseList {
    /// The suite's name; `None` for the main app.
    pub suite: Option<String>,
    /// The app folder.
    pub app: String,
    /// The cases of its oracle that real BC passes.
    pub oracle: BTreeSet<CaseId>,
    /// The list's lines.
    pub list: BTreeSet<CaseId>,
}

/// One oracle of a BC major at head (the new pin).
pub struct HeadOracle {
    pub suite: Option<String>,
    pub app: String,
    /// Every case of the oracle, real BC's failures too.
    pub cases: BTreeSet<CaseId>,
}

/// A base case that reaches a head list under another key or from another app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renamed {
    pub old: CaseId,
    /// The base list it came from.
    pub old_suite: Option<String>,
    pub new: CaseId,
    /// It passed on the old pin: real BC passed it and its list did not hold it.
    pub passed: bool,
}

/// The base cases of one head list, in head keys.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Carried {
    /// Every head key a base case reached: no new case.
    pub known: BTreeSet<CaseId>,
    /// The head keys whose base cases were all listed: the list's lines in head keys. A head key one of whose base
    /// cases passed is not here: it must pass.
    pub listed: BTreeSet<CaseId>,
    /// The base cases that reached this list under another key or from another app.
    pub renamed: Vec<Renamed>,
}

/// A base case that passed and that the map sends to a key no head oracle of the major has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lost {
    pub old: CaseId,
    pub old_suite: Option<String>,
    pub new: CaseId,
}

/// The base cases of a BC major in head keys, per head list (`None` key: the main app).
#[derive(Debug, Default)]
pub struct Carry {
    pub lists: BTreeMap<Option<String>, Carried>,
    pub lost: Vec<Lost>,
    map: RenameMap,
    /// Every case of the major's head oracles.
    head: BTreeSet<CaseId>,
}

impl Carry {
    pub fn of(&self, suite: Option<&str>) -> Option<&Carried> {
        self.lists.get(&suite.map(str::to_string))
    }

    /// `rewrite_list` with this pin move's map and head cases.
    pub fn rewrite_list(&self, text: &str) -> Result<Option<String>, String> {
        rewrite_list(text, &self.map, &|k| self.head.contains(k))
    }

    /// `translate_set` with this pin move's map and head cases.
    pub fn translate_set(&self, set: &BTreeSet<CaseId>) -> Result<BTreeSet<CaseId>, String> {
        translate_set(set, &self.map, &|k| self.head.contains(k))
    }
}

/// The app of each codeunit id over `oracles` (suite, app, cases). Two apps with one id: an error, because the case key
/// has no app part.
pub fn app_of<'a>(
    side: &str,
    oracles: impl Iterator<Item = (&'a str, &'a BTreeSet<CaseId>)>,
) -> Result<BTreeMap<u32, &'a str>, String> {
    let mut apps: BTreeMap<u32, &str> = BTreeMap::new();
    for (app, cases) in oracles {
        for c in cases {
            let had = *apps.entry(c.codeunit).or_insert(app);
            if had != app {
                return Err(format!(
                    "{side}: codeunit {} is in two corpus apps, {had} and {app}; a case key is the codeunit id and \
                     the test name, so codeunit ids must be unique across the corpus apps",
                    c.codeunit
                ));
            }
        }
    }
    Ok(apps)
}

/// One base case that reached a head key: (old key, old suite, passed).
type Source = (CaseId, Option<String>, bool);
/// The sources per head list (suite) and head key.
type Sources = BTreeMap<Option<String>, BTreeMap<CaseId, Vec<Source>>>;

/// The base lists of a major carried to its head oracles through `map`. A base case reaches a head list whose oracle
/// has its head key and that is the same list (same suite) or another app's (an app move). Errors: a codeunit id in two
/// apps, a cycle in the map.
pub fn carry(map: &RenameMap, base: &[BaseList], head: &[HeadOracle]) -> Result<Carry, String> {
    app_of("the base oracle", base.iter().map(|b| (b.app.as_str(), &b.oracle)))?;
    app_of("the head oracle", head.iter().map(|h| (h.app.as_str(), &h.cases)))?;
    let at_head = |k: &CaseId| head.iter().any(|h| h.cases.contains(k));
    // Per head list and head key: (old key, old suite, passed) of each base case that reached it.
    let mut sources: Sources = BTreeMap::new();
    let mut lost = Vec::new();
    for b in base {
        for old in &b.oracle {
            let passed = !b.list.contains(old);
            let t = map.translate(old, &at_head)?;
            if passed {
                lost.extend(t.dead.iter().map(|new| Lost {
                    old: old.clone(),
                    old_suite: b.suite.clone(),
                    new: new.clone(),
                }));
            }
            for new in &t.keys {
                for h in head
                    .iter()
                    .filter(|h| h.cases.contains(new) && (h.suite == b.suite || h.app != b.app))
                {
                    sources
                        .entry(h.suite.clone())
                        .or_default()
                        .entry(new.clone())
                        .or_default()
                        .push((old.clone(), b.suite.clone(), passed));
                }
            }
        }
    }
    let mut c = Carry {
        lost,
        map: map.clone(),
        head: head.iter().flat_map(|h| h.cases.iter().cloned()).collect(),
        ..Carry::default()
    };
    for h in head {
        let mut carried = Carried::default();
        for (new, from) in sources.remove(&h.suite).unwrap_or_default() {
            if from.iter().all(|(_, _, passed)| !passed) {
                carried.listed.insert(new.clone());
            }
            for (old, old_suite, passed) in from {
                if old != new || old_suite != h.suite {
                    carried.renamed.push(Renamed {
                        old,
                        old_suite,
                        new: new.clone(),
                        passed,
                    });
                }
            }
            carried.known.insert(new);
        }
        c.lists.insert(h.suite.clone(), carried);
    }
    Ok(c)
}

/// A list's text with every line whose key is gone at head replaced by its new keys' lines, in place, each new key once
/// and none the list already holds; comments, and the lines of keys that stay or were removed without a map line,
/// unchanged. `None` when no line changes. For the strict list (`strict_not_run.rs`), which only shrinks: a moved line
/// is not an added one.
pub fn rewrite_list(text: &str, map: &RenameMap, at_head: &dyn Fn(&CaseId) -> bool) -> Result<Option<String>, String> {
    let mut changed = false;
    let mut out = String::new();
    let mut lines = Vec::new();
    for line in text.lines() {
        let id = crate::not_passing::parse(line)?.into_iter().next();
        // The new keys of a line that moves; empty for a line that stays.
        let moved = match &id {
            Some(id) => Some(map.translate(id, at_head)?.keys)
                .filter(|k| !k.contains(id))
                .unwrap_or_default(),
            None => BTreeSet::new(),
        };
        lines.push((line, id, moved));
    }
    // A moved key is written once, and not when the list keeps a line of its own for it.
    let mut written: BTreeSet<CaseId> = lines
        .iter()
        .filter(|(_, _, moved)| moved.is_empty())
        .filter_map(|(_, id, _)| id.clone())
        .collect();
    for (line, id, moved) in lines {
        match id {
            Some(_) if !moved.is_empty() => {
                changed = true;
                for n in moved {
                    if written.insert(n.clone()) {
                        out.push_str(&format!("{} {}\n", n.codeunit, n.name));
                    }
                }
            }
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    Ok(changed.then_some(out))
}

/// The base keys in head keys, for a list compared against its base (`--list-changes` of the strict list): each key
/// the map moves is replaced by its new keys; every other key stays.
pub fn translate_set(
    set: &BTreeSet<CaseId>,
    map: &RenameMap,
    at_head: &dyn Fn(&CaseId) -> bool,
) -> Result<BTreeSet<CaseId>, String> {
    let mut out = BTreeSet::new();
    for k in set {
        let t = map.translate(k, at_head)?;
        if t.keys.is_empty() {
            out.insert(k.clone());
        } else {
            out.extend(t.keys);
        }
    }
    Ok(out)
}

/// What `--list-new-cases` does with one list after a pin move.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BumpLines {
    /// Moved lines to add: head keys whose base cases were all listed, that the list does not hold yet, that real BC
    /// passes and the runtime does not.
    pub moved: Vec<CaseId>,
    /// Renamed or app-moved cases that passed on the old pin and do not pass now, with what they do: the bump is red.
    pub lost_passes: Vec<(Renamed, String)>,
}

/// `BumpLines` of one list: `all` its head oracle, `recorded` the run's outcomes.
pub fn bump_lines(
    carried: &Carried,
    list: &BTreeSet<CaseId>,
    all: &[crate::oracle::OracleCase],
    recorded: &dyn Fn(&CaseId) -> Option<crate::report::Outcome>,
) -> BumpLines {
    use crate::report::Outcome;
    let bc_passed = crate::not_passing::bc_passed(all);
    let passes = |k: &CaseId| matches!(recorded(k), Some(Outcome::Pass));
    let moved = carried
        .listed
        .iter()
        .filter(|k| !list.contains(*k) && bc_passed.contains(*k) && !passes(k))
        .cloned()
        .collect();
    let lost_passes = carried
        .renamed
        .iter()
        .filter(|r| r.passed)
        .filter_map(|r| {
            let why = if !bc_passed.contains(&r.new) {
                "real BC fails the new key".to_string()
            } else {
                match recorded(&r.new) {
                    Some(Outcome::Pass) => return None,
                    Some(Outcome::Fail(why)) => crate::not_passing::fail_reason(&why),
                    Some(Outcome::NotRun(why)) => format!("not run: {}", al_testrun::refusals::clip(&why, 120)),
                    None => "no outcome".to_string(),
                }
            };
            Some((r.clone(), why))
        })
        .collect();
    BumpLines { moved, lost_passes }
}

/// The error line of a case that passed on the old pin under `r.old` and does not pass under `r.new`.
pub fn lost_pass_line(label: &str, r: &Renamed, why: &str) -> String {
    format!(
        "error: BC {label}: renamed or moved case lost its pass (ADR-065 amendment 1): old key {} {}{} -> new key {} {}: \
         {why}",
        r.old.codeunit,
        r.old.name,
        suite_part(&r.old_suite),
        r.new.codeunit,
        r.new.name
    )
}

/// The error line of a passing case the map sends to a key no head oracle of the major has.
pub fn lost_line(label: &str, l: &Lost) -> String {
    format!(
        "error: BC {label}: the rename map sends the passing case {} {}{} to {} {}, which no oracle of this version \
         has (ADR-065 amendment 1)",
        l.old.codeunit,
        l.old.name,
        suite_part(&l.old_suite),
        l.new.codeunit,
        l.new.name
    )
}

/// ` (suite <name>)` after an old key of a suite's base list; nothing for the main app's.
fn suite_part(suite: &Option<String>) -> String {
    suite.as_deref().map(|s| format!(" (suite {s})")).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(cu: u32, n: &str) -> CaseId {
        CaseId {
            codeunit: cu,
            name: n.into(),
        }
    }
    fn set(ids: &[(u32, &str)]) -> BTreeSet<CaseId> {
        ids.iter().map(|(c, n)| id(*c, n)).collect()
    }
    const HEADER: &str = "# Rename map\nold_key\tnew_key\n";
    fn map(lines: &[&str]) -> RenameMap {
        RenameMap::parse(&format!("{HEADER}{}\n", lines.join("\n"))).unwrap()
    }
    fn base(suite: Option<&str>, app: &str, oracle: &[(u32, &str)], list: &[(u32, &str)]) -> BaseList {
        BaseList {
            suite: suite.map(str::to_string),
            app: app.into(),
            oracle: set(oracle),
            list: set(list),
        }
    }
    fn head(suite: Option<&str>, app: &str, cases: &[(u32, &str)]) -> HeadOracle {
        HeadOracle {
            suite: suite.map(str::to_string),
            app: app.into(),
            cases: set(cases),
        }
    }

    #[test]
    fn the_app_part_of_a_key_is_ignored_and_must_not_compile_lines_are_skipped() {
        let m = map(&["al-language/1/A\tal-language-onprem/2/B", "mnc/Old\tmnc/New"]);
        let at = |k: &CaseId| *k == id(2, "B");
        assert_eq!(m.translate(&id(1, "A"), &at).unwrap().keys, set(&[(2, "B")]));
        assert!(RenameMap::parse("al-language/1/A\tmnc/X\n").is_err());
        assert!(RenameMap::parse("al-language/x/A\tal-language/1/B\n").is_err());
        assert!(RenameMap::parse("al-language/1/A al-language/1/B\n").is_err());
    }

    #[test]
    fn a_key_at_head_keeps_its_key_and_a_gone_key_follows_a_chain() {
        // A key a head oracle still has is not read through the map (facts.py), even when an old line names it.
        let m = map(&["a/1/A\ta/1/B", "a/1/B\ta/1/C"]);
        let at = |k: &CaseId| *k == id(1, "C") || *k == id(1, "A2");
        assert_eq!(m.translate(&id(1, "A"), &at).unwrap().keys, set(&[(1, "C")]));
        let at_a = |k: &CaseId| *k == id(1, "A");
        assert_eq!(m.translate(&id(1, "A"), &at_a).unwrap().keys, set(&[(1, "A")]));
        // Removed without a line: no key, nothing dead.
        let t = m.translate(&id(1, "Gone"), &at).unwrap();
        assert!(t.keys.is_empty() && t.dead.is_empty());
        // A line to a key that is nowhere: dead.
        let none = |_: &CaseId| false;
        assert_eq!(m.translate(&id(1, "A"), &none).unwrap().dead, set(&[(1, "C")]));
        let cycle = map(&["a/1/A\ta/1/B", "a/1/B\ta/1/A"]);
        assert!(cycle.translate(&id(1, "A"), &none).unwrap_err().contains("cycle"));
    }

    #[test]
    fn a_rename_moves_its_list_line_and_does_not_duplicate_it() {
        let m = map(&["al-language/1/Old\tal-language/1/New"]);
        let b = [base(
            None,
            "app",
            &[(1, "Old"), (1, "Same")],
            &[(1, "Old"), (1, "Same")],
        )];
        let h = [head(None, "app", &[(1, "New"), (1, "Same")])];
        let c = carry(&m, &b, &h).unwrap();
        let main = c.of(None).unwrap();
        assert_eq!(main.listed, set(&[(1, "New"), (1, "Same")]));
        assert_eq!(main.known, set(&[(1, "New"), (1, "Same")]));
        assert_eq!(
            main.renamed,
            vec![Renamed {
                old: id(1, "Old"),
                old_suite: None,
                new: id(1, "New"),
                passed: false
            }]
        );
        assert!(c.lost.is_empty());
    }

    #[test]
    fn a_renamed_case_that_passed_must_pass() {
        let m = map(&["al-language/1/Old\tal-language/2/New"]);
        let b = [base(None, "app", &[(1, "Old")], &[])];
        let h = [head(None, "app", &[(2, "New")])];
        let c = carry(&m, &b, &h).unwrap();
        let main = c.of(None).unwrap();
        assert!(main.listed.is_empty());
        assert_eq!(main.known, set(&[(2, "New")]));
        assert!(main.renamed[0].passed);
    }

    #[test]
    fn a_split_carries_to_each_new_key_and_a_merge_must_pass_when_any_old_key_passed() {
        let m = map(&[
            "a/1/Split\ta/1/S1",
            "a/1/Split\ta/1/S2",
            "a/1/M1\ta/1/Merged",
            "a/1/M2\ta/1/Merged",
            "a/1/L1\ta/1/BothListed",
            "a/1/L2\ta/1/BothListed",
        ]);
        let b = [base(
            None,
            "app",
            &[(1, "Split"), (1, "M1"), (1, "M2"), (1, "L1"), (1, "L2")],
            &[(1, "Split"), (1, "M2"), (1, "L1"), (1, "L2")],
        )];
        let h = [head(
            None,
            "app",
            &[(1, "S1"), (1, "S2"), (1, "Merged"), (1, "BothListed")],
        )];
        let c = carry(&m, &b, &h).unwrap();
        let main = c.of(None).unwrap();
        assert_eq!(main.listed, set(&[(1, "S1"), (1, "S2"), (1, "BothListed")]));
        assert_eq!(main.known.len(), 4);
        let merged: Vec<_> = main.renamed.iter().filter(|r| r.new == id(1, "Merged")).collect();
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|r| r.passed) && merged.iter().any(|r| !r.passed));
    }

    #[test]
    fn an_app_move_needs_no_line_and_a_same_app_suite_does_not_take_the_other_suites_cases() {
        let m = RenameMap::default();
        let b = [
            // The codeunit moved to another app with its tests (ids are unique across apps).
            base(None, "main", &[(1, "Moved"), (3, "Stays")], &[]),
            base(Some("iso-f"), "iso", &[(9, "T")], &[(9, "T")]),
            base(Some("iso-c"), "iso", &[(9, "T")], &[]),
        ];
        let h = [
            head(None, "main", &[(3, "Stays")]),
            head(Some("onprem"), "onprem", &[(1, "Moved")]),
            head(Some("iso-f"), "iso", &[(9, "T")]),
            head(Some("iso-c"), "iso", &[(9, "T")]),
        ];
        let c = carry(&m, &b, &h).unwrap();
        let onprem = c.of(Some("onprem")).unwrap();
        assert_eq!(onprem.known, set(&[(1, "Moved")]));
        assert!(onprem.listed.is_empty() && onprem.renamed[0].passed && onprem.renamed[0].old_suite.is_none());
        assert_eq!(c.of(Some("iso-f")).unwrap().listed, set(&[(9, "T")]));
        let iso_c = c.of(Some("iso-c")).unwrap();
        assert!(iso_c.listed.is_empty() && iso_c.renamed.is_empty());
    }

    #[test]
    fn a_passing_case_mapped_to_a_key_nowhere_is_lost_and_one_id_in_two_apps_is_an_error() {
        let m = map(&["a/1/Old\ta/1/Nowhere"]);
        let b = [base(None, "app", &[(1, "Old"), (1, "ListedOld")], &[(1, "ListedOld")])];
        let h = [head(None, "app", &[(1, "Other")])];
        let c = carry(&m, &b, &h).unwrap();
        assert_eq!(
            c.lost,
            vec![Lost {
                old: id(1, "Old"),
                old_suite: None,
                new: id(1, "Nowhere")
            }]
        );
        let two = [
            head(None, "app", &[(1, "A")]),
            head(Some("onprem"), "onprem", &[(1, "B")]),
        ];
        assert_eq!(
            lost_line(
                "27.5",
                &Lost {
                    old_suite: Some("iso".into()),
                    ..c.lost[0].clone()
                }
            ),
            "error: BC 27.5: the rename map sends the passing case 1 Old (suite iso) to 1 Nowhere, which no oracle of \
             this version has (ADR-065 amendment 1)"
        );
        let e = carry(&m, &[], &two).unwrap_err();
        assert!(e.contains("codeunit 1 is in two corpus apps, app and onprem"), "{e}");
    }

    #[test]
    fn a_gone_key_whose_only_line_is_a_self_line_is_dead() {
        // An app move written as a line, to a key no head oracle of the major has: the passing case is lost.
        let none = |_: &CaseId| false;
        let app_move = map(&["al-language/1/A\tal-language-onprem/1/A"]);
        let t = app_move.translate(&id(1, "A"), &none).unwrap();
        assert!(t.keys.is_empty());
        assert_eq!(t.dead, set(&[(1, "A")]));
        // The same line with the key at head keeps it.
        let at_a = |k: &CaseId| *k == id(1, "A");
        assert_eq!(
            app_move.translate(&id(1, "A"), &at_a).unwrap(),
            Translation {
                keys: set(&[(1, "A")]),
                dead: BTreeSet::new()
            }
        );
        // A chain that ends in a self line of a gone key.
        let chain = map(&["a/1/A\ta/1/B", "a/1/B\ta/1/B"]);
        assert_eq!(chain.translate(&id(1, "A"), &none).unwrap().dead, set(&[(1, "B")]));
        let b = [base(None, "app", &[(1, "A")], &[])];
        let h = [head(None, "app", &[(1, "Other")])];
        assert_eq!(
            carry(&chain, &b, &h).unwrap().lost,
            vec![Lost {
                old: id(1, "A"),
                old_suite: None,
                new: id(1, "B")
            }]
        );
    }

    #[test]
    fn a_map_line_is_read_by_its_first_two_columns() {
        // facts.py `load_map` takes parts[0] and parts[1]; further columns are not part of the key.
        let m = RenameMap::parse("a/1/A\ta/1/B\ta reason\n").unwrap();
        let at = |k: &CaseId| *k == id(1, "B");
        assert_eq!(m.translate(&id(1, "A"), &at).unwrap().keys, set(&[(1, "B")]));
        assert!(RenameMap::parse("a/1/A\t\n").is_err());
    }

    #[test]
    fn a_rewritten_strict_list_writes_each_key_once() {
        // Two lines merged into one key, and a new key the list already holds: one line each.
        let m = map(&["a/1/M1\ta/1/Merged", "a/1/M2\ta/1/Merged", "a/1/Old\ta/1/Kept"]);
        let at = |k: &CaseId| k.name == "Merged" || k.name == "Kept";
        assert_eq!(
            rewrite_list("1 M1\n1 M2\n1 Kept\n1 Old\n", &m, &at).unwrap().as_deref(),
            Some("1 Merged\n1 Kept\n")
        );
    }

    #[test]
    fn a_strict_list_line_is_rewritten_in_place_and_comments_stay() {
        let m = map(&["a/1/Old\ta/1/New1", "a/1/Old\ta/1/New2"]);
        let at = |k: &CaseId| k.name.starts_with("New") || k.name == "Kept";
        let text = "# header\n\n# reason\n1 Old\n1 Kept\n1 RemovedNoLine\n";
        assert_eq!(
            rewrite_list(text, &m, &at).unwrap().as_deref(),
            Some("# header\n\n# reason\n1 New1\n1 New2\n1 Kept\n1 RemovedNoLine\n")
        );
        assert_eq!(rewrite_list("# h\n1 Kept\n", &m, &at).unwrap(), None);
        assert_eq!(
            translate_set(&set(&[(1, "Old"), (1, "Kept")]), &m, &at).unwrap(),
            set(&[(1, "New1"), (1, "New2"), (1, "Kept")])
        );
    }

    #[test]
    fn bump_lines_add_moved_lines_once_and_name_each_lost_pass() {
        use crate::oracle::OracleCase;
        use crate::report::Outcome;
        let m = map(&[
            "a/1/OldListed\ta/1/NewListed",
            "a/1/OldPassed\ta/1/NewFails",
            "a/1/OldOk\ta/1/NewOk",
        ]);
        let b = [base(
            None,
            "app",
            &[(1, "OldListed"), (1, "OldPassed"), (1, "OldOk")],
            &[(1, "OldListed")],
        )];
        let h = [head(None, "app", &[(1, "NewListed"), (1, "NewFails"), (1, "NewOk")])];
        let c = carry(&m, &b, &h).unwrap();
        let all: Vec<_> = h[0]
            .cases
            .iter()
            .map(|id| OracleCase {
                id: id.clone(),
                bc_failed: false,
            })
            .collect();
        let recorded = |k: &CaseId| match k.name.as_str() {
            "NewOk" => Some(Outcome::Pass),
            _ => Some(Outcome::Fail("boom".into())),
        };
        let lines = bump_lines(c.of(None).unwrap(), &BTreeSet::new(), &all, &recorded);
        assert_eq!(lines.moved, vec![id(1, "NewListed")]);
        assert_eq!(lines.lost_passes.len(), 1);
        let (r, why) = &lines.lost_passes[0];
        assert_eq!(
            lost_pass_line("27.5", r, why),
            "error: BC 27.5: renamed or moved case lost its pass (ADR-065 amendment 1): old key 1 OldPassed -> new key \
             1 NewFails: fail: boom"
        );
        // The moved line is already there: not added twice.
        let again = bump_lines(c.of(None).unwrap(), &set(&[(1, "NewListed")]), &all, &recorded);
        assert!(again.moved.is_empty());
    }
}
