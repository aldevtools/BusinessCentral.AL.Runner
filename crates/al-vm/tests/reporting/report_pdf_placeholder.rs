//! ADR-040 amendment (owner, 2026-11-08): a Pdf save of a report with an RDLC or Word layout runs the report exactly as
//! the Xml path does and writes a small placeholder PDF (a `%PDF-` header and one blank page). The bytes are not BC's;
//! only "it exists, it is a PDF, it is not empty" is meant to be read: the PDF carries the placeholder mark, and only a read
//! inside the pinned signature `%PDF-` answers (owner decision 2026-11-12). The path marks the assumption
//! `report-pdf-placeholder`, so a test that passes through it is a flagged pass.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const SRC: &str = r#"
table 50950 "RPP Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } field(3; Other; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 50951 "RPP Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var Result: Text; begin Result := Entries; Entries := ''; exit(Result); end;
}

report 50952 "RPP Rdlc"
{
    UsageCategory = None;
    DefaultLayout = RDLC;
    RDLCLayout = 'RppRdlc.rdl';
    dataset
    {
        dataitem(T; "RPP Row")
        {
            column(Id; Id) { }
            trigger OnAfterGetRecord() var L: Codeunit "RPP Log"; begin L.Add('row' + Format(T.Id)); end;
        }
    }
    trigger OnInitReport() var L: Codeunit "RPP Log"; begin L.Add('init'); end;
    trigger OnPreReport() var L: Codeunit "RPP Log"; begin L.Add('pre'); end;
    trigger OnPostReport() var L: Codeunit "RPP Log"; begin L.Add('post'); end;
}

report 50953 "RPP Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset
    {
        dataitem(T; "RPP Row")
        {
            column(Id; Id) { }
            trigger OnAfterGetRecord() var L: Codeunit "RPP Log"; begin L.Add('row' + Format(T.Id)); end;
        }
    }
    rendering
    {
        layout(WordOne) { Type = Word; LayoutFile = 'RppWord.docx'; }
    }
}

report 50954 "RPP Excel"
{
    UsageCategory = None;
    DefaultRenderingLayout = ExcelOne;
    dataset { dataitem(T; "RPP Row") { column(Id; Id) { } } }
    rendering
    {
        layout(ExcelOne) { Type = Excel; LayoutFile = 'RppExcel.xlsx'; }
    }
}

report 50955 "RPP Failing"
{
    UsageCategory = None;
    DefaultLayout = RDLC;
    RDLCLayout = 'RppFailing.rdl';
    dataset { dataitem(T; "RPP Row") { column(Id; Id) { } } }
    trigger OnPreReport() begin Error('stop in pre'); end;
}

report 50958 "RPP Processing"
{
    UsageCategory = None;
    ProcessingOnly = true;
    dataset { dataitem(T; "RPP Row") { trigger OnAfterGetRecord() var L: Codeunit "RPP Log"; begin L.Add('row'); end; } }
}

