//! `[ReportHandler]`: a run that would show the request page calls the handler with the report instead
//! (reports/reporthandler/AeReportHandler.al, AeReportHandlerCalls.al).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61500 "RH Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 61504 "RH Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 61501 "RH Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Take() R: Text begin R := T; T := ''; end;
}
report 61502 "RH Proc"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RH Row") { trigger OnAfterGetRecord() var L: Codeunit "RH Log"; begin L.Add('row ' + Format(Row.Id)); end; } }
    requestpage
    {
        trigger OnInit() var L: Codeunit "RH Log"; begin L.Add('RP OnInit'); end;
        trigger OnOpenPage() var L: Codeunit "RH Log"; begin L.Add('RP OnOpenPage'); end;
    }
    trigger OnInitReport() var L: Codeunit "RH Log"; begin L.Add('OnInitReport'); end;
    trigger OnPreReport() var L: Codeunit "RH Log"; begin L.Add('OnPreReport'); end;
}
report 61505 "RH Rendered"
{
    DefaultLayout = RDLC;
    RDLCLayout = 'RHRendered.rdl';
    dataset { dataitem(Row; "RH Row") { column(Id; Id) { } } }
    requestpage { }
}
report 61503 "RH Layout"
{
    dataset { dataitem(Row; "RH Row") { column(Id; Id) { } } }
    requestpage { }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!(
        "{OBJECTS}\ncodeunit 61600 T {{ Subtype = Test; var L: Codeunit \"RH Log\"; Mode: Integer;
        local procedure Seed() var R: Record \"RH Row\"; begin R.DeleteAll(); R.Id := 1; R.Insert(); R.Id := 2; R.Insert(); Commit(); L.Take(); Mode := 0; end;
        procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
        [ReportHandler] procedure Handler(var R: Report \"RH Proc\") begin L.Add('handler'); if Mode = 1 then begin R.UseRequestPage(false); R.Run(); end; end;
        {tests} }}"
    );
    al_testkit::outcomes_in(&src, "T")
}

#[test]
fn the_handler_answers_a_run_with_the_request_page_and_the_report_runs_only_when_it_runs_it() {
    let got = outcomes(
        "[Test] [HandlerFunctions('Handler')] procedure Plain() begin Seed(); Report.Run(Report::\"RH Proc\");
            Check('OnInitReport;RP OnInit;handler;', L.Take()); end;
        [Test] [HandlerFunctions('Handler')] procedure RunsIt() begin Seed(); Mode := 1; Report.Run(Report::\"RH Proc\");
            Check('OnInitReport;RP OnInit;handler;OnPreReport;row 1;row 2;', L.Take()); end;
        [Test] [HandlerFunctions('Handler')] procedure RequestPageOnly() var P: Text; begin Seed(); P := Report.RunRequestPage(Report::\"RH Proc\");
            Check('', P); Check('OnInitReport;RP OnInit;handler;', L.Take()); end;
        [Test] procedure ExecuteCallsNoHandler() begin Seed(); Report.Execute(Report::\"RH Proc\", '');
            Check('OnInitReport;RP OnInit;OnPreReport;row 1;row 2;', L.Take()); end;
        [Test] procedure SaveAsCallsNoHandler() var B: Record \"RH Blob\" temporary; O: OutStream; begin Seed(); B.Data.CreateOutStream(O);
            Check('No', Format(Report.SaveAs(Report::\"RH Proc\", '', ReportFormat::Xml, O))); Check('OnInitReport;RP OnInit;', L.Take()); end;",
    );
    for (n, o) in &got {
        assert_eq!(o, &Outcome::Pass, "{n}");
    }
}

#[test]
fn a_report_handler_for_a_report_with_a_layout_is_refused() {
    let got = outcomes(
        "[ReportHandler] procedure LayoutHandler(var R: Report \"RH Layout\") begin end;
        [Test] [HandlerFunctions('LayoutHandler')] procedure Layout() begin Seed(); Report.Run(Report::\"RH Layout\"); end;",
    );
    match &got[0].1 {
        Outcome::NotRun(why) => assert!(why.contains("processing-only"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// A run without the request page of a report that renders output calls the handler bound for it (Run: reports/layouts-saveas/
/// Y43SystemPrinterRequestPage.al:313 Y43_G411_Layout_Run_ReqFalse_Sp_ReportHandler; RunModal:
/// reports/reporthandler/AeReportHandler.al:237 Report_RunModal_RdlcReportCallsRegisteredHandler, corpus pull request 1199, al-runner
/// issue 7269: `Report.RunModal(Report::"Assembly Order", false, false)` logs `rdlc-handler;`).
#[test]
fn a_run_without_the_page_of_a_rendering_report_calls_its_handler() {
    let got = outcomes(
        "[ReportHandler] procedure RenderedHandler(var R: Report \"RH Rendered\") begin L.Add('rendered-handler'); end;
        [Test] [HandlerFunctions('RenderedHandler')] procedure Rendered() begin Seed(); Report.RunModal(Report::\"RH Rendered\", false, false);
            Check('rendered-handler;', L.Take()); end;",
    );
    assert_eq!(got[0].1, Outcome::Pass, "{got:?}");
}

/// A processing-only report run without its page with a handler bound: whether the handler is called is not pinned.
#[test]
fn a_run_without_the_page_of_a_processing_only_report_with_a_handler_stays_as_it_was() {
    let got = outcomes(
        "[Test] [HandlerFunctions('Handler')] procedure NoPage() begin Seed(); Report.RunModal(Report::\"RH Proc\", false, false); end;",
    );
    assert!(!matches!(got[0].1, Outcome::Pass), "{got:?}");
}
