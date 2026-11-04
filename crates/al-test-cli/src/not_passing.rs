//! ADR-065: the lists of the corpus cases that real BC passes and the runtime does not pass yet
//! (`metrics/<major>/not-passing*.txt`, one per BC major and test app), and the CI rule over them.
//!
//! For each version and suite, over the cases a run selected (section 2):
//! 1. a case that is not listed must pass (a regression otherwise);
//! 2. a listed case that passes is a stale line (`--prune-not-passing` deletes it);
//! 3. a listed case that is no longer in the oracle is a stale line too;
//! 4. every oracle case has an outcome: a case the run recorded nothing for is a runner bug.
//!
//! Cases real BC fails (`bc_failed`) are in no list and are not checked. Pure functions; the CLI reads
//! and writes the files.

use crate::oracle::{CaseId, OracleCase};
use crate::report::{DOTNET_UNAVAILABLE, Outcome};
use al_testrun::refusals::clip;
use std::collections::{BTreeMap, BTreeSet};

/// The result of the rule over one list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Check {
    /// Not listed and not passing, with what they do now (`fail: ...`, `not run: ...`).
    pub regressions: Vec<(CaseId, String)>,
    /// Listed, and passing now.
    pub stale_passing: Vec<CaseId>,
    /// Listed, and not a case of the oracle that real BC passes.
    pub stale_gone: Vec<CaseId>,
    /// Oracle cases the run recorded no outcome for, listed or not.
    pub no_outcome: Vec<CaseId>,
    /// Not listed, and not run only because the machine has no usable .NET, with the reason (ADR-014
    /// section 11): printed, not a regression, unless `--require-dotnet`.
    pub skipped: Vec<(CaseId, String)>,
    /// Listed and still not passing.
    pub listed: Vec<CaseId>,
}

impl Check {
    pub fn stale(&self) -> impl Iterator<Item = &CaseId> {
        self.stale_passing.iter().chain(&self.stale_gone)
    }
}

/// Applies the rule. `selected` is the run's selection over the oracle (`all`): only the selected
/// cases ran. `recorded` is the outcome the run recorded for a case, `None` when it recorded none.
pub fn check(
    list: &BTreeSet<CaseId>,
    all: &[OracleCase],
    selected: &dyn Fn(&CaseId) -> bool,
    recorded: &dyn Fn(&CaseId) -> Option<Outcome>,
) -> Check {
    let mut c = Check::default();
    let mut checked = BTreeSet::new();
    for case in all.iter().filter(|x| !x.bc_failed) {
        checked.insert(&case.id);
        if !selected(&case.id) {
            continue;
        }
        let listed = list.contains(&case.id);
        let Some(o) = recorded(&case.id) else {
            c.no_outcome.push(case.id.clone());
            continue;
        };
        match (o, listed) {
            (Outcome::Pass, true) => c.stale_passing.push(case.id.clone()),
            (Outcome::Pass, false) => {}
            (_, true) => c.listed.push(case.id.clone()),
            (Outcome::NotRun(why), false) if why.contains(DOTNET_UNAVAILABLE) => c.skipped.push((case.id.clone(), why)),
            (Outcome::Fail(why), false) => c
                .regressions
                .push((case.id.clone(), format!("fail: {}", clip(&why, 120)))),
            (Outcome::NotRun(why), false) => c
                .regressions
                .push((case.id.clone(), format!("not run: {}", clip(&why, 120)))),
        }
    }
    c.stale_gone = list
        .iter()
        .filter(|l| !checked.contains(l) && selected(l))
        .cloned()
        .collect();
    c
}

/// What the CLI does about a check.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Ok,
    /// Exit 1: a regression, a case without an outcome, or a stale line that was not pruned.
    Fail,
    /// Exit 2: the run cannot be trusted; the text says why.
    Setup(String),
}

/// `pruned`: the stale lines were deleted in this run (`--prune-not-passing`), so they do not fail it.
pub fn decide(c: &Check, pruned: bool, require_dotnet: bool) -> Decision {
    if !c.regressions.is_empty() || !c.no_outcome.is_empty() || (!pruned && c.stale().next().is_some()) {
        return Decision::Fail;
    }
    if !c.skipped.is_empty() && require_dotnet {
        return Decision::Setup(format!(
            "--require-dotnet: {} case(s) could not run because .NET is unavailable",
            c.skipped.len()
        ));
    }
    Decision::Ok
}

/// The cases `--list-new-cases` adds: selected oracle cases real BC passes that `base` (the base branch's
/// oracle, the ids of its cases real BC passes) does not have, the runtime does not pass, and the list
/// does not hold yet. A case the base oracle had is never added: that line would hide a regression.
pub fn new_cases(
    list: &BTreeSet<CaseId>,
    all: &[OracleCase],
    base: &BTreeSet<CaseId>,
    recorded: &dyn Fn(&CaseId) -> Option<Outcome>,
) -> Vec<CaseId> {
    all.iter()
        .filter(|c| !c.bc_failed && !base.contains(&c.id) && !list.contains(&c.id))
        .filter(|c| !matches!(recorded(&c.id), Some(Outcome::Pass)))
        .map(|c| c.id.clone())
        .collect()
}

