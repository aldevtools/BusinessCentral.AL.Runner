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

codeunit 50997 "RF Tests"
{
    local procedure Save(var Tmp: Record "RT Row" temporary)
    var R: Record "RT Row"; Out: OutStream;
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        Tmp.Id := 9;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out);
        Tmp.Insert();
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
