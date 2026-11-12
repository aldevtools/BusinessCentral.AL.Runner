//! ADR-040 amendment (owner, 2026-11-11): placeholder Word, Excel and Html outputs, and a run without output.
//!
//! The corpus pins what BC answers (reports/layouts-saveas/AMReportOutputTests.al, reports/CiReportRunLeads.al,
//! reports/runtime/LtTkShowPrintStatus.al); these tests cover what the corpus cannot reach: that the placeholder's content
//! stays refused on every read path while presence, size and byte copies answer, the refusal plumbing (subscribers, layout
//! selections), and the assumption marks.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const SRC: &str = r#"
dotnet { assembly("netstandard") {
    type("System.IO.Compression.GZipStream"; GZipStream) { }
    type("System.IO.Compression.CompressionMode"; CompressionMode) { }
} }

table 50970 "RPO Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } field(3; Other; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 50971 "RPO Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var Result: Text; begin Result := Entries; Entries := ''; exit(Result); end;
}

report 50972 "RPO Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset
    {
        dataitem(T; "RPO Row")
        {
            column(Id; Id) { }
            trigger OnAfterGetRecord() var L: Codeunit "RPO Log"; begin L.Add('row' + Format(T.Id)); end;
        }
    }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'RpoWord.docx'; } }
    trigger OnPreReport() var L: Codeunit "RPO Log"; begin L.Add('pre'); end;
    trigger OnPostReport() var L: Codeunit "RPO Log"; begin L.Add('post'); end;
}

report 50980 "RPO Word Printer"
{
    UsageCategory = None;
    UseSystemPrinter = true;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RPO Row") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'RpoWord.docx'; } }
}

report 50973 "RPO Excel"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = ExcelOne;
    dataset { dataitem(T; "RPO Row") { column(Id; Id) { } } }
    rendering { layout(ExcelOne) { Type = Excel; LayoutFile = 'RpoExcel.xlsx'; } }
    trigger OnPreReport() var L: Codeunit "RPO Log"; begin L.Add('excel pre'); end;
}

report 50974 "RPO Rdlc"
{
    UsageCategory = None;
    DefaultLayout = RDLC;
    RDLCLayout = 'RpoRdlc.rdl';
    dataset { dataitem(T; "RPO Row") { column(Id; Id) { } } }
}

codeunit 2000000024 Base64Convert
{
    [Native] procedure ToBase64(InStream: InStream; InsertLineBreaks: Boolean): Text begin end;
}

