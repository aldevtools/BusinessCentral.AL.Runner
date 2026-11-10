//! Which report triggers run on `SaveAs`, a second `Run` and the static `Run` forms. Each test names the corpus test
//! that pins it.

use al_testkit::text;

const FIXTURE: &str = r#"
table 50950 "RTO Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 50951 "RTO Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var Result: Text; begin Result := Entries; Entries := ''; exit(Result); end;
}

report 50952 "RTO Processing"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(T; "RTO Row")
        {
            trigger OnAfterGetRecord() var L: Codeunit "RTO Log"; begin L.Add('row' + Format(T.Id)); end;
        }
    }
    trigger OnInitReport() var L: Codeunit "RTO Log"; begin L.Add('init'); end;
    trigger OnPreReport() var L: Codeunit "RTO Log"; begin L.Add('pre'); end;
    trigger OnPostReport() var L: Codeunit "RTO Log"; begin L.Add('post'); end;
}

report 50954 "RTO WithPage"
{
    ProcessingOnly = true;
    dataset { dataitem(T; "RTO Row") { } }
    requestpage { layout { area(Content) { field(Dummy; DummyTxt) { ApplicationArea = All; } } } }
    var DummyTxt: Text[10];
    trigger OnInitReport() var L: Codeunit "RTO Log"; begin L.Add('init'); end;
    trigger OnPreReport() var L: Codeunit "RTO Log"; begin L.Add('pre get=' + Format(CurrReport.UseRequestPage())); end;
}

report 50955 "RTO NoPage"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(T; "RTO Row") { } }
    requestpage { layout { area(Content) { field(Dummy; DummyTxt) { ApplicationArea = All; } } } }
    var DummyTxt: Text[10];
    trigger OnInitReport() var L: Codeunit "RTO Log"; begin L.Add('init'); end;
    trigger OnPreReport() var L: Codeunit "RTO Log"; begin L.Add('pre'); end;
}

report 50956 "RTO RpInit"
{
    dataset { dataitem(T; "RTO Row") { column(TId; Id) { } } }
    requestpage
    {
        layout { area(Content) { field(Dummy; DummyTxt) { ApplicationArea = All; } } }
        trigger OnInit() var L: Codeunit "RTO Log"; begin L.Add('rp init'); end;
        trigger OnOpenPage() var L: Codeunit "RTO Log"; begin L.Add('rp open'); end;
    }
    var DummyTxt: Text[10];
    trigger OnInitReport() var L: Codeunit "RTO Log"; begin L.Add('init'); end;
    trigger OnPreReport() var L: Codeunit "RTO Log"; begin L.Add('pre'); end;
}

report 50958 "RTO PreError"
{
    dataset { dataitem(T; "RTO Row") { column(TId; Id) { } } }
    trigger OnPreReport() begin Error('pre failed'); end;
}

report 50957 "RTO RpInit Proc"
{
    ProcessingOnly = true;
    dataset { dataitem(T; "RTO Row") { } }
    requestpage
    {
        layout { area(Content) { field(Dummy; DummyTxt) { ApplicationArea = All; } } }
        trigger OnInit() var L: Codeunit "RTO Log"; begin L.Add('rp init'); end;
        trigger OnOpenPage() var L: Codeunit "RTO Log"; begin L.Add('rp open'); end;
    }
    var DummyTxt: Text[10];
    trigger OnInitReport() var L: Codeunit "RTO Log"; begin L.Add('init'); end;
    trigger OnPreReport() var L: Codeunit "RTO Log"; begin L.Add('pre'); end;
}

