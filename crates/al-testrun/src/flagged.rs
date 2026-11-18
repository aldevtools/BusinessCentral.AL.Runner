//! Flagged passes: a test that passed and ran through code that rests on an unprobed assumption
//! (`al_builtins::assumptions`). The counts per assumption, their ranking, and the file the last run
//! leaves for `--list-assumptions`.

use al_builtins::assumptions::{self, Assumption};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many tests of a run went through one assumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub assumption: Assumption,
    /// Passing tests that ran through it: the flagged passes.
    pub flagged: usize,
    /// Failing and not-run tests that ran through it.
    pub other: usize,
}

/// The assumptions of a run ranked by flagged passes (then by the other tests, then by id). Only entries a
/// test hit are listed. `tests` gives, per test, whether it passed and the assumptions it hit.
pub fn rank<'a>(tests: impl IntoIterator<Item = (bool, &'a [Assumption])>) -> Vec<Row> {
    let mut counts: BTreeMap<Assumption, (usize, usize)> = BTreeMap::new();
    for (passed, hits) in tests {
        for a in hits {
            let e = counts.entry(*a).or_default();
            if passed {
                e.0 += 1;
            } else {
                e.1 += 1;
            }
        }
    }
    let mut rows: Vec<Row> = counts
        .into_iter()
        .map(|(assumption, (flagged, other))| Row {
            assumption,
            flagged,
            other,
        })
        .collect();
    rows.sort_by(|a, b| {
        b.flagged
            .cmp(&a.flagged)
            .then(b.other.cmp(&a.other))
            .then(a.assumption.info().id.cmp(b.assumption.info().id))
    });
    rows
}

/// `--top-assumptions`: the ranking as text, `n` rows at most.
pub fn render_top(rows: &[Row], n: usize) -> String {
    if rows.is_empty() {
        return "No test ran through an unprobed assumption.".to_string();
    }
    let flagged: usize = rows.iter().map(|r| r.flagged).sum();
    let mut s = format!(
        "Assumptions that tests ran through ({} entries; a probe for the top ones turns the most flagged passes into clean ones; {} test-assumption pairs among passes):\n",
        rows.len(),
        flagged
    );
    for r in rows.iter().take(n) {
        let i = r.assumption.info();
        s.push_str(&format!(
            "  {:>6} flagged passes, {:>5} other tests  {}\n      assumed: {}\n      probe wanted: {}\n",
            r.flagged, r.other, i.id, i.assumed, i.probe
        ));
    }
    if rows.len() > n {
        s.push_str(&format!(
            "  ... {} more (--top-assumptions N shows more)\n",
            rows.len() - n
        ));
    }
    s
}

/// The ids of a test's assumptions, comma separated.
pub fn id_list(hits: &[Assumption]) -> String {
    hits.iter().map(|a| a.info().id).collect::<Vec<_>>().join(",")
}

/// `--explain`: the assumptions a test ran through (al-runner issue 8232). A test that was not run lists their ids only. A flagged pass gives each id with what it assumes; a failure also the probe wanted and
/// the note. `--list-assumptions` has the whole entry.
pub fn render_explain(hits: &[Assumption], outcome: &crate::Outcome) -> String {
    let mut s = match outcome {
        crate::Outcome::NotRun(_) => {
            return format!(
                "Assumptions this test ran through before it was refused (--list-assumptions describes them): {}\n",
                hits.iter().map(|a| a.info().id).collect::<Vec<_>>().join(", ")
            );
        }
        crate::Outcome::Pass => String::from(
            "FLAGGED PASS: this test passed through code that rests on unprobed assumptions (--list-assumptions gives \
             the probe wanted):\n",
        ),
        crate::Outcome::Fail(_) => String::from("This test ran through code that rests on unprobed assumptions:\n"),
    };
    for a in hits {
        let i = a.info();
        s.push_str(&format!("  {}: {}\n", i.id, i.assumed));
        if matches!(outcome, crate::Outcome::Fail(_)) {
            s.push_str(&format!("      probe wanted: {}\n", i.probe));
            if let Some(n) = i.note {
                s.push_str(&format!("      NOTE: {n}\n"));
            }
        }
    }
    s
}

/// The file the last run leaves its counts in.
pub fn hits_path(cache_root: &Path) -> PathBuf {
    cache_root.join("assumption-hits.tsv")
}

/// Writes the counts of this run (`id<TAB>flagged<TAB>other`). A run that cannot write it says nothing: the
/// file only feeds `--list-assumptions`.
pub fn save_hits(cache_root: &Path, rows: &[Row]) {
    let mut s = String::new();
    for r in rows {
        s.push_str(&format!("{}\t{}\t{}\n", r.assumption.info().id, r.flagged, r.other));
    }
    let _ = std::fs::create_dir_all(cache_root);
    let _ = std::fs::write(hits_path(cache_root), s);
}