codeunit 50975 "RPO Tests"
{
    local procedure Seed()
    var R: Record "RPO Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
    end;

    // The placeholder of `Fmt` from the report `Id`, in the Data Blob of row 9 of a temporary record.
    local procedure Save(Id: Integer; Fmt: ReportFormat; var Tmp: Record "RPO Row" temporary): Boolean
    var Out: OutStream; Ok: Boolean;
    begin
        Seed();
        Tmp.Id := 9;
        Tmp.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(Id, '', Fmt, Out);
        Tmp.Insert();
        exit(Ok);
    end;

    local procedure Facts(Id: Integer; Fmt: ReportFormat): Text
    var Tmp: Record "RPO Row" temporary; L: Codeunit "RPO Log"; Ok: Boolean;
    begin
        ClearLastError();
        Ok := Save(Id, Fmt, Tmp);
        exit(Format(Ok) + '|' + L.Take() + '|' + Format(Tmp.Data.HasValue()) + '|' + Format(Tmp.Data.Length() > 0) + '|'
            + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure WordToWord(): Text begin exit(Facts(Report::"RPO Word", ReportFormat::Word)); end;
    procedure WordToHtml(): Text begin exit(Facts(Report::"RPO Word", ReportFormat::Html)); end;
    procedure WordToPdf(): Text begin exit(Facts(Report::"RPO Word", ReportFormat::Pdf)); end;
    procedure WordToExcel(): Text begin exit(Facts(Report::"RPO Word", ReportFormat::Excel)); end;
    procedure ExcelToExcel(): Text begin exit(Facts(Report::"RPO Excel", ReportFormat::Excel)); end;
    procedure ExcelToPdf(): Text begin exit(Facts(Report::"RPO Excel", ReportFormat::Pdf)); end;
    procedure ExcelToWord(): Text begin exit(Facts(Report::"RPO Excel", ReportFormat::Word)); end;
    procedure ExcelToHtml(): Text begin exit(Facts(Report::"RPO Excel", ReportFormat::Html)); end;
    procedure RdlcToWord(): Text begin exit(Facts(Report::"RPO Rdlc", ReportFormat::Word)); end;
    procedure RdlcToExcel(): Text begin exit(Facts(Report::"RPO Rdlc", ReportFormat::Excel)); end;

    procedure WordToExcelAsAStatement()
    var Tmp: Record "RPO Row" temporary; Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPO Word", '', ReportFormat::Excel, Out);
    end;

    procedure OnAVariable(): Text
    var Tmp: Record "RPO Row" temporary; Rep: Report "RPO Word"; Out: OutStream; Ok: Boolean;
    begin
        Seed();
        Tmp.Data.CreateOutStream(Out);
        Ok := Rep.SaveAs('', ReportFormat::Html, Out);
        exit(Format(Ok) + '|' + Format(Tmp.Data.Length() > 0));
    end;

    // ---- a read of the content is refused --------------------------------------------------------------------------

    local procedure HtmlStream(var Tmp: Record "RPO Row" temporary; var I: InStream)
    begin
        Save(Report::"RPO Word", ReportFormat::Html, Tmp);
        Tmp.Data.CreateInStream(I);
    end;

    procedure ReadText(): Text var Tmp: Record "RPO Row" temporary; I: InStream; Txt: Text; begin HtmlStream(Tmp, I); I.ReadText(Txt); exit(Txt); end;
    procedure ReadChar(): Text var Tmp: Record "RPO Row" temporary; I: InStream; C: Char; begin HtmlStream(Tmp, I); I.Read(C); exit(Format(C)); end;
    procedure ReadIntoText(): Text var Tmp: Record "RPO Row" temporary; I: InStream; Txt: Text; begin HtmlStream(Tmp, I); I.Read(Txt); exit(Txt); end;
    procedure Eos(): Boolean var Tmp: Record "RPO Row" temporary; I: InStream; begin HtmlStream(Tmp, I); exit(I.EOS()); end;
    procedure StreamLength(): BigInteger var Tmp: Record "RPO Row" temporary; I: InStream; begin HtmlStream(Tmp, I); exit(I.Length()); end;
    procedure CopyCount() var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; begin HtmlStream(Tmp, I); Tmp.Other.CreateOutStream(O); CopyStream(O, I, 3); end;
    procedure XmlReadFrom() var Tmp: Record "RPO Row" temporary; I: InStream; Doc: XmlDocument; begin HtmlStream(Tmp, I); XmlDocument.ReadFrom(I, Doc); end;
    procedure JsonReadFrom() var Tmp: Record "RPO Row" temporary; I: InStream; Obj: JsonObject; begin HtmlStream(Tmp, I); Obj.ReadFrom(I); end;
    procedure Base64(): Text var Tmp: Record "RPO Row" temporary; I: InStream; B: Codeunit Base64Convert; begin HtmlStream(Tmp, I); exit(B.ToBase64(I, false)); end;
    procedure BigTextRead(): Text var Tmp: Record "RPO Row" temporary; I: InStream; B: BigText; begin HtmlStream(Tmp, I); B.Read(I); exit(Format(B.Length())); end;
    procedure FormatOfBlob(): Text var Tmp: Record "RPO Row" temporary; begin Save(Report::"RPO Word", ReportFormat::Html, Tmp); exit(Format(Tmp.Data)); end;
    // Data Compression: System Application's GZipDecompress over the stream.
    procedure GZipDecompress()
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; Gz: DotNet GZipStream; Mode: DotNet CompressionMode;
    begin
        Save(Report::"RPO Excel", ReportFormat::Excel, Tmp);
        Tmp.Data.CreateInStream(I);
        Tmp.Other.CreateOutStream(O);
        Gz := Gz.GZipStream(I, Mode::Decompress);
        Gz.CopyTo(O);
    end;

    // A second OutStream on the Blob writes over the placeholder's first bytes and leaves the rest: still not BC's content.
    procedure ReadAfterASecondOutStream(): Text
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; Txt: Text;
    begin
        Save(Report::"RPO Word", ReportFormat::Html, Tmp);
        Tmp.Data.CreateOutStream(O);
        Tmp.Data.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;

    procedure ReadAfterAWriteOverIt(): Text
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; Txt: Text;
    begin
        Save(Report::"RPO Word", ReportFormat::Html, Tmp);
        Tmp.Data.CreateOutStream(O);
        O.WriteText('SIG');
        Tmp.Data.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;

    // ---- the size answers only as "greater than 0" -----------------------------------------------------------------

    procedure LengthAsANumber(): Text var Tmp: Record "RPO Row" temporary; begin Save(Report::"RPO Word", ReportFormat::Html, Tmp); exit(StrSubstNo('%1 bytes', Tmp.Data.Length())); end;
    procedure LengthOverAThousand(): Boolean var Tmp: Record "RPO Row" temporary; begin Save(Report::"RPO Word", ReportFormat::Html, Tmp); exit(Tmp.Data.Length() > 1000); end;
    procedure FormatOfLength(): Text var Tmp: Record "RPO Row" temporary; begin Save(Report::"RPO Word", ReportFormat::Html, Tmp); exit(Format(Tmp.Data.Length())); end;
    procedure LengthPlusOne(): Boolean var Tmp: Record "RPO Row" temporary; begin Save(Report::"RPO Word", ReportFormat::Html, Tmp); exit(Tmp.Data.Length() + 1 > 0); end;
    procedure TwoLengthsEqual(): Boolean
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream;
    begin
        HtmlStream(Tmp, I);
        Tmp.Other.CreateOutStream(O);
        CopyStream(O, I);
        exit(Tmp.Other.Length() = Tmp.Data.Length());
    end;
    procedure FileLenAsANumber(): Integer
    var F: File; Len: Integer; Twice: Integer;
    begin
        Seed();
        Report.SaveAsHtml(Report::"RPO Word", 'out/len.html');
        F.Open('out/len.html');
        Len := F.Len();
        F.Close();
        Twice := Len * 2;
        exit(Twice);
    end;

    // The size through a variable and a wrapper that returns it (as System Application's Temp Blob.Length() does).
    local procedure SizeOf(var Tmp: Record "RPO Row" temporary): Integer begin exit(Tmp.Data.Length()); end;
    procedure LengthThroughAWrapper(): Text
    var Tmp: Record "RPO Row" temporary; L: Integer;
    begin
        Save(Report::"RPO Word", ReportFormat::Html, Tmp);
        L := SizeOf(Tmp);
        exit(Format(SizeOf(Tmp) > 0) + '|' + Format(SizeOf(Tmp) = 0) + '|' + Format(0 < L) + '|' + Format(L <> 0));
    end;

    // ---- byte copies carry the content and its mark --------------------------------------------------------------

    procedure CopyToBlob(): Text
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; Ok: Boolean;
    begin
        HtmlStream(Tmp, I);
        Tmp.Other.CreateOutStream(O);
        Ok := CopyStream(O, I);
        exit(Format(Ok) + '|' + Format(Tmp.Other.HasValue()) + '|' + Format(Tmp.Other.Length() > 0));
    end;

    procedure ReadOfTheCopy(): Text
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; Txt: Text;
    begin
        HtmlStream(Tmp, I);
        Tmp.Other.CreateOutStream(O);
        CopyStream(O, I);
        Tmp.Other.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;

    // A stored Blob (Report Inbox, an e-mail attachment row): Modify, Get, CalcFields.
    procedure StoredBlob(): Text
    var R: Record "RPO Row"; Back: Record "RPO Row"; Out: OutStream;
    begin
        Seed();
        R.Id := 20;
        R.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPO Word", '', ReportFormat::Word, Out);
        R.Insert();
        Back.Get(20);
        Back.CalcFields(Data);
        exit(Format(Back.Data.HasValue()) + '|' + Format(Back.Data.Length() > 0));
    end;

    procedure ReadOfTheStoredBlob(): Text
    var R: Record "RPO Row"; Back: Record "RPO Row"; Out: OutStream; I: InStream; Txt: Text;
    begin
        Seed();
        R.Id := 20;
        R.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPO Word", '', ReportFormat::Word, Out);
        R.Insert();
        Back.Get(20);
        Back.CalcFields(Data);
        Back.Data.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;

    procedure RecordCopy(): Text
    var Tmp: Record "RPO Row" temporary; Copy: Record "RPO Row" temporary;
    begin
        Save(Report::"RPO Word", ReportFormat::Html, Tmp);
        Copy := Tmp;
        Copy.Other := Tmp.Data;
        exit(Format(Copy.Data.HasValue()) + '|' + Format(Copy.Other.Length() > 0));
    end;

    procedure CopyToFile(): Text
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; F: File; Len: Integer;
    begin
        HtmlStream(Tmp, I);
        F.Create('out/mail.html');
        F.CreateOutStream(O);
        CopyStream(O, I);
        F.Close();
        F.Open('out/mail.html');
        Len := F.Len();
        F.Close();
        exit(Format(Exists('out/mail.html')) + '|' + Format(Len > 0));
    end;

    procedure ReadOfTheFileCopy(): Text
    var Tmp: Record "RPO Row" temporary; I: InStream; O: OutStream; F: File; Txt: Text;
    begin
        HtmlStream(Tmp, I);
        F.Create('out/mail.html');
        F.CreateOutStream(O);
        CopyStream(O, I);
        F.Close();
        F.Open('out/mail.html');
        F.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;

    // ---- a file target -----------------------------------------------------------------------------------------

    procedure ToFile(): Text
    var F: File; L: Codeunit "RPO Log"; Ok: Boolean; Len: Integer; I: InStream; Tmp: Record "RPO Row" temporary; O: OutStream;
    begin
        Seed();
        Ok := Report.SaveAsHtml(Report::"RPO Word", 'out/body.html');
        F.Open('out/body.html');
        Len := F.Len();
        // A copy of the file into a Blob, as FileManagement.BLOBImportFromServerFile does.
        F.CreateInStream(I);
        Tmp.Data.CreateOutStream(O);
        CopyStream(O, I);
        F.Close();
        exit(Format(Ok) + '|' + L.Take() + '|' + Format(Exists('out/body.html')) + '|' + Format(Len > 0) + '|'
            + Format(Tmp.Data.Length() > 0) + '|' + Format(Erase('out/body.html')) + '|' + Format(Exists('out/body.html')));
    end;

    procedure ReadOfTheFile(): Text
    var F: File; I: InStream; Txt: Text;
    begin
        Seed();
        Report.SaveAsWord(Report::"RPO Word", 'out/doc.docx');
        F.Open('out/doc.docx');
        F.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;

    procedure FileOverwrittenByAl(): Text
    var F: File; I: InStream; Txt: Text;
    begin
        Seed();
        Report.SaveAsWord(Report::"RPO Word", 'out/doc.docx');
        F.TextMode(true);
        F.Create('out/doc.docx');
        F.Write('mine');
        F.Close();
        F.Open('out/doc.docx');
        F.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;

    // ---- Run, RunModal and Execute ---------------------------------------------------------------------------------

    procedure RunWord(): Text
    var L: Codeunit "RPO Log";
    begin
        Seed();
        Report.Run(Report::"RPO Word", false);
        exit(L.Take());
    end;

    procedure RunModalWordOnAVariable(): Text
    var L: Codeunit "RPO Log"; Rep: Report "RPO Word";
    begin
        Seed();
        Rep.UseRequestPage(false);
        Rep.RunModal();
        exit(L.Take());
    end;

    procedure RunWordOnTheSystemPrinter() begin Seed(); Report.Run(Report::"RPO Word", false, true); end;
    procedure RunModalWordOnTheSystemPrinter() begin Seed(); Report.RunModal(Report::"RPO Word", false, true); end;
    procedure RunWordThatUsesTheSystemPrinter() begin Seed(); Report.Run(Report::"RPO Word Printer", false); end;

    procedure RunExcel(): Text
    var L: Codeunit "RPO Log";
    begin
        Seed();
        asserterror Report.Run(Report::"RPO Excel");
        exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + L.Take());
    end;
}
"#;

