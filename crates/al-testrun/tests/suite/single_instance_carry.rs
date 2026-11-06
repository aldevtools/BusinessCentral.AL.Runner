//! Probe P7 of ADR-066 (al-runner issue 6924): within one test run, a SingleInstance codeunit's globals that one test
//! codeunit set (in a test, and in its OnRun) are still there in the next test codeunit, in run order; rows are rolled
//! back per test codeunit (corpus al-language/codeunits/TestCuP7ASetter.al:24-58, TestCuP7BReader.al:24-37). Test
//! codeunits run in parallel; the outcome must be the serial one.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// The state: a SingleInstance codeunit (in a dependency app, as Microsoft's test libraries are) and a table.
const BASE: &str = r#"
table 50250 "Carry Row" { fields { field(1; "Key"; Code[20]) { } } keys { key(PK; "Key") { } } }
codeunit 50251 "Carry State"
{
    SingleInstance = true;
    procedure NoteRan(What: Text) begin if Ran <> '' then Ran += ';'; Ran += What; end;
    procedure GetRan(): Text begin exit(Ran); end;
    procedure Bump() begin Count += 1; end;
    procedure GetCount(): Integer begin exit(Count); end;
    procedure Keep(Value: Text) begin Held.Add(Value); end;
    procedure HeldCount(): Integer begin exit(Held.Count()); end;
    var
        Ran: Text;
        Count: Integer;
        Held: List of [Text];
}
table 50252 "Carry Temp" { fields { field(1; "No"; Integer) { } field(2; "Name"; Text[30]) { } } keys { key(PK; "No") { } key(ByName; "Name") { } } }
table 50253 "Carry Auto" { fields { field(1; "No"; Integer) { AutoIncrement = true; } } keys { key(PK; "No") { } } }
table 50258 "Carry Ctx" { fields { field(1; "No"; Integer) { } field(2; "Rec"; RecordId) { } } keys { key(PK; "No") { } } }
table 50254 "Carry Setup" { fields { field(1; "Key"; Code[10]) { } field(2; "Value"; Integer) { } } keys { key(PK; "Key") { } } }
codeunit 50255 "Carry Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var Setup: Record "Carry Setup";
    begin
        Setup."Key" := 'S';
        Setup.Value := 7;
        Setup.Insert();
    end;
}
codeunit 50256 "Carry Other"
{
    SingleInstance = true;
    procedure Set(V: Integer) begin Held := V; end;
    procedure Get(): Integer begin exit(Held); end;
    var Held: Integer;
}
codeunit 50257 "Carry Handles"
{
    SingleInstance = true;
    procedure Fill()
    begin
        TempRows."No" := 1; TempRows.Name := 'B'; TempRows.Insert();
        TempRows."No" := 2; TempRows.Name := 'A'; TempRows.Insert();
        TempRows."No" := 3; TempRows.Name := 'C'; TempRows.Insert();
        TempRows.SetCurrentKey(Name);
        TempRows.SetFilter("No", '>1');
        TempRows.FindFirst();
        TempRows.Get(3);
        TempRows.Mark(true);
        Names.Set(1, 'one');
        Slots[2] := 22;
        Other.Set(5);
        RecRef.Open(Database::"Carry Temp", true);
        RecRef.Field(1).Value := 9;
        RecRef.Insert();
    end;
    procedure Check(): Text
    var Direct: Codeunit "Carry Other";
    begin
        if TempRows.GetFilters() <> 'No: >1' then exit('filters ' + TempRows.GetFilters());
        if TempRows.CurrentKey() <> 'Name' then exit('key ' + TempRows.CurrentKey());
        if TempRows."No" <> 3 then exit('buffer ' + Format(TempRows."No"));
        if TempRows.Count() <> 2 then exit('count ' + Format(TempRows.Count()));
        TempRows.FindFirst();
        if TempRows.Name <> 'A' then exit('order ' + TempRows.Name);
        TempRows.MarkedOnly(true);
        if TempRows.Count() <> 1 then exit('marked ' + Format(TempRows.Count()));
        TempRows.MarkedOnly(false);
        TempRows.Reset();
        if TempRows.Count() <> 3 then exit('rows ' + Format(TempRows.Count()));
        if Names.Get(1) <> 'one' then exit('dictionary');
        if Slots[2] <> 22 then exit('array');
        if Other.Get() <> 5 then exit('codeunit value');
        Direct.Set(6);
        if Other.Get() <> 6 then exit('the codeunit value is not the single instance');
        if RecRef.Count() <> 1 then exit('recordref ' + Format(RecRef.Count()));
        exit('ok');
    end;
    procedure LoadSetup() begin Setup.Get('S'); end;
    procedure SetupValue(): Integer begin exit(Setup.Value); end;
    procedure ModifySetup(V: Integer) begin Setup.Value := V; Setup.Modify(); end;
    procedure KeepNewRow() begin NewRow."Key" := 'NEW'; NewRow.Insert(); end;
    procedure KeepCommittedRow()
    var Other: Record "Carry Row";
    begin
        Other."Key" := 'NEW'; Other.Insert(); Commit();
        NewRow.Get('NEW');
    end;
    procedure NewRowKey(): Code[20] begin exit(NewRow."Key"); end;
    procedure ModifyNewRow() begin NewRow.Modify(); end;
    procedure NewRowCaption(): Text begin exit(NewRow.FieldCaption("Key")); end;
    procedure ClearNewRow() begin Clear(NewRow); NewRow."Key" := 'X'; NewRow.Insert(); end;
    procedure ShareTemp() begin TempRows."No" := 1; TempRows.Insert(); TempShared.Copy(TempRows, true); end;
    procedure SharedCount(): Integer begin exit(TempShared.Count()); end;
    procedure FillCtx()
    var Setup2: Record "Carry Setup";
    begin
        Setup2.Get('S');
        TempCtx."No" := 1; TempCtx.Rec := Setup2.RecordId(); TempCtx.Insert();
    end;
    procedure CtxTable(): Integer begin TempCtx.FindFirst(); exit(TempCtx.Rec.TableNo()); end;
    procedure FilterCtx()
    var Setup2: Record "Carry Setup";
    begin
        FillCtx();
        TempCtx.Init(); TempCtx."No" := 2; TempCtx.Insert();
        Setup2.Get('S');
        TempCtx.SetRange(Rec, Setup2.RecordId());
    end;
    procedure CtxFiltered(): Integer begin exit(TempCtx.Count()); end;
    procedure FillJson()
    var Inner: JsonObject;
    begin
        Inner.Add('b', 'x');
        Json.Add('a', 1);
        Json.Add('o', Inner);
    end;
    procedure CheckJson(): Text
    var T: JsonToken; Txt: Text;
    begin
        Json.SelectToken('o.b', T);
        if T.AsValue().AsText() <> 'x' then exit('o.b');
        if T.Path() <> 'o.b' then exit('path ' + T.Path());
        Json.Get('a', T);
        if T.AsValue().AsInteger() <> 1 then exit('a');
        Json.WriteTo(Txt);
        if Txt <> '{"a":1,"o":{"b":"x"}}' then exit(Txt);
        exit('ok');
    end;
    procedure FillAuto() begin TempAuto.Insert(); end;
    procedure AutoCount(): Integer begin exit(TempAuto.Count()); end;
    procedure ClearRecId() var R: RecordId; begin RecId := R; end;
    procedure RecIdTable(): Integer begin exit(RecId.TableNo()); end;
    procedure FilterAuto() begin TempAuto.SetRange("No", 5); end;
    procedure AutoFilter(): Text begin exit(TempAuto.GetFilter("No")); end;
    var
        TempRows: Record "Carry Temp" temporary;
        TempShared: Record "Carry Temp" temporary;
        TempAuto: Record "Carry Auto" temporary;
        TempCtx: Record "Carry Ctx" temporary;
        Json: JsonObject;
        RecId: RecordId;
        Setup: Record "Carry Setup";
        NewRow: Record "Carry Row";
        Names: Dictionary of [Integer, Text];
        Slots: array[3] of Integer;
        Other: Codeunit "Carry Other";
        RecRef: RecordRef;
}
codeunit 50260 "Carry Json Holder"
{
    procedure Init() begin J.ReadFrom('{}'); end;
    procedure Add(Name: Text): Codeunit "Carry Json Holder"
    var O: JsonObject; T: JsonToken; H: Codeunit "Carry Json Holder";
    begin
        O.ReadFrom('{}');
        T := O.AsToken();
        J.AsObject().Add(Name, T);
        H.SetTok(T);
        exit(H);
    end;
    procedure SetTok(var T: JsonToken) begin J := T; end;
    procedure Put(Name: Text; Value: Text) begin J.AsObject().Add(Name, Value); end;
    procedure AsText(): Text var Txt: Text; begin J.WriteTo(Txt); exit(Txt); end;
    var J: JsonToken;
}
codeunit 50259 "Carry Json Pair"
{
    SingleInstance = true;
    // As Microsoft's "Test Output" (Test Runner app): Cur holds a token inside Root's tree, and Cur comes first.
    procedure Fill() begin Root.Init(); Cur := Root.Add('t'); Cur.Put('a', 'x'); end;
    procedure AddMore(): Text begin Cur.Put('b', 'y'); exit(Root.AsText()); end;
    procedure KeepList() begin L1.Add('x'); L2 := L1; end;
    procedure ListShared(): Text begin L2.Add('y'); exit(Format(L1.Count())); end;
    var
        Cur: Codeunit "Carry Json Holder";
        Root: Codeunit "Carry Json Holder";
        L1: List of [Text];
        L2: List of [Text];
}
"#;

