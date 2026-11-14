//! A rendering save of a report whose `WordLayout` and `RDLCLayout` properties declare its layouts (`DefaultLayout = Word`,
//! no rendering section, one data item, no trigger, no extension), saved to a stream: the two cells of corpus pull request
//! 1381 (reports/html-email-body/RxdDeclLayoutTests.al, RxdDeclEmailBodyTests.al), which mark `report-save-decl-word-rdlc`
//! (`report_save::cells_decl`). The false answers come before the run, so they hold with no dataset row too (flagged); every
//! other neighbor, and every save while the test can see a subscriber to a layout event, keeps main's refusal.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const OBJECTS: &str = r#"
table 2000000232 "Tenant Report Layout"
{
    fields
    {
        field(1; "Report ID"; Integer) { } field(2; Name; Text[250]) { } field(5; "App ID"; Guid) { }
        field(6; "Company Name"; Text[30]) { } field(7; Layout; Media) { }
        field(8; "Layout Format"; Option) { OptionMembers = RDLC,Word,Excel,Custom; }
    }
    keys { key(PK; "Report ID", Name, "App ID") { } }
}

table 2000000233 "Tenant Report Layout Selection"
{
    fields { field(1; "Report ID"; Integer) { } field(2; "Company Name"; Text[30]) { } field(3; "Layout Name"; Text[250]) { } }
    keys { key(PK; "Report ID", "Company Name") { } }
}

table 51400 "RD Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

report 51401 "RD Decl Word"
{
    WordLayout = 'RdDeclWord.docx';
    RDLCLayout = 'RdDeclRdlc.rdl';
    DefaultLayout = Word;
    dataset { dataitem(T; "RD Row") { column(Id; Id) { } } }
}

report 51402 "RD Decl Trigger"
{
    WordLayout = 'RdDeclWord.docx';
    RDLCLayout = 'RdDeclRdlc.rdl';
    DefaultLayout = Word;
    dataset { dataitem(T; "RD Row") { column(Id; Id) { } } }
    trigger OnPreReport() begin end;
}

report 51403 "RD Decl Excel"
{
    WordLayout = 'RdDeclWord.docx';
    RDLCLayout = 'RdDeclRdlc.rdl';
    ExcelLayout = 'RdDeclExcel.xlsx';
    DefaultLayout = Word;
    dataset { dataitem(T; "RD Row") { column(Id; Id) { } } }
}