const SUBSCRIBER: &str = r#"
codeunit 50976 "RPO Layout Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'OnDocumentReady', '', false, false)]
    local procedure OnReady(ObjectId: Integer; ObjectPayload: JsonObject; DocumentStream: InStream; var TargetStream: OutStream; var Success: Boolean)
    begin
    end;
}
"#;

const SELECTION: &str = r#"
table 9651 "Report Layout Selection"
{
    fields
    {
        field(1; "Report ID"; Integer) { }
        field(3; "Company Name"; Text[30]) { }
        field(4; Type; Option) { OptionMembers = "RDLC (built-in)","Word (built-in)","Custom Layout","Excel Layout","External Layout"; }
    }
    keys { key(PK; "Report ID", "Company Name") { } }
}

codeunit 50977 "RPO Selection Tests"
{
    local procedure Select(Id: Integer; T: Option)
    var S: Record "Report Layout Selection";
    begin
        S."Report ID" := Id; S."Company Name" := CompanyName(); S.Type := T; S.Insert();
    end;

    procedure HtmlWithSelection(): Boolean
    var S: Record "Report Layout Selection"; Tmp: Record "RPO Row" temporary; Out: OutStream;
    begin
        Select(Report::"RPO Word", S.Type::"Custom Layout");
        Tmp.Data.CreateOutStream(Out);
        exit(Report.SaveAs(Report::"RPO Word", '', ReportFormat::Html, Out));
    end;

    procedure RunExcelWithSelection()
    var S: Record "Report Layout Selection";
    begin
        Select(Report::"RPO Excel", S.Type::"Word (built-in)");
        Report.Run(Report::"RPO Excel", false);
    end;

    procedure RunWithSelection()
    var S: Record "Report Layout Selection";
    begin
        Select(Report::"RPO Word", S.Type::"Excel Layout");
        Report.Run(Report::"RPO Word", false);
    end;
}
"#;

