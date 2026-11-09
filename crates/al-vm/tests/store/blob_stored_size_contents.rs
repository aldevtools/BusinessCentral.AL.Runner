//! The stored size Blob.Length gives before CalcFields for the contents whose stored size the corpus measured beyond the sizes
//! of stream.rs `blob_length_of`: 40 digits (record/flowfields/BlobCalcFieldsLength.al:470, 19), 'into field'
//! (streams/DeBlobToRecordRef.al:60, 16) and, on BC 29, 100, 10000 and 100000 `x` (record/PnABlobLength.al:55-60, 10, 31, 118).

use al_testkit::text;

const SRC: &str = r#"
table 69980 "BSC Row" { fields { field(1; Id; Integer) { } field(2; Content; Blob) { } } keys { key(PK; Id) { } } }
codeunit 69981 C
{
    local procedure Stored(Content: Text): Text
    var Row: Record "BSC Row"; Got: Record "BSC Row"; OutS: OutStream;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Content.CreateOutStream(OutS);
        OutS.WriteText(Content);
        Row.Insert();
        Got.Get(1);
        exit(Format(Got.Content.Length));
    end;

    procedure F(): Text
    begin
        exit(Stored('0123456789012345678901234567890123456789') + '|' + Stored('into field') + '|' +
            Stored(PadStr('', 100, 'x')) + '|' + Stored(PadStr('', 10000, 'x')) + '|' + Stored(PadStr('', 100000, 'x')));
    end;
}
"#;

#[test]
fn the_measured_contents_have_their_stored_size() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "19|16|10|31|118");
}
