//! `ExcelLayoutMultipleDataSheets = true` on a report (al-runner issue 7836): the calls the corpus probe measured run as the
//! same report without the property (assumption excel-layout-multiple-data-sheets, ExcelMultipleDataSheetsReport.al, corpus
//! pull request 1368), also with a SubstituteReport subscriber that leaves the report as it is; every other use of such a
//! report refuses. A report without the property keeps its answer. Rust tests until the probe is pinned.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::Built;

const DATASET: &str = r#"
    dataset
    {
        dataitem(Num; "XMS Num") { DataItemTableView = where(Number = filter(1 .. 2)); column(Number; Number) { } column(Doubled; Number * 2) { } }
        dataitem(Second; "XMS Num") { DataItemTableView = where(Number = const(7)); column(SecondNumber; Number) { } }
    }
"#;

fn src() -> String {
    format!(
        r#"
report 50960 "XMS Plain" {{ UsageCategory = None; UseRequestPage = false; {DATASET} }}
report 50961 "XMS Multi" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
report 50962 "XMS Multi Word"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultRenderingLayout = W;
    {DATASET}
    rendering {{ layout(W) {{ Type = Word; LayoutFile = 'XmsWord.docx'; }} }}
}}
report 50963 "XMS Multi Excel"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultRenderingLayout = X;
    {DATASET}
    rendering {{ layout(X) {{ Type = Excel; LayoutFile = 'XmsExcel.xlsx'; }} }}
}}
report 50967 "XMS Proc" {{ UsageCategory = None; UseRequestPage = false; ProcessingOnly = true; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
report 50968 "XMS Two Layouts"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultRenderingLayout = X;
    {DATASET}
    rendering {{ layout(X) {{ Type = Excel; LayoutFile = 'XmsExcel.xlsx'; }} layout(W) {{ Type = Word; LayoutFile = 'XmsWord.docx'; }} }}
}}
report 50969 "XMS Legacy"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultLayout = Word; WordLayout = 'XmsWord.docx';
    {DATASET}
}}
report 50970 "XMS Extended" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
reportextension 50971 "XMS Ext" extends "XMS Extended"
{{
    rendering {{ layout(E) {{ Type = Excel; LayoutFile = 'XmsExt.xlsx'; }} }}
}}
report 50972 "XMS Page Shown" {{ UsageCategory = None; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
report 50973 "XMS Trigger"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    {DATASET}
    trigger OnPreReport() begin end;
}}
report 50974 "XMS Request Ext" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
reportextension 50975 "XMS Req Ext" extends "XMS Request Ext"
{{
    requestpage {{ }}
}}
report 50979 "XMS Layout Caption"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultRenderingLayout = X;
    {DATASET}
    rendering {{ layout(X) {{ Type = Excel; LayoutFile = 'XmsExcel.xlsx'; Caption = 'Sheets'; }} }}
}}
report 50980 "XMS Both Defaults"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultLayout = Word; DefaultRenderingLayout = X;
    {DATASET}
    rendering {{ layout(X) {{ Type = Excel; LayoutFile = 'XmsExcel.xlsx'; }} }}
}}
report 50981 "XMS Caption" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; Caption = 'Cap'; {DATASET} }}
report 50982 "XMS Transaction" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; TransactionType = Update; {DATASET} }}
report 50983 "XMS Labels" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; {DATASET} labels {{ Lbl = 'Label'; }} }}
report 50984 "XMS Global" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; {DATASET} var G: Integer; }}
report 50985 "XMS Filter Fields"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ RequestFilterFields = Number; column(Number; Number) {{ }} }} }}
}}
report 50986 "XMS Nested"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ column(Number; Number) {{ }} dataitem(Inner; "XMS Num") {{ DataItemLink = Number = field(Number); column(InnerNumber; Number) {{ }} }} }} }}
}}
report 50987 "XMS No Items" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; dataset {{ }} }}
report 50988 "XMS Call Column"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ column(Number; Number) {{ }} column(Text; Format(Number)) {{ }} }} }}
}}
report 50994 "XMS Init"
{{
    UsageCategory = None; UseRequestPage = false;
    dataset {{ dataitem(Num; "XMS Num") {{ DataItemTableView = where(Number = const(7)); column(Number; Number) {{ }} }} }}
    trigger OnInitReport()
    var Tmp: Record "XMS Blob" temporary; Out: OutStream; Saved: Boolean;
    begin
        Tmp.Data.CreateOutStream(Out);
        Saved := Report.SaveAs(Report::"XMS Multi", '', ReportFormat::Xml, Out);
    end;
    [TryFunction]
    procedure T() begin end;
}}
report 50995 "XMS No View"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ column(Number; Number) {{ }} column(Doubled; Number * 2) {{ }} }} }}
}}
report 50996 "XMS Use Temporary"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ UseTemporary = true; column(Number; Number) {{ }} }} }}
}}
report 50997 "XMS New Page"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ DataItemTableView = where(Number = const(7)); NewPagePerRecord = true; column(Number; Number) {{ }} }} }}
}}
report 50998 "XMS Caption False"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ column(Number; Number) {{ IncludeCaption = false; }} }} }}
}}
report 50999 "XMS Column Obsolete"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    dataset {{ dataitem(Num; "XMS Num") {{ column(Number; Number) {{ ObsoleteState = Pending; ObsoleteReason = 'x'; }} }} }}
}}
report 50989 "XMS With Procedure"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true;
    {DATASET}
    procedure Foo(): Integer begin exit(41); end;
}}
table 50966 "XMS Num" {{ fields {{ field(1; Number; Integer) {{ }} }} keys {{ key(PK; Number) {{ }} }} }}
table 50964 "XMS Blob" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}

