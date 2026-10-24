//! `--awaiting-assumptions`: the assumptions that wait for a corpus pin (`Info::awaiting` in the registry,
//! `al-builtins/src/assumptions.rs`), checked against the pinned corpus.
//!
//! An assumption whose awaited files are all in the pinned corpus has outlived its reason: the check runs those
//! files and fails with one message per assumption, "the pin landed: remove the assumption" when the pin agrees
//! with the runtime, or "the pin contradicts the assumption: fix the code" when an awaited test fails. Some files
//! present is a warning. `--summary` prints one line for a status report, the oldest entry first.

use std::path::Path;

use al_testrun::Outcome;
use al_testrun::assumptions::{Assumption, Awaiting};

/// An assumption that waits for a corpus pin.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub id: &'static str,
    pub awaiting: Awaiting,
}

/// Every registry entry with `awaiting`, the oldest first (by `since`, then id).
pub fn entries() -> Vec<Entry> {
    sorted(
        Assumption::ALL
            .iter()
            .filter_map(|a| {
                let i = a.info();
                i.awaiting.map(|awaiting| Entry { id: i.id, awaiting })
            })
            .collect(),
    )
}

/// [`entries`] plus the entries of `extra` (the value of `AL_AWAITING_EXTRA`, for the tests of this check):
/// `id;issue,issue;since;file,file`, entries joined by `|`.
pub fn with_extra(extra: Option<&str>) -> Result<Vec<Entry>, String> {
    let mut v = entries();
    for e in extra.unwrap_or("").split('|').filter(|e| !e.is_empty()) {
        let bad = || format!("AL_AWAITING_EXTRA: `{e}` is not `id;issue,issue;since;file,file`");
        let [id, issues, since, files] = e.split(';').collect::<Vec<_>>()[..] else {
            return Err(bad());
        };
        let issues: Vec<u32> = issues
            .split(',')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| bad())?;
        let files: Vec<&'static str> = files
            .split(',')
            .map(|f| &*Box::leak(f.to_string().into_boxed_str()))
            .collect();
        v.push(Entry {
            id: Box::leak(id.to_string().into_boxed_str()),
            awaiting: Awaiting {
                issues: Box::leak(issues.into_boxed_slice()),
                files: Box::leak(files.into_boxed_slice()),
                since: Box::leak(since.to_string().into_boxed_str()),
            },
        });
    }
    Ok(sorted(v))
}

fn sorted(mut v: Vec<Entry>) -> Vec<Entry> {
    v.sort_by(|a, b| (a.awaiting.since, a.id).cmp(&(b.awaiting.since, b.id)));
    v
}

/// How much of an entry's pin is in the pinned corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presence {
    /// No awaited file is there yet.
    Waiting,
    /// Some are: the pin is landing in parts.
    Partial {
        present: Vec<&'static str>,
        missing: Vec<&'static str>,
    },
    /// Every awaited file is there.
    Landed,
}

/// Which of `w`'s files exist under `corpus_root` (the corpus checkout, the folder holding `tests/`).
pub fn presence(corpus_root: &Path, w: &Awaiting) -> Presence {
    let (present, missing): (Vec<&'static str>, Vec<&'static str>) =
        w.files.iter().partition(|f| corpus_root.join(f).is_file());
    if missing.is_empty() {
        Presence::Landed
    } else if present.is_empty() {
        Presence::Waiting
    } else {
        Presence::Partial { present, missing }
    }
}

/// One oracle case of an awaited file, as the runtime ran it.
#[derive(Debug, Clone)]
pub struct AwaitedCase {
    /// The test codeunit's file, relative to the corpus root.
    pub file: String,
    pub name: String,
    pub outcome: Outcome,
    /// Real BC failed the case on purpose: the runtime must fail it too.
    pub bc_failed: bool,
    /// The assumption ids the case ran through.
    pub marks: Vec<&'static str>,
}

/// What the pinned corpus says about a landed entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every awaited case ends as on real BC: delete the entry and cite the pin. `through` of `total` cases still
    /// ran through the assumption.
    Remove { total: usize, through: usize },
    /// Awaited cases the runtime ends otherwise than real BC: the assumption is wrong.
    Contradicts { cases: Vec<String> },
    /// Awaited cases the runtime does not run: the pin cannot be checked yet.
    NotRun { cases: Vec<String> },
    /// The oracle has no case in the awaited files.
    NoCases,
}

