//! What a session `StartSession` starts under Disabled test isolation sees of the record handed to it, and a start with a
//! Timeout. Measured on BC 29 by corpus writer Y30 (SaaS sandbox under Microsoft's runner 130451; probe branch
//! al-runtime-rs-probe-y30-session at 76761008, tests/al-language/session/lt-y30-startsession-isol-disabled/
//! Y30StartSessionIsolDisabled.al): a temporary record hands over a temporary record with all the caller's rows and its
//! current row, the caller keeps its rows; filters, marks with MarkedOnly and field values not yet saved are handed over.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};

const OBJECTS: &str = r#"
table 50780 "SR Row" { fields { field(1; Id; Integer) { } field(2; Info; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50781 "SR Result" { fields { field(1; Name; Text[50]) { } field(2; Value; Text[250]) { } } keys { key(PK; Name) { Clustered = true; } } }
codeunit 50782 "SR Worker"
{
    TableNo = "SR Row";
    trigger OnRun()
    var
        Ids: Text;
    begin
        Put('temp', Format(Rec.IsTemporary()));
        Put('current', Format(Rec.Id) + '|' + Rec.Info);
        Put('filters', Rec.GetFilters());
        Put('count', Format(Rec.Count()));
        if Rec.FindSet() then
            repeat
                Ids += Format(Rec.Id) + ',';
            until Rec.Next() = 0;
        Put('ids', Ids);
        Rec.Reset();
        Put('count_after_reset', Format(Rec.Count()));
        Commit();
    end;
    local procedure Put(Name: Text; Value: Text)
    var
        Res: Record "SR Result";
    begin
        Res.Name := CopyStr(Name, 1, MaxStrLen(Res.Name));
        Res.Value := CopyStr(Value, 1, MaxStrLen(Res.Value));
        if not Res.Insert() then
            Res.Modify();
    end;
}
codeunit 50783 "SR Sleeper"
{
    TableNo = "SR Row";
    trigger OnRun()
    var
        Res: Record "SR Result";
    begin
        Res.Name := 'start';
        Res.Insert();
        Commit();
        Sleep(10000);
        Res.Name := 'end';
        Res.Insert();
        Commit();
    end;
}
codeunit 50784 "SR Quick"
{
    TableNo = "SR Row";
    trigger OnRun()
    var
        Res: Record "SR Result";
    begin
        Res.Name := 'quick';
        Res.Insert();
    end;
}
codeunit 50785 "SR Tests"
{
    Subtype = Test;
    local procedure Seed()
    var
        Row: Record "SR Row";
        Res: Record "SR Result";
    begin
        Row.DeleteAll();
        Res.DeleteAll();
        Row.Id := 10; Row.Info := 'real10'; Row.Insert();
        Row.Id := 11; Row.Info := 'real11'; Row.Insert();
        Row.Id := 12; Row.Info := 'real12'; Row.Insert();
        Commit();
    end;
    local procedure Line(): Text
    var
        Res: Record "SR Result";
        T: Text;
    begin
        if Res.FindSet() then
            repeat
                T += Res.Name + '=' + Res.Value + ' ';
            until Res.Next() = 0;
        exit(T);
    end;
    local procedure Expect(Want: Text)
    begin
        if Line() <> Want then Error('worker saw: %1', Line());
    end;
    [Test] procedure Temporary()
    var
        TempRow: Record "SR Row" temporary;
        Sid: Integer;
    begin
        Seed();
        TempRow.Id := 1; TempRow.Info := 'temp1'; TempRow.Insert();
        TempRow.Id := 2; TempRow.Info := 'temp2'; TempRow.Insert();
        TempRow.Id := 3; TempRow.Info := 'temp3'; TempRow.Insert();
        TempRow.Get(2);
        if not StartSession(Sid, Codeunit::"SR Worker", CompanyName(), TempRow) then Error('not started');
        Expect('count=3 count_after_reset=3 current=2|temp2 filters= ids=1,2,3, temp=Yes ');
        if TempRow.Count() <> 3 then Error('caller count %1', TempRow.Count());
    end;
    [Test] procedure Filters()
    var
        Row: Record "SR Row";
        Sid: Integer;
    begin
        Seed();
        Row.SetRange(Id, 11, 12);
        Row.SetFilter(Info, '<>%1', 'x');
        Row.FindFirst();
        if not StartSession(Sid, Codeunit::"SR Worker", CompanyName(), Row) then Error('not started');
        Expect('count=2 count_after_reset=3 current=11|real11 filters=Id: 11..12, Info: <>x ids=11,12, temp=No ');
    end;
    [Test] procedure Marks()
    var
        Row: Record "SR Row";
        Sid: Integer;
    begin
        Seed();
        Row.Get(10); Row.Mark(true);
        Row.Get(12); Row.Mark(true);
        Row.MarkedOnly(true);
        Row.FindFirst();
        if not StartSession(Sid, Codeunit::"SR Worker", CompanyName(), Row) then Error('not started');
        Expect('count=2 count_after_reset=3 current=10|real10 filters=Marked: Yes ids=10,12, temp=No ');
    end;
    [Test] procedure Unsaved()
    var
        Row: Record "SR Row";
        Sid: Integer;
    begin
        Seed();
        Row.Get(11);
        Row.Info := 'changed-not-saved';
        if not StartSession(Sid, Codeunit::"SR Worker", CompanyName(), Row) then Error('not started');
        Expect('count=3 count_after_reset=3 current=11|changed-not-saved filters= ids=10,11,12, temp=No ');
    end;
    [Test] procedure TimeoutNotReached()
    var
        Row: Record "SR Row";
        Sid: Integer;
    begin
        Seed();
        Row.Get(10);
        if not StartSession(Sid, Codeunit::"SR Quick", CompanyName(), Row, 2000) then Error('not started');
        if Sid <= 0 then Error('sid %1', Sid);
        if Session.IsSessionActive(Sid) then Error('still active');
        Expect('quick= ');
    end;
    [Test] procedure SleepsWithoutTimeout()
    var
        Row: Record "SR Row";
        Sid: Integer;
    begin
        Seed();
        Row.Get(10);
        StartSession(Sid, Codeunit::"SR Sleeper", CompanyName(), Row);
    end;
    [Test] procedure TimeoutPassed()
    var
        Row: Record "SR Row";
        Sid: Integer;
    begin
        Seed();
        Row.Get(10);
        StartSession(Sid, Codeunit::"SR Sleeper", CompanyName(), Row, 2000);
    end;
}
"#;

fn results() -> Vec<(String, Outcome, Vec<&'static str>)> {
    let opts = RunOptions {
        isolation: Some(TestIsolation::Disabled),
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), OBJECTS.to_string())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    rt.results()
        .iter()
        .map(|r| {
            (
                r.test.clone(),
                r.outcome.clone(),
                r.assumptions.iter().map(|a| a.info().id).collect(),
            )
        })
        .collect()
}

fn of<'a>(r: &'a [(String, Outcome, Vec<&'static str>)], test: &str) -> &'a (String, Outcome, Vec<&'static str>) {
    r.iter()
        .find(|(t, _, _)| t == test)
        .unwrap_or_else(|| panic!("no {test} in {r:?}"))
}

#[test]
fn a_temporary_filtered_marked_or_unsaved_record_is_handed_to_the_session() {
    let r = results();
    for t in ["Temporary", "Filters", "Marks", "Unsaved"] {
        assert_eq!(of(&r, t).1, Outcome::Pass, "{t}: {r:?}");
    }
}

/// A session that ends before its Timeout runs as one without; one that would still run at the Timeout is refused
/// (BC stops it there while the caller runs on and `IsSessionActive` answers true: a session that runs alongside the
/// caller is not modelled).
#[test]
fn a_timeout_the_session_does_not_reach_runs_and_one_it_reaches_is_refused() {
    let r = results();
    let quick = of(&r, "TimeoutNotReached");
    assert_eq!(quick.1, Outcome::Pass, "{r:?}");
    assert!(quick.2.contains(&"startsession-timeout-not-reached"), "{quick:?}");
    assert!(
        matches!(&of(&r, "TimeoutPassed").1, Outcome::NotRun(m) if m.contains("Timeout")),
        "{r:?}"
    );
}

/// A session that waits (Sleep) is still active while the starting session runs on (Y30StartSessionIsolDisabled.al
/// Y30_StartSession_NoTimeout_Control: `active_after_start` true while the session sleeps): a session that runs alongside
/// the starting one is not modelled, so it is refused.
#[test]
fn a_session_that_sleeps_is_refused() {
    let r = results();
    assert!(
        matches!(&of(&r, "SleepsWithoutTimeout").1, Outcome::NotRun(m) if m.contains("Sleep")),
        "{r:?}"
    );
}