codeunit 51404 "RD Tests"
{
    local procedure Rows(N: Integer)
    var R: Record "RD Row"; I: Integer;
    begin
        R.DeleteAll();
        for I := 1 to N do begin R.Id := I; R.Insert(); end;
    end;

    local procedure RdlcRow(Id: Integer)
    var S: Record "Report Layout Selection";
    begin
        S."Report ID" := Id; S."Company Name" := CompanyName(); S.Type := S.Type::"RDLC (built-in)"; S.Insert();
    end;

    local procedure TempWord(Id: Integer)
    var L: Record "Tenant Report Layout"; H: Record "RD Row" temporary; D: Codeunit "Design-time Report Selection"; O: OutStream; I: InStream; NullId: Guid;
    begin
        H.Data.CreateOutStream(O);
        O.WriteText('PK-docx');
        H.Data.CreateInStream(I);
        L."Report ID" := Id; L.Name := '{5B2E8C41-9D7A-4F36-B1C0-2A6E4D8F9C17}'; L."Layout Format" := L."Layout Format"::Word;
        L.Layout.ImportStream(I, 'body.docx');
        L.Insert();
        D.SetSelectedLayout(L.Name, NullId);
    end;

    local procedure Saved(Id: Integer; Fmt: ReportFormat): Text
    var Tmp: Record "RD Row" temporary; Out: OutStream; Ok: Boolean;
    begin
        ClearLastError();
        Tmp.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(Id, '', Fmt, Out);
        exit(Format(Ok) + '|' + Format(Tmp.Data.HasValue()) + '|' + GetLastErrorText());
    end;

    local procedure Statement(Id: Integer; Fmt: ReportFormat)
    var Tmp: Record "RD Row" temporary; Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Id, '', Fmt, Out);
    end;

    procedure RdlcPdf(): Text begin Rows(2); RdlcRow(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Pdf)); end;
    procedure RdlcWord(): Text begin Rows(2); RdlcRow(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Word)); end;
    procedure RdlcExcel(): Text begin Rows(2); RdlcRow(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Excel)); end;
    procedure RdlcHtml(): Text begin Rows(2); RdlcRow(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Html)); end;
    procedure RdlcHtmlStatement() begin Rows(2); RdlcRow(Report::"RD Decl Word"); Statement(Report::"RD Decl Word", ReportFormat::Html); end;

    procedure TempHtml(): Text begin Rows(2); TempWord(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Html)); end;
    procedure TempWordFmt(): Text begin Rows(2); TempWord(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Word)); end;
    procedure TempPdf(): Text begin Rows(2); TempWord(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Pdf)); end;
    procedure TempExcel(): Text begin Rows(2); TempWord(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Excel)); end;
    procedure TempExcelStatement() begin Rows(2); TempWord(Report::"RD Decl Word"); Statement(Report::"RD Decl Word", ReportFormat::Excel); end;
    procedure TempOverRdlcHtml(): Text begin Rows(2); RdlcRow(Report::"RD Decl Word"); TempWord(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Html)); end;

    // Neighbors.
    procedure TriggerRdlcPdf(): Text begin Rows(2); RdlcRow(Report::"RD Decl Trigger"); exit(Saved(Report::"RD Decl Trigger", ReportFormat::Pdf)); end;
    procedure TriggerTempHtml(): Text begin Rows(2); TempWord(Report::"RD Decl Trigger"); exit(Saved(Report::"RD Decl Trigger", ReportFormat::Html)); end;
    procedure ExcelRdlcPdf(): Text begin Rows(2); RdlcRow(Report::"RD Decl Excel"); exit(Saved(Report::"RD Decl Excel", ReportFormat::Pdf)); end;
    procedure ExcelTempHtml(): Text begin Rows(2); TempWord(Report::"RD Decl Excel"); exit(Saved(Report::"RD Decl Excel", ReportFormat::Html)); end;
    procedure NoRowsRdlcPdf(): Text begin Rows(0); RdlcRow(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Pdf)); end;
    procedure NoRowsTempHtml(): Text begin Rows(0); TempWord(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Html)); end;
    procedure NoRowsRdlcHtml(): Text begin Rows(0); RdlcRow(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Html)); end;
    procedure NoRowsRdlcHtmlStatement() begin Rows(0); RdlcRow(Report::"RD Decl Word"); Statement(Report::"RD Decl Word", ReportFormat::Html); end;
    procedure NoRowsTempExcel(): Text begin Rows(0); TempWord(Report::"RD Decl Word"); exit(Saved(Report::"RD Decl Word", ReportFormat::Excel)); end;

    procedure RdlcRowOf(Id: Integer) begin RdlcRow(Id); end;
    procedure TempWordOf(Id: Integer) begin TempWord(Id); end;
    procedure RowsOf(N: Integer) begin Rows(N); end;
}
"#;

/// A manual subscriber of one layout event each: no cell of pull request 1381 has one.
const SUBSTITUTE: &str = r#"
codeunit 51405 "RD Substitute"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure Substitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
    end;
}
"#;
const READY: &str = r#"
codeunit 51405 "RD Ready"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'OnDocumentReady', '', false, false)]
    local procedure Ready(ObjectId: Integer; ObjectPayload: JsonObject; DocumentStream: InStream; var TargetStream: OutStream; var Success: Boolean)
    begin
    end;
}
"#;
const BUILTIN: &str = r#"
codeunit 51405 "RD Builtin"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectedBuiltinLayoutType', '', false, false)]
    local procedure Builtin(ObjectID: Integer; var LayoutType: Option "None",RDLC,Word,Excel,Custom)
    begin
    end;
}
"#;

