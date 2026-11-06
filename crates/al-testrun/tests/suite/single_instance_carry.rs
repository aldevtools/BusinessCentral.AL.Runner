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
    procedure ShareTemp() begin TempRows."No" := 1; TempRows.Insert(); TempShared.Copy(TempRows, true); end;
    procedure SharedCount(): Integer begin exit(TempShared.Count()); end;
    procedure FillCtx()
    var Setup2: Record "Carry Setup";
    begin
        Setup2.Get('S');
        TempCtx."No" := 1; TempCtx.Rec := Setup2.RecordId(); TempCtx.Insert();
    end;
    procedure CtxTable(): Integer begin TempCtx.FindFirst(); exit(TempCtx.Rec.TableNo()); end;
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
    var
        TempRows: Record "Carry Temp" temporary;
        TempShared: Record "Carry Temp" temporary;
        TempAuto: Record "Carry Auto" temporary;
        TempCtx: Record "Carry Ctx" temporary;
        Json: JsonObject;
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
fn a_record_buffer_of_a_row_an_earlier_codeunit_wrote_is_refused() {
    // BC rolled the row back with the writer; the next session hands out the same row versions again.
    let results = run(&pair("KeepNewRow", "R := H.NewRowKey();"), &RunOptions::default());
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    refused(outcome(&results, "Reads"), "rolled back");
}

#[test]
fn a_record_buffer_read_from_a_row_an_earlier_codeunit_committed_is_refused() {
    // Read by Get after the Commit: no own write, but a row version the earlier session handed out.
    let results = run(&pair("KeepCommittedRow", "R := H.NewRowKey();"), &RunOptions::default());
    assert_eq!(outcome(&results, "Writes"), Outcome::Pass);
    refused(outcome(&results, "Reads"), "row version an earlier test codeunit wrote");
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
