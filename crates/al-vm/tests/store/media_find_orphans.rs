//! `Media.FindOrphans()` and `MediaSet.FindOrphans()` (media/lt-y48-find-orphans/Y48FindOrphans.al:305-410): a media no stored
//! row holds is an orphan (never inserted, not modified after the import, held only by a temporary record), a held media and
//! the items of a held set are not; a set no stored row holds is an orphan.

use al_testkit::{run, text};

const SRC: &str = r#"
table 60040 "FO Row"
{
    fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Pics; MediaSet) { } }
    keys { key(PK; Id) { } }
}
codeunit 50170 C
{
    local procedure Stream(var TempBlob: Codeunit "Temp Blob"; var InS: InStream)
    var
        OutS: OutStream;
    begin
        TempBlob.CreateOutStream(OutS);
        OutS.WriteText('orphan probe');
        TempBlob.CreateInStream(InS);
    end;

    procedure F(): Text
    var
        Row: Record "FO Row";
        Other: Record "FO Row";
        TempRow: Record "FO Row" temporary;
        TempBlob: Codeunit "Temp Blob";
        InS: InStream;
        Held: Guid;
        HeldSet: Guid;
        NotInserted: Guid;
        Temp: Guid;
    begin
        Row.Id := 1; Row.Insert();
        Stream(TempBlob, InS); Row.Pic.ImportStream(InS, 'd');
        Stream(TempBlob, InS); Row.Pics.ImportStream(InS, 'a');
        Row.Modify();
        Held := Row.Pic.MediaId();
        HeldSet := Row.Pics.MediaId();
        Other.Id := 2;
        Stream(TempBlob, InS); Other.Pic.ImportStream(InS, 'd');
        NotInserted := Other.Pic.MediaId();
        TempRow.Id := 3;
        Stream(TempBlob, InS); TempRow.Pic.ImportStream(InS, 'd');
        TempRow.Insert();
        Temp := TempRow.Pic.MediaId();
        exit(Format(Media.FindOrphans().Contains(Held)) + Format(Media.FindOrphans().Contains(Row.Pics.Item(1))) +
             Format(MediaSet.FindOrphans().Contains(HeldSet)) + Format(Media.FindOrphans().Contains(NotInserted)) +
             Format(Media.FindOrphans().Contains(Temp)) + Format(Media.FindOrphans().Contains(HeldSet)));
    end;
}
"#;

#[test]
fn orphans_are_the_media_no_stored_row_holds() {
    assert_eq!(
        text(run(&format!("{SRC}{}", al_testkit_temp_blob()), "F")),
        "NoNoNoYesYesNo"
    );
}

/// A stand-in Temp Blob codeunit (the System Application's is not compiled here).
fn al_testkit_temp_blob() -> &'static str {
    r#"
codeunit 50171 "Temp Blob"
{
    var
        B: Record "FO Blob" temporary;
    procedure CreateOutStream(var OutS: OutStream) begin B.Blob.CreateOutStream(OutS); end;
    procedure CreateInStream(var InS: InStream) begin B.Blob.CreateInStream(InS); end;
}
table 60041 "FO Blob" { fields { field(1; K; Integer) { } field(2; Blob; Blob) { } } keys { key(PK; K) { } } }
"#
}