codeunit 50953 "RTO Tests"
{
    local procedure Seed()
    var R: Record "RTO Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
    end;

    procedure SaveAsProcessingOnly(): Text
    var
        R: Record "RTO Row";
        L: Codeunit "RTO Log";
        OutS: OutStream;
        Ok: Boolean;
    begin
        Seed();
        L.Take();
        R.Get(1);
        R.Data.CreateOutStream(OutS);
        Ok := Report.SaveAs(Report::"RTO Processing", '', ReportFormat::Xml, OutS);
        exit(Format(Ok) + '|' + L.Take() + '|' + GetLastErrorCode());
    end;

    procedure StaticRunWithoutWindow(): Text
    var L: Codeunit "RTO Log";
    begin
        L.Take();
        Report.Run(Report::"RTO WithPage", false, false);
        exit(L.Take());
    end;

    procedure StaticRunWithWindowOnNoPageReport(): Text
    var L: Codeunit "RTO Log";
    begin
        Seed();
        L.Take();
        asserterror Report.Run(Report::"RTO NoPage", true);
        exit(L.Take() + '|' + GetLastErrorText());
    end;

    procedure RunModalRefusedThenExecute(): Text
    var
        L: Codeunit "RTO Log";
        Rpt: Report "RTO WithPage";
        First: Text;
    begin
        Seed();
        L.Take();
        asserterror Rpt.RunModal();
        First := L.Take() + GetLastErrorCode();
        Rpt.Execute('');
        // Only that OnInitReport runs first is pinned here; what UseRequestPage answers inside Execute is not.
        exit(First + '|' + CopyStr(L.Take(), 1, 5));
    end;

    procedure GetterAfterExecute(): Text
    var Rpt: Report "RTO WithPage";
    begin
        Rpt.Execute('');
        exit(Format(Rpt.UseRequestPage()));
    end;

    procedure RequestPageOnInitWithoutPage(): Text
    var
        R: Record "RTO Row";
        L: Codeunit "RTO Log";
        Rpt: Report "RTO RpInit";
        Rpt2: Report "RTO RpInit Proc";
        Rpt3: Report "RTO RpInit Proc";
        OutS: OutStream;
        Result: Text;
    begin
        Seed();
        Commit();
        L.Take();
        R.Get(1);
        R.Data.CreateOutStream(OutS);
        Rpt.SaveAs('', ReportFormat::Xml, OutS);
        Result := 'saveas=' + L.Take();
        Rpt2.Execute('');
        Result += ' execute=' + L.Take();
        Rpt3.UseRequestPage(false);
        Rpt3.Run();
        exit(Result + ' run=' + L.Take());
    end;

    procedure SaveAsPreReportError(): Text
    var
        R: Record "RTO Row";
        Rpt: Report "RTO PreError";
        OutS: OutStream;
        Ok: Boolean;
    begin
        Seed();
        R.Get(1);
        R.Data.CreateOutStream(OutS);
        Ok := Rpt.SaveAs('', ReportFormat::Xml, OutS);
        exit(Format(Ok));
    end;

    procedure SecondSaveAs(): Text
    var
        R: Record "RTO Row";
        Rpt: Report "RTO RpInit";
        OutS: OutStream;
        Result: Text;
    begin
        Seed();
        R.Get(1);
        R.Data.CreateOutStream(OutS);
        Result := Format(Rpt.SaveAs('', ReportFormat::Html, OutS)) + ':' + GetLastErrorText();
        ClearLastError();
        Result += '|' + Format(Rpt.SaveAs('', ReportFormat::Pdf, OutS)) + ':' + GetLastErrorText();
        exit(Result);
    end;

    procedure StaticRunModalWithWindowOnNoPageReport()
    begin
        Report.RunModal(Report::"RTO NoPage", true);
    end;

    procedure GetterAfterExecuteOfSetVariable(): Text
    var Rpt: Report "RTO WithPage";
    begin
        Rpt.UseRequestPage(false);
        Rpt.Execute('');
        exit(Format(Rpt.UseRequestPage()));
    end;

    procedure SaveAsPreReportErrorLastError(): Text
    var
        R: Record "RTO Row";
        Rpt: Report "RTO PreError";
        OutS: OutStream;
        Ok: Boolean;
    begin
        Seed();
        R.Get(1);
        R.Data.CreateOutStream(OutS);
        Ok := Rpt.SaveAs('', ReportFormat::Xml, OutS);
        exit(GetLastErrorText());
    end;

    procedure SecondSaveAsCode(): Text
    var
        R: Record "RTO Row";
        Rpt: Report "RTO RpInit";
        OutS: OutStream;
        Ok: Boolean;
    begin
        Seed();
        R.Get(1);
        R.Data.CreateOutStream(OutS);
        Ok := Rpt.SaveAs('', ReportFormat::Html, OutS);
        Ok := Rpt.SaveAs('', ReportFormat::Pdf, OutS);
        exit(GetLastErrorCode());
    end;

    procedure RunTwice(): Text
    var
        L: Codeunit "RTO Log";
        Rpt: Report "RTO Processing";
        First: Text;
    begin
        Seed();
        L.Take();
        Rpt.Run();
        First := L.Take();
        Rpt.Run();
        exit(First + '|' + L.Take());
    end;
}
"#;

/// reports/request-page/RptTriggerOrder.al:308,:324 (SaveAs_NestedItems_Order, SaveAs_Siblings_Order) and
/// reports/PnH5aExt.al:380: a SaveAs of a processing-only report runs OnInitReport, then answers false.
#[test]
fn saveas_of_a_processing_only_report_runs_oninitreport_then_answers_false() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "SaveAsProcessingOnly")),
        "No|init;|ReportProcessingOnly"
    );
}

/// reports/request-page/RptTriggerOrder.al:353 (VariableRun_Twice_InitReportEachTime) and
/// reports/PnH5aExt.al:396: a second Run on one variable runs OnInitReport again.
#[test]
fn a_second_run_on_one_variable_runs_oninitreport_again() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "RunTwice")),
        "init;pre;row1;row2;post;|init;pre;row1;row2;post;"
    );
}

