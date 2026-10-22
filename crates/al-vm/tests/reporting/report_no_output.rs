//! ADR-040: a report run without rendered output. A rendering save of a report on the allow-list ("Post Inventory
//! Cost to G/L", "Check" of Base Application) runs the report as the Xml path does, writes nothing and marks the
//! target; the moment AL looks at the target the run is refused (`Signal::Unsupported`). One test per observation
//! path of the ADR (F1 to F11, S1 to S7), the controls (not an observation), the flag propagation, and the
//! boundaries (a report that is not listed, the render-stage errors).
//!
//! The fixture is one app with the id of Base Application, so the listed reports match by app id, object id and
//! name. The AL helpers do what the Microsoft codeunits do with the builtins (File Management, Temp Blob, Report
//! Inbox), because those codeunits are AL over the same builtins.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::{Built, compile_apps};

const BASE_APPLICATION: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

fn fixture(report_1002_extra: &str) -> String {
    format!(
        r#"
table 50900 "RNO Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} field(3; Other; Blob) {{ }} }}
    keys {{ key(PK; Id) {{ }} }}
}}

codeunit 50901 "RNO Log"
{{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var Result: Text; begin Result := Entries; Entries := ''; exit(Result); end;
}}

report 1002 "Post Inventory Cost to G/L"
{{
    UsageCategory = None;
    {report_1002_extra}
    dataset
    {{
        dataitem(T; "RNO Row")
        {{
            trigger OnAfterGetRecord() var L: Codeunit "RNO Log"; begin L.Add('row' + Format(T.Id)); end;
        }}
    }}
    trigger OnInitReport() var L: Codeunit "RNO Log"; begin L.Add('init'); end;
    trigger OnPreReport() var L: Codeunit "RNO Log"; begin L.Add('pre'); end;
    trigger OnPostReport() var L: Codeunit "RNO Log"; begin L.Add('post'); end;
}}

report 50904 "RNO Logged"
{{
    UsageCategory = None;
    dataset
    {{
        dataitem(T; "RNO Row")
        {{
            trigger OnAfterGetRecord() var L: Codeunit "RNO Log"; begin L.Add('row' + Format(T.Id)); end;
        }}
    }}
    trigger OnPreReport() var L: Codeunit "RNO Log"; begin L.Add('pre'); end;
    trigger OnPostReport() var L: Codeunit "RNO Log"; begin L.Add('post'); end;
}}

report 50902 "RNO Not Listed"
{{
    UsageCategory = None;
    dataset {{ dataitem(T; "RNO Row") {{ }} }}
}}