codeunit 50956 "RPP Tests"
{
    local procedure Seed()
    var R: Record "RPP Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
    end;

    local procedure Head(var R: Record "RPP Row"): Text
    var InS: InStream; C: Char; I: Integer; Result: Text;
    begin
        R.CalcFields(Data);
        R.Data.CreateInStream(InS);
        for I := 1 to 5 do begin
            InS.Read(C);
            Result += Format(C);
        end;
        exit(Result);
    end;

    procedure ToStream(): Text
    var L: Codeunit "RPP Log"; R: Record "RPP Row"; Out: OutStream; Ok: Boolean;
    begin
        Seed();
        R.Id := 9;
        R.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(Report::"RPP Rdlc", '', ReportFormat::Pdf, Out);
        R.Insert();
        exit(Format(Ok) + '|' + L.Take() + '|' + Format(R.Data.HasValue()) + '|' + Head(R));
    end;

    procedure ToStreamOnAVariable(): Text
    var L: Codeunit "RPP Log"; R: Record "RPP Row"; Rep: Report "RPP Rdlc"; Out: OutStream; Ok: Boolean;
    begin
        Seed();
        R.Id := 9;
        R.Data.CreateOutStream(Out);
        Ok := Rep.SaveAs('', ReportFormat::Pdf, Out);
        R.Insert();
        exit(Format(Ok) + '|' + L.Take() + '|' + Head(R));
    end;

    procedure ToFile(): Text
    var L: Codeunit "RPP Log"; F: File; Ok: Boolean; Len: Integer;
    begin
        Seed();
        Ok := Report.SaveAsPdf(Report::"RPP Rdlc", 'out/a.pdf');
        F.Open('out/a.pdf');
        Len := F.Len();
        F.Close();
        exit(Format(Ok) + '|' + L.Take() + '|' + Format(Exists('out/a.pdf')) + '|' + Format(Len > 0));
    end;

    procedure ToFileOnAVariable(): Text
    var L: Codeunit "RPP Log"; Rep: Report "RPP Rdlc"; Ok: Boolean;
    begin
        Seed();
        Ok := Rep.SaveAsPdf('out/b.pdf');
        exit(Format(Ok) + '|' + L.Take() + '|' + Format(Exists('out/b.pdf')));
    end;

    procedure WordLayout(): Text
    var L: Codeunit "RPP Log"; R: Record "RPP Row"; Out: OutStream; Ok: Boolean;
    begin
        Seed();
        R.Id := 9;
        R.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(Report::"RPP Word", '', ReportFormat::Pdf, Out);
        R.Insert();
        exit(Format(Ok) + '|' + L.Take() + '|' + Head(R));
    end;

    procedure ExcelLayout()
    var R: Record "RPP Row"; Out: OutStream;
    begin
        R.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPP Excel", '', ReportFormat::Pdf, Out);
    end;

    procedure WordFormat()
    var R: Record "RPP Row"; Out: OutStream;
    begin
        R.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPP Rdlc", '', ReportFormat::Word, Out);
    end;

    procedure ProcessingOnly(): Text
    var L: Codeunit "RPP Log"; R: Record "RPP Row"; Out: OutStream; Ok: Boolean;
    begin
        Seed();
        R.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(Report::"RPP Processing", '', ReportFormat::Pdf, Out);
        exit(Format(Ok) + '|' + GetLastErrorText() + '|' + L.Take());
    end;

    procedure TriggerError(): Text
    var R: Record "RPP Row"; Out: OutStream;
    begin
        R.Data.CreateOutStream(Out);
        asserterror Report.SaveAs(Report::"RPP Failing", '', ReportFormat::Pdf, Out);
        exit(GetLastErrorText());
    end;
}
"#;

// Reads of the PDF placeholder (owner decision 2026-11-12): only bytes inside the prefix the corpus pins as BC's (`%PDF-`)
// answer; every other read of the content, and the size as a number, is refused.
const READS: &str = r#"
codeunit 2000000024 Base64Convert
{
    [Native] procedure ToBase64(InStream: InStream; InsertLineBreaks: Boolean): Text begin end;
}