// An e-mail attachment: System Application's Email Message Impl. InsertAttachment imports the stream into the Media field
// of "Email Message Attachment", reads the Tenant Media row and stores Content.Length (EmailMessageImpl.Codeunit.al).
const ATTACHMENT: &str = r#"
table 2000000184 "Tenant Media"
{
    fields
    {
        field(1; ID; Guid) { } field(2; Description; Text[250]) { } field(3; Content; Blob) { }
        field(4; "Mime Type"; Text[100]) { } field(5; Height; Integer) { } field(6; Width; Integer) { }
    }
    keys { key(PK; ID) { } }
}

table 50978 "RPO Attachment"
{
    fields { field(1; Id; Integer) { } field(2; Data; Media) { } field(3; Length; Integer) { } }
    keys { key(PK; Id) { } }
}

codeunit 50979 "RPO Attachment Tests"
{
    local procedure Attach(var A: Record "RPO Attachment")
    var Tmp: Record "RPO Row" temporary; Out: OutStream; I: InStream; TenantMedia: Record "Tenant Media"; MediaId: Guid;
    begin
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RPO Word", '', ReportFormat::Html, Out);
        Tmp.Data.CreateInStream(I);
        A.Id := 1;
        MediaId := A.Data.ImportStream(I, 'body.html', 'text/html');
        TenantMedia.Get(MediaId);
        TenantMedia.CalcFields(Content);
        A.Insert();
    end;

    procedure AttachmentLength(): Boolean
    var A: Record "RPO Attachment"; TenantMedia: Record "Tenant Media";
    begin
        Attach(A);
        TenantMedia.Get(A.Data.MediaId());
        TenantMedia.CalcFields(Content);
        A.Length := TenantMedia.Content.Length;
        exit(A.Length > 0);
    end;

    procedure ReadOfTheAttachment(): Text
    var A: Record "RPO Attachment"; Tmp: Record "RPO Row" temporary; Out: OutStream; I: InStream; Txt: Text;
    begin
        Attach(A);
        Tmp.Data.CreateOutStream(Out);
        A.Data.ExportStream(Out);
        Tmp.Data.CreateInStream(I);
        I.ReadText(Txt);
        exit(Txt);
    end;
}
"#;

