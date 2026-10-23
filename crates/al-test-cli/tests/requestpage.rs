//! The request page of a report (ADR-012 follow-up): AL source → compile → run as a test codeunit
//! with `[RequestPageHandler]`s. Each test names the corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61200 "RQ Row"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } }
    keys { key(PK; "Entry No.") { } }
}
table 61201 "RQ Log"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Marker; Text[50]) { } }
    keys { key(PK; "Entry No.") { } }
    procedure Log(NewMarker: Text[50])
    var LogRec: Record "RQ Log"; NextNo: Integer;
    begin
        if LogRec.FindLast() then NextNo := LogRec."Entry No.";
        LogRec.Init(); LogRec."Entry No." := NextNo + 1; LogRec.Marker := NewMarker; LogRec.Insert();
    end;
    procedure MarkerCount(Wanted: Text[50]): Integer
    var LogRec: Record "RQ Log";
    begin
        LogRec.SetRange(Marker, Wanted);
        exit(LogRec.Count());
    end;
}
codeunit 61202 "RQ Trace"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Reset() begin Trail := ''; end;
    procedure Add(T: Text) begin Trail += T + ','; end;
    procedure Text(): Text begin exit(Trail); end;
}
// handlers/TestReportRunRequestPageReport.al
report 61210 "RQ Report"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Rows; "RQ Row")
        {
            trigger OnAfterGetRecord()
            var LogRec: Record "RQ Log";
            begin
                LogRec.Log('rp-row');
            end;
        }
    }
    requestpage
    {
        layout { area(Content) { group(Options) { field(EchoText; EchoText) { ApplicationArea = All; } } } }
        trigger OnOpenPage()
        var LogRec: Record "RQ Log";
        begin
            LogRec.Log('rp-open');
        end;
    }
    var EchoText: Text[50];
    procedure GetEchoText(): Text[50] begin exit(EchoText); end;
}
// handlers/TestReportRequestPageControlReport.al
report 61211 "RQ Ctrl"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Rows; "RQ Row")
        {
            trigger OnAfterGetRecord()
            var LogRec: Record "RQ Log";
            begin
                LogRec.Log(CopyStr('body-text:' + EchoText, 1, 50));
                if IncludeAll then LogRec.Log('body-flag:on') else LogRec.Log('body-flag:off');
            end;
        }
    }
    requestpage
    {
        layout { area(Content) { group(Options) {
            field(EchoText; EchoText) { ApplicationArea = All; Caption = 'Echo Text'; }
            field(IncludeAll; IncludeAll) { ApplicationArea = All; Caption = 'Include All'; }
        } } }
    }
    trigger OnInitReport() begin EchoText := 'from-report'; IncludeAll := false; end;
    var EchoText: Text[50]; IncludeAll: Boolean;
}
// handlers/TestReportRequestPageActionReport.al
report 61212 "RQ Action"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Rows; "RQ Row")
        {
            trigger OnAfterGetRecord()
            var LogRec: Record "RQ Log";
            begin
                LogRec.Log(CopyStr('body-text:' + EchoText, 1, 50));
                LogRec.Log(CopyStr(StrSubstNo('body-actions:%1', ActionRuns), 1, 50));
            end;
        }
    }
    requestpage
    {
        layout { area(Content) { group(Options) { field(EchoText; EchoText) { ApplicationArea = All; } } } }
        actions
        {
            area(Processing)
            {
                action(StampText) { ApplicationArea = All; trigger OnAction() begin EchoText := 'set-by-action'; ActionRuns += 1; end; }
                action(RefuseAction) { ApplicationArea = All; trigger OnAction() begin Error('refused'); end; }
            }
        }
    }
    trigger OnInitReport() begin EchoText := 'from-report'; ActionRuns := 0; end;
    var EchoText: Text[50]; ActionRuns: Integer;
}
// handlers/TestRequestPageOnInitReport.al
report 61213 "RQ Init"
{
    ProcessingOnly = true;
    dataset { dataitem(Rows; "RQ Row") { } }
    requestpage
    {
        layout { area(Content) {
            field(ReqInitField; ReqInitVar) { ApplicationArea = All; }
            field(ReqOpenField; ReqOpenVar) { ApplicationArea = All; }
            field(RptInitField; RptInitVar) { ApplicationArea = All; }
        } }
        trigger OnInit() begin ReqInitVar := 'req-oninit'; end;
        trigger OnOpenPage() begin ReqOpenVar := 'req-onopenpage'; end;
    }
    trigger OnInitReport() begin RptInitVar := 'rpt-oninitreport'; end;
    var ReqInitVar: Text[30]; ReqOpenVar: Text[30]; RptInitVar: Text[30];
}
// handlers/TestReportRequestPageSaveValues.al
report 61214 "RQ Saved"
{
    ProcessingOnly = true;
    requestpage { SaveValues = true; layout { area(Content) { field(Remembered; RememberedTxt) { ApplicationArea = All; } } } }
    trigger OnPreReport() var T: Codeunit "RQ Trace"; begin T.Add('ran:' + RememberedTxt); end;
    var RememberedTxt: Text[30];
}
report 61222 "RQ Saved Params"
{
    ProcessingOnly = true;
    requestpage { SaveValues = true; layout { area(Content) { field(Remembered; RememberedTxt) { ApplicationArea = All; } } } }
    var RememberedTxt: Text[30];
}
report 61215 "RQ Unsaved"
{
    ProcessingOnly = true;
    requestpage { layout { area(Content) { field(Remembered; RememberedTxt) { ApplicationArea = All; } } } }
    trigger OnPreReport() var T: Codeunit "RQ Trace"; begin T.Add('ran:' + RememberedTxt); end;
    var RememberedTxt: Text[30];
}
// handlers/TestReportRequestPageCaption.al
report 61216 "RQ Both Captions"
{
    Caption = 'RQ Report Caption'; ProcessingOnly = true;
    requestpage { Caption = 'RQ Request Page Caption'; layout { area(Content) { field(Dummy; DummyTxt) { ApplicationArea = All; } } } }
    var DummyTxt: Text[30];
}
report 61217 "RQ Request Caption Only"
{
    ProcessingOnly = true;
    requestpage { Caption = 'RQ Only Request Page Caption'; layout { area(Content) { field(Dummy; DummyTxt) { ApplicationArea = All; } } } }
    var DummyTxt: Text[30];
}
// handlers/TestReqPageBoundVisibilityObjects.al
report 61218 "RQ Visible"
{
    ProcessingOnly = true;
    requestpage
    {
        layout { area(Content) {
            group(Options) { field(BoundField; BoundValue) { ApplicationArea = All; Visible = ShowFieldGlobal; Editable = EditFieldGlobal; } }
            group(Toggled) { Visible = ShowGroupGlobal; field(InGroupField; InGroupValue) { ApplicationArea = All; } }
        } }
        trigger OnOpenPage() begin ShowFieldGlobal := ShowField; EditFieldGlobal := EditField; ShowGroupGlobal := ShowGroup; end;
    }
    procedure Flags(NewShowField: Boolean; NewEditField: Boolean; NewShowGroup: Boolean) begin ShowField := NewShowField; EditField := NewEditField; ShowGroup := NewShowGroup; end;
    var BoundValue: Text[30]; InGroupValue: Text[30]; ShowFieldGlobal: Boolean; EditFieldGlobal: Boolean; ShowGroupGlobal: Boolean; ShowField: Boolean; EditField: Boolean; ShowGroup: Boolean;
}
// testpageclose/TestPageClosingActions.al: report 69650 "TPC Report"
report 61219 "RQ Closing"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RQ Row") { } }
    requestpage
    {
        layout { area(Content) { field(QtyOpt; QtyOption) { ApplicationArea = All; } } }
        trigger OnOpenPage() var T: Codeunit "RQ Trace"; begin T.Add('open'); end;
        trigger OnClosePage() var T: Codeunit "RQ Trace"; begin T.Add('close'); end;
        trigger OnQueryClosePage(CloseAction: Action): Boolean var T: Codeunit "RQ Trace"; begin T.Add('qcp:' + Format(CloseAction)); exit(true); end;
    }
    var QtyOption: Integer;
}
// handlers/TestRequestPageControlErrorName.al:Ext5803Handler (cases 21-24): a reportextension that only adds request
// page controls bound to the PROTECTED variables of the base report (Base App's "Mfg. Reset Cost Is Adjusted").
report 61240 "RQ Protected Base"
{
    ProcessingOnly = true;
    requestpage { layout { area(Content) { group(Options) { field(BaseCtl; BaseFlag) { ApplicationArea = All; } } } } }
    var BaseFlag: Boolean;
    protected var ProdNo: Code[20]; ProdFlag: Boolean;
}
reportextension 61241 "RQ Protected Base Ext" extends "RQ Protected Base"
{
    requestpage
    {
        layout { addafter(Options) { group(ProdGroup)
        {
            field("Prod Flag"; ProdFlag) { ApplicationArea = All; Caption = 'Adjust Production Orders'; }
            field("Prod No."; ProdNo) { ApplicationArea = All; Caption = 'No.'; }
        } } }
    }
}
// handlers/TestReportRequestPageBoundVisibility (60008) and Rpt RequestPage SourceTable (60928): a request page
// that says ShowFilter = false.
report 61242 "RQ No Filters"
{
    ProcessingOnly = true;
    dataset { dataitem(Rows; "RQ Row") { } }
    requestpage { ShowFilter = false; layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } } }
    var Opt: Text[30];
}
// A report without a requestpage section: the page still shows the data items' filters.
report 61220 "RQ Default Page"
{
    ProcessingOnly = true;
    dataset { dataitem(Rows; "RQ Row") { trigger OnAfterGetRecord() var LogRec: Record "RQ Log"; begin LogRec.Log('default-row'); end; } }
}
// handlers/TestRequestPageControlErrorName.al: report 68660 and its reportextension
report 61223 "RQ Ext Base"
{
    ProcessingOnly = true;
    requestpage
    {
        layout { area(Content) { group(Options) {
            field(RpnTextCtl; RpnText) { ApplicationArea = All; Caption = 'Rpn Text Caption'; }
            field(OptCtl; Opt) { ApplicationArea = All; Caption = 'Request Option'; OptionCaption = 'Small,Large'; }
        } } }
        trigger OnOpenPage() var T: Codeunit "RQ Trace"; begin T.Add('base-open'); end;
    }
    trigger OnInitReport() begin RpnText := 'Delta'; end;
    var RpnText: Text[30]; Opt: Option Small,Large;
}
reportextension 61223 "RQ Ext Base Ext" extends "RQ Ext Base"
{
    requestpage
    {
        layout
        {
            addlast(Options)
            {
                field(RpnExtCtl; RpnExtVar) { ApplicationArea = All; Caption = 'Rpn Ext Caption'; }
                field(ExtValidatedCtl; ExtValidatedValue)
                {
                    ApplicationArea = All;
                    trigger OnValidate() begin ExtDerivedValue := 'D-' + ExtValidatedValue; end;
                }
                field(ExtDerivedCtl; ExtDerivedValue) { ApplicationArea = All; }
            }
            modify(OptCtl) { Caption = 'Ext Request Option'; OptionCaption = 'Smaller,Larger'; }
        }
        trigger OnOpenPage() var T: Codeunit "RQ Trace"; begin T.Add('ext-open'); end;
    }
    var RpnExtVar: Text[30]; ExtValidatedValue: Text[30]; ExtDerivedValue: Text[30];
}
// handlers/TestReportRunRequestPageDatasetReport.al: not ProcessingOnly, with a rendering layout
report 61224 "RQ Dataset"
{
    DefaultRenderingLayout = RqLayout;
    dataset { dataitem(Rows; "RQ Row") { column(EntryNo; "Entry No.") { } } }
    requestpage { layout { area(Content) { field(EchoText; EchoText) { ApplicationArea = All; } } } }
    rendering { layout(RqLayout) { Type = RDLC; LayoutFile = './x.rdl'; } }
    var EchoText: Text[50];
}
report 61225 "RQ Dataset No Page"
{
    DefaultRenderingLayout = RqLayout;
    UseRequestPage = false;
    dataset { dataitem(Rows; "RQ Row") { column(EntryNo; "Entry No.") { } } }
    rendering { layout(RqLayout) { Type = RDLC; LayoutFile = './x.rdl'; } }
}
// record/TestTxModelReportRunMarker.Reports.al, with a table of numbers for Integer
table 61226 "RQ Num"
{
    fields { field(1; Number; Integer) { } }
    keys { key(PK; Number) { } }
}
table 61227 "RQ Base"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Entry No.") { } }
}
report 61226 "RQ Tx Marker"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Loop; "RQ Num")
        {
            DataItemTableView = sorting(Number) where(Number = const(1));
            trigger OnAfterGetRecord()
            var B: Record "RQ Base";
            begin
                B.Init(); B."Entry No." := MarkerNo; B.Insert();
                if FailAfterWrite then Error('rq report body error');
            end;
        }
    }
    requestpage { layout { area(Content) { group(Options) { field(EchoText; EchoText) { ApplicationArea = All; } } } } }
    var MarkerNo: Integer; EchoText: Text[30]; FailAfterWrite: Boolean;
    procedure SetMarker(NewMarkerNo: Integer) begin MarkerNo := NewMarkerNo; end;
    procedure FailAfterTheWrite() begin FailAfterWrite := true; end;
}
report 61221 "RQ No Page"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(Rows; "RQ Row") { trigger OnAfterGetRecord() var LogRec: Record "RQ Log"; begin LogRec.Log('nopage-row'); end; } }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 61300 T {{ Subtype = Test; {tests} }}");
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