codeunit 50960 "RPP Read Tests"
{
    local procedure Save(var R: Record "RPP Row")
    var Out: OutStream;
    begin
        R.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPP Rdlc", '', ReportFormat::Pdf, Out);
    end;

    local procedure Open(var R: Record "RPP Row"; var InS: InStream)
    begin
        Save(R);
        R.Data.CreateInStream(InS, TextEncoding::Windows);
    end;

    procedure ReadTextFive(): Text
    var R: Record "RPP Row" temporary; InS: InStream; T: Text; N: Integer;
    begin
        Open(R, InS);
        N := InS.ReadText(T, 5);
        exit(T + '|' + Format(N));
    end;

    procedure BytesAndEos(): Text
    var R: Record "RPP Row" temporary; InS: InStream; B: Byte; I: Integer; Code: Integer; Result: Text;
    begin
        Save(R);
        R.Data.CreateInStream(InS);
        for I := 1 to 4 do
            if not InS.EOS() then begin
                InS.Read(B);
                Code := B;
                Result += Format(Code) + ',';
            end;
        exit(Result);
    end;

    procedure SizeAgainstZero(): Text
    var R: Record "RPP Row" temporary;
    begin
        Save(R);
        exit(Format(R.Data.Length() > 0) + '|' + Format(R.Data.Length() = 0));
    end;

    procedure PrefixOfTheCopy(): Text
    var R: Record "RPP Row" temporary; InS: InStream; Out: OutStream; T: Text;
    begin
        Open(R, InS);
        R.Other.CreateOutStream(Out);
        CopyStream(Out, InS);
        R.Other.CreateInStream(InS, TextEncoding::Windows);
        InS.ReadText(T, 5);
        exit(T);
    end;

    procedure PrefixOfTheFile(): Text
    var F: File; InS: InStream; T: Text;
    begin
        Report.SaveAsPdf(Report::"RPP Rdlc", 'out/r.pdf');
        F.Open('out/r.pdf');
        F.CreateInStream(InS);
        InS.ReadText(T, 5);
        exit(T);
    end;

    procedure ReadLine()
    var R: Record "RPP Row" temporary; InS: InStream; T: Text;
    begin
        Open(R, InS);
        InS.ReadText(T);
    end;

    procedure ReadTextSix()
    var R: Record "RPP Row" temporary; InS: InStream; T: Text;
    begin
        Open(R, InS);
        InS.ReadText(T, 6);
    end;

    procedure ReadTextPastThePrefix()
    var R: Record "RPP Row" temporary; InS: InStream; T: Text;
    begin
        Open(R, InS);
        InS.ReadText(T, 3);
        InS.ReadText(T, 3);
    end;

    procedure ReadTwentyBytes()
    var R: Record "RPP Row" temporary; InS: InStream; T: Text;
    begin
        Open(R, InS);
        InS.Read(T, 20);
    end;

    procedure ReadTextUtf8()
    var R: Record "RPP Row" temporary; InS: InStream; T: Text;
    begin
        Save(R);
        R.Data.CreateInStream(InS, TextEncoding::UTF8);
        InS.ReadText(T, 4);
    end;

    procedure SixthByte()
    var R: Record "RPP Row" temporary; InS: InStream; B: Byte; I: Integer;
    begin
        Open(R, InS);
        for I := 1 to 6 do
            InS.Read(B);
    end;

    procedure EosAfterThePrefix(): Boolean
    var R: Record "RPP Row" temporary; InS: InStream; T: Text;
    begin
        Open(R, InS);
        InS.ReadText(T, 5);
        exit(InS.EOS());
    end;

    procedure StreamLength(): BigInteger
    var R: Record "RPP Row" temporary; InS: InStream;
    begin
        Open(R, InS);
        exit(InS.Length());
    end;

    procedure Base64(): Text
    var R: Record "RPP Row" temporary; InS: InStream; B: Codeunit Base64Convert;
    begin
        Open(R, InS);
        exit(B.ToBase64(InS, false));
    end;

    procedure CopyStartingPastTheStart(): Text
    var R: Record "RPP Row" temporary; InS: InStream; Out: OutStream; T: Text;
    begin
        Open(R, InS);
        InS.ReadText(T, 2);
        R.Other.CreateOutStream(Out);
        CopyStream(Out, InS);
        R.Other.CreateInStream(InS, TextEncoding::Windows);
        InS.ReadText(T, 3);
        exit(T);
    end;

    procedure WrittenOver(): Text
    var R: Record "RPP Row" temporary; InS: InStream; Out: OutStream; T: Text;
    begin
        Save(R);
        R.Data.CreateOutStream(Out);
        Out.WriteText('SIG');
        R.Data.CreateInStream(InS, TextEncoding::Windows);
        InS.ReadText(T, 5);
        exit(T);
    end;

    procedure LengthAsANumber(): Text
    var R: Record "RPP Row" temporary;
    begin
        Save(R);
        exit(Format(R.Data.Length()));
    end;
}
"#;

const CONTENT: &str = "a read of the content of a placeholder Pdf";

fn reads(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&format!("{SRC}{READS}")).run_in("RPP Read Tests", proc)
}

#[test]
fn report_pdf_placeholder_a_read_inside_the_pinned_prefix_answers() {
    assert_eq!(al_testkit::text(reads("ReadTextFive")), "%PDF-|5");
    assert_eq!(al_testkit::text(reads("BytesAndEos")), "37,80,68,70,");
    assert_eq!(al_testkit::text(reads("SizeAgainstZero")), "Yes|No");
    assert_eq!(al_testkit::text(reads("PrefixOfTheCopy")), "%PDF-");
    assert_eq!(al_testkit::text(reads("PrefixOfTheFile")), "%PDF-");
}

#[test]
fn report_pdf_placeholder_every_other_read_is_refused() {
    for p in [
        "ReadLine",
        "ReadTextSix",
        "ReadTextPastThePrefix",
        "ReadTwentyBytes",
        "ReadTextUtf8",
        "SixthByte",
        "EosAfterThePrefix",
        "StreamLength",
        "Base64",
        "CopyStartingPastTheStart",
        "WrittenOver",
    ] {
        match reads(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains(CONTENT), "{p}: {why}"),
            other => panic!("{p}: expected the read to be refused, got {other:?}"),
        }
    }
}

