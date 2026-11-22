//! The cells of corpus pull request 1495 for al-runner issue 8637 (tests/al-language/pages/background-tasks/BgTaskQueryCache.al,
//! BQ Tests 4 to 9), the reporter's repro and the neighbors, run here until a bump pins the probe: the group reads of page
//! background tasks through the query cache (`al-vm` query_cache.rs). The AL is the neighbor app's
//! (tests/neighbors/8637-bg-cache-filtered-reads), one copy. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, BackgroundTaskQueryCache, BgTaskWorkerFilterGroupOne};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/Neighbors.al");
const REPRO: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/Repro.al");
const MORE: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/MoreNeighbors.al");

fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!("{src}\n{}", include_str!("../fixtures/outside-sc/Assert.al"));
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

/// The cells of BQ Tests 4 (corpus PR 1495, BgTaskQueryCache.al:1585-2113): every one passes. Those whose answer the cache
/// gives where the table holds another are flagged; a commit that ended an answer, the test session's own read after its write,
/// Find('+'), Find('-'), FindSet, a find after SetLoadFields and the first read after the test page's read see the table, clean.
#[test]
fn issue_8637_cells() {
    let o = outcomes(PROBE, "BQ Tests 4");
    assert_eq!(o.len(), 36);
    let clean = [
        "FindLast_ChangeCommitted_SeesTheChange",
        "FindLast_ChangeCommitted_TestReadSeesIt",
        "FindLast_TestWrote_TestReadSeesIt",
        "FindPlus_ChangedRow_SeesTheChange",
        "FindMinus_ChangedRow_SeesTheChange",
        "FindSet_ChangedRow_SeesTheChange",
        "FindFirstLoadFields_ChangedRow_SeesTheChange",
        "FindLastLoadFields_ChangedRow_SeesTheChange",
        "TestWroteThenFindLast_FirstReadSeesTheChange",
        "PageFilter_ChangedRow_FirstReadSeesTheChange",
    ];
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let short = name.trim_start_matches("BgQueryCache_Run_");
        assert_eq!(
            marks.contains(&BackgroundTaskQueryCache),
            !clean.contains(&short),
            "{name}: {marks:?}"
        );
    }
}

/// The reporter's repro of al-runner issue 8637: FindLast, IsEmpty and FindFirst after a SetFilter, repeated after a change,
/// answer their first answer (corpus PR 1495 :1238, :1255, :1274), flagged.
#[test]
fn issue_8637_repro() {
    let o = outcomes(REPRO, "Repro Cache Read Tests");
    assert_eq!(o.len(), 3);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// Neighbors of al-runner issue 8637 past what the cells measure (the fixture's header): 100001 filters, a FlowField filter
/// next to the key set to its value in three filter groups, the FlowField filters @k, <>K* and K*|X, a sum whose field() is a
/// FlowField of the row, and a worker's fresh Record and RecordRef Count while the test session holds a filter in filter
/// group 1 (:3391 measures the worker's own record setting it while the test session holds none). BC answers them from the
/// cache but @k, which reads the rows as they are; the runner keeps main's answer, the rows as they are, with no flag. The
/// worker's Count runs without the test session's group 1 (assumption `bg-task-worker-filter-group-one`), so it reads with
/// no filter, a measured shape, and answers from the cache as BC does, through both entries.
#[test]
fn issue_8637_neighbors() {
    let src = format!("{PROBE}\n{}", NEIGHBORS);
    let o = outcomes(&src, "BQ Gate Tests");
    assert_eq!(o.len(), 6);
    for (name, r, marks) in &o {
        if name.starts_with("TestSessionGroup1") {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(
                marks.contains(&BackgroundTaskQueryCache) && marks.contains(&BgTaskWorkerFilterGroupOne),
                "{name}: {marks:?}"
            );
            continue;
        }
        assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
        let main = match name.as_str() {
            n if n.starts_with("FlowFieldCaseInsensitiveValue") => {
                assert_eq!(*r, Outcome::Pass, "{name}");
                continue;
            }
            n if n.starts_with("FlowFieldPatternNotMatched") => "Actual:<a|a|1|No / -|-|0|Yes>",
            _ => "Actual:<-|-|0|Yes / a|a|1|No>",
        };
        assert!(matches!(r, Outcome::Fail(m) if m.contains(main)), "{name}: {r:?}");
    }
}

/// The cells of BQ Tests 6 (corpus PR 1495, BgTaskQueryCache.al:2587-2810): five and eight filters, filter groups 3, 5 and
/// 7, the OR group, compound filters, @ alone, an empty FILTER() of a view, a pattern not matched with @ and no filter answer from the cache, as do the
/// test session's reads first in every shape, flagged; Count and IsEmpty after SetLoadFields read the table, flagged too
/// (the entry's open probe measures that read).
#[test]
fn issue_8637_more_cells() {
    let o = outcomes(PROBE, "BQ Tests 6");
    assert_eq!(o.len(), 11);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 7 (corpus PR 1495, BgTaskQueryCache.al:2812-3567): 9 to 7650 filters, filter groups 1 and 8-255,
/// FindFirst and FindLast with a FlowField filter next to the key, and a FlowFilter filter answer from the cache, as do the test
/// session's reads first, flagged. A worker's filter in filter group 1 ends with it (:3400): the later reads of its table run
/// through main's `bg-task-worker-filter-group-one`, and those the cache does not answer through it alone. A FlowField filter alone or without the key reads the table, and <>@ alone keeps
/// no line, as on main.
#[test]
fn issue_8637_bq7_cells() {
    let o = outcomes(PROBE, "BQ Tests 7");
    assert_eq!(o.len(), 17);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&BackgroundTaskQueryCache) || marks.contains(&BgTaskWorkerFilterGroupOne),
            "{name}: {marks:?}"
        );
    }
}