codeunit 50903 "RNO Tests"
{{
    local procedure Seed()
    var R: Record "RNO Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
    end;

    local procedure ToFile(Name: Text)
    begin
        Report.SaveAsPdf(Report::"Post Inventory Cost to G/L", Name);
    end;

    local procedure ToStream(var R: Record "RNO Row")
    var Out: OutStream;
    begin
        R.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"Post Inventory Cost to G/L", '', ReportFormat::Pdf, Out);
    end;

    // ---- what the save does -------------------------------------------------------------------------------

    procedure RunsToAFile(): Text
    var L: Codeunit "RNO Log"; Ok: Boolean;
    begin
        Seed();
        Ok := Report.SaveAsPdf(Report::"Post Inventory Cost to G/L", 'out/a.pdf');
        exit(Format(Ok) + '|' + L.Take());
    end;

    procedure RunsToAStream(): Text
    var L: Codeunit "RNO Log"; R: Record "RNO Row"; Out: OutStream; Ok: Boolean;
    begin
        Seed();
        R.Data.CreateOutStream(Out);
        Ok := Report.SaveAs(Report::"Post Inventory Cost to G/L", '', ReportFormat::Pdf, Out);
        exit(Format(Ok) + '|' + L.Take());
    end;

    procedure RunsAnInstance(): Text
    var L: Codeunit "RNO Log"; Rep: Report "Post Inventory Cost to G/L"; Ok: Boolean;
    begin
        Seed();
        Ok := Rep.SaveAsExcel('out/a.xlsx');
        exit(Format(Ok) + '|' + L.Take());
    end;

    procedure WordAndHtmlRun(): Text
    var L: Codeunit "RNO Log"; A: Boolean; B: Boolean;
    begin
        Seed();
        A := Report.SaveAsWord(Report::"Post Inventory Cost to G/L", 'out/a.docx');
        B := Report.SaveAsHtml(Report::"Post Inventory Cost to G/L", 'out/a.html');
        exit(Format(A) + Format(B) + '|' + L.Take());
    end;

    procedure LayoutError(): Text
    var Ok: Boolean;
    begin
        Seed();
        Ok := Report.SaveAsPdf(Report::"Post Inventory Cost to G/L", 'out/a.pdf');
        exit(Format(Ok) + '|' + GetLastErrorText());
    end;

    procedure HtmlError(): Text
    var Ok: Boolean;
    begin
        Seed();
        Ok := Report.SaveAsHtml(Report::"Post Inventory Cost to G/L", 'out/a.html');
        exit(Format(Ok) + '|' + GetLastErrorText());
    end;

    // The DemoTool's company build (owner decision 2026-10-22): SaveAsPdf of a report variable to a file.
    procedure CompanyBuildPdf(): Text
    var L: Codeunit "RNO Log"; Rep: Report "RNO Logged";
    begin
        Seed();
        Rep.UseRequestPage(false);
        Rep.SaveAsPdf('out/c.pdf');
        exit(L.Take());
    end;
    procedure CompanyBuildPdfThenOpen() var F: File; Rep: Report "RNO Logged"; begin Seed(); Rep.SaveAsPdf('out/c.pdf'); F.Open('out/c.pdf'); end;
    procedure CompanyBuildWord() var Rep: Report "RNO Logged"; begin Seed(); Rep.SaveAsWord('out/c.docx'); end;
    procedure CompanyBuildPdfStream() var R: Record "RNO Row"; Out: OutStream; begin Seed(); R.Data.CreateOutStream(Out); Report.SaveAs(Report::"RNO Logged", '', ReportFormat::Pdf, Out); end;

    procedure NotListedStaysRefused()
    begin
        Seed();
        Report.SaveAsPdf(Report::"RNO Not Listed", 'out/b.pdf');
    end;

    procedure NotListedStreamStaysRefused()
    var R: Record "RNO Row"; Out: OutStream;
    begin
        Seed();
        R.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RNO Not Listed", '', ReportFormat::Pdf, Out);
    end;

    // ---- F1 to F11: a file name target ----------------------------------------------------------------------

    procedure F1Open() var F: File; begin Seed(); ToFile('out/a.pdf'); F.Open('out/a.pdf'); end;
    procedure F1CreateInStream() var F: File; I: InStream; begin Seed(); ToFile('out/a.pdf'); F.TextMode(true); F.Open('out/a.pdf'); F.CreateInStream(I); end;
    procedure F2Exists(): Boolean begin Seed(); ToFile('out/a.pdf'); exit(File.Exists('out/a.pdf')); end;
    procedure F3Erase() begin Seed(); ToFile('out/a.pdf'); File.Erase('out/a.pdf'); end;
    procedure F4Copy() begin Seed(); ToFile('out/a.pdf'); File.Copy('out/a.pdf', 'out/b.pdf'); end;
    procedure F4CopyOnto() var F: File; begin Seed(); F.Create('x/src.pdf'); F.Close(); ToFile('out/a.pdf'); File.Copy('x/src.pdf', 'out/a.pdf'); end;
    procedure F4Rename() begin Seed(); ToFile('out/a.pdf'); File.Rename('out/a.pdf', 'out/b.pdf'); end;
    // FileManagement.ServerFileExists, ClientFileExists, GetFileSize: Exists, or Open and Len.
    procedure F5ServerFileExists(): Boolean begin Seed(); ToFile('out/a.pdf'); exit(File.Exists('out/a.pdf')); end;
    procedure F5GetFileSize(): Integer var F: File; begin Seed(); ToFile('out/a.pdf'); F.Open('out/a.pdf'); exit(F.Len()); end;
    procedure F6Download() var Target: Text; begin Seed(); ToFile('out/a.pdf'); File.Download('out/a.pdf', 'Save', '', '', Target); end;
    // FileManagement.DeleteServerDirectory and ServerDirectoryExists go through System.IO.Directory, which the .NET policy
    // denies, so the folder of a pending file cannot be asked about at all.
    procedure F7FolderExists() var Dir: DotNet Directory; begin Seed(); ToFile('out/a.pdf'); Dir.Exists('out'); end;
    // FileManagement.GetFileContents / BLOBImportFromServerFile: open, then read the stream.
    procedure F8GetFileContents(): Text var F: File; I: InStream; Content: Text; begin Seed(); ToFile('out/a.pdf'); F.Open('out/a.pdf'); F.CreateInStream(I); I.ReadText(Content); exit(Content); end;
    // Excel Buffer OpenBook, LibraryReportValidation.OpenFile: open the file as a stream.
    procedure F9OpenBook() var F: File; I: InStream; begin Seed(); ToFile('out/a.xlsx'); F.Open('out/a.xlsx'); F.CreateInStream(I); end;
    // CheckFileNotEmpty: open and ask for the length.
    procedure F10CheckFileNotEmpty(): Boolean var F: File; begin Seed(); ToFile('out/a.pdf'); F.Open('out/a.pdf'); exit(F.Len() > 0); end;
    // Email.AddAttachment from a path: read the file into a stream and copy it.
    procedure F11Attach() var F: File; I: InStream; Tmp: Record "RNO Row"; O: OutStream; begin Seed(); ToFile('out/a.pdf'); F.Open('out/a.pdf'); F.CreateInStream(I); Tmp.Data.CreateOutStream(O); CopyStream(O, I); end;

    // ---- S1 to S7: a stream or blob target -------------------------------------------------------------------

    procedure S1ReadText(): Text var R: Record "RNO Row"; I: InStream; Line: Text; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); I.ReadText(Line); exit(Line); end;
    procedure S1EOS(): Boolean var R: Record "RNO Row"; I: InStream; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); exit(I.EOS()); end;
    procedure S1Length(): BigInteger var R: Record "RNO Row"; I: InStream; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); exit(I.Length()); end;
    procedure S1Position(): BigInteger var R: Record "RNO Row"; I: InStream; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); exit(I.Position()); end;
    procedure S1ResetPosition(): Boolean var R: Record "RNO Row"; I: InStream; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); exit(I.ResetPosition()); end;
    procedure S1CopyStreamSource() var R: Record "RNO Row"; I: InStream; O: OutStream; Other: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); Other.Id := 11; Other.Insert(); Other.Data.CreateOutStream(O); CopyStream(O, I); end;
    procedure S2HasValue(): Boolean var R: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); exit(R.Data.HasValue()); end;
    procedure S2Length(): Integer var R: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); exit(R.Data.Length()); end;
    procedure S2CreateInStream() var R: Record "RNO Row"; I: InStream; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); R.Data.CreateInStream(I); end;
    // Temp Blob: Length, HasValue, CreateInStream are these three on a Blob field of a temporary record.
    procedure S2TempBlobLength(): Integer var Tmp: Record "RNO Row" temporary; begin Tmp.Id := 1; Tmp.Insert(); ToStream(Tmp); exit(Tmp.Data.Length()); end;
    // Base64 Convert.ToBase64, Temp Blob.ToText, Cryptography Management over the stream: CreateInStream, then read.
    procedure S3ToText(): Text var R: Record "RNO Row"; I: InStream; Content: Text; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); R.Data.CreateInStream(I); I.ReadText(Content); exit(Content); end;
    procedure S4Assign() var R: Record "RNO Row"; Copy: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); Copy := R; Copy.Data.Length(); end;
    procedure S4InsertGet() var R: Record "RNO Row"; Back: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); R.Modify(); Back.Get(10); Back.CalcFields(Data); Back.Data.HasValue(); end;
    procedure S4TransferFields() var R: Record "RNO Row"; Other: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); Other.TransferFields(R); Other.Data.HasValue(); end;
    procedure S4CalcFields() var R: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); R.Modify(); R.Get(10); R.CalcFields(Data); R.Data.Length(); end;
    procedure S4BlobToBlob() var R: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); R.Other := R.Data; R.Other.HasValue(); end;
    procedure S4FormatOfTheBlob(): Text var R: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); exit(Format(R.Data)); end;
    procedure S5FileCopyStream() var R: Record "RNO Row"; I: InStream; F: File; O: OutStream; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); F.Create('out/c.bin'); F.CreateOutStream(O); CopyStream(O, I); end;
    procedure S5DownloadFromStream() var R: Record "RNO Row"; I: InStream; Target: Text; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); File.DownloadFromStream(I, 'Save', '', '', Target); end;
    procedure S5HttpBody() var R: Record "RNO Row"; I: InStream; Content: HttpContent; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); Content.WriteFrom(I); end;
    procedure S6XmlReadFrom() var R: Record "RNO Row"; I: InStream; Doc: XmlDocument; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); XmlDocument.ReadFrom(I, Doc); end;
    procedure S6JsonReadFrom() var R: Record "RNO Row"; I: InStream; Obj: JsonObject; begin Seed(); R.Id := 10; R.Insert(); R.Data.CreateInStream(I); ToStream(R); Obj.ReadFrom(I); end;
    // Report Inbox / Job Queue Log: the report writes its output to a Blob field of a stored row, which is read later.
    procedure S7ReportInbox(): Boolean var Inbox: Record "RNO Row"; Back: Record "RNO Row"; begin Seed(); Inbox.Id := 77; Inbox.Insert(); ToStream(Inbox); Inbox.Modify(); Back.Get(77); Back.CalcFields(Data); exit(Back.Data.HasValue()); end;

    // ---- the controls: not an observation ---------------------------------------------------------------------

    procedure ControlPathAsText(): Text var Path: Text; begin Seed(); Path := 'out/a.pdf'; ToFile(Path); exit(Path + '|' + Format(Path) + '|' + StrSubstNo('%1 saved', Path)); end;
    procedure ControlPureStringFunctions(): Text var Path: Text; begin Seed(); Path := 'out/a.pdf'; ToFile(Path); exit(CopyStr(Path, 5) + '|' + UpperCase(Path) + '|' + Format(StrPos(Path, '.'))); end;
    procedure ControlIsPathTemporary(): Boolean begin Seed(); ToFile('out/a.pdf'); exit(File.IsPathTemporary('out/a.pdf')); end;
    procedure ControlOtherPathIsUntouched(): Boolean begin Seed(); ToFile('out/a.pdf'); exit(File.Exists('out/other.pdf')); end;
    procedure ControlPassedToAProcedure(): Text begin Seed(); exit(Echo('out/a.pdf', true)); end;
    local procedure Echo(Path: Text; Render: Boolean): Text begin if Render then ToFile(Path); exit(Path); end;
    procedure ControlClearLastError(): Text begin Seed(); ToFile('out/a.pdf'); ClearLastError(); exit(GetLastErrorText()); end;
    procedure ControlStreamObjectItself(): Text var R: Record "RNO Row"; Out: OutStream; V: Variant; begin Seed(); R.Data.CreateOutStream(Out); Report.SaveAs(Report::"Post Inventory Cost to G/L", '', ReportFormat::Pdf, Out); V := Out; exit(Format(V.IsOutStream)); end;
    procedure ControlAnotherBlobIsUntouched(): Boolean var R: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); exit(R.Other.HasValue()); end;
    procedure ControlWriteOverThePath(): Boolean var F: File; begin Seed(); ToFile('out/a.pdf'); F.Create('out/a.pdf'); F.Close(); exit(File.Exists('out/a.pdf')); end;

    // ---- a fresh buffer or path clears the mark ---------------------------------------------------------------

    procedure RecreatingTheStreamClearsTheMark(): Boolean var R: Record "RNO Row"; Out: OutStream; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); R.Data.CreateOutStream(Out); exit(R.Data.HasValue()); end;
    procedure ClearingTheRecordClearsTheMark(): Boolean var R: Record "RNO Row"; begin Seed(); R.Id := 10; R.Insert(); ToStream(R); Clear(R); exit(R.Data.HasValue()); end;
}}
"#
    )
}

