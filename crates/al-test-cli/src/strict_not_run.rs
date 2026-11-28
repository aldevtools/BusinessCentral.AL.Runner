//! The not-passing rule of the whole corpus in BC's order (ADR-070 decision 5; `AL_STRICT_ORDER_ALL=1`, the nightly step
//! "Corpus in BC's order").
//!
//! That run holds the not-passing lists of the default run (`not_passing.rs`), plus one shrink-only list per BC major:
//! `metrics/<major>/strict-order-not-run.txt`, the main app's cases the default run passes and the strict run does not run
//! yet (orchestrator decision of 2026-11-23). Over the cases a run selected:
//! 1. a case on neither list that does not pass is a regression;
//! 2. a case on the strict list alone that fails is a regression too: the list excuses a case that is not run, never a
//!    wrong result;
//! 3. a line of the strict list whose case passes or left the oracle is stale (`--prune-not-passing` deletes it, keeping
//!    the comments of the other lines);
//! 4. a line of a default list whose case passes in BC's order is printed, never pruned and never a failure: that list
//!    belongs to the default run.
//!
//! A line may only leave the strict list: `--list-changes` reports a line added against a base branch that has the file,
//! and a line of a new file whose paragraph names no issue (`added`). Pure functions; `main.rs` `check_lists` reads and writes the files.

use crate::not_passing::{self, Check};
use crate::oracle::{CaseId, OracleCase};
use crate::report::Outcome;
use std::collections::BTreeSet;

/// The switch of the run in BC's order: `AL_STRICT_ORDER_ALL=1` is on, unset is off; any other value is an error
/// (al-runner issue 8948: `=0` used to count as on).
pub fn switch(value: Option<&std::ffi::OsStr>) -> Result<bool, String> {
    match value.map(|v| v.to_str()) {
        None => Ok(false),
        Some(Some("1")) => Ok(true),
        Some(v) => Err(format!(
            "AL_STRICT_ORDER_ALL is {}: set it to 1 to run every pass in BC's order, or unset it",
            v.unwrap_or("not UTF-8")
        )),
    }
}

/// The strict list of BC major `major`, relative to the repository root.
pub fn path(major: u16) -> String {
    format!("metrics/{major}/strict-order-not-run.txt")
}

/// The rule over one target in BC's order.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct StrictCheck {
    /// Rules 1 to 3; its stale lines are lines of the strict list only.
    pub check: Check,
    /// Rule 4: default-list lines whose case passes in BC's order.
    pub default_passing: Vec<CaseId>,
}

/// Applies the rule. `list` is the target's default not-passing list, `strict` its strict list (empty for a suite app).
pub fn check(
    list: &BTreeSet<CaseId>,
    strict: &BTreeSet<CaseId>,
    all: &[OracleCase],
    selected: &dyn Fn(&CaseId) -> bool,
    recorded: &dyn Fn(&CaseId) -> Option<Outcome>,
) -> StrictCheck {
    let union: BTreeSet<CaseId> = list.union(strict).cloned().collect();
    let mut c = not_passing::check(&union, all, selected, recorded);
    let default_passing = c.stale_passing.iter().filter(|x| list.contains(x)).cloned().collect();
    c.stale_passing.retain(|x| strict.contains(x));
    c.stale_gone.retain(|x| strict.contains(x));
    for x in std::mem::take(&mut c.listed) {
        match recorded(&x) {
            Some(Outcome::Fail(why)) if !list.contains(&x) => c.regressions.push((x, not_passing::fail_reason(&why))),
            _ => c.listed.push(x),
        }
    }
    StrictCheck {
        check: c,
        default_passing,
    }
}

