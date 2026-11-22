//! The corpus probe of al-runner issue 8467 (corpus pull request 1457, merged, not pinned yet: tests/al-language/pages/background-tasks/
//! BgTaskQueryCache.al, measured on BC 29.1, 34 of 34 green; and the cells of open corpus pull request 1495 after it, 76 of 76)
//! and the reporters' repros of al-runner issues 8467 and 8637, run here until a bump pins the probe: the
//! query cache page background tasks and the test session read through (`al-vm` query_cache.rs). Delete this file and the
//! fixtures when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, BackgroundTaskQueryCache};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../fixtures/outside-fb/Issue8467Probe.al");

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

/// The probe's tests: those whose answer the cache changes pass, flagged; those it does not change pass clean; the test
/// session's own read the cache answers, and a field load into a record the cache answered, are refused.
#[test]
fn issue_8467_probe_cells() {
    let mut o = outcomes(PROBE, "BQ Tests");
    o.extend(outcomes(PROBE, "BQ Tests 2"));
    assert_eq!(o.len(), 34);
    let clean = [
        "Page_FirstRead_SeesTheChange",
        "Page_Get_ChangeCommitted_SeesTheChange",
        "Page_TestWriteOnly_FirstReadSeesIt",
        "Run_Get_ModifyAllCommitted_SeesTheChange",
        "Run_GroupReads_InsertCommitted_SeeTheChange",
        "Run_GroupReads_OtherGroupCommitted_SeeTheChange",
        "Run_TestReadFirst_OtherRowChanged_FirstRead",
        "Run_TableWritten_TestRead_FirstReadSeesTheChange",
        "AutoRollback_1_ReadsTheKey",
        "OtherCodeunit_1_ReadsTheKey",
        "OtherCodeunit_2_SameKey_SeesItsRow",
    ];
    let refused = [
        (
            "Run_PartialFirst_FullLater_FirstAnswer",
            "a field SetLoadFields left out",
        ),
        ("AssertError_Rollback_FirstAnswer", "a read of the test session"),
    ];
    for (name, r, marks) in &o {
        let short = name.trim_start_matches("BgQueryCache_");
        if let Some((_, why)) = refused.iter().find(|(n, _)| *n == short) {
            assert!(matches!(r, Outcome::NotRun(w) if w.contains(why)), "{name}: {r:?}");
        } else if clean.contains(&short) {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
        } else {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
        }
    }
}

/// The cells of corpus pull request 1495 (BgTaskQueryCache.al:1207-1326): a Get after commits that inserted and deleted the row
/// reads the table, clean; a copy of a record a partial Get answered from the cache stays refused. The worker's first Count
/// after the test counted the group and then inserted answers the test's count (:1239), and FindLast and IsEmpty after a
/// SetRange of the group, and FindFirst after a SetFilter of it to one value, repeated after a change, answer their first answer
/// (:1276, :1293, :1312), flagged.
#[test]
fn issue_8467_neighbor_cells() {
    let o = outcomes(PROBE, "BQ Tests 3");
    let get = |n: &str| {
        o.iter()
            .find(|(name, _, _)| name == &format!("BgQueryCache_Run_{n}"))
            .expect("the test")
    };
    let (_, r, marks) = get("Get_RolledBack_InsertDeleteCommitted_SeesTheTable");
    assert_eq!(*r, Outcome::Pass);
    assert!(!marks.contains(&BackgroundTaskQueryCache), "{marks:?}");
    let (_, r, _) = get("PartialGet_Copy_FirstAnswer");
    assert!(
        matches!(r, Outcome::NotRun(w) if w.contains("a field SetLoadFields left out")),
        "{r:?}"
    );
    for name in [
        "GroupCount_TestCountThenInsert_FirstRead",
        "FindLast_ChangedRow_FirstAnswer",
        "IsEmpty_Insert_FirstAnswer",
        "SetFilterFindFirst_ChangedRow_FirstAnswer",
    ] {
        let (_, r, marks) = get(name);
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 4 (corpus PR 1495, BgTaskQueryCache.al:1623-2151): every one passes. Those whose answer the cache
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

/// The reporter's repro: the second background read of row A answers the first; the control sees the change.
#[test]
fn issue_8467_repro() {
    let o = outcomes(
        include_str!("../fixtures/outside-fb/Issue8467.al"),
        "Repro Bg Read Tests",
    );
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BackgroundTaskQueryCache),
            name.starts_with("SecondRead"),
            "{name}: {marks:?}"
        );
    }
}