/// A writer test codeunit (60270) that runs `fill` on "Carry Handles", and a reader (60271) whose test fails with the
/// text `check` returns unless it is `ok`, or (with `check` ending in a semicolon) runs those statements.
fn pair(fill: &str, check: &str) -> String {
    format!(
        r#"
codeunit 60270 "Carry Writer"
{{
    Subtype = Test;
    [Test] procedure Writes()
    var H: Codeunit "Carry Handles";
    begin
        H.{fill}();
    end;
}}
codeunit 60271 "Carry Reader"
{{
    Subtype = Test;
    [Test] procedure Reads()
    var H: Codeunit "Carry Handles"; R: Text;
    begin
        {check}
    end;
}}
"#
    )
}

fn outcome(results: &[(String, Outcome)], test: &str) -> Outcome {
    results
        .iter()
        .find(|(t, _)| t == test)
        .unwrap_or_else(|| panic!("{test} ran: {results:?}"))
        .1
        .clone()
}

fn refused(o: Outcome, why: &str) {
    match o {
        Outcome::NotRun(m) => assert!(m.contains(why), "{m}"),
        o => panic!("expected a refusal ({why}), got {o:?}"),
    }
}

/// Test codeunits 60250.., in run order. The first sets the state in OnRun and in a test and inserts a committed row;
/// each later one checks what all before it left and adds to it.
const IDS: [u32; 8] = [60250, 60251, 60252, 60253, 60254, 60255, 60256, 60257];