fn built(report_1002_extra: &str) -> Built {
    Built::new(compile_apps(vec![AppSource {
        id: BASE_APPLICATION.into(),
        name: "Base Application".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), fixture(report_1002_extra))],
    }]))
}

fn run(proc: &str) -> Result<Value, Signal> {
    built("").run_in("RNO Tests", proc)
}

const PENDING: &str = "rendered output of report Post Inventory Cost to G/L is not produced";

#[track_caller]
fn observed(proc: &str) {
    match run(proc) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(PENDING), "{proc}: {why}"),
        other => panic!("{proc}: expected the observation to be refused, got {other:?}"),
    }
}

#[track_caller]
fn text(proc: &str) -> String {
    al_testkit::text(run(proc))
}

// ---- what the save does ----------------------------------------------------------------------------------------

#[test]
fn report_no_output_a_listed_report_runs_its_triggers_and_answers_true_for_a_file() {
    assert_eq!(text("RunsToAFile"), "Yes|init;pre;row1;row2;post;");
    assert!(al_builtins::assumptions::hit(
        al_builtins::assumptions::Assumption::ReportRenderDiscarded
    ));
}

#[test]
fn report_no_output_a_listed_report_runs_its_triggers_and_answers_true_for_a_stream() {
    assert_eq!(text("RunsToAStream"), "Yes|init;pre;row1;row2;post;");
}