/// The strict list's text without the lines of `stale`. A paragraph (lines between blank lines) left with comments and no
/// case is dropped with them, so a reason goes with its last case; the first paragraph, the header, always stays.
pub fn prune(text: &str, stale: &BTreeSet<CaseId>) -> String {
    let mut out = Vec::new();
    for (n, para) in text.split("\n\n").enumerate() {
        let kept: Vec<&str> = para
            .lines()
            .filter(|l| not_passing::parse(l).map_or(true, |ids| ids.iter().all(|id| !stale.contains(id))))
            .collect();
        let has_case = kept
            .iter()
            .any(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
        if n == 0 || has_case {
            out.push(kept.join("\n"));
        }
    }
    let mut s = out.join("\n\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// The lines `now` (parsed from `text`) adds to the strict list against `base`, the list at the base revision. `None`:
/// the base revision has no such file, so this change seeds it; a seed adds each case line that is not in a paragraph
/// (lines between blank lines) with a `# not run:` comment naming its issue (`issue <number>`; al-runner issue 8948).
pub fn added(base: Option<&BTreeSet<CaseId>>, now: &BTreeSet<CaseId>, text: &str) -> Vec<CaseId> {
    match base {
        Some(b) => now.difference(b).cloned().collect(),
        None => {
            let mut out = BTreeSet::new();
            for para in text.split("\n\n") {
                let comment: Vec<&str> = para
                    .lines()
                    .filter_map(|l| l.trim_start().strip_prefix('#'))
                    .map(str::trim)
                    .collect();
                let reason =
                    comment.first().is_some_and(|c| c.starts_with("not run:")) && names_issue(&comment.join(" "));
                if !reason {
                    out.extend(para.lines().filter_map(|l| not_passing::parse(l).ok()).flatten());
                }
            }
            out.into_iter().collect()
        }
    }
}

/// Whether `text` has `issue <number>`.
fn names_issue(text: &str) -> bool {
    text.match_indices("issue ")
        .any(|(i, m)| text[i + m.len()..].starts_with(|c: char| c.is_ascii_digit()))
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
    fn oracle() -> Vec<OracleCase> {
        ["Passes", "Fails", "Refused", "DefaultFails", "DefaultRefused"]
            .iter()
            .map(|n| OracleCase {
                id: id(1, n),
                bc_failed: false,
            })
            .collect()
    }
    fn recorded(c: &CaseId) -> Option<Outcome> {
        match c.name.as_str() {
            "Passes" => Some(Outcome::Pass),
            "Fails" | "DefaultFails" => Some(Outcome::Fail("boom".into())),
            _ => Some(Outcome::NotRun("not pinned".into())),
        }
    }
    fn all(_: &CaseId) -> bool {
        true
    }
    fn default_list() -> BTreeSet<CaseId> {
        set(&[(1, "DefaultFails"), (1, "DefaultRefused")])
    }
    /// The default list with `Fails` too, for the tests about another case.
    fn with_fails() -> BTreeSet<CaseId> {
        set(&[(1, "Fails"), (1, "DefaultFails"), (1, "DefaultRefused")])
    }

    #[test]
    fn the_switch_is_on_for_1_only_and_refuses_other_values() {
        use std::ffi::OsStr;
        assert_eq!(switch(None), Ok(false));
        assert_eq!(switch(Some(OsStr::new("1"))), Ok(true));
        for v in ["0", "", "true", "yes"] {
            assert!(switch(Some(OsStr::new(v))).is_err(), "{v:?}");
        }
    }

    #[test]
    fn a_case_on_neither_list_that_does_not_pass_is_a_regression() {
        let s = check(&default_list(), &BTreeSet::new(), &oracle(), &all, &recorded);
        let names: Vec<_> = s
            .check
            .regressions
            .iter()
            .map(|(c, w)| (c.name.as_str(), w.as_str()))
            .collect();
        assert_eq!(names, [("Fails", "fail: boom"), ("Refused", "not run: not pinned")]);
        assert_eq!(not_passing::decide(&s.check, false, false), not_passing::Decision::Fail);
    }

    #[test]
    fn the_strict_list_excuses_a_case_that_is_not_run_and_never_one_that_fails() {
        let s = check(
            &default_list(),
            &set(&[(1, "Refused"), (1, "Fails")]),
            &oracle(),
            &all,
            &recorded,
        );
        let names: Vec<_> = s
            .check
            .regressions
            .iter()
            .map(|(c, w)| (c.name.as_str(), w.as_str()))
            .collect();
        assert_eq!(names, [("Fails", "fail: boom")]);
        assert!(s.check.listed.contains(&id(1, "Refused")) && !s.check.listed.contains(&id(1, "Fails")));
        let s = check(&with_fails(), &set(&[(1, "Refused")]), &oracle(), &all, &recorded);
        assert_eq!(not_passing::decide(&s.check, false, false), not_passing::Decision::Ok);
    }

    #[test]
    fn a_strict_line_that_passes_or_left_the_oracle_is_stale_and_fails_until_pruned() {
        let strict = set(&[(1, "Refused"), (1, "Passes"), (9, "Gone")]);
        let s = check(&with_fails(), &strict, &oracle(), &all, &recorded);
        assert_eq!(s.check.stale_passing, vec![id(1, "Passes")]);
        assert_eq!(s.check.stale_gone, vec![id(9, "Gone")]);
        assert!(s.check.regressions.is_empty());
        assert_eq!(not_passing::decide(&s.check, false, false), not_passing::Decision::Fail);
        assert_eq!(not_passing::decide(&s.check, true, false), not_passing::Decision::Ok);
    }

    #[test]
    fn a_default_line_that_passes_in_bcs_order_is_reported_and_never_stale() {
        let list = set(&[
            (1, "Passes"),
            (1, "Fails"),
            (1, "DefaultFails"),
            (1, "DefaultRefused"),
            (9, "Gone"),
        ]);
        let s = check(&list, &set(&[(1, "Refused")]), &oracle(), &all, &recorded);
        assert_eq!(s.default_passing, vec![id(1, "Passes")]);
        assert_eq!(s.check.stale().count(), 0, "{s:?}");
        assert_eq!(not_passing::decide(&s.check, false, false), not_passing::Decision::Ok);
    }

    #[test]
    fn prune_keeps_the_header_and_drops_a_reason_with_its_last_case() {
        let text = "# header\n# more\n\n# not run: reason A\n1 A1\n1 A2\n\n# not run: reason B\n2 B1\n";
        assert_eq!(
            prune(text, &set(&[(1, "A1")])),
            "# header\n# more\n\n# not run: reason A\n1 A2\n\n# not run: reason B\n2 B1\n"
        );
        assert_eq!(
            prune(text, &set(&[(2, "B1")])),
            "# header\n# more\n\n# not run: reason A\n1 A1\n1 A2\n"
        );
        assert_eq!(
            prune(text, &set(&[(1, "A1"), (1, "A2"), (2, "B1")])),
            "# header\n# more\n"
        );
        let kept = prune(text, &set(&[(1, "A2")]));
        assert_eq!(not_passing::parse(&kept).unwrap(), set(&[(1, "A1"), (2, "B1")]));
    }

    #[test]
    fn a_line_added_against_a_base_that_has_the_file_is_reported() {
        let text = "1 A\n2 B\n";
        let now = set(&[(1, "A"), (2, "B")]);
        assert_eq!(added(Some(&set(&[(1, "A")])), &now, text), vec![id(2, "B")]);
        assert_eq!(added(Some(&set(&[(1, "A"), (2, "B"), (3, "C")])), &now, text), vec![]);
    }

    /// Al-runner issue 8948 item 1: a seed (the base revision has no file) adds every case line that is not in a
    /// `# not run:` paragraph naming its issue.
    #[test]
    fn a_seed_adds_the_lines_whose_paragraph_names_no_issue() {
        let text = "# header, issue 1 named here does not count\n\n\
                    # not run: a reason that names\n# runner issue 8917.\n1 A\n1 B\n\n\
                    # not run: a reason without a number.\n2 C\n\n\
                    # a reason that names issue 9 but is no not-run reason\n3 D\n\n\
                    4 E\n";
        let now = not_passing::parse(text).unwrap();
        assert_eq!(added(None, &now, text), vec![id(2, "C"), id(3, "D"), id(4, "E")]);
    }
}