fn source() -> String {
    let mut out = String::new();
    for (i, id) in IDS.iter().enumerate() {
        let expected_ran: Vec<String> = (0..=i).map(|k| format!("{}", IDS[k])).collect();
        let expected_ran = expected_ran.join(";");
        out.push_str(&format!(
            r#"
codeunit {id} "Carry {id}"
{{
    Subtype = Test;
    trigger OnRun()
    var State: Codeunit "Carry State";
    begin
        State.NoteRan('{id}');
    end;

    [Test] procedure Sees{id}()
    var State: Codeunit "Carry State"; Row: Record "Carry Row"; I: Integer; W: BigInteger;
    begin
        // Some work, so that the codeunits overlap on the test threads.
        for I := 1 to 100000 do W += I;
        if State.GetRan() <> '{expected_ran}' then Error('codeunit {id}: ran %1', State.GetRan());
        if State.GetCount() <> {i} then Error('codeunit {id}: count %1', State.GetCount());
        if Row.Get('ROW') then Error('codeunit {id}: the row of an earlier codeunit is still there');
        State.Bump();
        Row."Key" := 'ROW';
        Row.Insert();
        Commit();
    end;
}}
"#
        ));
    }
    out
}

/// A codeunit that leaves a List in the state, and one after it that reads it.
const HANDLE: &str = r#"
codeunit 60260 "Carry List Writer"
{
    Subtype = Test;
    [Test] procedure KeepsAValue()
    var State: Codeunit "Carry State";
    begin
        State.Keep('x');
    end;
}
codeunit 60261 "Carry List Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheList()
    var State: Codeunit "Carry State";
    begin
        if State.HeldCount() <> 1 then Error('count %1', State.HeldCount());
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn manifest(p: &str) -> Manifest {
    Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    }
}