const HELPERS: &str = "var HandlerRan: Boolean; Seen: Text; Param: Text;
local procedure Seed() var R: Record \"RQ Row\"; L: Record \"RQ Log\"; T: Codeunit \"RQ Trace\"; begin
    L.DeleteAll(); R.DeleteAll(); T.Reset(); HandlerRan := false; Seen := ''; Param := '';
    R.Init(); R.\"Entry No.\" := 1; R.Name := 'first'; R.Insert();
    R.Init(); R.\"Entry No.\" := 2; R.Name := 'second'; R.Insert();
    Commit();
end;
local procedure Count(M: Text[50]): Integer var L: Record \"RQ Log\"; begin exit(L.MarkerCount(M)); end;
local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;";

// handlers/TestReportRunWithRequestPage.al:Report_Run_OpensTheRequestPageAndRunsTheBodyWhenTheHandlerConfirms
#[test]
fn run_opens_the_request_page_and_confirming_runs_the_body() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('OkHandler')]
        procedure Confirmed() var Probe: Report \"RQ Report\"; begin
            Seed(); Clear(Probe); Probe.Run();
            Check('1', Format(Count('rp-open'))); if not HandlerRan then Error('handler');
            Check('2', Format(Count('rp-row'))); end;
        [RequestPageHandler] procedure OkHandler(var RP: TestRequestPage \"RQ Report\") begin HandlerRan := true; RP.OK().Invoke(); end;"
    ));
}

