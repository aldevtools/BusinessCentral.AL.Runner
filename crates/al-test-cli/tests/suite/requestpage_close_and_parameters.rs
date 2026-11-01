//! The request page of a report: a handler that raises closes the page with Cancel first, the actions of
//! the page are not found by the test client, and the parameters text `RunRequestPage` returns. Each test
//! names the corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61400 "RC Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Flag; Boolean) { } }
    keys { key(PK; Id) { } key(ByName; Name, Id) { } }
}
codeunit 61401 "RC Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
report 61402 "RC Close"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RC Row") { } }
    requestpage
    {
        layout { area(Content) { field(TextCtl; TextVal) { ApplicationArea = All; } } }
        actions { area(Processing) { action(Plain) { trigger OnAction() begin L.Add('OnAction'); end; } action(HiddenAct) { Visible = false; } action(DisabledAct) { Enabled = false; } } }
        trigger OnQueryClosePage(CloseAction: Action): Boolean begin L.Add('qcp ' + Format(CloseAction)); exit(true); end;
        trigger OnClosePage() begin L.Add('cp'); end;
    }
    var TextVal: Text[30]; L: Codeunit "RC Log";
    trigger OnPreReport() begin L.Add('pre'); end;
}
reportextension 61404 "RC Close Ext" extends "RC Plain"
{
    requestpage { trigger OnClosePage() begin L.Add('ext cp'); end; }
    var L: Codeunit "RC Log";
}
report 61405 "RC Plain"
{
    ProcessingOnly = true;
    requestpage
    {
        layout { area(Content) { field(TextCtl; TextVal) { ApplicationArea = All; } } }
        trigger OnClosePage() begin L.Add('cp'); end;
    }
    var TextVal: Text[30]; L: Codeunit "RC Log";
}
report 61403 "RC Params"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Row; "RC Row") { DataItemTableView = sorting(Name) order(descending) where(Id = filter(1..4)); RequestFilterFields = Id, Flag; }
    }
    requestpage
    {
        layout { area(Content) { field(TxtCtl; TxtVal) { ApplicationArea = All; } field(DateCtl; DateVal) { ApplicationArea = All; } field(IntCtl; IntVal) { ApplicationArea = All; } } }
    }
    var TxtVal: Text[30]; DateVal: Date; IntVal: Integer;
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 61499 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

/// The reason the one test in `tests` is not run.
fn refused(tests: &str) -> String {
    let o = outcomes(tests);
    match &o[..] {
        [(_, Outcome::NotRun(r))] => r.clone(),
        other => panic!("expected one refusal, got {other:?}"),
    }
}

const HELPERS: &str = "var L: Codeunit \"RC Log\";
local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
local procedure Between(T: Text; A: Text; B: Text): Text var S: Integer; begin S := T.IndexOf(A) + StrLen(A); exit(CopyStr(T, S, T.IndexOf(B) - S)); end;";

// report/lt-report-requestpage/RptRequestPage.al:304, :413, :453: an error the handler does not catch closes the page with
// Cancel (OnQueryClosePage, then OnClosePage) before it leaves the run; the report does not run.
#[test]
fn a_handler_error_closes_the_request_page_with_cancel_first() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('FailHandler')]
        procedure HandlerFails() begin
            L.Take();
            asserterror Report.Run(Report::\"RC Close\");
            Check('boom', GetLastErrorText());
            Check('h;qcp Cancel;cp;', L.Take()); end;
        [RequestPageHandler] procedure FailHandler(var RP: TestRequestPage \"RC Close\") begin L.Add('h'); Error('boom'); end;"
    ));
}

// Whether the close triggers of a request page extension run after a handler error is not pinned.
#[test]
fn a_handler_error_on_a_request_page_whose_extension_has_close_triggers_is_refused() {
    let why = refused(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('FailHandler')]
        procedure HandlerFails() begin asserterror Report.Run(Report::\"RC Plain\"); end;
        [RequestPageHandler] procedure FailHandler(var RP: TestRequestPage \"RC Plain\") begin Error('boom'); end;"
    ));
    assert!(why.contains("close triggers in a request page extension"), "{why}");
}