fn run_in(src: &str, codeunit: &str, proc: &str) -> Result<Value, Signal> {
    Built::from_source(src).run_in(codeunit, proc)
}

fn run(proc: &str) -> Result<Value, Signal> {
    run_in(SRC, "RPO Tests", proc)
}

fn text(proc: &str) -> String {
    al_testkit::text(run(proc))
}

#[track_caller]
fn refused(result: Result<Value, Signal>, want: &str) {
    match result {
        Err(Signal::Unsupported(why)) => assert!(why.contains(want), "{why}"),
        other => panic!("expected a refusal naming {want:?}, got {other:?}"),
    }
}

const CONTENT: &str = "a read of the content of a placeholder";

// ---- the grid -----------------------------------------------------------------------------------------------------

#[test]
fn report_placeholder_a_word_layout_to_word_and_html_writes_a_placeholder_and_marks_its_assumption() {
    assumptions::clear_hits();
    assert_eq!(text("WordToWord"), "Yes|pre;row1;row2;post;|Yes|Yes||");
    assert!(assumptions::take_hits().contains(&Assumption::ReportWordPlaceholder));
    assert_eq!(text("WordToHtml"), "Yes|pre;row1;row2;post;|Yes|Yes||");
    assert!(assumptions::take_hits().contains(&Assumption::ReportHtmlPlaceholder));
    assert_eq!(text("WordToPdf"), "Yes|pre;row1;row2;post;|Yes|Yes||");
    assert_eq!(text("OnAVariable"), "Yes|Yes");
}

