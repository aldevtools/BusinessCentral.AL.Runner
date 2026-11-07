//! CalcFields of a Blob whose buffer holds an unsaved stream write: replaced by the stored content when another variable
//! modified the Blob since the write began, kept when the row is as it was
//! (streams/CalcFieldsBlobStaleBuffer.al:75-98 `Stale_UnsavedWriteInA_OtherModifies_CalcFields`,
//! streams/TestBlobCalcFieldsUncommittedWrite.al `Blob_CalcFields_AfterUncommittedWriteWithoutModify_KeepsWrite`).

use al_testkit::text;

const OBJECTS: &str = r#"
table 50471 BRow
{
    fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } }
    keys { key(PK; Code) { } }
}
"#;

fn run(body: &str) -> String {
    let src = format!("{OBJECTS}\ncodeunit 50479 T\n{{\n{body}\n}}\n");
    text(al_testkit::run_in(&src, "T", "F"))
}

const HELPERS: &str = r#"
    local procedure WriteBlob(var R: Record BRow; T: Text)
    var O: OutStream;
    begin R.Data.CreateOutStream(O); O.WriteText(T); end;
    local procedure ReadBlob(var R: Record BRow): Text
    var I: InStream; T: Text;
    begin R.Data.CreateInStream(I); if not I.EOS() then I.ReadText(T); exit(T); end;
"#;

#[test]
fn another_variables_modify_replaces_an_unsaved_write_at_calcfields() {
    let got = run(&format!(
        r#"{HELPERS}
    procedure F(): Text
    var A: Record BRow; B: Record BRow; C: Record BRow; Obs: Text;
    begin
        A.Init(); A.Code := 'S3'; A.Insert();
        WriteBlob(A, 'A-UNSAVED');
        B.Get('S3'); WriteBlob(B, 'B-TEXT'); B.Modify();
        Obs := 'before=' + ReadBlob(A);
        A.CalcFields(Data);
        Obs += ';after=' + ReadBlob(A);
        A.Modify();
        C.Get('S3'); C.CalcFields(Data);
        Obs += ';stored=' + ReadBlob(C);
        exit(Obs);
    end;"#
    ));
    assert_eq!(got, "before=A-UNSAVED;after=B-TEXT;stored=B-TEXT");
}

#[test]
fn an_unsaved_write_survives_calcfields_when_the_row_is_as_it_was() {
    let got = run(&format!(
        r#"{HELPERS}
    procedure F(): Text
    var A: Record BRow;
    begin
        A.Init(); A.Code := 'S4'; A.Insert();
        WriteBlob(A, 'A-UNSAVED');
        A.CalcFields(Data);
        exit(ReadBlob(A));
    end;"#
    ));
    assert_eq!(got, "A-UNSAVED");
}