// handlers/TestReportRunWithRequestPage.al:Report_Run_CancelledRequestPageRunsNothingAndRaisesNoError
#[test]
fn a_cancelled_request_page_stops_the_run_without_an_error() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('CancelHandler')]
        procedure Cancelled() var Probe: Report \"RQ Report\"; begin
            Seed(); Probe.Run();
            Check('1', Format(Count('rp-open'))); if not HandlerRan then Error('handler');
            Check('0', Format(Count('rp-row'))); end;
        [RequestPageHandler] procedure CancelHandler(var RP: TestRequestPage \"RQ Report\") begin HandlerRan := true; RP.Cancel().Invoke(); end;"
    ));
}

// handlers/TestReportRunRequestPage.al:TestReport_RunRequestPage_RunsTheHandlerAndReturnsTheFilterItSet,
// ..._CancelledHandlerReturnsNoParameters
#[test]
fn run_request_page_returns_the_parameters_with_the_filter_the_handler_set() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('FilterHandler')]
        procedure Confirmed() begin
            Seed(); Param := Report.RunRequestPage(Report::\"RQ Report\");
            Check('1', Format(Count('rp-open'))); if not HandlerRan then Error('handler');
            if StrPos(Param, 'ReportParameters') = 0 then Error(Param);
            if StrPos(Param, 'DataItem name=\"Rows\"') = 0 then Error(Param);
            if StrPos(Param, 'WHERE(Field1=1') = 0 then Error(Param);
            Check('0', Format(Count('rp-row'))); end;
        [Test] [HandlerFunctions('CancelHandler')]
        procedure Cancelled() begin
            Seed(); Param := Report.RunRequestPage(Report::\"RQ Report\");
            Check('1', Format(Count('rp-open'))); Check('', Param); end;
        [RequestPageHandler] procedure FilterHandler(var RP: TestRequestPage \"RQ Report\") begin
            HandlerRan := true; RP.Rows.SetFilter(\"Entry No.\", '1'); RP.OK().Invoke(); end;
        [RequestPageHandler] procedure CancelHandler(var RP: TestRequestPage \"RQ Report\") begin RP.Cancel().Invoke(); HandlerRan := true; end;"
    ));
}

// handlers/TestReportRunWithRequestPage.al: what the handler filters is what the run reads
#[test]
fn a_filter_the_handler_sets_narrows_the_run() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('FilterHandler')]
        procedure Filtered() var Probe: Report \"RQ Report\"; begin
            Seed(); Probe.Run(); Check('1', Format(Count('rp-row'))); end;
        [RequestPageHandler] procedure FilterHandler(var RP: TestRequestPage \"RQ Report\") begin
            RP.Rows.SetFilter(\"Entry No.\", '2'); RP.OK().Invoke(); end;"
    ));
}

// handlers/TestReportRequestPageControl.al (four tests)
#[test]
fn request_page_controls_are_the_reports_variables() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('SettingHandler')]
        procedure Write() begin
            Seed(); Report.Run(Report::\"RQ Ctrl\");
            Check('1', Format(Count('body-text:set-by-handler') div 2)); Check('0', Format(Count('body-text:from-report')));
            Check('2', Format(Count('body-flag:on'))); Check('0', Format(Count('body-flag:off'))); end;
        [Test] [HandlerFunctions('ReadingHandler')]
        procedure Read() begin
            Seed(); Report.Run(Report::\"RQ Ctrl\"); Check('from-report', Seen); end;
        [Test] [HandlerFunctions('RejectingHandler')]
        procedure Reject() begin
            Seed(); Report.Run(Report::\"RQ Ctrl\");
            if not HandlerRan then Error('handler');
            Check('0', Format(Count('body-flag:on'))); Check('0', Format(Count('body-flag:off'))); end;
        [RequestPageHandler] procedure SettingHandler(var RP: TestRequestPage \"RQ Ctrl\") begin
            RP.EchoText.SetValue('set-by-handler'); RP.IncludeAll.SetValue(true);
            if RP.EchoText.Value() <> 'set-by-handler' then Error('readback');
            RP.OK().Invoke(); end;
        [RequestPageHandler] procedure ReadingHandler(var RP: TestRequestPage \"RQ Ctrl\") begin Seen := RP.EchoText.Value(); RP.OK().Invoke(); end;
        [RequestPageHandler] procedure RejectingHandler(var RP: TestRequestPage \"RQ Ctrl\") begin
            HandlerRan := true;
            asserterror RP.IncludeAll.SetValue('Maybe');
            if StrPos(GetLastErrorText(), 'Maybe') = 0 then Error(GetLastErrorText());
            RP.Cancel().Invoke(); end;"
    ));
}

// handlers/TestRequestPageControlErrorName.al:SourceReport_BooleanControl_SetValue_Refusal_NamesTheControlAndTheCaption,
// ..._AssertEquals_Mismatch_NamesTheControl
#[test]
fn control_errors_name_the_control_and_its_caption() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Names')]
        procedure ControlErrors() begin
            Seed(); Report.Run(Report::\"RQ Ctrl\");
            Check('Validation error for Field: IncludeAll,  Message = ''Your entry of ''Maybe'' is not an acceptable value for ''Include All''.''', Seen);
            Check('AssertEquals for Field: EchoText Expected = ''Wrong'', Actual = ''from-report''', Param); end;
        [RequestPageHandler] procedure Names(var RP: TestRequestPage \"RQ Ctrl\") begin
            asserterror RP.IncludeAll.SetValue('Maybe'); Seen := GetLastErrorText();
            asserterror RP.EchoText.AssertEquals('Wrong'); Param := GetLastErrorText();
            RP.Cancel().Invoke(); end;"
    ));
}

// handlers/TestReportInstanceRunReqPage.al: the instance form leaves the control value on the instance
#[test]
fn instance_run_request_page_leaves_the_control_value_on_the_instance() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Setting')]
        procedure Instance() var Probe: Report \"RQ Report\"; begin
            Seed(); Param := Probe.RunRequestPage();
            if Param = '' then Error('no parameters');
            Check('set-by-handler', Probe.GetEchoText()); end;
        [RequestPageHandler] procedure Setting(var RP: TestRequestPage \"RQ Report\") begin
            RP.EchoText.SetValue('set-by-handler'); RP.OK().Invoke(); end;"
    ));
}

