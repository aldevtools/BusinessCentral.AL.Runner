//! A rendering save while a stored Report Layout Selection row of the company names the report
//! (`report_placeholder::stored`; corpus pull request 1371, reports/html-email-body/RxLayoutSelectionTests.al): the two
//! measured selections answer, and every neighbor keeps main's answer or refusal.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const OBJECTS: &str = r#"
table 9651 "Report Layout Selection"
{
    fields
    {
        field(1; "Report ID"; Integer) { }
        field(3; "Company Name"; Text[30]) { }
        field(4; Type; Option) { OptionMembers = "RDLC (built-in)","Word (built-in)","Custom Layout","Excel Layout","External Layout"; }
        field(6; "Custom Report Layout Code"; Code[20]) { }
    }
    keys { key(PK; "Report ID", "Company Name") { } }
}

table 9650 "Custom Report Layout"
{
    fields { field(1; "Code"; Code[20]) { } field(2; "Report ID"; Integer) { } }
    keys { key(PK; "Code") { } }
}

table 51060 "RS Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 51061 "RS Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin Entries += T + ';'; end;
    procedure Take() T: Text begin T := Entries; Entries := ''; end;
}

report 51062 "RS Word Rdlc"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(T; "RS Row") { column(Id; Id) { } } }
    rendering
    {
        layout(W) { Type = Word; LayoutFile = 'RsWord.docx'; }
        layout(R) { Type = RDLC; LayoutFile = 'RsRdlc.rdl'; }
    }
    trigger OnPreReport() var L: Codeunit "RS Log"; begin L.Add('OnPreReport'); end;
}

report 51063 "RS Word Only"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(T; "RS Row") { column(Id; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = 'RsWordOnly.docx'; } }
}

codeunit 51064 "RS Tests"
{
    local procedure Rows(N: Integer)
    var R: Record "RS Row"; I: Integer;
    begin
        R.DeleteAll();
        for I := 1 to N do begin R.Id := I; R.Insert(); end;
    end;

    local procedure Select(Id: Integer; Company: Text[30]; T: Option; LayoutCode: Code[20])
    var S: Record "Report Layout Selection";
    begin
        S."Report ID" := Id; S."Company Name" := Company; S.Type := T; S."Custom Report Layout Code" := LayoutCode; S.Insert();
    end;

    // The answer, whether the stream holds anything, the error code and the triggers that ran.
    local procedure Saved(ReportId: Integer; Fmt: ReportFormat): Text
    var Tmp: Record "RS Row" temporary; L: Codeunit "RS Log"; Out: OutStream; Ok: Boolean;
    begin
        L.Take();
        ClearLastError();
        Tmp.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(ReportId, '', Fmt, Out);
        exit(Format(Ok) + '|' + Format(Tmp.Data.HasValue()) + '|' + GetLastErrorCode() + '|' + L.Take());
    end;

    local procedure Rdlc(Company: Text[30])
    var S: Record "Report Layout Selection";
    begin
        Rows(2);
        Select(Report::"RS Word Rdlc", Company, S.Type::"RDLC (built-in)", '');
    end;

    procedure RdlcHtml(): Text begin Rdlc(CompanyName()); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Html)); end;
    procedure RdlcHtmlText(): Text begin Rdlc(CompanyName()); Saved(Report::"RS Word Rdlc", ReportFormat::Html); exit(GetLastErrorText()); end;
    procedure RdlcPdf(): Text begin Rdlc(CompanyName()); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Pdf)); end;
    procedure RdlcWord(): Text begin Rdlc(CompanyName()); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Word)); end;
    procedure RdlcExcel(): Text begin Rdlc(CompanyName()); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Excel)); end;
    procedure RdlcXml(): Text begin Rdlc(CompanyName()); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Xml)); end;
    procedure RdlcPdfNoRow(): Text begin Rdlc(CompanyName()); Rows(0); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Pdf)); end;
    procedure RdlcHtmlStatement()
    var Tmp: Record "RS Row" temporary; Out: OutStream;
    begin
        Rdlc(CompanyName());
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RS Word Rdlc", '', ReportFormat::Html, Out);
    end;
    procedure RdlcHtmlFile(): Boolean begin Rdlc(CompanyName()); exit(Report.SaveAsHtml(Report::"RS Word Rdlc", 'out.html')); end;

    // Rows BC does not read: the default Word layout renders.
    procedure EmptyCompanyHtml(): Text begin Rdlc(''); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Html)); end;
    procedure EmptyCompanyExcel(): Text begin Rdlc(''); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Excel)); end;
    procedure OtherCompanyHtml(): Text begin Rdlc('RS Other'); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Html)); end;

    // Two rows (the company's and another): not measured, main's answer.
    procedure TwoRowsHtml(): Text
    var S: Record "Report Layout Selection";
    begin
        Rdlc(CompanyName());
        Select(Report::"RS Word Rdlc", 'RS Other', S.Type::"RDLC (built-in)", '');
        exit(Saved(Report::"RS Word Rdlc", ReportFormat::Html));
    end;

    // A report without an RDLC layout: main's refusal.
    procedure RdlcRowWordOnly(): Text
    var S: Record "Report Layout Selection";
    begin
        Rows(2);
        Select(Report::"RS Word Only", CompanyName(), S.Type::"RDLC (built-in)", '');
        exit(Saved(Report::"RS Word Only", ReportFormat::Html));
    end;

    local procedure Custom(LayoutCode: Code[20])
    var S: Record "Report Layout Selection";
    begin
        Rows(2);
        Select(Report::"RS Word Rdlc", CompanyName(), S.Type::"Custom Layout", LayoutCode);
    end;

    procedure UnknownCode(Fmt: ReportFormat): Text begin Custom('RSNOSUCH'); exit(Saved(Report::"RS Word Rdlc", Fmt)); end;
    procedure UnknownHtml(): Text begin exit(UnknownCode(ReportFormat::Html)); end;
    procedure UnknownPdf(): Text begin exit(UnknownCode(ReportFormat::Pdf)); end;
    procedure UnknownWord(): Text begin exit(UnknownCode(ReportFormat::Word)); end;
    procedure UnknownExcel(): Text begin exit(UnknownCode(ReportFormat::Excel)); end;
    procedure UnknownText(): Text begin UnknownCode(ReportFormat::Pdf); exit(GetLastErrorText()); end;
    procedure KnownCode(): Text
    var C: Record "Custom Report Layout";
    begin
        C.Code := 'RSKNOWN'; C."Report ID" := Report::"RS Word Rdlc"; C.Insert();
        Custom('RSKNOWN');
        exit(Saved(Report::"RS Word Rdlc", ReportFormat::Pdf));
    end;
    procedure EmptyCode(): Text begin Custom(''); exit(Saved(Report::"RS Word Rdlc", ReportFormat::Pdf)); end;
}
"#;

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(OBJECTS).run_in("RS Tests", proc)
}