#[test]
fn report_placeholder_an_excel_layout_to_excel_writes_a_placeholder() {
    assumptions::clear_hits();
    assert_eq!(text("ExcelToExcel"), "Yes|excel pre;|Yes|Yes||");
    assert!(assumptions::take_hits().contains(&Assumption::ReportExcelPlaceholder));
}

// AMReportOutputTests.al:221, :257; CiReportRunLeads.al:601-605, :657-666, :267-270.
#[test]
fn report_placeholder_a_format_the_layout_cannot_give_answers_false_and_runs_nothing() {
    let answer = |f: &str| {
        format!(
            "No||No|No|The {f} output format is not supported by the current report layout format.|ReportUnsupportedFormat"
        )
    };
    assert_eq!(text("WordToExcel"), answer("Excel"));
    assert_eq!(text("ExcelToPdf"), answer("Pdf"));
    assert_eq!(text("ExcelToWord"), answer("Word"));
    assert_eq!(text("ExcelToHtml"), answer("Html"));
    refused(run("WordToExcelAsAStatement"), "as a statement");
}

// CiReportRunLeads.al:674-692 SaveAs_RdlcOnly_WordExcel: BC renders an RDLC layout as Word and as Excel.
#[test]
fn report_placeholder_an_rdlc_layout_to_word_or_excel_writes_a_placeholder() {
    assumptions::clear_hits();
    assert_eq!(text("RdlcToWord"), "Yes||Yes|Yes||");
    assert!(assumptions::take_hits().contains(&Assumption::ReportWordPlaceholder));
    assert_eq!(text("RdlcToExcel"), "Yes||Yes|Yes||");
    assert!(assumptions::take_hits().contains(&Assumption::ReportExcelPlaceholder));
}

#[test]
fn report_placeholder_the_files_are_of_the_right_kind() {
    use al_vm::report_placeholder::Format;
    assert!(Format::Word.bytes().starts_with(b"PK"));
    assert!(Format::Excel.bytes().starts_with(b"PK"));
    assert!(Format::Html.bytes().starts_with(b"<"));
    assert!(Format::Pdf.bytes().starts_with(b"%PDF-"));
}

// ---- the content stays refused ----------------------------------------------------------------------------------------

#[test]
fn report_placeholder_every_read_of_the_content_is_refused() {
    for p in [
        "ReadText",
        "ReadChar",
        "ReadIntoText",
        "Eos",
        "StreamLength",
        "CopyCount",
        "XmlReadFrom",
        "JsonReadFrom",
        "Base64",
        "BigTextRead",
        "GZipDecompress",
        "ReadOfTheCopy",
        "ReadOfTheStoredBlob",
        "ReadOfTheFileCopy",
        "ReadOfTheFile",
        "ReadAfterASecondOutStream",
        "ReadAfterAWriteOverIt",
    ] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains(CONTENT), "{p}: {why}"),
            other => panic!("{p}: expected the read to be refused, got {other:?}"),
        }
    }
}