/// The ids of the oracle cases real BC passes.
pub fn bc_passed(all: &[OracleCase]) -> BTreeSet<CaseId> {
    all.iter().filter(|c| !c.bc_failed).map(|c| c.id.clone()).collect()
}

/// List file format: one `<codeunit> <procedure>` per line, `#` comments.
pub fn parse(text: &str) -> Result<BTreeSet<CaseId>, String> {
    let mut set = BTreeSet::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (cu, name) = line
            .split_once(' ')
            .ok_or_else(|| format!("line {}: expected `<codeunit> <procedure>`", n + 1))?;
        let codeunit = cu
            .parse()
            .map_err(|_| format!("line {}: `{cu}` is not a codeunit number", n + 1))?;
        set.insert(CaseId {
            codeunit,
            name: name.trim().to_string(),
        });
    }
    Ok(set)
}

pub const HEADER: &str = "\
# Corpus cases of this oracle that real BC passes and the runtime does not pass yet (ADR-065).
# CI fails when a case that is not listed does not pass, and when a listed case passes or left the oracle.
# `al-test-cli --prune-not-passing` deletes stale lines; a corpus bump adds new cases with `--list-new-cases`.
# A line for a case the oracle already had is a regression: it needs the owner's OK (CLAUDE.md hard rule 2).
";

pub fn format(cases: &BTreeSet<CaseId>) -> String {
    let mut s = String::from(HEADER);
    for c in cases {
        s.push_str(&format!("{} {}\n", c.codeunit, c.name));
    }
    s
}

/// The check as text for `--baseline-diff`, each list grouped by area.
pub fn render_diff(label: &str, c: &Check, area_of: &dyn Fn(u32) -> Option<String>) -> String {
    let mut s = format!(
        "Baseline diff, {label}: {} regressions, {} stale lines to delete, {} cases still listed, {} cases without an outcome\n",
        c.regressions.len(),
        c.stale_passing.len() + c.stale_gone.len(),
        c.listed.len(),
        c.no_outcome.len()
    );
    let area = |cu: u32| area_of(cu).unwrap_or_else(|| "(no area)".into());
    let section = |s: &mut String, title: &str, items: Vec<(&CaseId, String)>| {
        if items.is_empty() {
            return;
        }
        s.push_str(&format!("\n{title} ({}):\n", items.len()));
        let mut by_area: BTreeMap<String, Vec<(&CaseId, String)>> = BTreeMap::new();
        for (c, why) in items {
            by_area.entry(area(c.codeunit)).or_default().push((c, why));
        }
        for (a, list) in by_area {
            s.push_str(&format!("  {a} ({}):\n", list.len()));
            for (c, why) in list {
                if why.is_empty() {
                    s.push_str(&format!("    {} {}\n", c.codeunit, c.name));
                } else {
                    s.push_str(&format!("    {} {}  [{why}]\n", c.codeunit, c.name));
                }
            }
        }
    };
    section(
        &mut s,
        "Regressions: not listed and not passing",
        c.regressions.iter().map(|(c, w)| (c, w.clone())).collect(),
    );
    section(
        &mut s,
        "Stale lines to delete (--prune-not-passing)",
        c.stale_passing
            .iter()
            .map(|c| (c, "passes".to_string()))
            .chain(c.stale_gone.iter().map(|c| (c, "not in the oracle".to_string())))
            .collect(),
    );
    section(
        &mut s,
        "Cases without an outcome (a runner bug)",
        c.no_outcome.iter().map(|c| (c, String::new())).collect(),
    );
    if !c.listed.is_empty() {
        let mut by_area: BTreeMap<String, usize> = BTreeMap::new();
        for l in &c.listed {
            *by_area.entry(area(l.codeunit)).or_default() += 1;
        }
        s.push_str(&format!("\nStill listed, by area ({}):\n", c.listed.len()));
        for (a, n) in by_area {
            s.push_str(&format!("  {a}: {n}\n"));
        }
    }
    s
}

/// How one list file changed against the base branch (`--list-changes`).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ListChange {
    pub path: String,
    /// Added lines for cases the base oracle did not have: a corpus bump's new cases.
    pub new_cases: Vec<CaseId>,
    /// Added lines for cases the base oracle had: regressions this pull request accepts, which need the
    /// owner's OK in review (ADR-065, owner decision 2).
    pub accepted_regressions: Vec<CaseId>,
    /// Deleted lines.
    pub deleted: usize,
}