codeunit 50965 "XMS Tests"
{{
    local procedure Setup()
    var N: Record "XMS Num";
    begin
        if not N.IsEmpty() then exit;
        N.Number := 1; N.Insert(); N.Number := 2; N.Insert(); N.Number := 7; N.Insert();
        Commit();
    end;

    [TryFunction]
    local procedure TrySave(Id: Integer; Fmt: ReportFormat; var Tmp: Record "XMS Blob" temporary; var Saved: Boolean)
    var Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out);
        Saved := Report.SaveAs(Id, '', Fmt, Out);
    end;

    local procedure Dataset(Id: Integer; Name: Text): Text
    var Tmp: Record "XMS Blob" temporary; InS: InStream; Saved: Boolean; Xml: Text;
    begin
        Setup();
        if not TrySave(Id, ReportFormat::Xml, Tmp, Saved) then
            exit('<failed: ' + GetLastErrorText() + '>');
        Tmp.Data.CreateInStream(InS);
        InS.Read(Xml);
        exit(Format(Saved) + '|' + Xml.Replace(Name, '<name>').Replace(Format(Id, 0, 9), '<id>'));
    end;

    procedure SameDataset(): Text
    var Plain: Text;
    begin
        Plain := Dataset(Report::"XMS Plain", 'XMS Plain');
        exit(CopyStr(Plain, 1, 3) + '|' + Format(Plain.Contains('SecondNumber')) + '|'
            + Format(Dataset(Report::"XMS Multi", 'XMS Multi') = Plain) + '|'
            + Format(Dataset(Report::"XMS Multi Word", 'XMS Multi Word') = Plain) + '|'
            + Format(Dataset(Report::"XMS Multi Excel", 'XMS Multi Excel') = Plain));
    end;

    [TryFunction]
    local procedure TryRun(Id: Integer; Modal: Boolean)
    begin
        if Modal then Report.RunModal(Id, false) else Report.Run(Id, false);
    end;

    local procedure RunFacts(Id: Integer; Modal: Boolean): Text
    begin
        Setup();
        ClearLastError();
        if TryRun(Id, Modal) then
            exit('Yes');
        exit('No|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure RunNoLayout(): Text begin exit(RunFacts(Report::"XMS Multi", false)); end;
    procedure RunModalNoLayout(): Text begin exit(RunFacts(Report::"XMS Multi", true)); end;
    procedure RunExcel(): Text begin exit(RunFacts(Report::"XMS Multi Excel", false)); end;
    procedure RunModalExcel(): Text begin exit(RunFacts(Report::"XMS Multi Excel", true)); end;
    procedure RunWord(): Text begin exit(RunFacts(Report::"XMS Multi Word", false)); end;
    procedure RunPlain(): Text begin exit(RunFacts(Report::"XMS Plain", false)); end;

    local procedure ExcelFacts(Id: Integer): Text
    var Tmp: Record "XMS Blob" temporary; Ok: Boolean; Saved: Boolean;
    begin
        Setup();
        Ok := TrySave(Id, ReportFormat::Excel, Tmp, Saved);
        exit(Format(Ok) + '|' + Format(Saved) + '|' + Format(Tmp.Data.Length() > 0));
    end;

    procedure SaveExcelLayout(): Text begin exit(ExcelFacts(Report::"XMS Multi Excel")); end;
    procedure SaveExcelNoLayout(): Text begin exit(ExcelFacts(Report::"XMS Multi")); end;
    // Every other call of a report with the property refuses (main's answer for the report), one per entry.
    procedure VarRunWord() var R: Report "XMS Multi Word"; begin Setup(); R.UseRequestPage(false); R.Run(); end;
    procedure VarRunNoLayout() var R: Report "XMS Multi"; begin Setup(); R.Run(); end;
    procedure VarRunModalExcel() var R: Report "XMS Multi Excel"; begin Setup(); R.RunModal(); end;
    procedure StaticRunWithPage() begin Setup(); Report.Run(Report::"XMS Multi Excel"); end;
    procedure StaticRunRequestPage(): Text begin Setup(); exit(Report.RunRequestPage(Report::"XMS Multi Excel")); end;
    procedure VarRunRequestPage(): Text var R: Report "XMS Multi"; begin Setup(); exit(R.RunRequestPage()); end;
    procedure StaticExecute() begin Setup(); Report.Execute(Report::"XMS Multi", ''); end;
    procedure StaticPrint() begin Setup(); Report.Print(Report::"XMS Multi", ''); end;
    procedure RunProcessingOnly() begin Setup(); Report.Run(Report::"XMS Proc", false); end;
    procedure RunPageShown(): Text begin exit(RunFacts(Report::"XMS Page Shown", false)); end;
    procedure RunTrigger(): Text begin exit(RunFacts(Report::"XMS Trigger", false)); end;
    local procedure SaveFacts(Id: Integer; Fmt: ReportFormat): Text
    var Tmp: Record "XMS Blob" temporary; Saved: Boolean;
    begin
        Setup();
        if TrySave(Id, Fmt, Tmp, Saved) then;
        exit(Format(Saved));
    end;
    procedure SavePlainXml(): Text begin exit(SaveFacts(Report::"XMS Plain", ReportFormat::Xml)); end;
    procedure SavePdfNoLayout(): Text begin exit(SaveFacts(Report::"XMS Multi", ReportFormat::Pdf)); end;
    procedure SaveExcelOfWord(): Text begin exit(SaveFacts(Report::"XMS Multi Word", ReportFormat::Excel)); end;
    procedure SaveXmlProcessingOnly(): Text begin exit(SaveFacts(Report::"XMS Proc", ReportFormat::Xml)); end;
    procedure SaveXmlTwoLayouts(): Text begin exit(SaveFacts(Report::"XMS Two Layouts", ReportFormat::Xml)); end;
    procedure SaveXmlLegacyLayout(): Text begin exit(SaveFacts(Report::"XMS Legacy", ReportFormat::Xml)); end;
    procedure SaveXmlExtended(): Text begin exit(SaveFacts(Report::"XMS Extended", ReportFormat::Xml)); end;
    procedure SaveXmlPageShown(): Text begin exit(SaveFacts(Report::"XMS Page Shown", ReportFormat::Xml)); end;
    procedure SaveXmlTrigger(): Text begin exit(SaveFacts(Report::"XMS Trigger", ReportFormat::Xml)); end;
    procedure SaveXmlRequestExt(): Text begin exit(SaveFacts(Report::"XMS Request Ext", ReportFormat::Xml)); end;
    procedure SaveXmlOfVariable(): Text
    var R: Report "XMS Multi"; Tmp: Record "XMS Blob" temporary; Out: OutStream;
    begin
        Setup();
        Tmp.Data.CreateOutStream(Out);
        exit(Format(R.SaveAs('', ReportFormat::Xml, Out)));
    end;
    procedure SaveXmlWithRecordRef(): Text
    var Tmp: Record "XMS Blob" temporary; Out: OutStream; RR: RecordRef;
    begin
        Setup();
        RR.Open(Database::"XMS Num");
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"XMS Multi", '', ReportFormat::Xml, Out, RR)));
    end;
    procedure SaveXmlWithParameters(): Text
    var Tmp: Record "XMS Blob" temporary; Out: OutStream;
    begin
        Setup();
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"XMS Multi", '<?xml version="1.0" standalone="yes"?><ReportParameters/>', ReportFormat::Xml, Out)));
    end;
    procedure SaveXmlLayoutCaption(): Text begin exit(SaveFacts(Report::"XMS Layout Caption", ReportFormat::Xml)); end;
    procedure SaveExcelLayoutCaption(): Text begin exit(SaveFacts(Report::"XMS Layout Caption", ReportFormat::Excel)); end;
    procedure RunLayoutCaption(): Text begin exit(RunFacts(Report::"XMS Layout Caption", false)); end;
    procedure SaveXmlBothDefaults(): Text begin exit(SaveFacts(Report::"XMS Both Defaults", ReportFormat::Xml)); end;
    procedure SaveXmlCaption(): Text begin exit(SaveFacts(Report::"XMS Caption", ReportFormat::Xml)); end;
    procedure RunTransaction(): Text begin exit(RunFacts(Report::"XMS Transaction", false)); end;
    procedure SaveXmlLabels(): Text begin exit(SaveFacts(Report::"XMS Labels", ReportFormat::Xml)); end;
    procedure SaveXmlGlobal(): Text begin exit(SaveFacts(Report::"XMS Global", ReportFormat::Xml)); end;
    procedure SaveXmlFilterFields(): Text begin exit(SaveFacts(Report::"XMS Filter Fields", ReportFormat::Xml)); end;
    procedure SaveXmlNested(): Text begin exit(SaveFacts(Report::"XMS Nested", ReportFormat::Xml)); end;
    procedure SaveXmlNoItems(): Text begin exit(SaveFacts(Report::"XMS No Items", ReportFormat::Xml)); end;
    procedure SaveXmlCallColumn(): Text begin exit(SaveFacts(Report::"XMS Call Column", ReportFormat::Xml)); end;
    procedure SaveXmlNoView(): Text begin exit(SaveFacts(Report::"XMS No View", ReportFormat::Xml)); end;
    procedure RunNoView(): Text begin exit(RunFacts(Report::"XMS No View", false)); end;
    // The measured calls outside the probe's context: a TryFunction whose result is used, the call in its own body.
    procedure SaveXmlBare(): Text
    var Tmp: Record "XMS Blob" temporary; Out: OutStream;
    begin
        Setup();
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"XMS Multi", '', ReportFormat::Xml, Out)));
    end;
    procedure RunBare(): Text begin Setup(); Report.Run(Report::"XMS Multi", false); exit('Yes'); end;
    procedure RunAssertError(): Text begin Setup(); asserterror Report.Run(Report::"XMS Multi", false); exit(GetLastErrorCode()); end;
    [TryFunction]
    local procedure TryAssert(Id: Integer) begin asserterror Report.Run(Id, false); end;
    procedure RunAssertErrorInTry(): Text begin Setup(); exit(Format(TryAssert(Report::"XMS Multi"))); end;
    procedure SaveXmlInInitOfTry(): Text var R: Report "XMS Init"; begin Setup(); exit(Format(R.T())); end;
    procedure SaveXmlTryResultDropped(): Text
    var Tmp: Record "XMS Blob" temporary; Saved: Boolean;
    begin
        Setup();
        TrySave(Report::"XMS Multi", ReportFormat::Xml, Tmp, Saved);
        exit(Format(Saved));
    end;
    [TryFunction]
    local procedure TrySaveThroughHelper(Id: Integer; var Tmp: Record "XMS Blob" temporary; var Saved: Boolean)
    begin
        SaveHelper(Id, Tmp, Saved);
    end;
    local procedure SaveHelper(Id: Integer; var Tmp: Record "XMS Blob" temporary; var Saved: Boolean)
    var Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out);
        Saved := Report.SaveAs(Id, '', ReportFormat::Xml, Out);
    end;
    procedure SaveXmlThroughHelper(): Text
    var Tmp: Record "XMS Blob" temporary; Saved: Boolean;
    begin
        Setup();
        if TrySaveThroughHelper(Report::"XMS Multi", Tmp, Saved) then;
        exit(Format(Saved));
    end;
    procedure SaveXmlUseTemporary(): Text begin exit(SaveFacts(Report::"XMS Use Temporary", ReportFormat::Xml)); end;
    procedure RunUseTemporary(): Text begin exit(RunFacts(Report::"XMS Use Temporary", false)); end;
    procedure SaveXmlNewPage(): Text begin exit(SaveFacts(Report::"XMS New Page", ReportFormat::Xml)); end;
    procedure SaveXmlCaptionFalse(): Text begin exit(SaveFacts(Report::"XMS Caption False", ReportFormat::Xml)); end;
    procedure SaveXmlColumnObsolete(): Text begin exit(SaveFacts(Report::"XMS Column Obsolete", ReportFormat::Xml)); end;
    [TryFunction]
    local procedure TrySaveDropped(Id: Integer; Fmt: ReportFormat; var Tmp: Record "XMS Blob" temporary)
    var Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Id, '', Fmt, Out);
    end;
    procedure SaveXmlAnswerDropped(): Text
    var Tmp: Record "XMS Blob" temporary;
    begin
        Setup();
        exit(Format(TrySaveDropped(Report::"XMS Multi", ReportFormat::Xml, Tmp)));
    end;
    procedure SaveExcelAnswerDropped(): Text
    var Tmp: Record "XMS Blob" temporary;
    begin
        Setup();
        exit(Format(TrySaveDropped(Report::"XMS Multi", ReportFormat::Excel, Tmp)));
    end;
    procedure SaveXmlOptionFormat(): Text
    var Tmp: Record "XMS Blob" temporary; Out: OutStream; F: Option Xml,Pdf;
    begin
        Setup();
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"XMS Multi", '', F, Out)));
    end;
    procedure MultiDataset(): Text begin exit(Dataset(Report::"XMS Multi", 'XMS Multi')); end;
    procedure PlainDataset(): Text begin exit(Dataset(Report::"XMS Plain", 'XMS Plain')); end;
    procedure SavePdfOfWord(): Text
    var Tmp: Record "XMS Blob" temporary; Saved: Boolean;
    begin
        Setup();
        TrySave(Report::"XMS Multi Word", ReportFormat::Pdf, Tmp, Saved);
        exit(Format(Saved));
    end;
}}
codeunit 50990 "XMS Variable Uses"
{{
    procedure Declared(): Text var R: Report "XMS Multi"; begin exit('x'); end;
    procedure Language(): Text var R: Report "XMS Multi"; begin exit(Format(R.Language())); end;
    procedure DefaultLayout(): Text var R: Report "XMS Multi Excel"; begin exit(Format(R.DefaultLayout())); end;
    procedure ExcelLayout(): Text var R: Report "XMS Multi Excel"; InS: InStream; begin exit(Format(R.ExcelLayout(InS))); end;
    procedure FormatRegion(): Text var R: Report "XMS Multi"; begin exit(R.FormatRegion()); end;
    procedure Procedure(): Text var R: Report "XMS With Procedure"; begin exit(Format(R.Foo())); end;
}}
codeunit 50991 "XMS Handler Tests"
{{
    Subtype = Test;
    [Test]
    [HandlerFunctions('ExcelHandler')]
    procedure RunWithReportHandler()
    var T: Codeunit "XMS Tests";
    begin
        Error(T.RunExcel());
    end;
    [ReportHandler]
    procedure ExcelHandler(var R: Report "XMS Multi Excel")
    begin
    end;
}}
"#
    )
}