/// The verdict on `id`, from the cases of all awaited files.
pub fn verdict(id: &str, w: &Awaiting, cases: &[AwaitedCase]) -> Verdict {
    let mine: Vec<&AwaitedCase> = cases.iter().filter(|c| w.files.contains(&c.file.as_str())).collect();
    if mine.is_empty() {
        return Verdict::NoCases;
    }
    let label = |c: &AwaitedCase| format!("{}::{}", c.file, c.name);
    let wrong: Vec<String> = mine
        .iter()
        .filter(|c| match c.outcome {
            Outcome::Pass => c.bc_failed,
            Outcome::Fail(_) => !c.bc_failed,
            Outcome::NotRun(_) => false,
        })
        .map(|c| label(c))
        .collect();
    if !wrong.is_empty() {
        return Verdict::Contradicts { cases: wrong };
    }
    let not_run: Vec<String> = mine
        .iter()
        .filter(|c| matches!(c.outcome, Outcome::NotRun(_)))
        .map(|c| label(c))
        .collect();
    if !not_run.is_empty() {
        return Verdict::NotRun { cases: not_run };
    }
    Verdict::Remove {
        total: mine.len(),
        through: mine.iter().filter(|c| c.marks.contains(&id)).count(),
    }
}

fn issues(w: &Awaiting) -> String {
    let n: Vec<String> = w.issues.iter().map(|i| format!("#{i}")).collect();
    format!("al-runner {}", n.join(", "))
}

/// The failure message of a landed entry.
pub fn message(e: &Entry, v: &Verdict) -> String {
    let head = format!(
        "assumption `{}` ({}, waiting since {})",
        e.id,
        issues(&e.awaiting),
        e.awaiting.since
    );
    let files = e.awaiting.files.join(", ");
    match v {
        Verdict::Remove { total, through } => format!(
            "{head}: the pin landed: remove the assumption and cite the pin ({files}). {total} awaited test(s) pass as on real BC, {through} of them still run through the assumption."
        ),
        Verdict::Contradicts { cases } => format!(
            "{head}: the pin contradicts the assumption: fix the code. Awaited test(s) that end otherwise than on real BC: {}.",
            cases.join(", ")
        ),
        Verdict::NotRun { cases } => format!(
            "{head}: the pin landed ({files}) but the runtime does not run {} of its test(s): see why with --explain, make them run, then remove the assumption or fix the code. Not run: {}.",
            cases.len(),
            cases.join(", ")
        ),
        Verdict::NoCases => format!(
            "{head}: the pin's files are in the corpus ({files}) but the oracle has no case in them: check the awaited file names."
        ),
    }
}

/// The warning for an entry whose pin is partly in the corpus.
pub fn partial_warning(e: &Entry, present: &[&str], missing: &[&str]) -> String {
    format!(
        "warning: assumption `{}` ({}): part of its pin is in the corpus ({}); still missing: {}",
        e.id,
        issues(&e.awaiting),
        present.join(", "),
        missing.join(", ")
    )
}