#[test]
fn report_pdf_placeholder_the_size_as_a_number_is_refused() {
    match reads("LengthAsANumber") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("the byte size of a placeholder Pdf"), "{why}"),
        other => panic!("expected the size to be refused, got {other:?}"),
    }
}

const SUBSCRIBER: &str = r#"
codeunit 50957 "RPP Layout Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectReportLayoutCode', '', false, false)]
    local procedure OnSelect(ObjectId: Integer; var LayoutCode: Text; var LayoutType: Option "None",RDLC,Word,Excel,Custom; var Success: Boolean)
    begin
    end;
}
"#;

// Base Application's ReportManagement answers SelectedBuiltinLayoutType and SelectReportLayoutCode from table 9651 "Report
// Layout Selection" and from the SingleInstance codeunit 9654 "Design-time Report Selection" (`report_management_stand_in`
// holds them). A selection there can pick a layout that is not the report's default (an Excel layout writes no output:
// reports/layouts-saveas/AMReportOutputTests.al:257 ExcelLayout_SaveAsPdf), so a placeholder is written only while none of
// them selects something other than an RDLC or Word layout of the report.
const SELECTION: &str = r#"
codeunit 50959 "RPP Selection Tests"
{
    local procedure Save(Id: Integer): Text
    var R: Record "RPP Row"; Out: OutStream; Ok: Boolean;
    begin
        R.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(Id, '', ReportFormat::Pdf, Out);
        exit(Format(Ok));
    end;

    local procedure Select(Id: Integer; T: Option)
    var S: Record "Report Layout Selection";
    begin
        S."Report ID" := Id; S."Company Name" := CompanyName(); S.Type := T; S.Insert();
    end;

    procedure CustomSelection(): Text
    var S: Record "Report Layout Selection";
    begin
        Select(Report::"RPP Rdlc", S.Type::"Custom Layout");
        exit(Save(Report::"RPP Rdlc"));
    end;

    procedure BuiltinSelection(): Text
    var S: Record "Report Layout Selection";
    begin
        Select(Report::"RPP Rdlc", S.Type::"RDLC (built-in)");
        exit(Save(Report::"RPP Rdlc"));
    end;

    procedure DesignTimeSelection(): Text
    var D: Codeunit "Design-time Report Selection";
    begin
        D.SetSelectedLayout('Other');
        exit(Save(Report::"RPP Rdlc"));
    end;

    // Html of an RDLC report answers false for its own layout; a selection may pick a Word layout that renders Html.
    procedure DesignTimeSelectionHtml(): Text
    var D: Codeunit "Design-time Report Selection"; R: Record "RPP Row"; Out: OutStream;
    begin
        D.SetSelectedLayout('Other');
        R.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"RPP Rdlc", '', ReportFormat::Html, Out)));
    end;
}
"#;

fn refused_with(src: &str, codeunit: &str, proc: &str, want: &str) {
    match super::report_management_stand_in::built(src).run_in(codeunit, proc) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(want), "{why}"),
        other => panic!("expected a refusal naming {want}, got {other:?}"),
    }
}

#[test]
fn report_pdf_placeholder_a_report_layout_selection_of_a_custom_layout_with_no_code_writes_a_pdf() {
    // Base Application's ReportManagement answers SelectReportLayoutCode with nothing for a Custom Layout row whose code is
    // empty, and SelectedBuiltinLayoutType with 0 (`report_management_stand_in`): no selection, the RDLC layout's placeholder.
    // Main read the row itself and refused.
    let src = format!("{SRC}{SELECTION}");
    assert_eq!(
        al_testkit::text(
            super::report_management_stand_in::built(&src).run_in("RPP Selection Tests", "CustomSelection")
        ),
        "Yes"
    );
}

#[test]
fn report_pdf_placeholder_a_report_layout_selection_of_the_builtin_rdlc_layout_writes_a_pdf() {
    let src = format!("{SRC}{SELECTION}");
    assert_eq!(
        al_testkit::text(
            super::report_management_stand_in::built(&src).run_in("RPP Selection Tests", "BuiltinSelection")
        ),
        "Yes"
    );
}

#[test]
fn report_pdf_placeholder_a_design_time_layout_selection_is_refused() {
    let src = format!("{SRC}{SELECTION}");
    refused_with(
        &src,
        "RPP Selection Tests",
        "DesignTimeSelection",
        "layout selection = a design-time selection",
    );
}