/// A SubstituteReport subscriber whose body is `body` (`ReportId`, `RunMode` and `var NewReportId` in scope), and a
/// SingleInstance log of the calls it saw.
fn subscriber(body: &str) -> String {
    subscriber_with("", body)
}

/// [`subscriber`] with the local variables `vars` (`Log` is always declared).
fn subscriber_with(vars: &str, body: &str) -> String {
    format!(
        r#"
codeunit 50976 "XMS Sub Log"
{{
    SingleInstance = true;
    var Seen: Text;
    procedure Add(T: Text) begin Seen += T + ';'; end;
    procedure Get(): Text begin exit(Seen); end;
}}
codeunit 50977 "XMS Substitute"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    var Log: Codeunit "XMS Sub Log"; {vars}
    begin
        Log.Add(Format(ReportId) + ',' + Format(RunMode));
        {body}
    end;
}}
codeunit 50978 "XMS Sub Tests"
{{
    procedure RunNoLayout(): Text
    var T: Codeunit "XMS Tests"; Log: Codeunit "XMS Sub Log"; R: Text;
    begin
        R := T.RunNoLayout();
        exit(R + '|' + Log.Get());
    end;
    procedure RunModalExcel(): Text
    var T: Codeunit "XMS Tests"; Log: Codeunit "XMS Sub Log"; R: Text;
    begin
        R := T.RunModalExcel();
        exit(R + '|' + Log.Get());
    end;
    procedure SaveXml(): Text
    var T: Codeunit "XMS Tests"; Log: Codeunit "XMS Sub Log"; R: Text;
    begin
        R := T.MultiDataset();
        exit(CopyStr(R, 1, 3) + '|' + Format(R = T.PlainDataset()) + '|' + Log.Get());
    end;
}}
"#
    )
}