/// The `--summary` line: how many entries wait, the oldest first, each with its state.
pub fn summary(entries: &[(Entry, Presence)]) -> String {
    if entries.is_empty() {
        return "awaiting assumptions: 0".into();
    }
    let parts: Vec<String> = entries
        .iter()
        .map(|(e, p)| {
            let state = match p {
                Presence::Waiting => "waiting".to_string(),
                Presence::Partial { present, missing } => {
                    format!("{} of {} files landed", present.len(), present.len() + missing.len())
                }
                Presence::Landed => "pin landed, act now".to_string(),
            };
            format!(
                "{} ({}, since {}, {state})",
                e.id,
                issues(&e.awaiting),
                e.awaiting.since
            )
        })
        .collect();
    let landed = entries.iter().filter(|(_, p)| *p == Presence::Landed).count();
    format!(
        "awaiting assumptions: {} ({landed} with the pin landed), oldest first: {}",
        entries.len(),
        parts.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Awaiting = Awaiting {
        issues: &[5806],
        files: &["tests/al-language/x/A.al", "tests/al-language/x/B.al"],
        since: "2026-10-24",
    };

    fn entry(id: &'static str, since: &'static str) -> Entry {
        Entry {
            id,
            awaiting: Awaiting { since, ..A },
        }
    }

    fn case(file: &str, outcome: Outcome, bc_failed: bool, marks: &[&'static str]) -> AwaitedCase {
        AwaitedCase {
            file: file.into(),
            name: "T".into(),
            outcome,
            bc_failed,
            marks: marks.to_vec(),
        }
    }

    #[test]
    fn presence_says_waiting_partial_or_landed() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(presence(d.path(), &A), Presence::Waiting);
        std::fs::create_dir_all(d.path().join("tests/al-language/x")).unwrap();
        std::fs::write(d.path().join("tests/al-language/x/A.al"), "").unwrap();
        assert_eq!(
            presence(d.path(), &A),
            Presence::Partial {
                present: vec!["tests/al-language/x/A.al"],
                missing: vec!["tests/al-language/x/B.al"],
            }
        );
        std::fs::write(d.path().join("tests/al-language/x/B.al"), "").unwrap();
        assert_eq!(presence(d.path(), &A), Presence::Landed);
    }

    #[test]
    fn passing_awaited_tests_say_remove_the_assumption() {
        let cases = [
            case("tests/al-language/x/A.al", Outcome::Pass, false, &["a"]),
            case("tests/al-language/x/B.al", Outcome::Fail("e".into()), true, &[]),
            case("tests/al-language/other/C.al", Outcome::Fail("e".into()), false, &[]),
        ];
        let v = verdict("a", &A, &cases);
        assert_eq!(v, Verdict::Remove { total: 2, through: 1 });
        let m = message(&entry("a", "2026-10-24"), &v);
        assert!(
            m.contains("the pin landed: remove the assumption and cite the pin"),
            "{m}"
        );
        assert!(m.contains("1 of them still run through the assumption"), "{m}");
    }

    #[test]
    fn a_failing_awaited_test_says_fix_the_code() {
        let cases = [
            case("tests/al-language/x/A.al", Outcome::Fail("e".into()), false, &["a"]),
            case("tests/al-language/x/B.al", Outcome::Pass, true, &[]),
            case("tests/al-language/x/B.al", Outcome::NotRun("r".into()), false, &[]),
        ];
        let v = verdict("a", &A, &cases);
        assert_eq!(
            v,
            Verdict::Contradicts {
                cases: vec![
                    "tests/al-language/x/A.al::T".into(),
                    "tests/al-language/x/B.al::T".into()
                ]
            }
        );
        assert!(message(&entry("a", "2026-10-24"), &v).contains("the pin contradicts the assumption: fix the code"));
    }

    #[test]
    fn refused_or_missing_awaited_tests_are_named() {
        let cases = [case(
            "tests/al-language/x/A.al",
            Outcome::NotRun("r".into()),
            false,
            &[],
        )];
        let v = verdict("a", &A, &cases);
        assert!(matches!(v, Verdict::NotRun { ref cases } if cases.len() == 1));
        assert!(message(&entry("a", "2026-10-24"), &v).contains("does not run 1 of its test(s)"));
        assert_eq!(verdict("a", &A, &[]), Verdict::NoCases);
    }

    #[test]
    fn the_summary_counts_and_lists_the_oldest_first() {
        assert_eq!(summary(&[]), "awaiting assumptions: 0");
        let list = sorted(vec![entry("new", "2026-10-24"), entry("old", "2026-10-20")]);
        let with: Vec<(Entry, Presence)> = list
            .into_iter()
            .map(|e| {
                let p = if e.id == "old" {
                    Presence::Landed
                } else {
                    Presence::Waiting
                };
                (e, p)
            })
            .collect();
        assert_eq!(
            summary(&with),
            "awaiting assumptions: 2 (1 with the pin landed), oldest first: old (al-runner #5806, since 2026-10-20, pin landed, act now); new (al-runner #5806, since 2026-10-24, waiting)"
        );
    }

    #[test]
    fn a_partial_pin_is_a_warning_naming_both_sides() {
        let w = partial_warning(&entry("a", "2026-10-24"), &["p.al"], &["m.al"]);
        assert!(
            w.starts_with("warning: assumption `a`") && w.contains("p.al") && w.contains("missing: m.al"),
            "{w}"
        );
    }
}