#[test]
fn report_pdf_placeholder_html_of_an_rdlc_report_while_a_design_time_selection_is_held_is_refused() {
    let src = format!("{SRC}{SELECTION}");
    refused_with(
        &src,
        "RPP Selection Tests",
        "DesignTimeSelectionHtml",
        "layout selection = a design-time selection",
    );
}

fn run(src: &str, proc: &str) -> Result<Value, Signal> {
    Built::from_source(src).run_in("RPP Tests", proc)
}

#[test]
fn report_pdf_placeholder_save_as_pdf_to_a_stream_runs_the_report_and_writes_a_pdf() {
    assumptions::clear_hits();
    assert_eq!(
        al_testkit::text(run(SRC, "ToStream")),
        "Yes|init;pre;row1;row2;post;|Yes|%PDF-"
    );
    assert!(assumptions::take_hits().contains(&Assumption::ReportPdfPlaceholder));
}

#[test]
fn report_pdf_placeholder_the_variable_form_writes_a_pdf_too() {
    assert_eq!(
        al_testkit::text(run(SRC, "ToStreamOnAVariable")),
        "Yes|init;pre;row1;row2;post;|%PDF-"
    );
}

#[test]
fn report_pdf_placeholder_save_as_pdf_to_a_file_writes_a_file_that_exists_and_is_not_empty() {
    assert_eq!(
        al_testkit::text(run(SRC, "ToFile")),
        "Yes|init;pre;row1;row2;post;|Yes|Yes"
    );
    assert_eq!(
        al_testkit::text(run(SRC, "ToFileOnAVariable")),
        "Yes|init;pre;row1;row2;post;|Yes"
    );
}

#[test]
fn report_pdf_placeholder_a_word_layout_writes_a_pdf() {
    assert_eq!(al_testkit::text(run(SRC, "WordLayout")), "Yes|row1;row2;|%PDF-");
}

#[test]
fn report_pdf_placeholder_an_excel_layout_keeps_the_refusal() {
    match run(SRC, "ExcelLayout") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("as Pdf"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// An RDLC layout saved as Word writes the Word placeholder (CiReportRunLeads.al:674-692 SaveAs_RdlcOnly_WordExcel; ADR-040
// amendment 2026-11-11, `report_save`).
#[test]
fn report_pdf_placeholder_an_rdlc_layout_saved_as_word_writes_the_word_placeholder() {
    assert!(run(SRC, "WordFormat").is_ok());
}

// reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:372-388: Pdf of a processing-only report answers false
// with this text, runs no trigger and writes nothing (also reports/runtime/LtTkShowPrintStatus.al:621 Proc_T_SaveAsPdf).
#[test]
fn report_pdf_placeholder_a_processing_only_report_keeps_the_pinned_answer() {
    assert_eq!(
        al_testkit::text(run(SRC, "ProcessingOnly")),
        "No|Report 50958 is a processing-only report. Output cannot be rendered and saved to a file.|"
    );
}

#[test]
fn report_pdf_placeholder_an_error_of_a_trigger_reaches_the_caller_as_on_the_xml_path() {
    assert_eq!(al_testkit::text(run(SRC, "TriggerError")), "stop in pre");
}

#[test]
fn report_pdf_placeholder_a_subscriber_to_a_layout_event_is_refused() {
    let src = format!("{SRC}{SUBSCRIBER}");
    match run(&src, "ToStream") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("SelectReportLayoutCode"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn report_pdf_placeholder_the_bytes_are_a_valid_pdf_with_one_page() {
    let bytes = al_vm::report_placeholder::placeholder_pdf();
    let text = String::from_utf8(bytes.clone()).expect("ascii");
    assert!(text.starts_with("%PDF-1.4\n"));
    assert!(text.ends_with("%%EOF\n"));
    assert_eq!(text.matches("/Type /Page ").count(), 1);
    // Every offset in the cross-reference table points at its object.
    let xref = text.find("xref\n").expect("xref");
    let entries: Vec<usize> = text[xref..]
        .lines()
        .skip(3)
        .take(3)
        .map(|l| l[..10].parse().expect("offset"))
        .collect();
    for (i, at) in entries.iter().enumerate() {
        assert!(text[*at..].starts_with(&format!("{} 0 obj", i + 1)), "object {}", i + 1);
    }
    let start: usize = text
        .lines()
        .rev()
        .nth(1)
        .expect("startxref value")
        .parse()
        .expect("number");
    assert_eq!(start, xref);
}