fn run(src: &str, opts: &RunOptions) -> Vec<(String, Outcome)> {
    let rt = runtime(src, opts);
    rt.results()
        .iter()
        .map(|r| (r.test.clone(), r.outcome.clone()))
        .collect()
}

fn runtime(src: &str, opts: &RunOptions) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], src), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

#[test]
fn single_instance_globals_reach_the_next_test_codeunit_and_rows_do_not() {
    let results = run(&source(), &RunOptions::default());
    assert_eq!(results.len(), IDS.len());
    for (test, outcome) in &results {
        assert_eq!(*outcome, Outcome::Pass, "{test}");
    }
}

#[test]
fn a_selected_codeunit_starts_from_what_the_codeunits_that_ran_before_it_left() {
    // Only the fourth codeunit runs: no earlier codeunit ran in this run, so it sees the state fresh (as on BC).
    let opts = RunOptions {
        only: Some(vec![(0, IDS[3])]),
        ..Default::default()
    };
    let results = run(&source(), &opts);
    assert_eq!(results.len(), 1);
    match &results[0].1 {
        Outcome::Fail(m) => assert!(m.contains(&format!("codeunit {}: ran {}", IDS[3], IDS[3])), "{m}"),
        o => panic!("expected the fresh state's failure, got {o:?}"),
    }
}

#[test]
fn a_list_left_in_a_single_instance_reaches_the_next_codeunit() {
    let results = run(HANDLE, &RunOptions::default());
    assert_eq!(outcome(&results, "KeepsAValue"), Outcome::Pass);
    assert_eq!(outcome(&results, "ReadsTheList"), Outcome::Pass);
}