/// Neighbors the probe does not measure, on the probe's table and page. Refused: a field load into a copy of a record the
/// cache answered; a row read again after a commit of a ModifyAll of another row (the probe's cell for it differs between BC
/// versions). A group read of a worker after the test session read that group and then wrote the table answers the test's
/// read (corpus PR 1495, BgTaskQueryCache.al:1239). A row read again
/// after commits that wrote the row while no commit held it before or after reads the table (corpus PR 1495,
/// BgTaskQueryCache.al:1207). FindLast, IsEmpty and a SetFilter to one value answer their first answer, flagged (corpus PR
/// 1495 :1276-1312). Reading the rows as they are (no flag), as on main: a ReadIsolation read; two filter values with the same
/// first characters keep their own entries; a field load after a fetch of another row is not refused.
const NEIGHBORS: &str = r#"
codeunit 97510 "BQ Other Reader"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        case Parameters.Get('Mode') of
            'last':
                begin
                    Row.SetRange("Group", Group);
                    if Row.FindLast() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'empty':
                begin
                    Row.SetRange("Group", Group);
                    Seen := Format(Row.IsEmpty());
                end;
            'setfilter':
                begin
                    Row.SetFilter("Group", Group);
                    if Row.FindFirst() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97512 "BQ Neighbor Reader"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Copy: Record "BQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        case Parameters.Get('Mode') of
            'copynote':
                begin
                    Row.SetLoadFields(Description);
                    Row.Get(Group, 1);
                    Copy := Row;
                    Seen := Copy.Note;
                end;
            'refetchnote':
                begin
                    Row.SetLoadFields(Description);
                    Row.Get(Group, 1);
                    Row.Get(Group, 2);
                    Seen := Row.Note;
                end;
            'readcommitted':
                begin
                    Row.ReadIsolation := IsolationLevel::ReadCommitted;
                    Row.Get(Group, 1);
                    Seen := Row.Description;
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97511 "BQ Neighbor Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        BqMode: Codeunit "BQ Mode";

    local procedure Run(var Rows: TestPage "BQ Rows"; Worker: Integer; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', '0');
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Worker, Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Insert();
    end;

    local procedure OpenOn(var Rows: TestPage "BQ Rows"; Group: Code[20])
    begin
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", Group);
        Rows.First();
    end;

    local procedure AddNoted(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Note := Note;
        Row.Insert();
    end;

    local procedure Change(Group: Code[20])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, 1);
        Row.Description := 'second';
        Row.Note := 'n9';
        Row.Modify();
    end;

    local procedure RunGet(var Rows: TestPage "BQ Rows"; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', '1');
        Parameters.Add('Mode', 'get');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    [Test]
    procedure CopyOfReplayed_LoadsOmittedField()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddNoted('NB4', 1, 'first', 'n1');
        OpenOn(Rows, 'NB4');
        Assert.AreEqual('first', RunGet(Rows, 'NB4'), 'Get');
        Change('NB4');
        Assert.AreEqual('n1', Run(Rows, Codeunit::"BQ Neighbor Reader", 'copynote', 'NB4'), 'Note of the copy');
        Rows.Close();
    end;

    [Test]
    procedure FetchAfterReplay_LoadsOmittedField()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddNoted('NB5', 1, 'first', 'n1');
        AddNoted('NB5', 2, 'other', 'n2');
        OpenOn(Rows, 'NB5');
        Assert.AreEqual('first', RunGet(Rows, 'NB5'), 'Get');
        Change('NB5');
        Assert.AreEqual('n2', Run(Rows, Codeunit::"BQ Neighbor Reader", 'refetchnote', 'NB5'), 'Note of line 2');
        Rows.Close();
    end;

    [Test]
    procedure ReadIsolation_ReadsTheRows()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddNoted('NB6', 1, 'first', 'n1');
        OpenOn(Rows, 'NB6');
        Assert.AreEqual('first', RunGet(Rows, 'NB6'), 'Get');
        Change('NB6');
        Assert.AreEqual('second', Run(Rows, Codeunit::"BQ Neighbor Reader", 'readcommitted', 'NB6'), 'ReadCommitted Get');
        Rows.Close();
    end;

    [Test]
    procedure LongFilterValues_KeepTheirOwnEntries()
    var
        Rows: TestPage "BQ Rows";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
        Stem: Text[40];
    begin
        BqMode.Set('');
        Stem := 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
        AddNoted('NB7', 1, Stem + '1', '');
        AddNoted('NB7', 2, Stem + '2', '');
        AddNoted('NB7', 3, Stem + '2', '');
        OpenOn(Rows, 'NB7');
        Parameters.Add('Group', 'NB7');
        Parameters.Add('Line', '0');
        Parameters.Add('Mode', 'groupdesccount');
        Parameters.Add('Desc', Stem + '1');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        Assert.AreEqual('1', Seen, 'Count of the first');
        Parameters.Set('Desc', Stem + '2');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        Assert.AreEqual('2', Seen, 'Count of the second');
        Rows.Close();
    end;

    [Test]
    procedure RowGoneAgain_AfterCommits()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
    begin
        BqMode.Set('');
        AddNoted('NB8H', 1, 'h', '');
        Commit();
        OpenOn(Rows, 'NB8H');
        asserterror begin
            AddNoted('NB8', 1, 'gone', '');
            Assert.AreEqual('gone', RunGet(Rows, 'NB8'), 'Get');
            Error('roll back');
        end;
        AddNoted('NB8', 1, 'back', '');
        Commit();
        Row.Get('NB8', 1);
        Row.Delete();
        Commit();
        Assert.AreEqual('-', RunGet(Rows, 'NB8'), 'Get after the commits');
        Rows.Close();
    end;

    [Test]
    procedure OtherRowModifyAllCommitted_ThenChanged()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
    begin
        BqMode.Set('');
        AddNoted('NB9', 1, 'first', '');
        AddNoted('NB9', 2, 'other', '');
        Commit();
        OpenOn(Rows, 'NB9');
        Assert.AreEqual('first', RunGet(Rows, 'NB9'), 'Get');
        Row.SetRange("Group", 'NB9');
        Row.SetRange("Line No.", 2);
        Row.ModifyAll(Description, 'other2');
        Commit();
        Row.SetRange("Line No.", 1);
        Row.ModifyAll(Description, 'second');
        Assert.AreEqual('first', RunGet(Rows, 'NB9'), 'Get after the commit');
        Rows.Close();
    end;

    [Test]
    procedure GroupRead_AfterTestReadAndWrite()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
    begin
        BqMode.Set('');
        AddRow('NB1', 1, 'a');
        Commit();
        Row.SetRange("Group", 'NB1');
        Assert.AreEqual(1, Row.Count(), 'the test''s Count');
        AddRow('NB1', 2, 'b');
        OpenOn(Rows, 'NB1');
        Assert.AreEqual('1', Run(Rows, Codeunit::"BQ Reader", 'count', 'NB1'), 'Count');
        Rows.Close();
    end;

    [Test]
    procedure OtherForms_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        Other: Integer;
    begin
        BqMode.Set('');
        Other := Codeunit::"BQ Other Reader";
        AddRow('NB2', 1, 'a');
        OpenOn(Rows, 'NB2');
        Assert.AreEqual('a', Run(Rows, Other, 'last', 'NB2'), 'FindLast');
        Assert.AreEqual('Yes', Run(Rows, Other, 'empty', 'NB3'), 'IsEmpty');
        Assert.AreEqual('a', Run(Rows, Other, 'setfilter', 'NB2'), 'SetFilter FindFirst');
        Row.Get('NB2', 1);
        Row.Description := 'b';
        Row.Modify();
        AddRow('NB3', 1, 'c');
        Assert.AreEqual('a', Run(Rows, Other, 'last', 'NB2'), 'FindLast again');
        Assert.AreEqual('Yes', Run(Rows, Other, 'empty', 'NB3'), 'IsEmpty again');
        Assert.AreEqual('a', Run(Rows, Other, 'setfilter', 'NB2'), 'SetFilter FindFirst again');
        Rows.Close();
    end;
}
"#;

#[test]
fn issue_8467_unmeasured_neighbors() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "BQ Neighbor Tests");
    let get = |n: &str| o.iter().find(|(name, _, _)| name == n).expect("the test");
    let (_, r, marks) = get("GroupRead_AfterTestReadAndWrite");
    assert_eq!(*r, Outcome::Pass);
    assert!(marks.contains(&BackgroundTaskQueryCache), "{marks:?}");
    let (_, r, marks) = get("OtherForms_FirstAnswer");
    assert_eq!(*r, Outcome::Pass);
    assert!(marks.contains(&BackgroundTaskQueryCache), "{marks:?}");
    let (_, r, _) = get("CopyOfReplayed_LoadsOmittedField");
    assert!(
        matches!(r, Outcome::NotRun(w) if w.contains("a field SetLoadFields left out")),
        "{r:?}"
    );
    let (_, r, marks) = get("FetchAfterReplay_LoadsOmittedField");
    assert_eq!(*r, Outcome::Pass);
    assert!(marks.contains(&BackgroundTaskQueryCache), "{marks:?}");
    for name in ["ReadIsolation_ReadsTheRows", "LongFilterValues_KeepTheirOwnEntries"] {
        let (_, r, marks) = get(name);
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
    let (_, r, _) = get("OtherRowModifyAllCommitted_ThenChanged");
    assert!(matches!(r, Outcome::NotRun(w) if w.contains("may have ended")), "{r:?}");
    let (_, r, marks) = get("RowGoneAgain_AfterCommits");
    assert_eq!(*r, Outcome::Pass);
    assert!(!marks.contains(&BackgroundTaskQueryCache), "{marks:?}");
}