fn text(proc: &str) -> String {
    al_testkit::text(Built::from_source(&src()).run_in("XMS Tests", proc))
}

fn with_subscriber(body: &str, proc: &str) -> Result<al_builtins::value::Value, Signal> {
    Built::from_source(&(src() + &subscriber(body))).run_in("XMS Sub Tests", proc)
}

/// The refusal of a report that declares the property, as main gives it for a variable of the report, or as the gate gives it
/// for a call outside the measured ones.
fn assert_refused(what: &str, r: Result<al_builtins::value::Value, Signal>) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(
            why.to_ascii_lowercase().contains("excellayoutmultipledatasheets"),
            "{what}: {why}"
        ),
        other => panic!("{what}: expected a refusal, got {other:?}"),
    }
}

const NO_LAYOUT: &str = "No|Report 50961 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.|ReportNoLayout";
const EXCEL: &str =
    "No|The current report operation is not supported for Excel Layout reports.|ReportUnsupportedOperation";

#[test]
fn multiple_data_sheets_saveas_xml_gives_the_dataset_of_the_report_without_it() {
    assumptions::take_hits();
    assert_eq!(text("SameDataset"), "Yes|Yes|Yes|Yes|Yes");
    assert!(assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets));
}

#[test]
fn multiple_data_sheets_run_without_a_request_page_raises_as_measured() {
    assumptions::take_hits();
    assert_eq!(text("RunNoLayout"), NO_LAYOUT);
    assert!(assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets));
    assert_eq!(text("RunModalNoLayout"), NO_LAYOUT);
    assert_eq!(text("RunExcel"), EXCEL);
    assert_eq!(text("RunModalExcel"), EXCEL);
}