#[test]
fn report_placeholder_presence_size_and_byte_copies_answer() {
    assert_eq!(text("CopyToBlob"), "Yes|Yes|Yes");
    assert_eq!(text("StoredBlob"), "Yes|Yes");
    assert_eq!(text("RecordCopy"), "Yes|Yes");
    assert_eq!(text("CopyToFile"), "Yes|Yes");
    assert_eq!(text("FormatOfBlob"), "*");
}

// ADR-040 amendment 2026-11-12, A4: the size answers only a comparison with 0 (BC's rendering is not empty); a number would be
// the placeholder's, and `Length > 1000` is true on BC.
const SIZE: &str = "the byte size of a placeholder";

#[test]
fn report_placeholder_the_size_answers_only_against_0() {
    assert_eq!(text("LengthThroughAWrapper"), "Yes|No|Yes|Yes");
    for p in [
        "LengthAsANumber",
        "LengthOverAThousand",
        "FormatOfLength",
        "LengthPlusOne",
        "TwoLengthsEqual",
        "FileLenAsANumber",
    ] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains(SIZE), "{p}: {why}"),
            other => panic!("{p}: expected the size to be refused, got {other:?}"),
        }
    }
}

// Email Message Impl. stores the attachment's Content.Length in a field: the number is refused.
#[test]
fn report_placeholder_an_e_mail_attachment_takes_the_bytes_and_keeps_the_content_refused() {
    let src = format!("{SRC}{ATTACHMENT}");
    refused(run_in(&src, "RPO Attachment Tests", "AttachmentLength"), SIZE);
    refused(run_in(&src, "RPO Attachment Tests", "ReadOfTheAttachment"), CONTENT);
}

#[test]
fn report_placeholder_a_file_target_exists_has_a_size_copies_and_erases() {
    assert_eq!(text("ToFile"), "Yes|pre;row1;row2;post;|Yes|Yes|Yes|Yes|No");
}

#[test]
fn report_placeholder_a_file_al_writes_again_is_readable() {
    assert_eq!(text("FileOverwrittenByAl"), "mine");
}

// ---- refusal plumbing -------------------------------------------------------------------------------------------------

#[test]
fn report_placeholder_a_layout_event_subscriber_refuses_the_save_and_the_run() {
    let src = format!("{SRC}{SUBSCRIBER}");
    refused(run_in(&src, "RPO Tests", "WordToHtml"), "OnDocumentReady");
    refused(run_in(&src, "RPO Tests", "RunWord"), "OnDocumentReady");
    refused(run_in(&src, "RPO Tests", "RunExcel"), "OnDocumentReady");
}

#[test]
fn report_placeholder_a_layout_selection_refuses_the_save_and_the_run() {
    let src = format!("{SRC}{SELECTION}");
    refused(
        run_in(&src, "RPO Selection Tests", "HtmlWithSelection"),
        "Report Layout Selection",
    );
    refused(
        run_in(&src, "RPO Selection Tests", "RunWithSelection"),
        "Report Layout Selection",
    );
    refused(
        run_in(&src, "RPO Selection Tests", "RunExcelWithSelection"),
        "Report Layout Selection",
    );
}

// ADR-040 amendment 2026-11-11, "What stays refused": printing with the system printer.
#[test]
fn report_placeholder_a_run_on_the_system_printer_is_refused() {
    for p in [
        "RunWordOnTheSystemPrinter",
        "RunModalWordOnTheSystemPrinter",
        "RunWordThatUsesTheSystemPrinter",
    ] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("system printer"), "{p}: {why}"),
            other => panic!("{p}: expected the run to be refused, got {other:?}"),
        }
    }
}

// ---- Run, RunModal and Execute ---------------------------------------------------------------------------------------

#[test]
fn report_placeholder_a_run_of_a_word_layout_report_runs_and_writes_nothing() {
    assumptions::clear_hits();
    assert_eq!(text("RunWord"), "pre;row1;row2;post;");
    assert!(assumptions::take_hits().contains(&Assumption::ReportRunWithoutOutput));
    assert_eq!(text("RunModalWordOnAVariable"), "pre;row1;row2;post;");
}

// CiReportRunLeads.al:245-248: no trigger runs.
#[test]
fn report_placeholder_a_run_of_an_excel_layout_report_raises_the_pinned_error() {
    assert_eq!(
        text("RunExcel"),
        "The current report operation is not supported for Excel Layout reports.|ReportUnsupportedOperation|"
    );
}