/// The reporter's repro of al-runner issue 8637: FindLast, IsEmpty and FindFirst after a SetFilter, repeated after a change,
/// answer their first answer (corpus PR 1495 :1276, :1293, :1312), flagged.
#[test]
fn issue_8637_repro() {
    let o = outcomes(
        include_str!("../fixtures/outside-fb/Issue8637.al"),
        "Repro Cache Read Tests",
    );
    assert_eq!(o.len(), 3);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// Neighbors of al-runner issue 8637 past what the cells measure (the fixture's header): 7651 filters, and a FlowField filter
/// next to the key with a filter outside the key, as a range, or with the key in filter group 5; and a worker's fresh Record
/// and RecordRef Count while the test session holds a filter in filter group 1 (:3429 measures the worker's own record
/// setting it while the test session holds none). BC answers them from the cache; the runner keeps main's answer, the rows as
/// they are, with no flag.
#[test]
fn issue_8637_neighbors() {
    let src = format!(
        "{PROBE}\n{}",
        include_str!("../fixtures/outside-fb/Issue8637Neighbors.al")
    );
    let o = outcomes(&src, "BQ Gate Tests");
    assert_eq!(o.len(), 5);
    for (name, r, marks) in &o {
        let main = if name.starts_with("TestSessionGroup1") {
            "Actual:<1,1>"
        } else {
            "Actual:<-|-|0|Yes / a|a|1|No>"
        };
        assert!(matches!(r, Outcome::Fail(m) if m.contains(main)), "{name}: {r:?}");
        assert!(!marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 6 (corpus PR 1495, BgTaskQueryCache.al:2625-2848): five and eight filters, filter groups 3, 5 and
/// 7, the OR group, compound filters, @ alone, an empty FILTER() of a view, a pattern not matched with @ and no filter answer from the cache, as do the
/// test session's reads first in every shape, flagged; Count and IsEmpty after SetLoadFields read the table, clean.
#[test]
fn issue_8637_more_cells() {
    let o = outcomes(PROBE, "BQ Tests 6");
    assert_eq!(o.len(), 11);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BackgroundTaskQueryCache),
            !name.contains("LoadFields"),
            "{name}: {marks:?}"
        );
    }
}
/// The cells of BQ Tests 7 (corpus PR 1495, BgTaskQueryCache.al:2850-3605): 9 to 7650 filters, filter groups 1 and 8-255,
/// FindFirst and FindLast with a FlowField filter next to the key, and a FlowFilter filter answer from the cache, as do the test
/// session's reads first, flagged. A worker's filter in filter group 1 ends with it (:3438): every later read of its table,
/// the test page's included, is flagged too. A FlowField filter alone or without the key reads the table, and <>@ alone keeps
/// no line, as on main.
#[test]
fn issue_8637_bq7_cells() {
    let o = outcomes(PROBE, "BQ Tests 7");
    assert_eq!(o.len(), 17);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}

/// The cells of BQ Tests 5 (corpus PR 1495, BgTaskQueryCache.al:2229-2409). A test page's read of a line fills the row cache,
/// as the test session's Get does, through filters on every key field (:2300, :2322) and through a filter on the group alone
/// (:2355; the page reads its line by the full key). The test session's four reads first answer the worker's first four reads
/// for every shape of :2375-:2405. All flagged.
#[test]
fn issue_8637_review_cells() {
    let o = outcomes(PROBE, "BQ Tests 5");
    assert_eq!(o.len(), 9);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BackgroundTaskQueryCache), "{name}: {marks:?}");
    }
}