#[test]
fn multiple_data_sheets_saveas_excel_answers_as_measured() {
    assumptions::take_hits();
    assert_eq!(text("SaveExcelLayout"), "Yes|Yes|Yes");
    assert!(assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets));
    assert_eq!(text("SaveExcelNoLayout"), "Yes|No|No");
    assert!(assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets));
}

/// The probe ran with SubstituteReport subscribers that leave the report as it is (Base Application's ReportManagement, the
/// corpus app's AMReports.al:229-234): the subscriber runs and the run is the measured one, flagged.
#[test]
fn multiple_data_sheets_run_admitted_when_a_subscriber_leaves_the_report() {
    for (body, what) in [("", "untouched"), ("NewReportId := ReportId;", "same id")] {
        assumptions::take_hits();
        assert_eq!(
            al_testkit::text(with_subscriber(body, "RunNoLayout")),
            format!("{NO_LAYOUT}|50961,Normal;"),
            "{what}"
        );
        assert!(
            assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets),
            "{what}"
        );
        assert_eq!(
            al_testkit::text(with_subscriber(body, "RunModalExcel")),
            format!("{EXCEL}|50963,RunModal;"),
            "{what}"
        );
    }
}

/// A subscriber that puts another report with the property in its place: that report is not the one the call asked for, so it
/// refuses.
#[test]
fn multiple_data_sheets_run_refused_when_a_subscriber_sets_another_such_report() {
    assert_refused(
        "substituted",
        with_subscriber("if ReportId = 50961 then NewReportId := 50963;", "RunNoLayout"),
    );
}

