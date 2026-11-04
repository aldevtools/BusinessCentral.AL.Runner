//! A request page handler that returns without OK, Cancel or another close action (al-runner issue 6996): the page
//! closes with Cancel (OnQueryClosePage(Cancel), then OnClosePage), the report does not run, `RunRequestPage` returns ''
//! and a SaveValues page stores nothing. Each test names the corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61420 "LO Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 61421 "LO Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
report 61422 "LO Rpt"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "LO Row") { } }
    requestpage
    {
        layout { area(Content) { field(TextCtl; TextVal) { ApplicationArea = All; } } }
        trigger OnOpenPage() begin L.Add('open text=' + TextVal); end;
        trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('qc ' + Format(CloseAction) + ' text=' + TextVal); exit(true); end;
        trigger OnClosePage() begin L.Add('close text=' + TextVal); end;
    }
    var TextVal: Text[30]; L: Codeunit "LO Log";
    trigger OnPreReport() begin L.Add('pre text=' + TextVal); end;
}
report 61423 "LO Save"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "LO Row") { } }
    requestpage
    {
        SaveValues = true;
        layout { area(Content) { field(TextCtl; TextVal) { ApplicationArea = All; } } }
        trigger OnOpenPage() begin L.Add('open text=' + TextVal); end;
    }
    var TextVal: Text[30]; L: Codeunit "LO Log";
    trigger OnPreReport() begin L.Add('pre text=' + TextVal); end;
}
report 61425 "LO NoLayout"
{
    dataset { dataitem(Row; "LO Row") { column(Id; Id) { } } }
    requestpage { layout { area(Content) { field(TextCtl; TextVal) { ApplicationArea = All; } } } }
    var TextVal: Text[30];
}
report 61424 "LO QcFalse"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "LO Row") { } }
    requestpage
    {
        layout { area(Content) { field(TextCtl; TextVal) { ApplicationArea = All; } } }
        trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(false); end;
    }
    var TextVal: Text[30];
}
"#;

const HELPERS: &str = "var L: Codeunit \"LO Log\"; Mode: Integer;
local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
[RequestPageHandler] procedure H(var RP: TestRequestPage \"LO Rpt\") begin if Mode = 1 then RP.TextCtl.SetValue('typed'); end;
[RequestPageHandler] procedure Empty(var RP: TestRequestPage \"LO Rpt\") begin end;
[RequestPageHandler] procedure SaveH(var RP: TestRequestPage \"LO Save\") begin
    case Mode of 1: RP.TextCtl.SetValue('typed'); 2: begin RP.TextCtl.SetValue('prime'); RP.OK().Invoke(); end; end; end;
[RequestPageHandler] procedure NoLayoutH(var RP: TestRequestPage \"LO NoLayout\") begin
    L.Add('preview=' + Format(RP.Preview().Visible()) + ' print=' + Format(RP.Print().Visible())); RP.Cancel().Invoke(); end;
[RequestPageHandler] procedure QcH(var RP: TestRequestPage \"LO QcFalse\") begin end;";

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 61429 T {{ Subtype = Test; {HELPERS} {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// reports/W3ReportRequestPageRun.al:322 Run_HandlerLeavesThePageOpen; reports/request-page/RptRequestPage.al:750
// Handler_DoesNotClosePage (OnQueryClosePage sees the handler's SetValue).
#[test]
fn report_run_with_a_handler_that_leaves_the_page_open_closes_it_with_cancel() {
    passes(
        "[Test] [HandlerFunctions('H')]
        procedure LeftOpen() begin
            L.Take(); Mode := 1;
            Report.Run(Report::\"LO Rpt\");
            Check('open text=;qc Cancel text=typed;close text=typed;', L.Take()); end;",
    );
}

// reports/W3ReportRequestPageRun.al:351 RunRequestPage_HandlerLeavesThePageOpen; reports/request-page/
// Y42RequestLeftOpen.al:777 RqOpen_RunRequestPage_NotProcessing (`rrp result` '').
#[test]
fn run_request_page_with_an_empty_handler_returns_empty_text() {
    passes(
        "[Test] [HandlerFunctions('Empty')]
        procedure EmptyHandler() var T: Text; begin
            L.Take();
            T := Report.RunRequestPage(Report::\"LO Rpt\");
            Check('', T);
            Check('open text=;qc Cancel text=;close text=;', L.Take()); end;",
    );
}

// reports/request-page/Y42RequestLeftOpen.al:723 RqOpen_RunRequestPage_SetValue: the page RunRequestPage opens shows the
// defaults, not the stored values; the Cancel close stores nothing, the next run shows what the earlier OK stored.
#[test]
fn a_save_values_page_left_open_stores_nothing() {
    passes(
        "[Test] [HandlerFunctions('SaveH')]
        procedure SaveValuesLeftOpen() var T: Text; begin
            Mode := 2; Report.Run(Report::\"LO Save\");
            L.Take(); Mode := 1;
            T := Report.RunRequestPage(Report::\"LO Save\");
            Check('', T);
            Check('open text=;', L.Take());
            Mode := 0; Report.Run(Report::\"LO Save\");
            Check('open text=prime;', L.Take()); end;",
    );
}

// reports/request-page/Y42RequestLeftOpen.al:753 RqOpen_QueryCloseAnswersFalse pins a veto of the platform's close
// (FormCloseNotAllowed, the report runs); not implemented here, so still refused.
#[test]
fn a_left_open_page_whose_on_query_close_page_answers_false_is_refused() {
    let o = outcomes(
        "[Test] [HandlerFunctions('QcH')]
        procedure Veto() begin Report.Run(Report::\"LO QcFalse\"); end;",
    );
    match &o[..] {
        [(_, Outcome::NotRun(r))] => assert!(r.contains("OnQueryClosePage"), "{r}"),
        other => panic!("expected one refusal, got {other:?}"),
    }
}

// reports/W3ReportRequestPageRun.al:388 Run_NonProcessingOnly_ActionsOfThePage: on the request page Report.Run opens for a
// report that renders and has no layout, Preview is not visible and Print is.
#[test]
fn preview_is_not_visible_on_the_request_page_of_a_report_without_a_layout() {
    passes(
        "[Test] [HandlerFunctions('NoLayoutH')]
        procedure NoLayout() begin
            L.Take(); Report.Run(Report::\"LO NoLayout\");
            Check('preview=No print=Yes;', L.Take()); end;",
    );
}