/// Compares a list with its version on the base branch. `base_oracle` holds the ids of the cases the
/// base branch's oracle has that real BC passes.
pub fn list_change(
    path: &str,
    now: &BTreeSet<CaseId>,
    base: &BTreeSet<CaseId>,
    base_oracle: &BTreeSet<CaseId>,
) -> ListChange {
    let mut ch = ListChange {
        path: path.to_string(),
        deleted: base.difference(now).count(),
        ..ListChange::default()
    };
    for c in now.difference(base) {
        if base_oracle.contains(c) {
            ch.accepted_regressions.push(c.clone());
        } else {
            ch.new_cases.push(c.clone());
        }
    }
    ch
}

/// The changes as Markdown, for the CI job summary.
pub fn render_changes(base: &str, changes: &[ListChange]) -> String {
    let mut s = format!("## Not-passing lists against {base}\n\n");
    let changed: Vec<_> = changes
        .iter()
        .filter(|c| c.deleted > 0 || !c.new_cases.is_empty() || !c.accepted_regressions.is_empty())
        .collect();
    if changed.is_empty() {
        s.push_str("No list changed.\n");
        return s;
    }
    s.push_str("| List | Lines deleted | New cases | Regressions accepted |\n|---|---:|---:|---:|\n");
    for c in &changed {
        s.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            c.path,
            c.deleted,
            c.new_cases.len(),
            c.accepted_regressions.len()
        ));
    }
    let accepted: Vec<_> = changed
        .iter()
        .flat_map(|c| c.accepted_regressions.iter().map(move |x| (&c.path, x)))
        .collect();
    if !accepted.is_empty() {
        s.push_str(&format!(
            "\n### Regressions accepted by this pull request ({})\n\nEach line names a case the base oracle already had. It needs the owner's OK in review.\n\n",
            accepted.len()
        ));
        for (p, c) in accepted {
            s.push_str(&format!("- `{p}`: {} {}\n", c.codeunit, c.name));
        }
    }
    let new: Vec<_> = changed
        .iter()
        .flat_map(|c| c.new_cases.iter().map(move |x| (&c.path, x)))
        .collect();
    if !new.is_empty() {
        s.push_str(&format!(
            "\n<details><summary>New cases listed ({}): not in the base oracle</summary>\n\n",
            new.len()
        ));
        for (p, c) in new {
            s.push_str(&format!("- `{p}`: {} {}\n", c.codeunit, c.name));
        }
        s.push_str("\n</details>\n");
    }
    s
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
    fn case(cu: u32, n: &str, bc_failed: bool) -> OracleCase {
        OracleCase {
            id: id(cu, n),
            bc_failed,
        }
    }
    fn set(ids: &[(u32, &str)]) -> BTreeSet<CaseId> {
        ids.iter().map(|(c, n)| id(*c, n)).collect()
    }
    fn oracle() -> Vec<OracleCase> {
        vec![
            case(1, "Passes", false),
            case(1, "Fails", false),
            case(1, "Refused", false),
            case(1, "BcFails", true),
            case(2, "NoOutcome", false),
            case(2, "NeedsDotNet", false),
        ]
    }
    fn recorded(c: &CaseId) -> Option<Outcome> {
        match c.name.as_str() {
            "Passes" => Some(Outcome::Pass),
            "Fails" | "BcFails" => Some(Outcome::Fail("boom".into())),
            "Refused" => Some(Outcome::NotRun("not implemented".into())),
            "NeedsDotNet" => Some(Outcome::NotRun(format!("{DOTNET_UNAVAILABLE}: no dotnet"))),
            _ => None,
        }
    }
    fn all(_: &CaseId) -> bool {
        true
    }

    #[test]
    fn an_unlisted_case_that_does_not_pass_is_a_regression() {
        let list = set(&[(1, "Refused")]);
        let c = check(&list, &oracle(), &all, &recorded);
        let names: Vec<_> = c.regressions.iter().map(|(c, _)| c.name.as_str()).collect();
        assert_eq!(names, ["Fails"]);
        assert_eq!(c.regressions[0].1, "fail: boom");
        assert_eq!(decide(&c, false, false), Decision::Fail);
    }

    #[test]
    fn a_listed_case_that_passes_is_stale_and_fails_until_pruned() {
        let list = set(&[(1, "Passes"), (1, "Fails"), (1, "Refused"), (2, "NoOutcome")]);
        let mut c = check(&list, &oracle(), &all, &recorded);
        assert_eq!(c.stale_passing, vec![id(1, "Passes")]);
        assert!(c.regressions.is_empty());
        c.no_outcome.clear();
        assert_eq!(decide(&c, false, false), Decision::Fail);
        assert_eq!(decide(&c, true, false), Decision::Ok);
    }

    #[test]
    fn a_listed_case_no_longer_in_the_oracle_is_stale() {
        // A case real BC now fails is not checked either: its line goes too.
        let list = set(&[(1, "Fails"), (1, "Refused"), (9, "Removed"), (1, "BcFails")]);
        let c = check(&list, &oracle(), &all, &recorded);
        assert_eq!(c.stale_gone, vec![id(1, "BcFails"), id(9, "Removed")]);
        let pruned: BTreeSet<_> = list.iter().filter(|l| !c.stale().any(|s| s == *l)).cloned().collect();
        assert_eq!(pruned, set(&[(1, "Fails"), (1, "Refused")]));
    }

    #[test]
    fn an_oracle_case_without_an_outcome_fails_listed_or_not() {
        for list in [
            set(&[(1, "Fails"), (1, "Refused")]),
            set(&[(1, "Fails"), (1, "Refused"), (2, "NoOutcome")]),
        ] {
            let c = check(&list, &oracle(), &all, &recorded);
            assert_eq!(c.no_outcome, vec![id(2, "NoOutcome")]);
            assert!(c.regressions.is_empty() && c.stale().next().is_none(), "{c:?}");
            assert_eq!(decide(&c, true, false), Decision::Fail);
        }
    }

    #[test]
    fn a_case_skipped_without_dotnet_is_reported_and_fails_only_with_require_dotnet() {
        let oracle: Vec<_> = oracle().into_iter().filter(|c| c.id.name != "NoOutcome").collect();
        let list = set(&[(1, "Fails"), (1, "Refused")]);
        let c = check(&list, &oracle, &all, &recorded);
        assert_eq!(c.skipped.len(), 1);
        assert_eq!(decide(&c, false, false), Decision::Ok);
        let Decision::Setup(why) = decide(&c, false, true) else {
            panic!()
        };
        assert!(why.contains("--require-dotnet"), "{why}");
    }

    #[test]
    fn a_selection_checks_only_the_selected_cases() {
        let list = set(&[(9, "Removed")]);
        let only_passes = |c: &CaseId| c.name == "Passes";
        let c = check(&list, &oracle(), &only_passes, &recorded);
        assert_eq!(c, Check::default());
    }

    #[test]
    fn new_cases_are_only_those_the_base_oracle_lacks_and_the_runtime_does_not_pass() {
        let list = set(&[(1, "Refused")]);
        let base = set(&[(1, "Fails"), (1, "Passes")]);
        assert_eq!(
            new_cases(&list, &oracle(), &base, &recorded),
            vec![id(2, "NoOutcome"), id(2, "NeedsDotNet")]
        );
        // Against a base without the oracle, every case is new: that is how the lists were made (ADR-065 section 5).
        assert_eq!(
            new_cases(&BTreeSet::new(), &oracle(), &BTreeSet::new(), &recorded).len(),
            4
        );
    }

    #[test]
    fn list_round_trips_and_skips_comments() {
        let s = set(&[(60002, "A_Works"), (60001, "Z")]);
        assert_eq!(parse(&format(&s)).unwrap(), s);
        assert!(format(&s).starts_with("# Corpus cases"));
        assert!(parse("abc A\n").is_err());
        assert!(parse("60002\n").is_err());
    }

    #[test]
    fn a_line_added_for_a_case_the_base_oracle_had_is_an_accepted_regression() {
        let base = set(&[(1, "A"), (1, "B")]);
        let now = set(&[(1, "A"), (1, "C"), (2, "New")]);
        let base_oracle = set(&[(1, "A"), (1, "B"), (1, "C")]);
        let ch = list_change("metrics/29/not-passing.txt", &now, &base, &base_oracle);
        assert_eq!(ch.deleted, 1);
        assert_eq!(ch.accepted_regressions, vec![id(1, "C")]);
        assert_eq!(ch.new_cases, vec![id(2, "New")]);
        let md = render_changes("origin/main", &[ch]);
        assert!(
            md.contains("Regressions accepted by this pull request (1)") && md.contains("1 C"),
            "{md}"
        );
        assert!(render_changes("x", &[]).contains("No list changed."));
    }

    #[test]
    fn the_diff_lists_regressions_stale_lines_and_counts_what_is_still_listed() {
        let list = set(&[(1, "Passes"), (1, "Refused"), (9, "Gone")]);
        let c = check(&list, &oracle(), &all, &recorded);
        let text = render_diff("BC 1", &c, &|cu| Some(format!("area{cu}")));
        assert!(text.starts_with("Baseline diff, BC 1: 1 regressions, 2 stale lines to delete, 1 cases still listed, 1 cases without an outcome"), "{text}");
        assert!(
            text.contains("    1 Passes  [passes]") && text.contains("    9 Gone  [not in the oracle]"),
            "{text}"
        );
        assert!(text.contains("Still listed, by area (1):\n  area1: 1"), "{text}");
    }
}