/// A subscriber that puts a report without the property in its place: that report runs as it does without this change.
#[test]
fn multiple_data_sheets_run_substituted_by_a_report_without_it_keeps_that_reports_answer() {
    assumptions::take_hits();
    let plain = "No|Report 50960 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.|ReportNoLayout";
    assert_eq!(text("RunPlain"), plain);
    assert_eq!(
        al_testkit::text(with_subscriber(
            "if ReportId = 50961 then NewReportId := 50960;",
            "RunNoLayout"
        )),
        format!("{plain}|50961,Normal;")
    );
    assert!(!assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets));
}

/// Only the instance the measured call makes runs as measured: a variable of the same report that a subscriber uses while the
/// call runs refuses.
#[test]
fn multiple_data_sheets_variable_used_during_an_admitted_run_refuses() {
    let src = src() + &subscriber_with("V: Report \"XMS Multi\";", "V.UseRequestPage(false);");
    assert_refused(
        "subscriber variable",
        Built::from_source(&src).run_in("XMS Sub Tests", "RunNoLayout"),
    );
}

/// SaveAs of these layouts raises no SubstituteReport in the runner, as for the report without the property
/// (docs/probes-needed.md): a subscriber that would set another report is not asked, and the report asked for is saved.
#[test]
fn multiple_data_sheets_saveas_xml_does_not_ask_a_substituting_subscriber() {
    assumptions::take_hits();
    assert_eq!(
        al_testkit::text(with_subscriber(
            "if ReportId = 50961 then NewReportId := 50960;",
            "SaveXml"
        )),
        "Yes|Yes|"
    );
    assert!(assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets));
}