// report/W3ReportActions.al:186: Visible, Enabled and Invoke of every action of a request page raise the not-found error,
// whatever the action's Visible and Enabled say, and OnAction does not run.
#[test]
fn visible_and_enabled_of_a_request_page_action_are_not_found() {
    passes(&format!(
        "{HELPERS}
        [TryFunction] local procedure TryVisible(var RP: TestRequestPage \"RC Close\"; Which: Integer; var B: Boolean) begin
            case Which of 1: B := RP.Plain.Visible(); 2: B := RP.HiddenAct.Visible(); 3: B := RP.DisabledAct.Visible(); end; end;
        [TryFunction] local procedure TryEnabled(var RP: TestRequestPage \"RC Close\"; Which: Integer; var B: Boolean) begin
            case Which of 1: B := RP.Plain.Enabled(); 2: B := RP.HiddenAct.Enabled(); 3: B := RP.DisabledAct.Enabled(); end; end;
        [Test] [HandlerFunctions('ActionHandler')]
        procedure Actions() begin
            L.Take(); Report.Run(Report::\"RC Close\");
            Check('v1 no;e1 no;v2 no;e2 no;v3 no;e3 no;qcp OK;cp;pre;', L.Take()); end;
        [RequestPageHandler] procedure ActionHandler(var RP: TestRequestPage \"RC Close\") var B: Boolean; I: Integer; begin
            for I := 1 to 3 do begin
                if TryVisible(RP, I, B) then L.Add('v' + Format(I) + ' found') else begin
                    if StrPos(GetLastErrorText(), 'is not found on the page.') = 0 then Error(GetLastErrorText()); L.Add('v' + Format(I) + ' no'); end;
                if TryEnabled(RP, I, B) then L.Add('e' + Format(I) + ' found') else begin
                    if StrPos(GetLastErrorText(), 'is not found on the page.') = 0 then Error(GetLastErrorText()); L.Add('e' + Format(I) + ' no'); end;
            end;
            RP.OK().Invoke(); end;"
    ));
}

// report/W3ReportParametersText.al:255 (empty values as an empty element, a descending view as ORDER(1)), :272 (a quote in a
// text value is not escaped), :306 (a Boolean filter as 1); report/W3ReportSaveAsExecute.al:209 (the filters of the
// DataItemTableView are not in the text, only the ones the page set).
#[test]
fn the_parameters_text_spells_values_and_views_as_bc_does() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('EmptyHandler')]
        procedure Defaults() var P: Text; begin
            P := Report.RunRequestPage(Report::\"RC Params\");
            Check('<Field name=\"TxtVal\" /><Field name=\"DateVal\" /><Field name=\"IntVal\">0</Field>', Between(P, '<Options>', '</Options>'));
            Check('<DataItem name=\"Row\">VERSION(1) SORTING(Field2,Field1) ORDER(1)</DataItem>', Between(P, '<DataItems>', '</DataItems>')); end;
        [Test] [HandlerFunctions('ValuesHandler')]
        procedure Values() var P: Text; begin
            P := Report.RunRequestPage(Report::\"RC Params\");
            Check('<Field name=\"TxtVal\">a&lt;b&amp;\"c\"&gt;</Field><Field name=\"DateVal\" /><Field name=\"IntVal\">0</Field>', Between(P, '<Options>', '</Options>'));
            Check('<DataItem name=\"Row\">VERSION(1) SORTING(Field2,Field1) ORDER(1) WHERE(Field1=1(3..9),Field3=1(1))</DataItem>', Between(P, '<DataItems>', '</DataItems>')); end;
        [RequestPageHandler] procedure EmptyHandler(var RP: TestRequestPage \"RC Params\") begin RP.OK().Invoke(); end;
        [RequestPageHandler] procedure ValuesHandler(var RP: TestRequestPage \"RC Params\") begin
            RP.TxtCtl.SetValue('a<b&\"c\">'); RP.Row.SetFilter(Id, '3..9'); RP.Row.SetFilter(Flag, 'Yes'); RP.OK().Invoke(); end;"
    ));
}

// A Boolean filter on No: the corpus pins only Yes as 1 (report/W3ReportParametersText.al:306).
#[test]
fn a_boolean_filter_on_no_in_the_parameters_text_is_refused() {
    let why = refused(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('NoHandler')]
        procedure NoFilter() var P: Text; begin P := Report.RunRequestPage(Report::\"RC Params\"); end;
        [RequestPageHandler] procedure NoHandler(var RP: TestRequestPage \"RC Params\") begin RP.Row.SetFilter(Flag, 'No'); RP.OK().Invoke(); end;"
    ));
    assert!(why.contains("Boolean filter"), "{why}");
}

const PREVIEW_OBJECTS: &str = r#"
report 61406 "RC Preview"
{
    DefaultRenderingLayout = Rdlc;
    dataset
    {
        dataitem(Row; "RC Row")
        {
            column(Id; Id) { }
            trigger OnAfterGetRecord() begin Log('get'); end;
        }
    }
    requestpage
    {
        layout { area(Content) { field(Ctl; CtlVal) { ApplicationArea = All; } } }
        trigger OnOpenPage() begin Log('rp open'); end;
        trigger OnQueryClosePage(CloseAction: Action): Boolean begin Log('rp close ' + Format(CloseAction)); exit(true); end;
    }
    rendering { layout(Rdlc) { Type = RDLC; LayoutFile = './RcPreview.rdl'; } }
    var CtlVal: Integer; L: Codeunit "RC Log";
    local procedure Log(Where: Text) begin L.Add(Where + ' preview=' + Format(CurrReport.Preview())); end;
    trigger OnInitReport() begin Log('init'); end;
    trigger OnPreReport() begin Log('pre'); end;
}
report 61407 "RC No Layout"
{
    dataset { dataitem(Row; "RC Row") { column(Id; Id) { } } }
    requestpage { layout { area(Content) { field(Ctl; CtlVal) { ApplicationArea = All; } } } }
    var CtlVal: Integer; L: Codeunit "RC Log";
    trigger OnPreReport() begin L.Add('pre'); end;
}
report 61408 "RC Layout"
{
    DefaultRenderingLayout = Rdlc;
    dataset { dataitem(Row; "RC Row") { column(Id; Id) { } } }
    requestpage { layout { area(Content) { field(Ctl; CtlVal) { ApplicationArea = All; } } } }
    rendering { layout(Rdlc) { Type = RDLC; LayoutFile = './RcLayout.rdl'; } }
    var CtlVal: Integer;
}
"#;

const PREVIEW_HELPERS: &str = "var L: Codeunit \"RC Log\"; Act: Text;
local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
local procedure Seed() var R: Record \"RC Row\"; begin R.DeleteAll(); R.Id := 1; R.Insert(); Commit(); L.Take(); end;
[RequestPageHandler] procedure PreviewHandler(var RP: TestRequestPage \"RC Preview\") begin
    case Act of 'ok': RP.OK().Invoke(); 'preview': RP.Preview().Invoke(); 'print': RP.Print().Invoke(); end;
end;
[RequestPageHandler] procedure NoLayoutHandler(var RP: TestRequestPage \"RC No Layout\") begin
    case Act of 'preview': RP.Preview().Invoke(); 'print': RP.Print().Invoke(); end;
    asserterror RP.Cancel().Invoke();
    L.Add(GetLastErrorText());
end;
[RequestPageHandler] procedure LayoutHandler(var RP: TestRequestPage \"RC Layout\") begin
    case Act of 'preview': RP.Preview().Invoke(); 'print': RP.Print().Invoke(); end;
end;
[RequestPageHandler] procedure CloseHandler(var RP: TestRequestPage \"RC Close\") begin RP.Preview().Invoke(); end;";

fn preview_outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\n{PREVIEW_OBJECTS}\ncodeunit 61499 T {{ Subtype = Test; {PREVIEW_HELPERS} {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

// report/W3ReportObjectIdPreviewLanguage.al:484-511 Preview_RequestPagePreviewAction: the Preview action closes the request page
// with the CloseAction Preview; a report without a valid layout then fails with ReportNoLayout, as after OK, and the page is no
// longer open.
#[test]
fn the_preview_action_closes_the_request_page_of_a_report_without_a_layout() {
    let o = preview_outcomes(
        "[Test] [HandlerFunctions('NoLayoutHandler')]
        procedure P() begin
            Seed(); Act := 'preview';
            asserterror Report.Run(Report::\"RC No Layout\");
            Check('ReportNoLayout', GetLastErrorCode());
            Check('The TestPage is not open.;', L.Take());
        end;",
    );
    assert_eq!(o, vec![("P".to_string(), Outcome::Pass)]);
}

// report/W3ReportObjectIdPreviewLanguage.al:696-714 QV_Preview_RunRequestPage: a handler that names Preview and Print but presses
// OK on the page RunRequestPage opens runs, and the page's triggers answer CurrReport.Preview() false.
#[test]
fn a_handler_that_names_preview_runs_when_it_presses_ok() {
    let o = preview_outcomes(
        "[Test] [HandlerFunctions('PreviewHandler')]
        procedure P() begin
            Seed(); Act := 'ok';
            Check('Yes', Format(Report.RunRequestPage(Report::\"RC Preview\") <> ''));
            Check('init preview=No;rp open preview=No;rp close OK preview=No;', L.Take());
        end;",
    );
    assert_eq!(o, vec![("P".to_string(), Outcome::Pass)]);
}

// Not pinned, or not built: Preview and Print of a report with a layout (W3ReportObjectIdPreviewLanguage.al:717-752 pins the
// triggers, but the runtime renders no output), on a request page with OnQueryClosePage (the CloseAction Preview and Print is not
// built), Print of a report without a layout, either on the page RunRequestPage opens or on a processing-only report.
#[test]
fn preview_and_print_where_they_are_not_pinned_are_refused() {
    for (handler, run, needle) in [
        (
            "LayoutHandler",
            "Act := 'preview'; Report.Run(Report::\"RC Layout\");",
            "renders output",
        ),
        (
            "LayoutHandler",
            "Act := 'print'; Report.Run(Report::\"RC Layout\");",
            "renders output",
        ),
        (
            "PreviewHandler",
            "Act := 'preview'; Report.Run(Report::\"RC Preview\");",
            "OnQueryClosePage",
        ),
        (
            "PreviewHandler",
            "Act := 'print'; Report.Run(Report::\"RC Preview\");",
            "OnQueryClosePage",
        ),
        (
            "NoLayoutHandler",
            "Act := 'print'; Report.Run(Report::\"RC No Layout\");",
            "not pinned",
        ),
        (
            "PreviewHandler",
            "Act := 'preview'; Report.RunRequestPage(Report::\"RC Preview\");",
            "not pinned",
        ),
        (
            "PreviewHandler",
            "Act := 'print'; Report.RunRequestPage(Report::\"RC Preview\");",
            "not pinned",
        ),
        ("CloseHandler", "Report.Run(Report::\"RC Close\");", "not pinned"),
    ] {
        let o = preview_outcomes(&format!(
            "[Test] [HandlerFunctions('{handler}')] procedure P() begin Seed(); {run} end;"
        ));
        match &o[..] {
            [(_, Outcome::NotRun(r))] => assert!(r.contains(needle), "{run}: {r}"),
            other => panic!("{run}: expected a refusal, got {other:?}"),
        }
    }
}
