//! The length of a 3 MB BLOB (streams/lt-blob-length-integer/BlobLengthInteger.al, writer Y57): it fits an Integer,
//! BigInteger and Decimal; read back with Get and no CalcFields the stored (compressed) size of that content is 3072; a
//! second CreateOutStream on a blob that holds data writes over it from the start and does not truncate it.

use al_testkit::text;

const SRC: &str = r#"
table 69960 "BLL Row" { fields { field(1; Id; Integer) { } field(2; Content; Blob) { } } keys { key(PK; Id) { } } }
codeunit 69961 C
{
    local procedure Fill(var Row: Record "BLL Row"; Size: Integer)
    var OutS: OutStream; Chunk: Text; N: Integer;
    begin
        Row.Content.CreateOutStream(OutS);
        Chunk := PadStr('', 1024, 'x');
        for N := 1 to Size div 1024 do
            OutS.WriteText(Chunk);
        OutS.WriteText(PadStr('', Size mod 1024, 'y'));
    end;

    procedure F(): Text
    var Row: Record "BLL Row"; I: Integer; Big: BigInteger; D: Decimal; R: Text;
    begin
        Row.Id := 1;
        Fill(Row, 3 * 1024 * 1024 + 7);
        I := Row.Content.Length; Big := Row.Content.Length; D := Row.Content.Length;
        R := Format(I) + '|' + Format(Big) + '|' + Format(D, 0, 9) + '|' + Format(Round(Row.Content.Length / (1024 * 1024)));
        Row.Insert();
        Clear(Row);
        Row.Get(1);
        R += '|' + Format(Row.Content.Length);
        Row.CalcFields(Content);
        R += '|' + Format(Row.Content.Length);
        Fill(Row, 1536 * 1024);
        R += '|' + Format(Row.Content.Length);
        Clear(Row);
        Row.Id := 2;
        Fill(Row, 3 * 1024 * 1024);
        Fill(Row, 1536 * 1024);
        exit(R + '|' + Format(Row.Content.Length));
    end;
}
"#;

#[test]
fn a_3_mb_blob_length_and_its_stored_size() {
    // BlobLengthInteger.al:48-103 (BlobLength_IntoInteger).
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "3145735|3145735|3145735|3|3072|3145735|3145735|3145728"
    );
}