/// A test codeunit that names one subscriber codeunit (`SUB`) and binds it before each save.
const SUBSCRIBER_TESTS: &str = r#"
codeunit 51406 "RD Tests SUB"
{
    var
        T: Codeunit "RD Tests";
        Sub: Codeunit "RD SUB";

    local procedure Used(Fmt: ReportFormat): Text
    var Tmp: Record "RD Row" temporary; Out: OutStream;
    begin
        BindSubscription(Sub);
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"RD Decl Word", '', Fmt, Out)));
    end;

    procedure RdlcHtml(): Text begin T.RowsOf(2); T.RdlcRowOf(Report::"RD Decl Word"); exit(Used(ReportFormat::Html)); end;
    procedure RdlcHtmlStatement()
    var Tmp: Record "RD Row" temporary; Out: OutStream;
    begin
        T.RowsOf(2); T.RdlcRowOf(Report::"RD Decl Word"); BindSubscription(Sub);
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RD Decl Word", '', ReportFormat::Html, Out);
    end;
    procedure RdlcHtmlRecordRef(): Text
    var Tmp: Record "RD Row" temporary; R: Record "RD Row"; Ref: RecordRef; Out: OutStream;
    begin
        T.RowsOf(2); T.RdlcRowOf(Report::"RD Decl Word"); BindSubscription(Sub);
        Ref.GetTable(R);
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"RD Decl Word", '', ReportFormat::Html, Out, Ref)));
    end;
    procedure RdlcPdf(): Text begin T.RowsOf(2); T.RdlcRowOf(Report::"RD Decl Word"); exit(Used(ReportFormat::Pdf)); end;
    procedure TempExcel(): Text begin T.RowsOf(2); T.TempWordOf(Report::"RD Decl Word"); exit(Used(ReportFormat::Excel)); end;
    procedure TempHtml(): Text begin T.RowsOf(2); T.TempWordOf(Report::"RD Decl Word"); exit(Used(ReportFormat::Html)); end;
}
"#;

const UNSUPPORTED: &str = "The Html output format is not supported by the current report layout format.";

fn run(proc: &str) -> Result<Value, Signal> {
    super::report_management_stand_in::built(OBJECTS).run_in("RD Tests", proc)
}

#[track_caller]
fn marked(proc: &str, want: &str) {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run(proc)), want, "{proc}");
    let hits = assumptions::take_hits();
    assert!(hits.contains(&Assumption::ReportSaveDeclWordRdlc), "{proc}: {hits:?}");
}

/// RxdDeclLayoutTests.al:130, :177, :219, :266: the company's "RDLC (built-in)" row renders the RDLC layout as Pdf, Word and
/// Excel; Html answers false with ReportUnsupportedFormat, and the statement form raises it (:284).
#[test]
fn decl_word_rdlc_the_companys_rdlc_row_renders_the_rdlc_layout() {
    for p in ["RdlcPdf", "RdlcWord", "RdlcExcel"] {
        marked(p, "Yes|Yes|");
    }
    marked("RdlcHtml", &format!("No|No|{UNSUPPORTED}"));
    match run("RdlcHtmlStatement") {
        Err(Signal::Error(e)) => assert_eq!(e.message, UNSUPPORTED),
        other => panic!("expected the error, got {other:?}"),
    }
}

/// RxdDeclLayoutTests.al:313, :340, :367, :391, :811: a temporary selection of a user-defined Word layout renders it as Html,
/// Word and Pdf, alone and over the company's RDLC row; Excel answers false and its statement raises.
#[test]
fn decl_word_rdlc_a_temporary_selection_renders_the_selected_word_layout() {
    for p in ["TempHtml", "TempWordFmt", "TempPdf", "TempOverRdlcHtml"] {
        marked(p, "Yes|Yes|");
    }
    let excel = "The Excel output format is not supported by the current report layout format.";
    marked("TempExcel", &format!("No|No|{excel}"));
    match run("TempExcelStatement") {
        Err(Signal::Error(e)) => assert_eq!(e.message, excel),
        other => panic!("expected the error, got {other:?}"),
    }
}