/// reports/request-page/P147UseRequestPagePreGet.al:79: `Report.Run(id, false, false)`
/// runs without the page and `CurrReport.UseRequestPage()` then answers false.
#[test]
fn static_run_with_requestwindow_false_answers_false_to_userequestpage() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "StaticRunWithoutWindow")),
        "init;pre get=No;"
    );
}

/// reports/PnH2UseRp.al:347-355: `Report.Run(id, true)` on a report declaring `UseRequestPage = false`
/// shows the request page (with no handler: the error, after OnInitReport only).
#[test]
fn static_run_with_requestwindow_true_shows_the_page_of_a_report_that_declares_none() {
    assert_eq!(
        text(al_testkit::run_in(
            FIXTURE,
            "RTO Tests",
            "StaticRunWithWindowOnNoPageReport"
        )),
        "init;|An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance."
    );
}

/// reports/PnH2UseRp.al:388 (WithRP_UseFalse_Run_RunModal_Execute): a RunModal whose request page cannot
/// open (a write is pending) leaves the variable reset, so the Execute that follows runs OnInitReport again.
#[test]
fn a_runmodal_refused_at_its_request_page_resets_the_variable() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "RunModalRefusedThenExecute")),
        "init;DB:Ok|init;"
    );
}

/// reports/PnH2UseRp.al:389 (WithRP_UseFalse_Run_RunModal_Execute, `getter`): after an Execute on a variable
/// whose UseRequestPage was never set, the getter answers false, although the report shows its page by default.
#[test]
fn userequestpage_answers_false_after_an_execute() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "GetterAfterExecute")),
        "No"
    );
}

/// The request page's OnInit runs although the page is not shown, after OnInitReport and before OnPreReport; OnOpenPage does
/// not run: SaveAs (text/PnBReportSaveAs.al:348 TriggerOrder, :262 Rows_SkipAndBreak), Execute and Run
/// with UseRequestPage(false) (reports/reporthandler/AeReportHandler.al:85, :121).
#[test]
fn the_request_page_oninit_runs_when_the_page_is_not_shown() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "RequestPageOnInitWithoutPage")),
        "saveas=init;rp init;pre; execute=init;rp init;pre; run=init;rp init;pre;"
    );
}

/// text/PnBReportSaveAs.al:412-413 (TriggerError_InPreReport): an error in OnPreReport of a report without
/// a layout makes a SaveAs whose answer is used answer false, and nothing is raised.
#[test]
fn an_error_in_onprereport_makes_a_used_saveas_answer_false() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "SaveAsPreReportError")),
        "No"
    );
}

/// text/PnBReportSaveAs.al:387-391 (OtherFormats_NoLayout): after a SaveAs that stopped at the missing
/// layout, a second SaveAs on the variable answers false with "The object has been run".
#[test]
fn a_second_saveas_on_one_variable_answers_false_with_the_object_has_been_run() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "RTO Tests", "SecondSaveAs")),
        "No:Report 50956 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.|No:The object has been run. Use the function CLEAR(Report)."
    );
}

fn assert_refused(r: Result<al_builtins::value::Value, al_builtins::native::Signal>, needle: &str) {
    match r {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains(needle), "{m}"),
        other => panic!("expected a refusal containing {needle:?}, got {other:?}"),
    }
}

/// Only `Report.Run(id, true)` is pinned on a report declaring `UseRequestPage = false` (PnH2UseRp.al:347); the static
/// RunModal form is refused.
#[test]
fn static_runmodal_with_requestwindow_true_on_a_report_that_declares_none_is_refused() {
    assert_refused(
        al_testkit::run_in(FIXTURE, "RTO Tests", "StaticRunModalWithWindowOnNoPageReport"),
        "RunModal(id, true)",
    );
}

/// PnH2UseRp.al:389 pins the getter after an Execute only for a variable whose UseRequestPage was never set.
#[test]
fn the_getter_after_an_execute_of_a_set_variable_is_refused() {
    assert_refused(
        al_testkit::run_in(FIXTURE, "RTO Tests", "GetterAfterExecuteOfSetVariable"),
        "UseRequestPage of report",
    );
}

/// PnBReportSaveAs.al:412-413 pins only the answer of the SaveAs; what GetLastErrorText then holds is refused.
#[test]
fn the_last_error_after_a_failed_used_saveas_without_layout_is_refused() {
    assert_refused(
        al_testkit::run_in(FIXTURE, "RTO Tests", "SaveAsPreReportErrorLastError"),
        "GetLastErrorText",
    );
}

/// PnBReportSaveAs.al:387 pins the text of the second SaveAs's error, not its code.
#[test]
fn the_error_code_of_a_second_saveas_is_refused() {
    assert_refused(
        al_testkit::run_in(FIXTURE, "RTO Tests", "SecondSaveAsCode"),
        "GetLastErrorCode",
    );
}
