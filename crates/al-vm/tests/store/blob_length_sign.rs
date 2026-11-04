//! `Blob.Length` before CalcFields of a stored blob (al-runner issue 6896). The stored size of a Compressed field is host-dependent and
//! pinned only as positive for non-empty content; a field with `Compressed = false` stores the content size
//! (record/flowfields/BlobStoredSizeSign.al StoredSize_*_Result: `p` for g, fs, mg of `LT BSS Compressed`, the content size for `LT BSS Plain`).

use al_builtins::value::Value;
use al_testkit::{int, run_with};

const SRC: &str = r#"
table 69970 "BLS Packed" { fields { field(1; Id; Integer) { } field(2; Content; Blob) { Compressed = true; } } keys { key(PK; Id) { } } }
table 69972 "BLS Plain" { fields { field(1; Id; Integer) { } field(2; Content; Blob) { Compressed = false; } } keys { key(PK; Id) { } } }
codeunit 69971 C
{
    procedure Packed(N: Integer): Integer
    var Row: Record "BLS Packed"; Got: Record "BLS Packed"; OutS: OutStream;
    begin
        Row.Id := 1;
        Row.Content.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', N, 'x'));
        Row.Insert();
        Got.Get(1);
        exit(Size(Got.Content.Length));
    end;

    procedure Plain(N: Integer): Integer
    var Row: Record "BLS Plain"; Got: Record "BLS Plain"; OutS: OutStream;
    begin
        Row.Id := 1;
        Row.Content.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', N, 'x'));
        Row.Insert();
        Got.Get(1);
        exit(Size(Got.Content.Length));
    end;

    local procedure Size(Value: Integer): Integer
    begin
        exit(Value);
    end;
}
"#;

fn run_n(routine: &str, n: i32) -> i32 {
    int(run_with(SRC, "C", routine, vec![Value::Integer(n)]).map(|(v, _)| v))
}

#[test]
fn a_compressed_field_gives_a_positive_stored_size_for_any_content() {
    for n in [1, 600, 1000, 4000, 100_000] {
        assert!(run_n("Packed", n) > 0, "{n}");
    }
}

#[test]
fn a_field_without_compression_stores_the_content_size() {
    for n in [1, 2, 5, 600, 1000, 4000, 100_000] {
        assert_eq!(run_n("Plain", n), n, "{n}");
    }
}
