//! The e-mail body round trip through a server file (al-runner issue 6924): Base Application's `#if not CLEAN27` block of
//! Report Selections (GetEmailBodyForVend, GetEmailBodyTextForCust) writes the saved Html of a Word layout report to a server
//! temp file (File Management.TempBlobToServerFile), imports it back (BLOBImportFromServerFile) and deletes the file
//! (DeleteServerFile, a static `System.IO.File.Delete`). The stand-ins below do what those File Management procedures do with
//! the builtins. The placeholder mark goes along with the byte copies; the delete of the file is no read of its content; the
//! body read whole afterwards is the rendered content's mark (ADR-040 amendment 2026-11-12, A2).

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

use super::rendered_text::FIXTURE;

const ROUND_TRIP: &str = r#"
dotnet { assembly("mscorlib") { type("System.IO.File"; "IOFile") { } } }

report 50996 "RF Rdlc"
{
    UsageCategory = None;
    DefaultRenderingLayout = RdlcOne;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(RdlcOne) { Type = RDLC; LayoutFile = 'RfRdlc.rdl'; } }
}

codeunit 50997 "RF Tests"
{
    local procedure Save(var Tmp: Record "RT Row" temporary)
    begin
        SaveAs(Tmp, ReportFormat::Html);
    end;

    local procedure SaveAs(var Tmp: Record "RT Row" temporary; Fmt: ReportFormat)
    var R: Record "RT Row"; Out: OutStream;
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        Tmp.Id := 9;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RT Word", '', Fmt, Out);
        Tmp.Insert();
    end;

    // A server temp file name, as File Management.ServerTempFileName gives one.
    local procedure TempName(Ext: Text) Name: Text
    var TempFile: File;
    begin
        TempFile.CreateTempFile();
        Name := TempFile.Name + '.' + Ext;
        TempFile.Close();
    end;

    // File Management.TempBlobToServerFile: an UTF-8 InStream on the blob, ServerTempFileName, InstreamExportToServerFile.
    local procedure ToServerFile(var Tmp: Record "RT Row" temporary) Name: Text
    var TempFile: File; OutputFile: File; I: InStream; O: OutStream;
    begin
        Tmp.Data.CreateInStream(I, TextEncoding::UTF8);
        TempFile.CreateTempFile();
        Name := TempFile.Name + '.html';
        TempFile.Close();
        OutputFile.WriteMode(true);
        OutputFile.Create(Name);
        OutputFile.CreateOutStream(O);
        CopyStream(O, I);
        OutputFile.Close();
    end;

    // File Management.BLOBImportFromServerFile.
    local procedure FromServerFile(var Tmp: Record "RT Row" temporary; Name: Text)
    var InputFile: File; I: InStream; O: OutStream;
    begin
        if not File.Exists(Name) then
            Error('missing');
        InputFile.Open(Name);
        InputFile.CreateInStream(I);
        Tmp.Data.CreateOutStream(O);
        CopyStream(O, I);
        InputFile.Close();
    end;

    // File Management.DeleteServerFile.
    local procedure DeleteServerFile(Name: Text): Boolean
    var Helper: DotNet IOFile;
    begin
        if not Exists(Name) then
            exit(false);
        Helper.Delete(Name);
        exit(true);
    end;

    procedure RoundTrip(): Text
    var Tmp: Record "RT Row" temporary; Name: Text; I: InStream; Body: Text; Deleted: Boolean;
    begin
        Save(Tmp);
        if Tmp.Data.HasValue() then
            Name := ToServerFile(Tmp);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        Deleted := DeleteServerFile(Name);
        Tmp.Data.CreateInStream(I);
        I.Read(Body);
        exit(Format(Deleted) + Format(File.Exists(Name)) + Format(Body <> ''));
    end;

    // The measured chain with a Pdf (HtmlServerFileRoundTrip.al HSF_Pdf_TempBlobToServerFile_RoundTrip).
    procedure RoundTripPdf(): Text
    var Tmp: Record "RT Row" temporary; Name: Text; Deleted: Boolean;
    begin
        SaveAs(Tmp, ReportFormat::Pdf);
        Name := ToServerFile(Tmp);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        Deleted := DeleteServerFile(Name);
        exit(Format(Deleted) + Format(File.Exists(Name)) + Format(Tmp.Data.HasValue()));
    end;

    // Not measured: a Word placeholder through the same chain.
    procedure WordChain(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text;
    begin
        SaveAs(Tmp, ReportFormat::Word);
        Name := ToServerFile(Tmp);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(DeleteServerFile(Name));
    end;

    // Not measured: the rendering save writes the file itself, then the delete.
    procedure SaveAsPdfToFileThenDelete(): Boolean
    var R: Record "RT Row"; Name: Text; Helper: DotNet IOFile;
    begin
        R.DeleteAll(); R.Id := 1; R.Insert();
        Name := TempName('pdf');
        Report.SaveAsPdf(Report::"RT Word", Name);
        Helper.Delete(Name);
        exit(File.Exists(Name));
    end;
    procedure SaveAsHtmlToFileThenDelete(): Boolean
    var R: Record "RT Row"; Name: Text; Helper: DotNet IOFile;
    begin
        R.DeleteAll(); R.Id := 1; R.Insert();
        Name := TempName('html');
        Report.SaveAsHtml(Report::"RT Word", Name);
        Helper.Delete(Name);
        exit(File.Exists(Name));
    end;
    procedure SaveAsHtmlToFileThenDeleteServerFile(): Boolean
    var R: Record "RT Row"; Name: Text;
    begin
        R.DeleteAll(); R.Id := 1; R.Insert();
        Name := TempName('html');
        Report.SaveAsHtml(Report::"RT Word", Name);
        exit(DeleteServerFile(Name));
    end;

    // Not measured: the delete while an AL File holds the file open with a stream on it.
    procedure DeleteWhileOpen(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; I: InStream; Helper: DotNet IOFile;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        F.Open(Name);
        F.CreateInStream(I);
        Helper.Delete(Name);
        exit(File.Exists(Name));
    end;

    local procedure Seed()
    var R: Record "RT Row";
    begin
        R.DeleteAll(); R.Id := 1; R.Insert();
    end;

    local procedure Delete(Name: Text): Boolean
    var Helper: DotNet IOFile;
    begin
        Helper.Delete(Name);
        exit(File.Exists(Name));
    end;

    // The export of ToServerFile to a given name.
    local procedure ExportTo(var Tmp: Record "RT Row" temporary; Name: Text)
    var OutputFile: File; I: InStream; O: OutStream;
    begin
        Tmp.Data.CreateInStream(I, TextEncoding::UTF8);
        OutputFile.WriteMode(true);
        OutputFile.Create(Name);
        OutputFile.CreateOutStream(O);
        CopyStream(O, I);
        OutputFile.Close();
    end;

    local procedure OpenAndLeave(Name: Text)
    var F: File; I: InStream;
    begin
        F.Open(Name);
        F.CreateInStream(I);
    end;

    // Not measured (review round 2, N01, N02): the rendering save wrote the file, an AL File opened it for writing and closed
    // it, copying nothing.
    procedure SaveAsHtmlToFileWriteOpenCloseDelete(): Boolean
    var Name: Text; F: File;
    begin
        Seed();
        Name := TempName('html');
        Report.SaveAsHtml(Report::"RT Word", Name);
        F.WriteMode(true); F.Open(Name); F.Close();
        exit(Delete(Name));
    end;
    procedure SaveAsPdfToFileWriteOpenCloseDelete(): Boolean
    var Name: Text; F: File;
    begin
        Seed();
        Name := TempName('pdf');
        Report.SaveAsPdf(Report::"RT Word", Name);
        F.WriteMode(true); F.Open(Name); F.Close();
        exit(Delete(Name));
    end;

    // Not measured (N04): after the export a second File opens the file for writing and appends.
    procedure ExportWriteOpenAppendCloseDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; O: OutStream;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        F.WriteMode(true); F.Open(Name); F.CreateOutStream(O); O.WriteText('extra'); F.Close();
        exit(Delete(Name));
    end;

    // Not measured (N06): a File holds the file open while a second File exports over it.
    procedure OpenHeldSecondFileExportsOverDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F1: File;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        F1.Open(Name);
        ExportTo(Tmp, Name);
        exit(Delete(Name));
    end;

    // Not measured (N15): a File opened the plain file before the export made it a placeholder file, and still holds it.
    procedure PlainOpenHeldThenExportDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; G: File; O: OutStream;
    begin
        Save(Tmp);
        Name := TempName('html');
        F.WriteMode(true); F.Create(Name); F.CreateOutStream(O); O.WriteText('plain'); F.Close();
        G.Open(Name);
        ExportTo(Tmp, Name);
        exit(Delete(Name));
    end;

    // Not measured (D19): the export over an existing plain file, then the import.
    procedure ExportOverPlainFileDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; O: OutStream;
    begin
        Save(Tmp);
        Name := TempName('html');
        F.WriteMode(true); F.Create(Name); F.CreateOutStream(O); O.WriteText('plain'); F.Close();
        ExportTo(Tmp, Name);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;

    // Not measured (N13): the Pdf of an RDLC layout report through the measured chain.
    procedure RdlcPdfChain(): Boolean
    var Tmp: Record "RT Row" temporary; Out: OutStream; Name: Text;
    begin
        Seed();
        Tmp.Id := 9;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RF Rdlc", '', ReportFormat::Pdf, Out);
        Tmp.Insert();
        Name := ToServerFile(Tmp);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;

    // Not measured (N09, N12): no import, or two, between the export and the delete.
    procedure ExportDeleteNoImport(): Boolean
    var Tmp: Record "RT Row" temporary;
    begin
        Save(Tmp);
        exit(Delete(ToServerFile(Tmp)));
    end;
    procedure ExportTwoImportsDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;

    // Not measured (N07, N08): the export handle also writes text of its own, after or before the stream copy.
    procedure CopyThenWriteTextDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; I: InStream; O: OutStream;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I, TextEncoding::UTF8);
        Name := TempName('html');
        F.WriteMode(true); F.Create(Name); F.CreateOutStream(O);
        CopyStream(O, I);
        O.WriteText('extra');
        F.Close();
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;
    procedure WriteTextThenCopyDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; I: InStream; O: OutStream;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I, TextEncoding::UTF8);
        Name := TempName('html');
        F.WriteMode(true); F.Create(Name); F.CreateOutStream(O);
        O.WriteText('before');
        CopyStream(O, I);
        F.Close();
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;

    // Not measured: the export handle differs from File Management's in one call (no WriteMode, TextMode, an encoding, a
    // second OutStream).
    local procedure ExportVariant(Variant: Integer): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; I: InStream; O: OutStream; O2: OutStream;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I, TextEncoding::UTF8);
        Name := TempName('html');
        if Variant <> 1 then
            F.WriteMode(true);
        if Variant = 2 then
            F.TextMode(true);
        if Variant = 3 then
            F.Create(Name, TextEncoding::UTF8)
        else
            F.Create(Name);
        if Variant = 4 then
            F.CreateOutStream(O2);
        F.CreateOutStream(O);
        CopyStream(O, I);
        F.Close();
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;
    procedure ExportWithoutWriteMode(): Boolean begin exit(ExportVariant(1)); end;
    procedure ExportInTextMode(): Boolean begin exit(ExportVariant(2)); end;
    procedure ExportWithEncoding(): Boolean begin exit(ExportVariant(3)); end;
    procedure ExportWithTwoOutStreams(): Boolean begin exit(ExportVariant(4)); end;

    // Not measured: the import handle differs from File Management's in one call (no InStream, TextMode, WriteMode set after
    // the Open, an encoding).
    local procedure ImportVariant(Variant: Integer): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; I: InStream;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        if Variant = 2 then
            F.TextMode(true);
        if Variant = 3 then
            F.Open(Name, TextEncoding::UTF8)
        else
            F.Open(Name);
        if Variant <> 1 then
            F.CreateInStream(I);
        if Variant = 4 then
            F.WriteMode(true);
        F.Close();
        exit(Delete(Name));
    end;
    procedure ImportWithoutInStream(): Boolean begin exit(ImportVariant(1)); end;
    procedure ImportInTextMode(): Boolean begin exit(ImportVariant(2)); end;
    procedure ImportWithEncoding(): Boolean begin exit(ImportVariant(3)); end;
    procedure ImportWriteModeAfterOpen(): Boolean begin exit(ImportVariant(4)); end;

    // Not measured (N03, N05): a File that opened the file is never closed: it went out of scope, or is one of two. Clear of
    // a File variable (N14) is refused on its own, as on main.
    procedure OpenOutOfScopeDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        OpenAndLeave(Name);
        exit(Delete(Name));
    end;
    procedure OpenClearFileDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        F.Open(Name);
        Clear(F);
        exit(Delete(Name));
    end;
    procedure ImportWhileSecondFileHoldsDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F1: File;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        F1.Open(Name);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;

    // Not measured: a rendering save writes over the exported file, then the import.
    procedure ExportSaveAsHtmlOverImportDelete(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        Report.SaveAsHtml(Report::"RT Word", Name);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Delete(Name));
    end;

    // The measured chain on a path whose earlier file AL erased: the path is new again.
    procedure ChainEraseChain(): Text
    var Tmp: Record "RT Row" temporary; Name: Text;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        Erase(Name);
        ExportTo(Tmp, Name);
        Clear(Tmp.Data);
        FromServerFile(Tmp, Name);
        exit(Format(Delete(Name)) + Format(Tmp.Data.HasValue()));
    end;

    // As on main: AL Erase of a placeholder file deletes it, whatever opened and closed it before (N18, D18).
    procedure SaveAsHtmlToFileWriteOpenCloseErase(): Boolean
    var Name: Text; F: File;
    begin
        Seed();
        Name := TempName('html');
        Report.SaveAsHtml(Report::"RT Word", Name);
        F.WriteMode(true); F.Open(Name); F.Close();
        Erase(Name);
        exit(File.Exists(Name));
    end;
    procedure ExportOpenHeldErase(): Boolean
    var Tmp: Record "RT Row" temporary; Name: Text; F: File;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        F.Open(Name);
        Erase(Name);
        exit(File.Exists(Name));
    end;

    // As on main: File.Copy and File.Rename read the placeholder file, which is refused before anything is written.
    procedure CopyThePlaceholderFile()
    var Tmp: Record "RT Row" temporary; Name: Text;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        File.Copy(Name, Name + '.copy');
    end;
    procedure RenameThePlaceholderFile()
    var Tmp: Record "RT Row" temporary; Name: Text;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        File.Rename(Name, Name + '.moved');
    end;

    procedure ReadTheFileBeforeTheDelete(): Text
    var Tmp: Record "RT Row" temporary; Name: Text; F: File; I: InStream; Body: Text;
    begin
        Save(Tmp);
        Name := ToServerFile(Tmp);
        F.Open(Name);
        F.CreateInStream(I);
        I.ReadText(Body);
        exit(Body);
    end;
}
"#;

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&format!("{FIXTURE}{ROUND_TRIP}")).run_in("RF Tests", proc)
}