#[test]
fn report_no_output_the_instance_form_and_the_other_formats_run_too() {
    assert_eq!(text("RunsAnInstance"), "Yes|init;pre;row1;row2;post;");
    assert_eq!(
        text("WordAndHtmlRun"),
        "YesYes|init;pre;row1;row2;post;init;pre;row1;row2;post;"
    );
}

#[test]
fn report_no_output_a_report_that_is_not_listed_keeps_the_refusal() {
    for proc in ["NotListedStaysRefused", "NotListedStreamStaysRefused"] {
        match run(proc) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("layout renderer"), "{proc}: {why}"),
            other => panic!("{proc}: {other:?}"),
        }
    }
}

#[test]
fn report_no_output_a_same_named_report_of_another_app_is_not_listed() {
    let mut b = Built::new(compile_apps(vec![AppSource {
        id: "someone-else".into(),
        name: "Mine".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), fixture(""))],
    }]));
    match b.run_in("RNO Tests", "RunsToAFile") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("layout renderer"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// ---- the render-stage errors stay, and run before the no-output path ------------------------------------------------

#[test]
fn report_no_output_a_report_with_a_layout_and_no_file_answers_false_as_before() {
    // report/PRSaveAsFormats.al: DefaultLayout without a layout file answers false and sets this text; no trigger runs.
    let mut b = built("DefaultLayout = RDLC;");
    assert_eq!(
        al_testkit::text(b.run_in("RNO Tests", "LayoutError")),
        "No|Report 1002 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom."
    );
    assert_eq!(al_testkit::text(b.run_in("RNO Tests", "RunsToAFile")), "No|");
}

#[test]
fn report_no_output_html_of_a_report_with_an_rdlc_layout_answers_false_as_before() {
    let mut b = built("DefaultLayout = RDLC; RDLCLayout = 'x.rdl';");
    assert_eq!(
        al_testkit::text(b.run_in("RNO Tests", "HtmlError")),
        "No|The Html output format is not supported by the current report layout format."
    );
}

#[test]
fn report_no_output_pdf_of_a_processing_only_report_keeps_the_pinned_answer() {
    // handlers/TestReportSaveAsColumnTypesContracts.al:372-388: false, this text, no trigger runs.
    let mut b = built("ProcessingOnly = true;");
    assert_eq!(
        al_testkit::text(b.run_in("RNO Tests", "LayoutError")),
        "No|Report 1002 is a processing-only report. Output cannot be rendered and saved to a file."
    );
    assert_eq!(al_testkit::text(b.run_in("RNO Tests", "RunsToAFile")), "No|");
}

// ---- F1 to F11 ----------------------------------------------------------------------------------------------

#[test]
fn report_no_output_f1_open_and_create_in_stream() {
    observed("F1Open");
    observed("F1CreateInStream");
}
#[test]
fn report_no_output_f2_exists() {
    observed("F2Exists");
}
#[test]
fn report_no_output_f3_erase() {
    observed("F3Erase");
}
#[test]
fn report_no_output_f4_copy_and_rename() {
    observed("F4Copy");
    observed("F4CopyOnto");
    observed("F4Rename");
}
#[test]
fn report_no_output_f5_presence_and_size() {
    observed("F5ServerFileExists");
    observed("F5GetFileSize");
}
#[test]
fn report_no_output_f6_download() {
    observed("F6Download");
}
#[test]
fn report_no_output_f7_the_folder_cannot_be_asked_about() {
    match run("F7FolderExists") {
        Err(Signal::Unsupported(_)) => {}
        other => panic!("{other:?}"),
    }
}
#[test]
fn report_no_output_f8_get_file_contents() {
    observed("F8GetFileContents");
}
#[test]
fn report_no_output_f9_open_book() {
    observed("F9OpenBook");
}
#[test]
fn report_no_output_f10_check_file_not_empty() {
    observed("F10CheckFileNotEmpty");
}
#[test]
fn report_no_output_f11_attach() {
    observed("F11Attach");
}

// ---- S1 to S7 -----------------------------------------------------------------------------------------------

#[test]
fn report_no_output_s1_reading_the_stream() {
    for p in [
        "S1ReadText",
        "S1EOS",
        "S1Length",
        "S1Position",
        "S1ResetPosition",
        "S1CopyStreamSource",
    ] {
        observed(p);
    }
}
#[test]
fn report_no_output_s2_presence_size_and_create_in_stream() {
    for p in ["S2HasValue", "S2Length", "S2CreateInStream", "S2TempBlobLength"] {
        observed(p);
    }
}
#[test]
fn report_no_output_s3_helpers_that_read_the_stream() {
    observed("S3ToText");
}
#[test]
fn report_no_output_s4_the_mark_follows_the_buffer_into_rows() {
    for p in [
        "S4Assign",
        "S4InsertGet",
        "S4TransferFields",
        "S4BlobToBlob",
        "S4CalcFields",
        "S4FormatOfTheBlob",
    ] {
        observed(p);
    }
}
#[test]
fn report_no_output_s5_copying_and_sending_the_stream() {
    for p in ["S5FileCopyStream", "S5DownloadFromStream", "S5HttpBody"] {
        observed(p);
    }
}
#[test]
fn report_no_output_s6_parsing_the_stream() {
    observed("S6XmlReadFrom");
    observed("S6JsonReadFrom");
}
#[test]
fn report_no_output_s7_report_inbox() {
    observed("S7ReportInbox");
}

// ---- controls ---------------------------------------------------------------------------------------------------

#[test]
fn report_no_output_s8_and_the_path_controls_are_not_observations() {
    assert_eq!(text("ControlPathAsText"), "out/a.pdf|out/a.pdf|out/a.pdf saved");
    assert_eq!(text("ControlPureStringFunctions"), "a.pdf|OUT/A.PDF|6");
    // A relative path with a subfolder is temporary (FileEraseAndTempPath.al:IsPathTemporary_Paths `relative_subfolder`).
    assert!(al_testkit::boolean(run("ControlIsPathTemporary")));
    assert!(!al_testkit::boolean(run("ControlOtherPathIsUntouched")));
    assert_eq!(text("ControlPassedToAProcedure"), "out/a.pdf");
    assert_eq!(text("ControlClearLastError"), "");
    assert_eq!(text("ControlStreamObjectItself"), "Yes");
    assert!(!al_testkit::boolean(run("ControlAnotherBlobIsUntouched")));
}

#[test]
fn report_no_output_writing_over_the_path_or_recreating_the_stream_ends_the_mark() {
    assert!(al_testkit::boolean(run("ControlWriteOverThePath")));
    assert!(!al_testkit::boolean(run("RecreatingTheStreamClearsTheMark")));
    assert!(!al_testkit::boolean(run("ClearingTheRecordClearsTheMark")));
}

// Owner decision 2026-10-22 ("Run triggers, no file"): inside the DemoTool company build only, SaveAsPdf to a file runs the report
// (triggers and data) and writes no file (MakeCorrections.Codeunit.al:62, PostRevaluation.Codeunit.al:82,
// RunPostInventoryCosttoGL.Codeunit.al:17). A look at the file, another format, a stream, and the same call outside it are refused.
#[test]
fn report_no_output_in_the_company_build_save_as_pdf_to_a_file_runs_the_report_and_writes_no_file() {
    let mut b = built("");
    b.session.set_company_build(true);
    assert_eq!(al_testkit::text(b.run_in("RNO Tests", "CompanyBuildPdf")), "pre;row1;row2;post;");
    assert!(al_builtins::assumptions::hit(
        al_builtins::assumptions::Assumption::ReportRenderDiscarded
    ));
    match b.run_in("RNO Tests", "CompanyBuildPdfThenOpen") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("RNO Logged"), "{why}"),
        other => panic!("{other:?}"),
    }
    for proc in ["CompanyBuildWord", "CompanyBuildPdfStream"] {
        match b.run_in("RNO Tests", proc) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("layout renderer"), "{proc}: {why}"),
            other => panic!("{proc}: {other:?}"),
        }
    }
    match run("CompanyBuildPdf") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("layout renderer"), "{why}"),
        other => panic!("outside the company build: {other:?}"),
    }
}