#[test]
fn temporary_rows_filters_key_marks_dictionaries_arrays_codeunits_and_recordrefs_reach_the_next_codeunit() {
    // The temporary table goes with the record (BC rolls back no temporary table); the codeunit value of a
    // SingleInstance codeunit stays that codeunit's one instance.
    let results = run(
        &pair("Fill", "R := H.Check(); if R <> 'ok' then Error(R);"),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

#[test]
fn a_record_buffer_of_a_row_committed_before_the_codeunits_is_carried_and_can_be_written() {
    // Its row version names the install row, the same in every session; its origin's Commit count is from an ended
    // transaction (`RowOrigin::EARLIER_SESSION`): no stale rule applies to the unchanged row.
    let results = run(
        &pair(
            "LoadSetup",
            "if H.SetupValue() <> 7 then Error('value %1', H.SetupValue()); Commit(); Commit(); H.ModifySetup(8);",
        ),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

#[test]
fn the_fields_of_a_record_buffer_of_a_row_an_earlier_codeunit_wrote_reach_the_next_codeunit() {
    // BC rolled the row back with the writer, not the variable's buffer (Microsoft's "Unit of Measure Management"
    // keeps such a buffer as a cache). The row version is not handed on: the next session hands out the same numbers.
    for fill in ["KeepNewRow", "KeepCommittedRow"] {
        let results = run(
            &pair(fill, "if H.NewRowKey() <> 'NEW' then Error('key %1', H.NewRowKey());"),
            &RunOptions::default(),
        );
        assert_eq!(outcome(&results, "Writes"), Outcome::Pass, "{fill}");
        assert_eq!(outcome(&results, "Reads"), Outcome::Pass, "{fill}");
    }
}

#[test]
fn a_write_through_a_record_buffer_of_a_row_an_earlier_codeunit_wrote_is_refused() {
    for fill in ["KeepNewRow", "KeepCommittedRow"] {
        let results = run(&pair(fill, "H.ModifyNewRow();"), &RunOptions::default());
        assert_eq!(outcome(&results, "Writes"), Outcome::Pass, "{fill}");
        refused(outcome(&results, "Reads"), "rolled back");
    }
}

#[test]
fn a_temporary_table_two_carried_records_share_is_refused() {
    let results = run(
        &pair("ShareTemp", "if H.SharedCount() <> 1 then Error('x');"),
        &RunOptions::default(),
    );
    refused(
        outcome(&results, "Reads"),
        "temporary table another carried record shares",
    );
}

#[test]
fn temporary_rows_with_a_recordid_field_reach_the_next_codeunit() {
    // Microsoft's "Notification Lifecycle Mgt." holds such a table ("Notification Context").
    let results = run(
        &pair(
            "FillCtx",
            "if H.CtxTable() <> 50254 then Error('table %1', H.CtxTable());",
        ),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

#[test]
fn a_json_tree_reaches_the_next_codeunit() {
    // Microsoft's "Test Output" holds a JSON value.
    let results = run(
        &pair("FillJson", "R := H.CheckJson(); if R <> 'ok' then Error(R);"),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

#[test]
fn a_temporary_table_with_an_autoincrement_field_is_refused() {
    let results = run(
        &pair("FillAuto", "if H.AutoCount() <> 1 then Error('x');"),
        &RunOptions::default(),
    );
    refused(outcome(&results, "Reads"), "AutoIncrement field");
}

#[test]
fn the_serial_check_runs_each_session_in_one_go_and_finds_the_same_outcomes() {
    // Owner requirement (2026-11-06): the codeunits of a BC session run one after the other, the single instances
    // moved from session to session (nothing copied), must give every outcome the carrying gave.
    let opts = RunOptions {
        single_serial_check: true,
        ..Default::default()
    };
    for src in [
        source(),
        pair("Fill", "R := H.Check(); if R <> 'ok' then Error(R);"),
        HANDLE.to_string(),
        pair_shared("Fill", "AddMore", r#"{"t":{"a":"x","b":"y"}}"#),
        pair_shared("KeepList", "ListShared", "2"),
    ] {
        let rt = runtime(&src, &opts);
        let chain = &rt.phases.chain;
        assert!(chain.serial_check);
        assert!(chain.serial_checked >= 2, "{}", chain.serial_checked);
        assert!(chain.serial_mismatches.is_empty(), "{:?}", chain.serial_mismatches);
        for r in rt.results() {
            assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
        }
    }
}

/// A writer (60272) that runs `fill` on "Carry Json Pair" and a reader (60273) whose test fails unless `check` returns
/// `want`.
fn pair_shared(fill: &str, check: &str, want: &str) -> String {
    format!(
        r#"
codeunit 60272 "Shared Writer"
{{
    Subtype = Test;
    [Test] procedure WritesShared() var P: Codeunit "Carry Json Pair"; begin P.{fill}(); end;
}}
codeunit 60273 "Shared Reader"
{{
    Subtype = Test;
    [Test] procedure ReadsShared()
    var P: Codeunit "Carry Json Pair"; R: Text;
    begin
        R := P.{check}();
        if R <> '{want}' then Error(R);
    end;
}}
"#
    )
}

#[test]
fn a_token_inside_another_global_s_json_tree_reaches_the_next_codeunit_still_inside_it() {
    // The globals of one single instance are carried as one group: the token stays the node of the tree, so a value
    // added through it is in the tree (Microsoft's "Test Output", TestOutput.Codeunit.al:125-127 of the Test Runner).
    let results = run(
        &pair_shared("Fill", "AddMore", r#"{"t":{"a":"x","b":"y"}}"#),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "WritesShared"), Outcome::Pass);
    assert_eq!(outcome(&results, "ReadsShared"), Outcome::Pass);
}

#[test]
fn a_list_two_globals_share_reaches_the_next_codeunit_still_shared() {
    let results = run(&pair_shared("KeepList", "ListShared", "2"), &RunOptions::default());
    assert_eq!(outcome(&results, "WritesShared"), Outcome::Pass);
    assert_eq!(outcome(&results, "ReadsShared"), Outcome::Pass);
}

#[test]
fn a_temporary_record_filtered_on_a_recordid_reaches_the_next_codeunit() {
    // Microsoft's "Notification Lifecycle Mgt." leaves TempNotificationContext filtered on "Record ID".
    let results = run(
        &pair(
            "FilterCtx",
            "if H.CtxFiltered() <> 1 then Error('count %1', H.CtxFiltered());",
        ),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

#[test]
fn an_empty_temporary_table_with_an_autoincrement_field_whose_counter_never_moved_reaches_the_next_codeunit() {
    // Microsoft's "Serv. Sales-Post" holds ServItemManagement, whose temporary tables are empty between postings.
    let results = run(
        &pair(
            "FilterAuto",
            "if H.AutoFilter() <> '5' then Error('filter %1', H.AutoFilter());",
        ),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

#[test]
fn a_global_that_did_not_read_back_stays_refused_when_another_global_of_its_codeunit_changes() {
    // The group is handed on whole when one member changes; a member that did not read back still stands for what the
    // first codeunit left, not for the fresh value it holds.
    let src = r#"
codeunit 60274 "Group Fill"
{
    Subtype = Test;
    [Test] procedure FillsAuto() var H: Codeunit "Carry Handles"; begin H.FillAuto(); end;
}
codeunit 60275 "Group Other"
{
    Subtype = Test;
    [Test] procedure ChangesOther() var H: Codeunit "Carry Handles"; begin H.FillJson(); end;
}
codeunit 60276 "Group Read"
{
    Subtype = Test;
    [Test] procedure ReadsAuto() var H: Codeunit "Carry Handles"; begin if H.AutoCount() <> 1 then Error('x'); end;
}
"#;
    let results = run(src, &RunOptions::default());
    assert_eq!(outcome(&results, "ChangesOther"), Outcome::Pass);
    refused(outcome(&results, "ReadsAuto"), "AutoIncrement field");
}

#[test]
fn a_recordid_nothing_was_assigned_to_reaches_the_next_codeunit() {
    // Microsoft's "Last Error Context Element" holds one in its "Error Context Element".
    let results = run(
        &pair(
            "ClearRecId",
            "if H.RecIdTable() <> 0 then Error('table %1', H.RecIdTable());",
        ),
        &RunOptions::default(),
    );
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

#[test]
fn a_chain_member_that_changes_fewer_tables_from_the_carried_state_hands_its_rows_on() {
    // Run from the serial state, the middle codeunit skips a setup its first run (from a fresh state) did, as an
    // `isInitialized` flag in a single instance makes Microsoft's tests do: it changes a subset of the tables.
    let src = r#"
codeunit 60277 "Chain Sets"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SetsFlag() var S: Codeunit "Carry State"; begin S.Bump(); end;
}
codeunit 60278 "Chain Middle"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SetsUpOnce()
    var S: Codeunit "Carry State"; R: Record "Carry Row"; T: Record "Carry Temp";
    begin
        if S.GetCount() = 0 then begin
            T."No" := 1; T.Insert();
            R."Key" := 'FRESH'; R.Insert();
        end else begin
            R."Key" := 'SERIAL'; R.Insert();
        end;
    end;
}
codeunit 60279 "Chain Reads"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SeesTheSerialRows()
    var R: Record "Carry Row"; T: Record "Carry Temp";
    begin
        if not R.Get('SERIAL') then Error('no SERIAL row');
        if R.Get('FRESH') then Error('a FRESH row');
        if not T.IsEmpty() then Error('Carry Temp has rows');
    end;
}
"#;
    // On the TestRunnerHub, where a codeunit with RequiredTestIsolation = Disabled hands its rows on.
    let opts = RunOptions {
        required_isolation: true,
        ..Default::default()
    };
    let results = run(src, &opts);
    assert_eq!(outcome(&results, "SetsUpOnce"), Outcome::Pass);
    assert_eq!(outcome(&results, "SeesTheSerialRows"), Outcome::Pass);
}

#[test]
fn a_chain_member_that_takes_no_number_from_the_carried_state_hands_on_the_state_it_was_given() {
    // Run from the serial state, the middle codeunit skips the number its first run (from a fresh state) took: it
    // leaves the state it was given (`baseline: None`, numbers_taken.rs). Corpus P119ReportColumn.al's codeunit, run
    // again after the Probe Observer's state, did this on every major, and every later codeunit was refused.
    let src = r#"
codeunit 60277 "Chain Sets"
{
    Subtype = Test;
    [Test] procedure SetsFlag() var S: Codeunit "Carry State"; begin S.Bump(); end;
}
codeunit 60278 "Chain Middle"
{
    Subtype = Test;
    [Test] procedure TakesOnce()
    var S: Codeunit "Carry State";
    begin
        if S.GetCount() = 0 then begin
            if not NumberSequence.Exists('CARRYSEQ') then NumberSequence.Insert('CARRYSEQ');
            NumberSequence.Next('CARRYSEQ');
        end;
    end;
}
codeunit 60279 "Chain Reads"
{
    Subtype = Test;
    [Test] procedure SeesNoSequence()
    begin
        if NumberSequence.Exists('CARRYSEQ') then Error('the sequence exists');
    end;
}
"#;
    let results = run(src, &RunOptions::default());
    assert_eq!(outcome(&results, "TakesOnce"), Outcome::Pass);
    assert_eq!(outcome(&results, "SeesNoSequence"), Outcome::Pass);
}

#[test]
fn a_rolled_back_buffer_is_handed_on_again_and_answers_its_field_captions() {
    let src = r#"
codeunit 60280 "Rolled Keep"
{
    Subtype = Test;
    [Test] procedure KeepsRow() var H: Codeunit "Carry Handles"; begin H.KeepNewRow(); end;
}
codeunit 60281 "Rolled Read 1"
{
    Subtype = Test;
    [Test] procedure ReadsOnce() var H: Codeunit "Carry Handles"; begin if H.NewRowKey() <> 'NEW' then Error('1'); end;
}
codeunit 60282 "Rolled Read 2"
{
    Subtype = Test;
    [Test] procedure ReadsTwice()
    var H: Codeunit "Carry Handles";
    begin
        if H.NewRowKey() <> 'NEW' then Error('2');
        if H.NewRowCaption() <> 'Key' then Error(H.NewRowCaption());
    end;
}
"#;
    let results = run(src, &RunOptions::default());
    assert_eq!(outcome(&results, "ReadsOnce"), Outcome::Pass);
    assert_eq!(outcome(&results, "ReadsTwice"), Outcome::Pass);
}

#[test]
fn clear_on_a_rolled_back_buffer_leaves_a_new_record() {
    let results = run(&pair("KeepNewRow", "H.ClearNewRow();"), &RunOptions::default());
    assert_eq!(outcome(&results, "Reads"), Outcome::Pass);
}

/// Bindings across the test codeunit boundary: BC keeps the binding of a SingleInstance subscriber and of an ordinary
/// subscriber a SingleInstance codeunit's global holds, and drops that of a subscriber the test codeunit held
/// (corpus al-language/codeunits/TestEventSingleInstanceBindingBoundary.al:91, :98-99).
const BINDINGS: &str = r#"
codeunit 60280 "Bind Pub"
{
    [IntegrationEvent(false, false)] procedure OnPing(var Hits: Text) begin end;
    procedure Raise(): Text var Hits: Text; begin OnPing(Hits); exit(Hits); end;
}
codeunit 60281 "Bind SI Sub"
{
    SingleInstance = true;
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Bind Pub", 'OnPing', '', false, false)]
    local procedure Ping(var Hits: Text) begin Hits += 'S'; end;
}
codeunit 60282 "Bind Plain Sub"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Bind Pub", 'OnPing', '', false, false)]
    local procedure Ping(var Hits: Text) begin Hits += 'P'; end;
}
codeunit 60283 "Bind Held Sub"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Bind Pub", 'OnPing', '', false, false)]
    local procedure Ping(var Hits: Text) begin Hits += 'H'; end;
}
codeunit 60284 "Bind Holder"
{
    SingleInstance = true;
    var Held: Codeunit "Bind Held Sub";
    procedure BindHeld(): Boolean begin exit(BindSubscription(Held)); end;
    procedure UnbindHeld() begin UnbindSubscription(Held); end;
}
codeunit 60285 "Bind 1 Setup"
{
    Subtype = Test;
    var Plain: Codeunit "Bind Plain Sub";
    [Test] procedure Binds()
    var Pub: Codeunit "Bind Pub"; Si: Codeunit "Bind SI Sub"; Holder: Codeunit "Bind Holder";
    begin
        BindSubscription(Si);
        BindSubscription(Plain);
        if not Holder.BindHeld() then Error('not bound');
        if Pub.Raise() <> 'SPH' then Error('setup %1', Pub.Raise());
    end;
}
codeunit 60286 "Bind 2 Check"
{
    Subtype = Test;
    [Test] procedure SeesTheKeptBindings()
    var Pub: Codeunit "Bind Pub";
    begin
        if Pub.Raise() <> 'SH' then Error('check %1', Pub.Raise());
    end;
}
codeunit 60287 "Bind 3 Unbind"
{
    Subtype = Test;
    [Test] procedure UnbindsTheHeldOne()
    var Pub: Codeunit "Bind Pub"; Holder: Codeunit "Bind Holder";
    begin
        Holder.UnbindHeld();
        if Pub.Raise() <> 'S' then Error('unbind %1', Pub.Raise());
    end;
}
codeunit 60288 "Bind 4 Check"
{
    Subtype = Test;
    [Test] procedure SeesOnlyTheSingleInstance()
    var Pub: Codeunit "Bind Pub";
    begin
        if Pub.Raise() <> 'S' then Error('last %1', Pub.Raise());
    end;
}
"#;

#[test]
fn bindings_a_single_instance_keeps_alive_reach_the_next_codeunit_and_others_do_not() {
    for serial in [false, true] {
        let opts = RunOptions {
            single_serial_check: serial,
            ..Default::default()
        };
        let rt = runtime(BINDINGS, &opts);
        assert!(rt.phases.chain.serial_mismatches.is_empty(), "{:?}", rt.phases.chain.serial_mismatches);
        for r in rt.results() {
            assert_eq!(r.outcome, Outcome::Pass, "{} (serial check {serial})", r.test);
        }
    }
}

/// A bound subscriber held in a SingleInstance global that cannot be handed on (its own temporary table has an
/// AutoIncrement field with rows): its binding is not dropped silently; an event with a manual subscriber in the next
/// codeunit is refused.
const BINDING_NOT_CARRIED: &str = r#"
codeunit 60290 "Bind Pub2"
{
    [IntegrationEvent(false, false)] procedure OnPing(var Hits: Text) begin end;
    procedure Raise(): Text var Hits: Text; begin OnPing(Hits); exit(Hits); end;
}
codeunit 60291 "Bind Auto Sub"
{
    EventSubscriberInstance = Manual;
    var Rows: Record "Carry Auto" temporary;
    procedure Fill() begin Rows.Insert(); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Bind Pub2", 'OnPing', '', false, false)]
    local procedure Ping(var Hits: Text) begin Hits += 'A'; end;
}
codeunit 60292 "Bind Auto Holder"
{
    SingleInstance = true;
    var Held: Codeunit "Bind Auto Sub";
    procedure BindHeld() begin Held.Fill(); BindSubscription(Held); end;
}
codeunit 60293 "Bind Auto Setup"
{
    Subtype = Test;
    [Test] procedure Binds()
    var Holder: Codeunit "Bind Auto Holder";
    begin
        Holder.BindHeld();
    end;
}
codeunit 60294 "Bind Auto Check"
{
    Subtype = Test;
    [Test] procedure Raises()
    var Pub: Codeunit "Bind Pub2";
    begin
        if Pub.Raise() <> 'A' then Error('raise %1', Pub.Raise());
    end;
}
"#;

#[test]
fn a_binding_held_where_it_cannot_be_handed_on_is_refused_not_dropped() {
    let results = run(BINDING_NOT_CARRIED, &RunOptions::default());
    assert_eq!(outcome(&results, "Binds"), Outcome::Pass);
    refused(outcome(&results, "Raises"), "an event with a manual subscriber");
}