#[test]
fn rendered_text_file_round_trip_keeps_the_body_and_deletes_the_file() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("RoundTrip")), "YesNoYes");
    let hits = assumptions::take_hits();
    assert!(hits.contains(&Assumption::ReportHtmlRenderedText), "{hits:?}");
    assert!(hits.contains(&Assumption::DotNetFileDeletePlaceholder), "{hits:?}");
}

#[test]
fn rendered_text_file_round_trip_a_read_of_the_file_stays_refused() {
    match run("ReadTheFileBeforeTheDelete") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("a read of the content of a placeholder Html"), "{why}"),
        other => panic!("expected the read refusal, got {other:?}"),
    }
}

#[test]
fn rendered_text_file_round_trip_of_a_pdf_deletes_the_file() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("RoundTripPdf")), "YesNoYes");
    assert!(assumptions::take_hits().contains(&Assumption::DotNetFileDeletePlaceholder));
}

// Shapes the probe (HtmlServerFileRoundTrip.al) does not measure. Each refuses at the delete, as on main.
#[test]
fn rendered_text_file_round_trip_unmeasured_deletes_refuse() {
    let wrong = [
        "WordChain",
        "SaveAsPdfToFileThenDelete",
        "SaveAsHtmlToFileThenDelete",
        "SaveAsHtmlToFileThenDeleteServerFile",
        "DeleteWhileOpen",
        // Review round 2, MED 1a and 1b.
        "SaveAsHtmlToFileWriteOpenCloseDelete",
        "SaveAsPdfToFileWriteOpenCloseDelete",
        "ExportWriteOpenAppendCloseDelete",
        "OpenHeldSecondFileExportsOverDelete",
        "PlainOpenHeldThenExportDelete",
        // Review round 2, LOW 2.
        "ExportOverPlainFileDelete",
        "RdlcPdfChain",
        "ExportDeleteNoImport",
        "ExportTwoImportsDelete",
        "CopyThenWriteTextDelete",
        "WriteTextThenCopyDelete",
        // One call of the export or the import handle differs.
        "ExportWithoutWriteMode",
        "ExportInTextMode",
        "ExportWithEncoding",
        "ExportWithTwoOutStreams",
        "ImportWithoutInStream",
        "ImportInTextMode",
        "ImportWithEncoding",
        "ImportWriteModeAfterOpen",
        // A File that is never closed.
        "OpenOutOfScopeDelete",
        "ImportWhileSecondFileHoldsDelete",
        "ExportSaveAsHtmlOverImportDelete",
    ]
    .into_iter()
    .filter_map(|p| match run(p) {
        Err(Signal::Unsupported(why)) if why.contains("DotNet System.IO.File.Delete") => None,
        other => Some(format!("{p}: {other:?}")),
    })
    .collect::<Vec<_>>();
    assert!(wrong.is_empty(), "not refused at the delete:\n{}", wrong.join("\n"));
}