/// A variable of a report that declares the property refuses before the code runs, as on main: declared only, its getters,
/// its layout streams and a procedure of the report. The probe used none.
#[test]
fn multiple_data_sheets_report_variable_refuses_as_on_main() {
    for p in [
        "Declared",
        "Language",
        "DefaultLayout",
        "ExcelLayout",
        "FormatRegion",
        "Procedure",
    ] {
        assert_refused(p, Built::from_source(&src()).run_in("XMS Variable Uses", p));
    }
}

/// A [ReportHandler] for such a report takes a variable of it: the handler is refused before the run, so a test that binds it
/// is not run, as on main.
#[test]
fn multiple_data_sheets_report_handler_is_not_run() {
    let l = al_testkit::compile(&src());
    let oi = (l.program.objects.iter())
        .position(|o| o.name == "XMS Handler Tests")
        .expect("codeunit");
    let ri = (l.program.objects[oi].routines.iter())
        .position(|r| r.name == "ExcelHandler")
        .expect("handler");
    let why = al_testkit::blocked(&l)[oi][ri]
        .clone()
        .expect("the handler is refused before the run");
    assert!(
        why.to_ascii_lowercase().contains("excellayoutmultipledatasheets"),
        "{why}"
    );
}

#[test]
fn multiple_data_sheets_shapes_not_measured_refuse() {
    for p in [
        "RunWord",
        "SavePdfOfWord",
        "VarRunWord",
        "VarRunNoLayout",
        "VarRunModalExcel",
        "StaticRunWithPage",
        "StaticRunRequestPage",
        "VarRunRequestPage",
        "StaticExecute",
        "StaticPrint",
        "RunProcessingOnly",
        "RunPageShown",
        "RunTrigger",
        "SavePdfNoLayout",
        "SaveExcelOfWord",
        "SaveXmlProcessingOnly",
        "SaveXmlTwoLayouts",
        "SaveXmlLegacyLayout",
        "SaveXmlExtended",
        "SaveXmlPageShown",
        "SaveXmlTrigger",
        "SaveXmlRequestExt",
        "SaveXmlOfVariable",
        "SaveXmlWithRecordRef",
        "SaveXmlWithParameters",
        "SaveXmlLayoutCaption",
        "SaveExcelLayoutCaption",
        "RunLayoutCaption",
        "SaveXmlBothDefaults",
        "SaveXmlCaption",
        "RunTransaction",
        "SaveXmlLabels",
        "SaveXmlGlobal",
        "SaveXmlFilterFields",
        "SaveXmlNested",
        "SaveXmlNoItems",
        "SaveXmlCallColumn",
        "SaveXmlNoView",
        "RunNoView",
        "SaveXmlBare",
        "RunBare",
        "RunAssertError",
        "RunAssertErrorInTry",
        "SaveXmlInInitOfTry",
        "SaveXmlTryResultDropped",
        "SaveXmlThroughHelper",
        "SaveXmlUseTemporary",
        "RunUseTemporary",
        "SaveXmlNewPage",
        "SaveXmlCaptionFalse",
        "SaveXmlColumnObsolete",
        "SaveXmlAnswerDropped",
        "SaveExcelAnswerDropped",
        "SaveXmlOptionFormat",
    ] {
        assert_refused(p, Built::from_source(&src()).run_in("XMS Tests", p));
    }
}

#[test]
fn report_without_the_property_saves_its_dataset() {
    assert_eq!(text("SavePlainXml"), "Yes");
}