/// The counts of the last run that wrote them, by id.
pub fn load_hits(cache_root: &Path) -> BTreeMap<String, (usize, usize)> {
    let mut out = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(hits_path(cache_root)) else {
        return out;
    };
    for line in text.lines() {
        let mut p = line.split('\t');
        if let (Some(id), Some(f), Some(o)) = (p.next(), p.next(), p.next()) {
            if let (Ok(f), Ok(o)) = (f.parse(), o.parse()) {
                out.insert(id.to_string(), (f, o));
            }
        }
    }
    out
}

/// `--list-assumptions`: the registry with the counts of the last run.
pub fn list(cache_root: &Path) -> String {
    let hits = load_hits(cache_root);
    assumptions::listing(&|id| hits.get(id).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_counts_flagged_passes_and_other_tests_and_orders_by_flagged() {
        let a = [
            Assumption::TextDigitsToInteger,
            Assumption::CodeCoverageSnapshotWhileLogging,
        ];
        let b = [Assumption::CodeCoverageSnapshotWhileLogging];
        let c: [Assumption; 0] = [];
        let rows = rank([(true, &a[..]), (true, &b[..]), (false, &b[..]), (true, &c[..])]);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0].assumption, rows[0].flagged, rows[0].other),
            (Assumption::CodeCoverageSnapshotWhileLogging, 2, 1)
        );
        assert_eq!(
            (rows[1].assumption, rows[1].flagged, rows[1].other),
            (Assumption::TextDigitsToInteger, 1, 0)
        );
        let text = render_top(&rows, 1);
        assert!(
            text.contains("code-coverage-snapshot-while-logging") && !text.contains("text-digits-to-integer"),
            "{text}"
        );
        assert!(text.contains("... 1 more"), "{text}");
        assert_eq!(render_top(&[], 5), "No test ran through an unprobed assumption.");
    }

    #[test]
    fn id_list_joins_the_ids() {
        assert_eq!(
            id_list(&[
                Assumption::TextDigitsToInteger,
                Assumption::CodeCoverageSnapshotWhileLogging
            ]),
            "text-digits-to-integer,code-coverage-snapshot-while-logging"
        );
    }

    // al-runner issue 8232: a test that was not run lists only the ids of its assumptions.
    #[test]
    fn explain_of_a_test_not_run_lists_only_the_assumption_ids() {
        let hits = [
            Assumption::TextDigitsToInteger,
            Assumption::CodeCoverageSnapshotWhileLogging,
        ];
        let text = render_explain(&hits, &crate::Outcome::NotRun("refused".into()));
        assert_eq!(
            text,
            "Assumptions this test ran through before it was refused (--list-assumptions describes them): \
             text-digits-to-integer, code-coverage-snapshot-while-logging\n"
        );
    }

    // A flagged pass keeps each id with what it assumes; the probe wanted is left to --list-assumptions.
    #[test]
    fn explain_of_a_flagged_pass_gives_each_id_with_what_it_assumes() {
        let text = render_explain(&[Assumption::TextDigitsToInteger], &crate::Outcome::Pass);
        let i = Assumption::TextDigitsToInteger.info();
        assert!(text.starts_with("FLAGGED PASS: "), "{text}");
        assert!(
            text.contains(&format!("  text-digits-to-integer: {}\n", i.assumed)),
            "{text}"
        );
        assert!(!text.contains("probe wanted:"), "{text}");
    }

    // Neighbor: a failure keeps main's full text, the probe wanted included.
    #[test]
    fn explain_of_a_failure_keeps_the_probe_wanted() {
        let text = render_explain(&[Assumption::TextDigitsToInteger], &crate::Outcome::Fail("boom".into()));
        let i = Assumption::TextDigitsToInteger.info();
        assert_eq!(
            text,
            format!(
                "This test ran through code that rests on unprobed assumptions:\n  text-digits-to-integer: {}\n      probe wanted: {}\n",
                i.assumed, i.probe
            )
        );
    }

    #[test]
    fn the_counts_of_a_run_are_read_back() {
        let dir = std::env::temp_dir().join(format!("al-flagged-{}", std::process::id()));
        let rows = [Row {
            assumption: Assumption::TextDigitsToInteger,
            flagged: 3,
            other: 1,
        }];
        save_hits(&dir, &rows);
        let loaded = load_hits(&dir);
        assert_eq!(loaded.get("text-digits-to-integer"), Some(&(3, 1)));
        let text = list(&dir);
        assert!(
            text.contains("text-digits-to-integer  (3 flagged passes, 1 other tests)"),
            "{text}"
        );
        assert!(
            text.contains("code-coverage-snapshot-while-logging  (not hit in the last run)"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