// handlers/TestRequestPageOnInit.al
#[test]
fn the_request_pages_init_and_open_triggers_run_before_the_handler() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Reading')]
        procedure Triggers() begin
            Seed(); Report.Run(Report::\"RQ Init\", true);
            Check('req-oninit|req-onopenpage|rpt-oninitreport', Seen); end;
        [RequestPageHandler] procedure Reading(var RP: TestRequestPage \"RQ Init\") begin
            Seen := RP.ReqInitField.Value() + '|' + RP.ReqOpenField.Value() + '|' + RP.RptInitField.Value(); RP.OK().Invoke(); end;"
    ));
}

// handlers/TestReportRequestPageAction.al (the three tests)
#[test]
fn invoking_a_request_page_action_is_refused_with_its_id() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Actions')]
        procedure InvokeAction() begin
            Seed(); Report.Run(Report::\"RQ Action\");
            Check('2', Format(Count('body-text:set-by-handler'))); Check('2', Format(Count('body-actions:0'))); end;
        [RequestPageHandler] procedure Actions(var RP: TestRequestPage \"RQ Action\") begin
            asserterror RP.StampText.Invoke();
            // 337124109 is the id the pinned algorithm derives for report 61212 and `StampText`.
            Check('The action with ID = 337124109 is not found on the page.', GetLastErrorText());
            RP.EchoText.SetValue('set-by-handler');
            RP.OK().Invoke(); end;"
    ));
}