/// A report trigger, an `ExcelLayout` property, or no dataset row: not a cell of pull request 1381. A stored RDLC row
/// refuses as on main; an Html save under the temporary selection keeps main's row (HtRenderedTextTests.al:494, which reads
/// neither the trigger nor the layout properties), and a save that writes with no dataset row refuses as on main.
#[test]
fn decl_word_rdlc_neighbors_keep_mains_answer() {
    for (p, want) in [
        ("TriggerRdlcPdf", None),
        ("ExcelRdlcPdf", None),
        ("NoRowsRdlcPdf", None),
        ("NoRowsTempHtml", None),
        ("TriggerTempHtml", Some(Assumption::ReportLayoutTempSelection)),
        ("ExcelTempHtml", Some(Assumption::ReportLayoutTempSelection)),
    ] {
        assumptions::clear_hits();
        match (run(p), want) {
            (Err(Signal::Unsupported(why)), None) => assert!(why.contains("not measured on BC"), "{p}: {why}"),
            (Ok(v), Some(a)) => {
                assert_eq!(al_testkit::text(Ok(v)), "Yes|Yes|", "{p}");
                let hits = assumptions::take_hits();
                assert!(hits.contains(&a), "{p}");
                assert!(!hits.contains(&Assumption::ReportSaveDeclWordRdlc), "{p}");
            }
            (other, _) => panic!("{p}: got {other:?}"),
        }
    }
}

/// A test that binds a subscriber to SubstituteReport, OnDocumentReady or SelectedBuiltinLayoutType: BC raises
/// SubstituteReport first for a static SaveAs (events/platform-triggers/PeVTests.al:721) and this runtime raises none of them
/// to a test's subscriber, and pull request 1381 measured no subscriber. Every save refuses. Main refused these shapes too,
/// except a temporary Word selection saved as Html with a SelectedBuiltinLayoutType subscriber, which main answered (flagged).
#[test]
fn decl_word_rdlc_a_subscriber_to_a_layout_event_refuses() {
    const PROCS: [&str; 6] = [
        "RdlcHtml",
        "RdlcHtmlStatement",
        "RdlcHtmlRecordRef",
        "RdlcPdf",
        "TempExcel",
        "TempHtml",
    ];
    let mut answered = Vec::new();
    for (sub, subscriber) in [("Substitute", SUBSTITUTE), ("Ready", READY), ("Builtin", BUILTIN)] {
        let src = format!("{OBJECTS}{subscriber}{}", SUBSCRIBER_TESTS.replace("SUB", sub));
        for p in PROCS {
            match super::report_management_stand_in::built(&src).run_in(&format!("RD Tests {sub}"), p) {
                Err(Signal::Unsupported(why)) if why.contains(&format!("\"RD {sub}\"")) => {}
                other => answered.push(format!("{sub} {p}: {other:?}")),
            }
        }
    }
    assert!(answered.is_empty(), "expected refusals: {answered:#?}");
}

/// With no dataset row, Html of the company's RDLC row and Excel of the temporary selection answer before the run, as with
/// two rows; pull request 1381 measured them with two rows only, so the answer stays flagged.
#[test]
fn decl_word_rdlc_false_answers_come_before_the_run() {
    marked("NoRowsRdlcHtml", &format!("No|No|{UNSUPPORTED}"));
    let excel = "The Excel output format is not supported by the current report layout format.";
    marked("NoRowsTempExcel", &format!("No|No|{excel}"));
    assumptions::clear_hits();
    match run("NoRowsRdlcHtmlStatement") {
        Err(Signal::Error(e)) => assert_eq!(e.message, UNSUPPORTED),
        other => panic!("expected the error, got {other:?}"),
    }
    assert!(assumptions::take_hits().contains(&Assumption::ReportSaveDeclWordRdlc));
}
