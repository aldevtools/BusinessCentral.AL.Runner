//! A temporary record whose Blob is written with CreateOutStream after Insert, under a new key, and inserted again: the first
//! row keeps its own Blob. xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_ImportTwice_Direct (Buffer rows 1
//! `A` and 2 `C`; Get(1) then reads `A`).

use al_testkit::{run, text};

const SRC: &str = r#"
table 50990 "TBR Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50991 P {
    procedure Go(): Text
    var
        Buffer: Record "TBR Blob" temporary;
        OutS: OutStream;
        InS: InStream;
        T: Text;
    begin
        Buffer.Id := 1;
        Buffer.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('A');
        Buffer.Insert();
        Buffer.Id := 2;
        Buffer.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('C');
        Buffer.Insert();
        Buffer.Get(1);
        Buffer.CalcFields(Data);
        Buffer.Data.CreateInStream(InS, TextEncoding::UTF8);
        InS.ReadText(T);
        exit(T);
    end;
}
"#;

#[test]
fn a_blob_rewritten_after_insert_leaves_the_stored_row_alone() {
    assert_eq!(text(run(SRC, "Go")), "A");
}