fn text(proc: &str) -> String {
    al_testkit::text(run(proc))
}

#[track_caller]
fn refused(proc: &str, want: &str) {
    match run(proc) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(want), "{proc}: {why}"),
        other => panic!("{proc}: expected a refusal naming {want:?}, got {other:?}"),
    }
}

/// RxLayoutSelectionTests.al:452 and :477: Html false with ReportUnsupportedFormat and no trigger; Pdf, Word and Excel render.
#[test]
fn stored_selection_the_companys_rdlc_row_renders_the_rdlc_layout() {
    assumptions::clear_hits();
    assert_eq!(text("RdlcHtml"), "No|No|ReportUnsupportedFormat|");
    assert!(assumptions::take_hits().contains(&Assumption::ReportStoredLayoutSelection));
    assert_eq!(
        text("RdlcHtmlText"),
        "The Html output format is not supported by the current report layout format."
    );
    for p in ["RdlcPdf", "RdlcWord", "RdlcExcel"] {
        assumptions::clear_hits();
        assert_eq!(text(p), "Yes|Yes||OnPreReport;", "{p}");
        assert!(
            assumptions::take_hits().contains(&Assumption::ReportStoredLayoutSelection),
            "{p}"
        );
    }
    // Xml is the dataset, as without a row (:1082 `xml returned` Yes).
    assumptions::clear_hits();
    assert_eq!(text("RdlcXml"), "Yes|Yes||OnPreReport;");
    assert!(!assumptions::take_hits().contains(&Assumption::ReportStoredLayoutSelection));
}

/// RxLayoutSelectionTests.al:795 and :1022: a row of another company or with an empty Company Name is not read. Main's answer.
#[test]
fn stored_selection_a_row_that_is_not_the_companys_keeps_the_default_word_layout() {
    assumptions::clear_hits();
    assert_eq!(text("EmptyCompanyHtml"), "Yes|Yes||OnPreReport;");
    assert_eq!(text("OtherCompanyHtml"), "Yes|Yes||OnPreReport;");
    assert_eq!(text("EmptyCompanyExcel"), "No|No|ReportUnsupportedFormat|");
    assert!(!assumptions::take_hits().contains(&Assumption::ReportStoredLayoutSelection));
}

/// RxLayoutSelectionTests.al:746 and :771: every format false with ReportNoLayout, no trigger.
#[test]
fn stored_selection_a_custom_layout_code_that_names_nothing_answers_no_layout() {
    for p in ["UnknownHtml", "UnknownPdf", "UnknownWord", "UnknownExcel"] {
        assumptions::clear_hits();
        assert_eq!(text(p), "No|No|ReportNoLayout|", "{p}");
        assert!(
            assumptions::take_hits().contains(&Assumption::ReportStoredLayoutSelection),
            "{p}"
        );
    }
    assert_eq!(
        text("UnknownText"),
        "Report 51062 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom."
    );
}

/// Neighbors of the two cells keep main's outcome: its refusal of a selection, or its answer for the report's own layout.
#[test]
fn stored_selection_neighbors_keep_mains_outcome() {
    for p in ["KnownCode", "EmptyCode", "RdlcRowWordOnly"] {
        refused(p, "a Report Layout Selection row selects a layout for it");
    }
    // The statement form, two rows of the report, a save to a file: not measured; main takes the company's built-in row for
    // the report's own layout and writes the Word layout's placeholder.
    run("RdlcHtmlStatement").expect("main's answer");
    assert_eq!(text("TwoRowsHtml"), "Yes|Yes||OnPreReport;");
    assert!(matches!(run("RdlcHtmlFile"), Ok(Value::Boolean(true))));
    refused("RdlcPdfNoRow", "selects its RDLC layout (not measured)");
}