/// FlowFields that read more tables than their CalcFormula's own: a count whose condition is a FlowField of the lines, and a
/// sum whose `field()` is a FlowField of the row. A worker's FindFirst with a filter on one next to its key, after the test
/// session committed a row only that other FlowField reads: no cell measures whether the commit ends the query cache's
/// entry, so the runner reads the rows as they are, as on main, with no flag.
#[test]
fn issue_8637_indirect_flowfields() {
    let o = outcomes(
        include_str!("../fixtures/outside-fb/Issue8637IndirectFlowField.al"),
        "IX Tests",
    );
    assert_eq!(o.len(), 2);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 8 (corpus PR 1495, BgTaskQueryCache.al:3568-4273): up to 26214 filters, and FindFirst and FindLast
/// with FlowField filters of any kind, group and method next to one value on each key field in any filter group but the OR
/// group, answer from the cache, flagged. The key in the OR group or as two values, and a commit of the counted table, read
/// the table; after the worker's key filter in filter group 1 (:4226) a later read of its table runs through main's
/// `bg-task-worker-filter-group-one`.
#[test]
fn issue_8637_bq8_cells() {
    let o = outcomes(PROBE, "BQ Tests 8");
    assert_eq!(o.len(), 9);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let flagged = match name.as_str() {
            "BgQueryCache_Run_FlowFieldMin_CommitOfTheLine_SeesTheChange" => false,
            "BgQueryCache_Run_FlowFieldWithoutOneKeyValue_SeesTheChange" => {
                assert!(marks.contains(&BgTaskWorkerFilterGroupOne), "{name}: {marks:?}");
                false
            }
            _ => true,
        };
        assert_eq!(marks.contains(&BackgroundTaskQueryCache), flagged, "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 8b (corpus PR 1495, BgTaskQueryCache.al:4275-4804): FindFirst and FindLast with a FlowField filter
/// next to the key split across filter groups, of the kinds >=, &, < and <=, in filter groups 1-255, next to a filter outside
/// the key in the OR group or filter group 1 or 255, on a lookup filtered on its own FlowFilter or a FlowField of a table
/// extension, or in the OR group, answer from the cache, flagged. A wildcard on the FlowField, and a FlowField whose condition
/// is a FlowField of its source table, read the table, clean. An empty FILTER() of a view on a Code field is refused before
/// any read, on main too (:4800).
#[test]
fn issue_8637_bq8b_cells() {
    let o = outcomes(PROBE, "BQ Tests 8b");
    assert_eq!(o.len(), 8);
    let clean = [
        "FlowFieldWildcard_SeesTheChange",
        "FlowFieldOverAFlowField_SeesTheChange",
    ];
    for (name, r, marks) in &o {
        let short = name.trim_start_matches("BgQueryCache_Run_");
        if short == "FlowFieldEmptyFilter_DescriptionChanged" {
            assert!(
                matches!(r, Outcome::NotRun(w) if w.contains("empty filter term")),
                "{name}: {r:?}"
            );
            continue;
        }
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BackgroundTaskQueryCache),
            !clean.contains(&short),
            "{name}: {marks:?}"
        );
    }
}

/// The cells of BQ Tests 9 (corpus PR 1495, BgTaskQueryCache.al:4806-5635) but :5564 (`issue_8637_heavy`): up to 60000
/// filters, the key set twice, and
/// FindFirst and FindLast with FlowField filters of open ranges, an & inside a | and case-sensitive wildcards next to the key
/// answer from the cache, flagged. `@` alone on a lookup FlowField (:5609) and on a Code field (:5616) stay refused, as on
/// main: the "@ alone" pull request answers them.
#[test]
fn issue_8637_bq9_cells() {
    let o = outcomes(PROBE, "BQ Tests 9");
    assert_eq!(o.len(), 7);
    for (name, r, marks) in &o {
        if name.contains("AtSignAlone") {
            assert!(
                matches!(r, Outcome::NotRun(w) if w.contains("value expected after @")),
                "{name}: {r:?}"
            );
            continue;
        }
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 5 (corpus PR 1495, BgTaskQueryCache.al:2191-2371). A test page's read of a line fills the row cache,
/// as the test session's Get does, through filters on every key field (:2262, :2284) and through a filter on the group alone
/// (:2317; the page reads its line by the full key). The test session's four reads first answer the worker's first four reads
/// for every shape of :2337-:2367. All flagged.
#[test]
fn issue_8637_review_cells() {
    let o = outcomes(PROBE, "BQ Tests 5");
    assert_eq!(o.len(), 9);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cell of 100000 filters (BgTaskQueryCache.al:5564) and the neighbor of 100001 filters, from the fixture
/// Issue8637Heavy.al: main's al-runner runs each past the neighbor check's CPU limit on the CI runners. 100000 filters answer
/// from the cache, flagged; 100001 filters read the rows as they are, as on main, with no flag (BC answers from the cache).
#[test]
fn issue_8637_heavy() {
    let src = format!(
        "{PROBE}\n{NEIGHBORS}\n{}",
        include_str!("../fixtures/outside-fb/Issue8637Heavy.al")
    );
    let o = outcomes(&src, "BQ Heavy Tests");
    assert_eq!(o.len(), 2);
    for (name, r, marks) in &o {
        if name.starts_with("Filters100001") {
            assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
            assert!(
                matches!(r, Outcome::Fail(m) if m.contains("Actual:<-|-|0|Yes / a|a|1|No>")),
                "{name}: {r:?}"
            );
        } else {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
        }
    }
}

/// The cells of BQ Tests 13 (corpus PR 1495 at d0497714, BgTaskQueryCache.al:7106-7322, the review of al-runner PR 8490),
/// each green 3 of 3 on BC 29.1: with every read in one worker codeunit the Get after SelectLatestVersion reads the table and
/// the next Get answers it (:7282); a table with a MediaSet field answers the first read (:7300). Both pass, flagged. The
/// CalcFields after SetLoadFields (:7255) is al-runner issue 8677's: here a worker computes it again, as main does.
#[test]
fn issue_8637_bq13_cells() {
    let o = outcomes(PROBE, "BQ Tests 13");
    assert_eq!(o.len(), 3);
    for (name, r, marks) in &o {
        if name.starts_with("BgCalcFields") {
            assert!(
                matches!(r, Outcome::Fail(m) if m.contains("Actual:<1 / 3>")),
                "{name}: {r:?}"
            );
            assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
            continue;
        }
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// Neighbors from the reviews of al-runner PRs 8656 and 8682 (MoreNeighbors.al; BC answers each from the cache, a local BC
/// 29.1 container, 3 runs). A worker's Count of a group ascending twice answers the first read, flagged; ascending then
/// descending reads the table here (the entry is keyed by the sort direction, BC shares it), as on main. A count and an exist
/// over a count FlowField of the row, a CalcSums with no filter, and a CalcFields and a CalcSums after SetAscending(false)
/// compute again, as on main: a page background task's CalcFields and CalcSums are al-runner issue 8677's.
#[test]
fn issue_8637_more_neighbors() {
    let src = format!("{PROBE}\n{MORE}");
    let o = outcomes(&src, "BQ More Tests");
    assert_eq!(o.len(), 7);
    for (name, r, marks) in &o {
        if name.starts_with("CountAscendingTwice") {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
            continue;
        }
        assert!(matches!(r, Outcome::Fail(_)), "{name}: {r:?}");
        assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}
