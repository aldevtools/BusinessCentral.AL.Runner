//! A report with SaveValues stores what its request page shows when the page closes with OK (corpus
//! report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:P147_Filter_SetTableViewVsStored: the
//! data item filter confirmed by OK is the filter the next instance's request page shows). The stored record is a copy:
//! filters the report body sets after the page closed are not request page values and must not reach a later
//! instance (al-runner issue 6065, the reporter's cases). A filter edit closed by Cancel is not kept, on the
//! assumption request-page-cancel-keeps-saved-filters.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};

const SRC: &str = r#"
table 50301 "SVI Parent" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50302 "SVI Child"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; ParentCode; Code[10]) { }
        field(3; ValidFrom; Date) { }
        field(4; Code; Code[10]) { }
    }
    keys { key(PK; Id) { } }
}
report 50303 "SVI Rpt"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(ParentItem; "SVI Parent")
        {
            RequestFilterFields = Code;
            dataitem(ChildItem; "SVI Child")
            {
                RequestFilterFields = Code;
                trigger OnPreDataItem()
                begin
                    ChildItem.SetRange(ParentCode, 'P');
                    ChildItem.SetFilter(ValidFrom, '<=%1', 20211130D);
                    BodyRan := true;
                end;
            }
        }
    }
    requestpage { SaveValues = true; }
    trigger OnPreReport()
    begin
        if ChildItem.GetFilters() <> '' then
            Error('stale body filters: %1', ChildItem.GetFilters());
    end;
    procedure CheckBody()
    begin
        if not BodyRan then Error('body did not run');
        if ChildItem.GetFilter(ParentCode) <> 'P' then Error('body filter lost');
    end;
    var BodyRan: Boolean;
}
codeunit 50304 "SVI Tests"
{
    Subtype = Test;
    var Calls: Integer;

    local procedure Seed()
    var P: Record "SVI Parent";
    begin
        if not P.Get('P') then begin P.Code := 'P'; P.Insert(); end;
        Commit();
    end;

    [RequestPageHandler]
    procedure Plain(var RP: TestRequestPage "SVI Rpt")
    begin
        RP.ChildItem.SetFilter(Code, '');
        RP.OK().Invoke();
    end;

    [RequestPageHandler]
    procedure Saved(var RP: TestRequestPage "SVI Rpt")
    begin
        Calls += 1;
        if Calls = 1 then begin
            RP.ParentItem.SetFilter(Code, 'P');
            RP.OK().Invoke();
            exit;
        end;
        if RP.ParentItem.GetFilter(Code) <> 'P' then Error('saved filter lost at call %1: [%2]', Calls, RP.ParentItem.GetFilter(Code));
        if Calls = 2 then begin
            RP.ParentItem.SetFilter(Code, 'OTHER');
            RP.Cancel().Invoke();
        end else
            RP.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('Plain')]
    procedure TwoVariables()
    var First: Report "SVI Rpt"; Second: Report "SVI Rpt";
    begin
        Seed();
        First.RunModal();
        First.CheckBody();
        Second.RunModal();
        Second.CheckBody();
        First.CheckBody();
    end;

    [Test]
    [HandlerFunctions('Plain')]
    procedure ClearOneVariable()
    var R: Report "SVI Rpt";
    begin
        Seed();
        R.RunModal();
        R.CheckBody();
        Clear(R);
        R.RunModal();
        R.CheckBody();
    end;

    [Test]
    [HandlerFunctions('Plain')]
    procedure OneRunA()
    var R: Report "SVI Rpt";
    begin
        Seed(); R.RunModal(); R.CheckBody();
    end;

    [Test]
    [HandlerFunctions('Plain')]
    procedure OneRunB()
    var R: Report "SVI Rpt";
    begin
        Seed(); R.RunModal(); R.CheckBody();
    end;

    [Test]
    [HandlerFunctions('Saved')]
    procedure ZCancelledEditNotKept()
    var First: Report "SVI Rpt"; Cancelled: Report "SVI Rpt"; Last: Report "SVI Rpt";
    begin
        Seed();
        Calls := 0;
        First.RunModal();
        First.CheckBody();
        Cancelled.RunModal();
        Last.RunModal();
        Last.CheckBody();
        if Calls <> 3 then Error('expected three request pages, got %1', Calls);
        First.CheckBody();
    end;
}
"#;

fn results() -> Vec<(String, Outcome, Vec<&'static str>)> {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let mut out: Vec<_> = rt
        .results()
        .iter()
        .map(|r| {
            (
                r.test.clone(),
                r.outcome.clone(),
                r.assumptions.iter().map(|a| a.info().id).collect(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn body_filters_do_not_reach_a_later_instance() {
    let r = results();
    for name in ["TwoVariables", "ClearOneVariable", "OneRunA", "OneRunB"] {
        let (_, o, a) = r
            .iter()
            .find(|(t, _, _)| t == name)
            .unwrap_or_else(|| panic!("no {name} in {r:?}"));
        assert_eq!(*o, Outcome::Pass, "{name}: {r:?}");
        assert!(!a.contains(&"request-page-cancel-keeps-saved-filters"), "{name}: {a:?}");
    }
}

#[test]
fn a_cancelled_filter_edit_is_not_kept() {
    let r = results();
    let (_, o, a) = r
        .iter()
        .find(|(t, _, _)| t == "ZCancelledEditNotKept")
        .unwrap_or_else(|| panic!("{r:?}"));
    assert_eq!(*o, Outcome::Pass, "{r:?}");
    assert!(a.contains(&"request-page-cancel-keeps-saved-filters"), "{a:?}");
}
