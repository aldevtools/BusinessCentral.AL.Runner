//! The cells of corpus pull request 1495 for al-runner issues 8637 and 8677 (tests/al-language/pages/background-tasks/
//! BgTaskQueryCache.al, BQ Tests 4 to 10), the reporters' repros and the neighbors, run here until a bump pins the probe: the
//! group reads and calculations of page background tasks through the query cache (`al-vm` query_cache.rs). The AL is the
//! neighbor apps' (tests/neighbors/8637-bg-cache-filtered-reads and 8677-bg-cache-calcfields), one copy. Delete this file
//! when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, BackgroundTaskQueryCache, BgTaskWorkerFilterGroupOne};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/Neighbors.al");
const REPRO: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/Repro.al");
const CALC_NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8677-bg-cache-calcfields/src/Neighbors.al");
const CALC_REPRO: &str = include_str!("../../../../tests/neighbors/8677-bg-cache-calcfields/src/Repro.al");
const CALC_PROBE: &str = include_str!("../../../../tests/neighbors/8677-bg-cache-calcfields/src/CalcProbe.al");
const MORE: &str = include_str!("../../../../tests/neighbors/8637-bg-cache-filtered-reads/src/MoreNeighbors.al");
const EDGES: &str = include_str!("../../../../tests/neighbors/8677-bg-cache-calcfields/src/EdgesProbe.al");

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
/// answer from the cache, flagged. `@` alone on a lookup FlowField (:5609) and on a Code field (:5616) run through the "@ alone"
/// rule (al-builtins `filter_at_sign`), flagged by its assumption: the worker's FindFirst and FindLast check the row, its Count
/// and IsEmpty read the database.
#[test]
fn issue_8637_bq9_cells() {
    let o = outcomes(PROBE, "BQ Tests 9");
    assert_eq!(o.len(), 7);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let mark = if name.contains("AtSignAlone") {
            al_builtins::assumptions::Assumption::AtSignAloneDatabaseBlank
        } else {
            BackgroundTaskQueryCache
        };
        assert!(marks.contains(&mark), "{name}: {marks:?}");
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

/// The reporter's repro of al-runner issue 8677 (green on real BC 3/3): a worker's CalcFields again after the test session's
/// uncommitted insert answers its first value, flagged.
#[test]
fn issue_8677_repro() {
    let o = outcomes(CALC_REPRO, "Repro Calc Worker Tests");
    assert_eq!(o.len(), 1);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// Neighbors of al-runner issue 8677 (the neighbor app's Neighbors.al), each asserting BC's answer: BC answers each from the
/// cache, and so does the runner now, flagged, as corpus PR 1533 measures each shape (al-runner issue 8803): a negated sum,
/// FieldRef.CalcField, a FlowFilter range, a filter on a stored field, a CalcSums with a `>`, a count over a sum FlowField
/// of the row, a temporary record, a field(filter()) of a stored field set to A and then to B in one worker run (keyed by its
/// value: 5,0 / 5,0), and SetLoadFields, Get, CalcFields (:7255). From the review of al-runner PR 8872, one per fact the cache
/// took past the first cells: a `|` of values on a stored field (BgTaskCalcCache.al:534), a CalcFields and a CalcSums ascending
/// first and descending again (:540, :546), a count at a sum whose field(filter()) reads a stored field (:552), and a
/// CalcFields of a temporary record a Variant got from another Variant (its table's rows are marked not measured; main and
/// BC answer, the runner refused it before it read such a CalcFields through the database store). (The exist over Tenant
/// Media, Platform.al, needs the platform tables: the neighbor check runs it.)
#[test]
fn issue_8677_neighbors() {
    let o = outcomes(CALC_NEIGHBORS, "BQN Tests");
    assert_eq!(o.len(), 14);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 10 (corpus PR 1495, BgTaskQueryCache.al:5833-6290, al-runner issue 8677): a worker's CalcFields of
/// one FlowField of every method or of several, with a FlowFilter or a sum whose field() is a FlowField of the row, and its
/// CalcSums answer the first answer, flagged; the test session's CalcFields before its insert answers the worker's first
/// one, flagged. Another set or order of fields, another FlowFilter value, a commit, SetAutoCalcFields and a count over a
/// FlowField of its source read the table, clean.
#[test]
fn issue_8677_bq10_cells() {
    let o = outcomes(PROBE, "BQ Tests 10");
    assert_eq!(o.len(), 24);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BackgroundTaskQueryCache),
            name.ends_with("_FirstAnswer"),
            "{name}: {marks:?}"
        );
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

/// The cells of BQ Tests 13 (corpus PR 1495, BgTaskQueryCache.al:7106-7322, reviews of al-runner PRs 8490 and 8682), each
/// green 3 of 3 on BC 29.1: a worker's CalcFields after SetLoadFields and Get answers its first answer (:7255); with every
/// read in one worker codeunit the Get after SelectLatestVersion reads the table and the next Get answers it (:7282); a
/// table with a MediaSet field answers the first read (:7300). All flagged.
#[test]
fn issue_8677_bq13_cells() {
    let o = outcomes(PROBE, "BQ Tests 13");
    assert_eq!(o.len(), 3);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of corpus PR 1533 (BgTaskCalcCache.al at fe7a324c, al-runner issue 8803), each green 3 of 3 on BC 29.1, but the
/// two Tenant Media cells (CalcProbe.al leaves them out; Platform.al has the neighbor): a worker's repeated CalcFields and
/// CalcSums answer the first answer for a negated sum, FieldRef.CalcField, a FlowFilter range, a filter on a stored field, a
/// CalcSums with a `>` or with no filter, a count over a sum FlowField of the row, a temporary record, a field(filter()) of a
/// stored field, a count, min, exist and lookup over a count FlowField of the row, after SetAscending(false), a `|` of values
/// on a stored field (:534), a CalcFields and a CalcSums ascending first and descending again (:540, :546), and a count over a
/// sum FlowField whose field(filter()) reads a stored field (:552). All pass, flagged.
#[test]
fn issue_8803_cells() {
    let o = outcomes(CALC_PROBE, "BCC Tests");
    assert_eq!(o.len(), 19);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The Count of a group read ascending and then descending (corpus PR 1533, BgTaskCalcCache.al:613, green 3 of 3 on BC 29.1):
/// BC answers the first answer, one entry for both sort directions, and so does the runner, flagged.
#[test]
fn issue_8803_count_both_directions() {
    let o = outcomes(CALC_PROBE, "BCC Count Tests");
    assert_eq!(o.len(), 1);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// Neighbors from the reviews of al-runner PRs 8656 and 8682 (MoreNeighbors.al; BC answers each from the cache, a local BC
/// 29.1 container, 3 runs). Each answers from the cache, flagged: a worker's Count ascending twice, a count and an exist over a
/// count FlowField of the row and a CalcSums with no filter (with #8682), and a Count ascending then descending and a
/// CalcFields and a CalcSums after SetAscending(false) (BgTaskCalcCache.al:613, :522, :528, open corpus PR 1533).
#[test]
fn issue_8637_more_neighbors() {
    let src = format!("{PROBE}\n{MORE}");
    let o = outcomes(&src, "BQ More Tests");
    assert_eq!(o.len(), 7);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// Neighbors from the review of al-runner PR 8808 (al-runner issue 8879; MoreNeighbors.al, "BQ More Follow-up Tests", a local BC
/// 29.1 container, 3 runs). A FindFirst with filters on two FlowFields of different source tables answers the first answer,
/// flagged, with the filters set in the same order and in the other order (`flow_field_sources` sorts the tables). A worker's
/// SelectLatestVersion with an Option, a Char, a Byte or an Enum of value 0, which names no table, keeps the row entry, as on
/// BC: the Get answers the first read, flagged.
#[test]
fn issue_8879_follow_up_neighbors() {
    let src = format!("{PROBE}\n{MORE}");
    let o = outcomes(&src, "BQ More Follow-up Tests");
    assert_eq!(o.len(), 6);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of corpus PR 1558 (BgTaskCacheEdges.al, corpus issue 1555), each green 3 of 3 on BC 29.1. The worker answers its
/// first answer for a CalcFields, a CalcSums and a Count descending first and ascending again (:374-:386) and the test
/// session's answer after the test session read first (:392, :398); for a count at an inner count with field(filter()) of a
/// stored field (:404), a CalcSums with a `>` on another field and with two (:410, :416), FieldRef.CalcField of a temporary
/// RecordRef (:422), a CalcFields after a commit of another row of its row's table or of the row itself, of a temporary record
/// too (:428-:446), and two FlowField filters in either order (:458, :464). SelectLatestVersion with an Enum whose value is
/// the cached table's id ends the row entry (:452): the Get reads the table. From the review of al-runner PR 8897 (BCE Commit
/// Tests): a commit of a ModifyAll, a DeleteAll or a Delete of another row of the row's table keeps a CalcFields entry (:639,
/// :645, :651), so does the test session's CalcFields first, then a commit, then the lines (:657), and FieldRef.CalcField of a
/// database RecordRef after a commit (:681); SelectLatestVersion with an Option, a Char, a Byte or an Enum of value 0 keeps the
/// row entry (:687-:705). BCE Row Commit Tests: a commit of a ModifyAll, a DeleteAll or a Delete of the row itself keeps the
/// entry (:915-:927), so do a commit of a Truncate of the row's table (:958) and SelectLatestVersion with an Option, a Char, a
/// Byte or an Enum of value 2, which names no table (:933-:951). All flagged.
#[test]
fn issue_1555_cells() {
    for (codeunit, n) in [("BCE Tests", 16), ("BCE Commit Tests", 9), ("BCE Row Commit Tests", 8)] {
        let o = outcomes(EDGES, codeunit);
        assert_eq!(o.len(), n, "{codeunit}");
        for (name, r, marks) in &o {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
        }
    }
}

/// Neighbors from the review of al-runner PR 8897 (Neighbors.al, "BQN Commit Tests", a local BC 29.1 container, 3 runs): the
/// cells' commits of the row's table and FieldRef.CalcField after one keep the entry, flagged; a FlowField whose formula reads
/// the row's own table ends with a commit of it and reads the table, clean.
#[test]
fn issue_1555_neighbors() {
    let o = outcomes(CALC_NEIGHBORS, "BQN Commit Tests");
    assert_eq!(o.len(), 6);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let own_table = name.starts_with("FlowFieldOverItsOwnTable");
        assert_eq!(
            marks.contains(&BackgroundTaskQueryCache),
            !own_table,
            "{name}: {marks:?}"
        );
    }
}

/// A worker's SelectLatestVersion with a Char, a Byte or an Option that names the cached table (table 200) is refused: no cell
/// measures one (an Enum that does is :452 of BgTaskCacheEdges.al; each of the four of value 0, :687-:705, keeps the entry).
#[test]
fn issue_1555_char_byte_option_naming_a_table_are_refused() {
    const SRC: &str = r#"
table 200 "SLV Small Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }
    keys { key(PK; "No.") { } }
}

page 50141 "SLV Small Rows"
{
    PageType = List;
    SourceTable = "SLV Small Row";
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
}

codeunit 50142 "SLV Small Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Row: Record "SLV Small Row";
        C: Char;
        B: Byte;
        O: Option A,B;
    begin
        Parameters := Page.GetBackgroundParameters();
        case Parameters.Get('Mode') of
            'char':
                begin
                    C := 200;
                    Database.SelectLatestVersion(C);
                end;
            'byte':
                begin
                    B := 200;
                    Database.SelectLatestVersion(B);
                end;
            'option':
                begin
                    O := 200;
                    Database.SelectLatestVersion(O);
                end;
        end;
        Row.Get('A');
        Results.Add('Seen', Row.Description);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50143 "SLV Small Tests"
{
    Subtype = Test;

    local procedure Run(var Rows: TestPage "SLV Small Rows"; Mode: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"SLV Small Reader", Parameters, false);
        exit(Results.Get('Seen'));
    end;

    local procedure Select(Mode: Text)
    var
        Rows: TestPage "SLV Small Rows";
        Row: Record "SLV Small Row";
    begin
        Row."No." := 'A';
        Row.Description := 'first';
        if not Row.Insert() then
            Row.Modify();
        Commit();
        Rows.OpenView();
        Run(Rows, 'get');
        Row.Get('A');
        Row.Description := 'second';
        Row.Modify();
        Run(Rows, Mode);
    end;

    [Test]
    procedure CharNamingTheTable()
    begin
        Select('char');
    end;

    [Test]
    procedure ByteNamingTheTable()
    begin
        Select('byte');
    end;

    [Test]
    procedure OptionNamingTheTable()
    begin
        Select('option');
    end;
}
"#;
    let o = outcomes(SRC, "SLV Small Tests");
    assert_eq!(o.len(), 3);
    for (name, r, _) in &o {
        assert!(
            matches!(r, Outcome::NotRun(w) if w.starts_with("SelectLatestVersion(Table) in a page background task with a Char")),
            "{name}: {r:?}"
        );
    }
}