// handlers/TestReportRequestPageSaveValues.al:SourceReport_SaveValues_NextRunShowsTheConfirmedValue,
// ..._NoSaveValues_NextRunShowsTheDefault, ..._RunRequestPageReturnsTheValueWithoutSavingIt
#[test]
fn save_values_reopens_on_the_confirmed_value() {
    passes(&format!(
        "{HELPERS}
        var SetOnThisRun: Boolean; SetValueTxt: Text;
        [Test] [HandlerFunctions('SavedHandler')]
        procedure Saved() begin
            Seed(); SetOnThisRun := true; SetValueTxt := 'remember-me';
            Report.RunModal(Report::\"RQ Saved\", true, false);
            Check('ran:remember-me,', Text());
            SetOnThisRun := false; Seen := '';
            Report.RunModal(Report::\"RQ Saved\", true, false);
            Check('remember-me', Seen); end;
        [Test] [HandlerFunctions('UnsavedHandler')]
        procedure Unsaved() begin
            Seed(); SetOnThisRun := true; SetValueTxt := 'forget-me';
            Report.RunModal(Report::\"RQ Unsaved\", true, false);
            SetOnThisRun := false; Seen := 'not read';
            Report.RunModal(Report::\"RQ Unsaved\", true, false);
            Check('', Seen); end;
        [Test] [HandlerFunctions('ParamsHandler')]
        procedure RunRequestPageDoesNotSave() begin
            Seed(); SetOnThisRun := true; SetValueTxt := 'params-only';
            Param := Report.RunRequestPage(Report::\"RQ Saved Params\");
            if StrPos(Param, 'params-only') = 0 then Error(Param);
            SetOnThisRun := false; Seen := 'not read';
            Report.RunModal(Report::\"RQ Saved Params\", true, false);
            Check('', Seen); end;
        local procedure Text(): Text var T: Codeunit \"RQ Trace\"; begin exit(T.Text()); end;
        [RequestPageHandler] procedure SavedHandler(var RP: TestRequestPage \"RQ Saved\") begin
            if SetOnThisRun then begin RP.Remembered.SetValue(SetValueTxt); RP.OK().Invoke(); end
            else begin Seen := RP.Remembered.Value(); RP.Cancel().Invoke(); end; end;
        [RequestPageHandler] procedure ParamsHandler(var RP: TestRequestPage \"RQ Saved Params\") begin
            if SetOnThisRun then begin RP.Remembered.SetValue(SetValueTxt); RP.OK().Invoke(); end
            else begin Seen := RP.Remembered.Value(); RP.Cancel().Invoke(); end; end;
        [RequestPageHandler] procedure UnsavedHandler(var RP: TestRequestPage \"RQ Unsaved\") begin
            if SetOnThisRun then begin RP.Remembered.SetValue(SetValueTxt); RP.OK().Invoke(); end
            else begin Seen := RP.Remembered.Value(); RP.Cancel().Invoke(); end; end;"
    ));
}

// handlers/TestReportRequestPageCaption.al
#[test]
fn the_reports_caption_wins_over_the_request_pages() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Both')]
        procedure BothCaptions() begin Report.RunModal(Report::\"RQ Both Captions\", true, false); Check('RQ Report Caption', Seen); end;
        [Test] [HandlerFunctions('RequestOnly')]
        procedure RequestCaptionOnly() begin Report.RunModal(Report::\"RQ Request Caption Only\", true, false); Check('RQ Only Request Page Caption', Seen); end;
        [RequestPageHandler] procedure Both(var RP: TestRequestPage \"RQ Both Captions\") begin Seen := RP.Caption(); RP.Cancel().Invoke(); end;
        [RequestPageHandler] procedure RequestOnly(var RP: TestRequestPage \"RQ Request Caption Only\") begin Seen := RP.Caption(); RP.Cancel().Invoke(); end;"
    ));
}

// handlers/TestReqPageBoundVisibility.al:OwnReport_*
#[test]
fn visible_and_editable_follow_the_reports_variables() {
    passes(&format!(
        "{HELPERS}
        var V: Boolean; E: Boolean;
        [Test] [HandlerFunctions('Bound')]
        procedure Hidden() var P: Report \"RQ Visible\"; begin P.Flags(false, true, true); P.Run(); if V then Error('visible'); end;
        [Test] [HandlerFunctions('Bound')]
        procedure ReadOnly() var P: Report \"RQ Visible\"; begin P.Flags(true, false, true); P.Run(); if not V then Error('hidden'); if E then Error('editable'); end;
        [Test] [HandlerFunctions('InGroup')]
        procedure GroupHidden() var P: Report \"RQ Visible\"; begin P.Flags(true, true, false); P.Run(); if V then Error('visible'); end;
        [Test] [HandlerFunctions('InGroup')]
        procedure GroupShown() var P: Report \"RQ Visible\"; begin P.Flags(true, true, true); P.Run(); if not V then Error('hidden'); end;
        [RequestPageHandler] procedure Bound(var RP: TestRequestPage \"RQ Visible\") begin V := RP.BoundField.Visible(); E := RP.BoundField.Editable(); RP.Cancel().Invoke(); end;
        [RequestPageHandler] procedure InGroup(var RP: TestRequestPage \"RQ Visible\") begin V := RP.InGroupField.Visible(); RP.Cancel().Invoke(); end;"
    ));
}

// testpageclose/TestPageClosingActions.al:RequestPage_OK_ClosesThePage_AndASecondActionCannotChangeTheResult
#[test]
fn a_request_page_closes_at_its_first_action() {
    passes(&format!(
        "{HELPERS}
        var ReadError: Text; SecondError: Text;
        [Test] [HandlerFunctions('OkThenMore')]
        procedure OkCloses() begin
            Seed(); Param := Report.RunRequestPage(Report::\"RQ Closing\");
            if StrPos(ReadError, 'The TestPage is not open.') = 0 then Error(ReadError);
            if StrPos(SecondError, 'The TestPage is not open.') = 0 then Error(SecondError);
            if StrLen(Param) = 0 then Error('no parameters');
            Check('open,qcp:OK,close,', Text()); end;
        [Test] [HandlerFunctions('CancelThenMore')]
        procedure CancelCloses() begin
            Seed(); Param := Report.RunRequestPage(Report::\"RQ Closing\");
            if StrPos(ReadError, 'The TestPage is not open.') = 0 then Error(ReadError);
            Check('', Param); Check('open,qcp:Cancel,close,', Text()); end;
        local procedure Text(): Text var T: Codeunit \"RQ Trace\"; begin exit(T.Text()); end;
        [RequestPageHandler] procedure OkThenMore(var R: TestRequestPage \"RQ Closing\") var T: Text; begin
            R.OK().Invoke();
            asserterror T := R.QtyOpt.Value(); ReadError := GetLastErrorText();
            asserterror R.Cancel().Invoke(); SecondError := GetLastErrorText(); end;
        [RequestPageHandler] procedure CancelThenMore(var R: TestRequestPage \"RQ Closing\") var T: Text; begin
            R.Cancel().Invoke();
            asserterror T := R.QtyOpt.Value(); ReadError := GetLastErrorText(); end;"
    ));
}

// A report with no requestpage section still opens a request page over its data items.
#[test]
fn a_report_without_a_requestpage_section_opens_a_page_of_data_item_filters() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Filtering')]
        procedure DefaultPage() begin
            Seed(); Report.Run(Report::\"RQ Default Page\"); Check('1', Format(Count('default-row'))); end;
        [RequestPageHandler] procedure Filtering(var RP: TestRequestPage \"RQ Default Page\") begin
            RP.Rows.SetFilter(\"Entry No.\", '1'); RP.OK().Invoke(); end;"
    ));
}

// UseRequestPage = false: the page never opens, and a run needs no handler
// (record/TestTxModelReportRun.al:60-61)
#[test]
fn a_report_that_does_not_use_its_request_page_runs_without_one() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure NoPage() begin Seed(); Report.Run(Report::\"RQ No Page\"); Check('2', Format(Count('nopage-row'))); end;"
    ));
}

// A run that opens its request page where the test declares no handler raises the Unhandled UI
// error (handlers/TestReportRuntimeContracts.al:416-430: text and code).
#[test]
fn a_request_page_with_no_handler_raises_unhandled_ui() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure NoHandler() begin Seed(); asserterror Report.Run(Report::\"RQ Report\");
            Check('Unhandled UI: RequestPage 61210', GetLastErrorText()); Check('MissingUIHandler', GetLastErrorCode()); end;"
    ));
}

// handlers/TestReportInstanceRunReqPage.al:TestReport_InstanceRunRequestPage_SeededWithParameters_StillRunsTheHandler
// (the corpus pins that the handler runs and the filter comes back; that the page opens on the
// seed is this runtime's reading, docs/probes-needed.md)
#[test]
fn run_request_page_takes_the_parameters_it_gave_out_as_a_seed() {
    passes(&format!(
        "{HELPERS}
        var Reads: Text;
        [Test] [HandlerFunctions('Reading')]
        procedure Seeded() var First, Second: Report \"RQ Report\"; Seed: Text; begin
            Seed(); Seed := First.RunRequestPage();
            Check('|', Reads);
            Reads := '';
            Param := Second.RunRequestPage(Seed);
            Check('set-by-handler|1', Reads);
            if StrPos(Param, 'WHERE(Field1=1') = 0 then Error(Param); end;
        [RequestPageHandler] procedure Reading(var RP: TestRequestPage \"RQ Report\") begin
            Reads += RP.EchoText.Value() + '|' + RP.Rows.GetFilter(\"Entry No.\");
            if Reads = '|' then begin RP.EchoText.SetValue('set-by-handler'); RP.Rows.SetFilter(\"Entry No.\", '1'); end;
            RP.OK().Invoke(); end;"
    ));
}

// A parameters string this runtime did not produce is refused (docs/probes-needed.md)
#[test]
fn a_foreign_seed_is_refused() {
    let why = refused(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Cancel')]
        procedure ForeignSeed() var Probe: Report \"RQ Report\"; begin
            Seed(); Param := Probe.RunRequestPage('<ReportParameters/>'); end;
        [RequestPageHandler] procedure Cancel(var RP: TestRequestPage \"RQ Report\") begin RP.Cancel().Invoke(); end;"
    ));
    assert!(why.contains("did not produce"), "{why}");
}

// handlers/TestRequestPageControlErrorName.al:SourceReportExtension_Control_AssertEquals_*,
// pageapplicationarea/TestReqPageApplicationAreaReportExtension.al:ExtField_SetValue_RunsItsOnValidate,
// handlers/TestPageExtensionModifyCaption_Tests.al:SameAppReport_ModifiedRequestPageCaptions;
// the parameters name a field by its variable, not by its control (report/TestProbeReportContracts.al)
#[test]
fn a_reportextension_adds_controls_and_modifies_captions_of_the_request_page() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('ExtHandler')]
        procedure ExtensionControls() var T: Codeunit \"RQ Trace\"; begin
            Seed();
            Param := Report.RunRequestPage(Report::\"RQ Ext Base\");
            Check('AssertEquals for Field: RpnExtCtl Expected = ''Wrong'', Actual = ''''', Seen);
            Check('base-open,ext-open,', T.Text());
            if StrPos(Param, '<Field name=\"RpnExtVar\">Eps</Field>') = 0 then Error(Param);
            if StrPos(Param, '<Field name=\"ExtDerivedValue\">D-X</Field>') = 0 then Error(Param); end;
        [RequestPageHandler] procedure ExtHandler(var RP: TestRequestPage \"RQ Ext Base\") begin
            asserterror RP.RpnExtCtl.AssertEquals('Wrong'); Seen := GetLastErrorText();
            RP.RpnExtCtl.SetValue('Eps'); RP.RpnExtCtl.AssertEquals('Eps');
            RP.ExtValidatedCtl.SetValue('X'); RP.ExtDerivedCtl.AssertEquals('D-X');
            if RP.OptCtl.Caption() <> 'Ext Request Option' then Error(RP.OptCtl.Caption());
            RP.OptCtl.SetValue('Larger');
            if RP.OptCtl.Value() <> 'Larger' then Error(RP.OptCtl.Value());
            RP.OK().Invoke(); end;"
    ));
}

// handlers/TestReportRunWithRequestPage.al:Report_Run_NonProcessingOnlyRequestPageHasNoOKAction:
// a report that is not ProcessingOnly offers no plain OK, and Cancel stops its run
#[test]
fn the_request_page_of_a_report_that_renders_has_no_ok_action() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('PlainOk')]
        procedure NoOk() var Probe: Report \"RQ Dataset\"; begin
            Seed(); Clear(Probe);
            asserterror Probe.Run();
            if not HandlerRan then Error('handler');
            if StrPos(GetLastErrorText(), 'built-in action = OK is not found') = 0 then Error(GetLastErrorText()); end;
        [Test] [HandlerFunctions('CancelDataset')]
        procedure CancelRuns() var Probe: Report \"RQ Dataset\"; begin
            Seed(); Probe.Run(); if not HandlerRan then Error('handler'); end;
        [RequestPageHandler] procedure PlainOk(var RP: TestRequestPage \"RQ Dataset\") begin HandlerRan := true; RP.OK().Invoke(); end;
        [RequestPageHandler] procedure CancelDataset(var RP: TestRequestPage \"RQ Dataset\") begin HandlerRan := true; RP.Cancel().Invoke(); end;"
    ));
}

// A run of a report that renders gets as far as its output and is refused there (docs/probes-needed.md)
#[test]
fn a_run_that_would_render_is_refused() {
    let why = refused(&format!(
        "{HELPERS}
        [Test]
        procedure Renders() begin Seed(); Report.Run(Report::\"RQ Dataset No Page\"); end;"
    ));
    assert!(why.contains("renders output"), "{why}");
}

const TX_HELPERS: &str = "var HandlerRan: Boolean;
local procedure Seed() var N: Record \"RQ Num\"; B: Record \"RQ Base\"; begin
    N.DeleteAll(); B.DeleteAll(); N.Number := 1; N.Insert(); Commit(); HandlerRan := false; end;
local procedure SeedNoCommit() begin HandlerRan := false; end;
local procedure InsertBase(EntryNo: Integer) var B: Record \"RQ Base\"; begin B.Init(); B.\"Entry No.\" := EntryNo; B.Insert(); end;
local procedure BaseExists(EntryNo: Integer): Boolean var B: Record \"RQ Base\"; begin exit(B.Get(EntryNo)); end;
local procedure Check(Expected: Boolean; Actual: Boolean; What: Text) begin if Expected <> Actual then Error('%1: expected %2', What, Expected); end;
[RequestPageHandler] procedure Confirm(var RP: TestRequestPage \"RQ Tx Marker\") begin HandlerRan := true; RP.OK().Invoke(); end;";

// record/TestTxModelReportRun.al:Test04, Test05, Test06, Test09, Test12 (with a table of numbers
// for the Integer table the corpus reports loop over)
#[test]
fn a_report_that_shows_its_request_page_follows_the_transaction_rules() {
    passes(&format!(
        "{TX_HELPERS}
        [Test]
        procedure PendingWriteIsRefused() var R: Report \"RQ Tx Marker\"; begin
            Seed(); InsertBase(11);
            R.SetMarker(12); asserterror R.Run();
            if StrPos(GetLastErrorText(), 'the transaction is stopped') = 0 then Error(GetLastErrorText());
            Check(false, BaseExists(12), 'refused report ran'); Check(false, BaseExists(11), 'pending write kept'); end;
        [Test] [HandlerFunctions('Confirm')]
        procedure CommitsItsOwnWrites() var R: Report \"RQ Tx Marker\"; begin
            Seed(); R.SetMarker(21); R.Run();
            Check(true, HandlerRan, 'handler'); Check(true, BaseExists(21), 'body');
            InsertBase(22); asserterror Error('probe');
            Check(true, BaseExists(21), 'committed'); Check(false, BaseExists(22), 'uncommitted'); end;
        [Test] [TransactionModel(TransactionModel::AutoRollback)] [HandlerFunctions('Confirm')]
        procedure AutoRollbackRunsWithAPendingWrite() var R: Report \"RQ Tx Marker\"; begin
            SeedNoCommit(); InsertBase(31); R.SetMarker(32); R.Run();
            Check(true, HandlerRan, 'handler'); Check(true, BaseExists(32), 'body'); Check(true, BaseExists(31), 'prior write'); end;
        [Test] [HandlerFunctions('Confirm')]
        procedure AFailingBodyCommitsNothing() var R: Report \"RQ Tx Marker\"; begin
            Seed(); R.SetMarker(41); R.FailAfterTheWrite();
            asserterror R.Run();
            if StrPos(GetLastErrorText(), 'rq report body error') = 0 then Error(GetLastErrorText());
            Check(true, HandlerRan, 'handler'); Check(false, BaseExists(41), 'failed write kept'); end;
        [Test]
        procedure RunRequestPageWithAPendingWriteIsRefused() var R: Report \"RQ Tx Marker\"; P: Text; begin
            Seed(); InsertBase(51);
            asserterror P := R.RunRequestPage();
            if StrPos(GetLastErrorText(), 'the transaction is stopped') = 0 then Error(GetLastErrorText());
            Check(false, BaseExists(51), 'pending write kept');
            Check(false, HandlerRan, 'the page opened'); end;"
    ));
}

// handlers/TestRequestPageControlErrorName.al:PrecompiledReportExtension_BooleanControl_AssertEquals_Mismatch_NamesTheControl,
// _TextControl_AssertEquals_Mismatch_..., _BooleanControl_SetValue_Refusal_..., _AssertEquals_Match_RaisesNothing
#[test]
fn a_reportextension_that_only_adds_controls_bound_to_protected_base_variables_opens() {
    passes(&format!(
        "{HELPERS}
        var Seen1: Text; Seen2: Text; Seen3: Text;
        [Test] [HandlerFunctions('ProtectedHandler')]
        procedure ProtectedVars() begin
            Report.RunRequestPage(Report::\"RQ Protected Base\");
            Check('AssertEquals for Field: Prod Flag Expected = ''Wrong'', Actual = ''No''', Seen1);
            Check('AssertEquals for Field: Prod No. Expected = ''Wrong'', Actual = ''''', Seen2);
            Check('Validation error for Field: Prod Flag,  Message = ''Your entry of ''Maybe'' is not an acceptable value for ''Adjust Production Orders''.''', Seen3); end;
        [RequestPageHandler] procedure ProtectedHandler(var RP: TestRequestPage \"RQ Protected Base\") begin
            asserterror RP.\"Prod Flag\".AssertEquals('Wrong'); Seen1 := GetLastErrorText();
            asserterror RP.\"Prod No.\".AssertEquals('Wrong'); Seen2 := GetLastErrorText();
            asserterror RP.\"Prod Flag\".SetValue('Maybe'); Seen3 := GetLastErrorText();
            RP.\"Prod Flag\".AssertEquals('No'); RP.\"Prod No.\".AssertEquals('');
            RP.Cancel().Invoke(); end;"
    ));
}

// A request page with ShowFilter = false opens and its controls work; touching a data item of it is refused (not pinned).
#[test]
fn a_request_page_with_show_filter_false_opens_but_its_data_items_are_refused() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('ControlOnly')]
        procedure ControlsWork() begin
            Seed(); Report.Run(Report::\"RQ No Filters\"); if not HandlerRan then Error('handler'); end;
        [RequestPageHandler] procedure ControlOnly(var RP: TestRequestPage \"RQ No Filters\") begin
            HandlerRan := true; RP.OptCtl.SetValue('x'); RP.OptCtl.AssertEquals('x'); RP.Cancel().Invoke(); end;"
    ));
    let why = refused(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('FilterItem')]
        procedure Filters() begin Seed(); Report.Run(Report::\"RQ No Filters\"); end;
        [RequestPageHandler] procedure FilterItem(var RP: TestRequestPage \"RQ No Filters\") begin
            RP.Rows.SetFilter(\"Entry No.\", '1'); RP.Cancel().Invoke(); end;"
    ));
    assert!(why.contains("ShowFilter"), "{why}");
}

// al-language-onprem/handlers/TestRequestPageSaveAsXmlContracts.al:SaveAsXml_ParametersFile_RawForm and
// :TemporaryPath_EndsWithTheTempFolderAndABackslash: CR LF line ends, no line end after the last element, the parameter names
// followed by their fixed GUIDs, an execution time with seven fraction digits and +00:00.
#[test]
fn save_as_xml_writes_the_parameters_file_in_its_raw_form() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('SaveHandler')]
        procedure Params() var Probe: Report \"RQ Report\"; F: File; Ins: InStream; B: Byte; One: Text[1]; Raw: Text; At: Integer; Cr: Text[1];
        begin
            Seed();
            if not TemporaryPath().EndsWith('TEMP\\') then Error('TemporaryPath %1', TemporaryPath());
            Probe.Run();
            F.Open(TemporaryPath() + 'p.xml'); F.CreateInStream(Ins);
            while not Ins.EOS() do begin Ins.Read(B); One[1] := B; Raw += One; end;
            F.Close();
            Cr[1] := 13;
            if not Raw.Contains(Cr) then Error('no CR');
            if not Raw.EndsWith('</ArrayOfReportParameter>') then Error('end %1', CopyStr(Raw, StrLen(Raw) - 30));
            if not Raw.Contains('    <Name>NavUserIdAE6AEB42076449B78A3E78A5EE366751</Name>') then Error('user');
            if not Raw.Contains('    <Name>ExecutionTime7587CE19257C4965B7E6066BDFF21EE3</Name>') then Error('time');
            At := Raw.IndexOf('<string>2');
            Raw := Raw.Substring(At + 8, 33);
            Check('T', CopyStr(Raw, 11, 1)); Check('.', CopyStr(Raw, 20, 1)); Check('+00:00', CopyStr(Raw, 28, 6));
        end;
        [RequestPageHandler] procedure SaveHandler(var RP: TestRequestPage \"RQ Report\") begin RP.SaveAsXml(TemporaryPath() + 'p.xml', TemporaryPath() + 'd.xml'); end;"
    ));
}

/// Lane T8: a request page with two actions out of reach, a seeded control and a caption set in OnOpenPage
/// (report/lt-aq/AqFixtures.al:172-205, :550-610).
const LANE_T8: &str = r#"
table 61404 "T8 Row" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
codeunit 61400 "T8 Log"
{
    SingleInstance = true;
    var Lines: Text; CurrentMode: Integer;
    procedure Add(Line: Text) begin Lines += Line + ';'; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
    procedure SetMode(M: Integer) begin CurrentMode := M; end;
    procedure Mode(): Integer begin exit(CurrentMode); end;
}
report 61401 "T8 Caption"
{
    Caption = 'T8 static caption';
    ProcessingOnly = true;
    dataset { dataitem(Row; "T8 Row") { } }
    requestpage
    {
        Caption = 'T8 request page caption';
        layout { area(Content) { field(Ctl; Ctl) { ApplicationArea = All; } } }
        trigger OnOpenPage() begin RequestOptionsPage.Caption := 'T8 caption set in OnOpenPage'; end;
    }
    var Ctl: Integer;
}
report 61402 "T8 Req Page"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "T8 Row") { trigger OnPreDataItem() begin CurrReport.Break(); end; } }
    requestpage
    {
        layout { area(Content) { field(IntCtl; IntVal) { ApplicationArea = All; } field(OptCtl; OptVal) { ApplicationArea = All; } } }
        actions
        {
            area(Processing)
            {
                action(DisAct) { ApplicationArea = All; Enabled = false; trigger OnAction() begin Log.Add('DisAct ran'); end; }
                action(InvAct) { ApplicationArea = All; Visible = false; trigger OnAction() begin Log.Add('InvAct ran'); end; }
            }
        }
        trigger OnInit()
        begin
            if Log.Mode() = 1 then
                IntVal := 5;
            Log.Add('OnInit int=' + Format(IntVal) + ' opt=' + Format(OptVal));
        end;
        trigger OnOpenPage() begin Log.Add('OnOpenPage int=' + Format(IntVal) + ' opt=' + Format(OptVal)); end;
    }
    var Log: Codeunit "T8 Log"; IntVal: Integer; OptVal: Option Alpha,Beta,Gamma;
}
codeunit 61403 T
{
    Subtype = Test;
    var Log: Codeunit "T8 Log"; Seen: Text; ReadPhase: Boolean;
    local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected [%1] got [%2]', Expected, Actual); end;

    [Test] [HandlerFunctions('CaptionHandler')]
    procedure CaptionSetInOnOpenPage() begin Seen := ''; Report.Run(Report::"T8 Caption"); Check('T8 caption set in OnOpenPage', Seen); end;
    [RequestPageHandler] procedure CaptionHandler(var RP: TestRequestPage "T8 Caption") begin Seen := RP.Caption(); RP.Cancel().Invoke(); end;

    [Test] [HandlerFunctions('ActionsHandler')]
    procedure ActionsOutOfReach()
    var Params: Text;
    begin
        Log.SetMode(0); Log.Take(); Seen := '';
        Params := Report.RunRequestPage(Report::"T8 Req Page");
        Check('disabled: The action with ID = 1719343332 is not found on the page. code=TestActionNotFound;hidden: The action with ID = 1370392163 is not found on the page. code=TestActionNotFound;', Seen);
        Check('OnInit int=0 opt=Alpha;OnOpenPage int=0 opt=Alpha;', Log.Take());
    end;
    [RequestPageHandler] procedure ActionsHandler(var RP: TestRequestPage "T8 Req Page")
    begin
        asserterror RP.DisAct.Invoke();
        Seen += 'disabled: ' + GetLastErrorText() + ' code=' + GetLastErrorCode() + ';';
        asserterror RP.InvAct.Invoke();
        Seen += 'hidden: ' + GetLastErrorText() + ' code=' + GetLastErrorCode() + ';';
        RP.Cancel().Invoke();
    end;

    [Test] [HandlerFunctions('ValuesHandler')]
    procedure SeedAppliesAfterOnInit()
    var Params: Text;
    begin
        Log.SetMode(1); Log.Take();
        Params := Report.RunRequestPage(Report::"T8 Req Page");
        Log.Take(); Seen := ''; ReadPhase := true;
        Params := Report.RunRequestPage(Report::"T8 Req Page", Params);
        ReadPhase := false; Log.SetMode(0);
        Check('int=9 opt=Gamma', Seen);
        Check('OnInit int=5 opt=Alpha;OnOpenPage int=9 opt=Gamma;', Log.Take());
    end;
    [RequestPageHandler] procedure ValuesHandler(var RP: TestRequestPage "T8 Req Page")
    begin
        if ReadPhase then
            Seen := 'int=' + RP.IntCtl.Value() + ' opt=' + RP.OptCtl.Value()
        else begin
            RP.IntCtl.SetValue(9);
            RP.OptCtl.SetValue('Gamma');
        end;
        RP.OK().Invoke();
    end;
}
"#;

// report/lt-aq/AqTests.al:313-330 (caption set in OnOpenPage), :334-358 (actions out of reach), :362-381 (seed after OnInit).
#[test]
fn lane_t8_request_page_caption_actions_and_seed() {
    let o = al_testkit::outcomes_in(LANE_T8, "T");
    assert_eq!(o.len(), 3, "{o:?}");
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}: {r:?}");
    }
}

// Review of lane T8: a parameters string that sets a data item view, on a report whose request page has OnInit (or
// whose report has OnInitReport). The view now loads after OnInit (report/lt-aq/AqTests.al:362-381 pins only a control
// value); whether a view set in OnInit is replaced by the seed is not pinned: refused.
#[test]
fn a_seeded_data_item_view_on_a_request_page_with_on_init_is_refused() {
    let why = refused(&format!(
        "{HELPERS}
        var Reads: Integer;
        [Test] [HandlerFunctions('Filtering')]
        procedure SeededView() var First, Second: Report \"RQ Init\"; Seed: Text; begin
            Seed(); Seed := First.RunRequestPage();
            if StrPos(Seed, 'WHERE(Field1=1') = 0 then Error(Seed);
            Param := Second.RunRequestPage(Seed); end;
        [RequestPageHandler] procedure Filtering(var RP: TestRequestPage \"RQ Init\") begin
            Reads += 1;
            if Reads = 1 then RP.Rows.SetFilter(\"Entry No.\", '1');
            RP.OK().Invoke(); end;"
    ));
    assert!(why.contains("OnInit"), "{why}");
}

/// al-runner issue 5691: a request page handler whose SetValue raises and does not catch it.
const RI_OBJECTS: &str = r#"
table 61250 "RI Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 61251 "RI Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Add(T: Text) begin Trail += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Trail; Trail := ''; exit(R); end;
}
report 61252 "RI Refused"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RI Row") { trigger OnAfterGetRecord() begin L.Add('row ' + Format(Row.Id)); end; } }
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(ApplyCtl; ApplyVal)
                {
                    trigger OnValidate()
                    begin
                        L.Add('validate apply=' + Format(ApplyVal));
                        if ApplyVal then Error('LT RI apply rejected');
                    end;
                }
                field(IntCtl; IntVal) { }
            }
        }
        trigger OnOpenPage() begin L.Add('RP OnOpenPage'); end;
        trigger OnQueryClosePage(CloseAction: Action): Boolean
        begin
            L.Add('RP OnQueryClosePage action=' + Format(CloseAction) + ' apply=' + Format(ApplyVal) + ' int=' + Format(IntVal));
            exit(true);
        end;
        trigger OnClosePage() begin L.Add('RP OnClosePage'); end;
    }
    trigger OnPreReport() begin L.Add('OnPreReport apply=' + Format(ApplyVal) + ' int=' + Format(IntVal)); end;
    var ApplyVal: Boolean; IntVal: Integer; L: Codeunit "RI Log";
}
"#;

fn ri_outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{RI_OBJECTS}\ncodeunit 61260 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

const RI_HELPERS: &str = "var L: Codeunit \"RI Log\"; Mode: Text;
local procedure Seed() var R: Record \"RI Row\"; begin R.DeleteAll(); R.Id := 1; R.Insert(); R.Id := 2; R.Insert(); Commit(); L.Take(); end;
local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
[RequestPageHandler] procedure RiHandler(var RP: TestRequestPage \"RI Refused\") begin
    L.Add('h ' + Mode);
    case Mode of
        'apply': RP.ApplyCtl.SetValue(true);
        'int': RP.IntCtl.SetValue('abc');
        'caught': begin asserterror RP.ApplyCtl.SetValue(true); Error('other'); end;
    end;
    if Mode <> 'ok' then L.Add('h after SetValue');
    if Mode = 'ok' then RP.OK().Invoke() else RP.Cancel().Invoke();
end;";

// report/lt-report-requestpage/RptRequestPage.al:1243 RI_HandlerSetValue_OnValidateError_ReachesRun and :1264
// RI_HandlerSetValue_NotEvaluable_ReachesRun: the error leaves Report.Run as the TestValidation error, the page closes with
// Cancel (the refused value is not kept), OnPreReport does not run, and the next run is a normal one.
#[test]
fn a_set_value_error_the_request_page_handler_does_not_catch_closes_the_page_with_cancel() {
    let o = ri_outcomes(&format!(
        "{RI_HELPERS}
        [Test] [HandlerFunctions('RiHandler')]
        procedure OnValidateError() begin
            Seed(); Mode := 'apply';
            asserterror Report.Run(Report::\"RI Refused\");
            Check('Validation error for Field: ApplyCtl,  Message = ''LT RI apply rejected''', GetLastErrorText());
            Check('TestValidation', GetLastErrorCode());
            Check('RP OnOpenPage;h apply;validate apply=Yes;RP OnQueryClosePage action=Cancel apply=No int=0;RP OnClosePage;', L.Take());
            Mode := 'ok'; Report.Run(Report::\"RI Refused\");
            Check('RP OnOpenPage;h ok;RP OnQueryClosePage action=OK apply=No int=0;RP OnClosePage;OnPreReport apply=No int=0;row 1;row 2;', L.Take());
        end;
        [Test] [HandlerFunctions('RiHandler')]
        procedure NotEvaluable() begin
            Seed(); Mode := 'int';
            asserterror Report.Run(Report::\"RI Refused\");
            Check('Validation error for Field: IntCtl,  Message = ''Your entry of ''abc'' is not an acceptable value for ''IntCtl''. The value \"abc\" can''t be evaluated into type Integer.''', GetLastErrorText());
            Check('TestValidation', GetLastErrorCode());
            Check('RP OnOpenPage;h int;RP OnQueryClosePage action=Cancel apply=No int=0;RP OnClosePage;', L.Take());
        end;"
    ));
    assert_eq!(o.len(), 2);
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// Not pinned: a handler that caught the SetValue error and then raised another one leaves a refused entry on the page while
// a different error leaves the handler.
#[test]
fn a_handler_error_other_than_the_refused_entry_is_still_refused() {
    let o = ri_outcomes(&format!(
        "{RI_HELPERS}
        [Test] [HandlerFunctions('RiHandler')]
        procedure Caught() begin Seed(); Mode := 'caught'; asserterror Report.Run(Report::\"RI Refused\"); end;"
    ));
    match &o[..] {
        [(_, Outcome::NotRun(why))] => assert!(why.contains("refused or unsaved entry"), "{why}"),
        other => panic!("expected one refusal, got {other:?}"),
    }
}