// The measured chain again on a path AL erased in between runs: the second export makes a new file.
#[test]
fn rendered_text_file_round_trip_after_an_erase_of_the_path_deletes_the_file() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("ChainEraseChain")), "NoYes");
    assert!(assumptions::take_hits().contains(&Assumption::DotNetFileDeletePlaceholder));
}

// Out of scope of the delete gate, as on main: AL Erase deletes a placeholder file whatever handles came before, File.Copy
// and File.Rename stop at the read of the file, and Clear of a File variable is refused.
#[test]
fn rendered_text_file_round_trip_erase_copy_and_rename_keep_main() {
    assumptions::clear_hits();
    for p in ["SaveAsHtmlToFileWriteOpenCloseErase", "ExportOpenHeldErase"] {
        assert!(matches!(run(p), Ok(Value::Boolean(false))), "{p}: the file is gone");
    }
    assert!(!assumptions::take_hits().contains(&Assumption::DotNetFileDeletePlaceholder));
    for p in ["CopyThePlaceholderFile", "RenameThePlaceholderFile"] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("a placeholder Html"), "{p}: {why}"),
            other => panic!("{p}: expected the refusal of the read, got {other:?}"),
        }
    }
    match run("OpenClearFileDelete") {
        Err(Signal::Unsupported(why)) => assert_eq!(why.to_string(), "Clear of a File value"),
        other => panic!("expected main's refusal of the Clear, got {other:?}"),
    }
}
