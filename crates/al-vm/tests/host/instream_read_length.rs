//! InStream.Read(Text, Length) and ReadText(Text, Length) on UTF-8 and single-byte streams (al-runner issue 7113).
//! Pins: streams/W7StreamRead.al:254-305 (UTF-8 counts characters, the result is the bytes of the text plus one, a zero byte
//! stops the read on UTF-8 and on Windows), :336-370 (what one UTF-8 read consumes), streams/instream-outstream/
//! StreamReadMatrix.al (`u8`, `win` and `dos` lines with the lengths 0 and 3), StreamUtf8CharsRead.al:156-284 (one character per
//! stream, a surrogate pair counts once, ReadText(Text, n) counts characters).

use al_testkit::text;

/// Writes `bytes` to a Blob, opens it with `enc`, runs `read` (an AL statement that sets `N` and `T`) and answers
/// `N [codes] pos P` or `err CODE`.
fn read_with(bytes: &str, enc: &str, read: &str) -> String {
    let src = format!(
        r#"
table 50196 B {{ TableType = Temporary; fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50196 C
{{
    [TryFunction]
    local procedure TryRead(var InS: InStream; var T: Text; var N: Integer)
    begin
        {read}
    end;

    procedure F(): Text
    var
        Rec: Record B temporary; OutS: OutStream; InS: InStream; Bytes: List of [Integer]; V: Integer; By: Byte; N: Integer;
        T: Text; R: Text; I: Integer; Code: Integer;
    begin
        Bytes.AddRange({bytes});
        Rec.Data.CreateOutStream(OutS);
        foreach V in Bytes do begin By := V; OutS.Write(By); end;
        Rec.Data.CreateInStream(InS, TextEncoding::{enc});
        if not TryRead(InS, T, N) then
            exit('err ' + GetLastErrorCode());
        for I := 1 to StrLen(T) do begin
            Code := T[I];
            if R <> '' then R += ' ';
            R += Format(Code);
        end;
        exit(Format(N) + ' [' + R + '] pos ' + Format(InS.Position()));
    end;
}}
"#
    );
    text(al_testkit::run_in(&src, "C", "F"))
}

fn read_len(bytes: &str, enc: &str, len: i32) -> String {
    read_with(bytes, enc, &format!("N := InS.Read(T, {len});"))
}

const A_EACUTE_EURO_B: &str = "97, 195, 169, 226, 130, 172, 98";

// W7StreamRead.al:268-277 ReadTextWithLength_Utf8Sequences; :358-366 the position after each length.
#[test]
fn a_utf8_read_with_a_length_counts_characters_and_answers_the_bytes_plus_one() {
    assert_eq!(read_len(A_EACUTE_EURO_B, "UTF8", 1), "2 [97] pos 2");
    assert_eq!(read_len(A_EACUTE_EURO_B, "UTF8", 2), "4 [97 233] pos 4");
    assert_eq!(read_len(A_EACUTE_EURO_B, "UTF8", 3), "7 [97 233 8364] pos 7");
    assert_eq!(read_len(A_EACUTE_EURO_B, "UTF8", 5), "8 [97 233 8364 98] pos 8");
    assert_eq!(
        read_len("104, 101, 108, 108, 111", "UTF8", 5),
        "6 [104 101 108 108 111] pos 6"
    );
}

// StreamUtf8CharsRead.al:242-258 Utf8_Emoji_ReadVariants: a surrogate pair is one character.
#[test]
fn a_utf8_read_with_a_length_takes_a_surrogate_pair_as_one_character() {
    assert_eq!(read_len("240, 159, 152, 128", "UTF8", 1), "5 [55357 56832] pos 5");
}

// W7StreamRead.al:296-303 ReadTextWithLength_AsciiAndZeroByte; StreamReadMatrix.al:415 Matrix_Utf16Ab `u8 len 3` (the zero byte is
// consumed: the rest reads `b`), :180 Matrix_AbcZero `u8 len 3` (a zero byte after the last character is left).
#[test]
fn a_utf8_read_with_a_length_stops_at_a_zero_byte_and_consumes_it() {
    assert_eq!(read_len("97, 98, 0, 99, 100", "UTF8", 5), "3 [97 98] pos 4");
    assert_eq!(read_len("97, 98, 99, 0", "UTF8", 3), "4 [97 98 99] pos 4");
    assert_eq!(read_len("97, 98, 0, 99, 100", "UTF8", 0), "1 [] pos 1");
}

// StreamReadMatrix.al:535 Matrix_Utf8Bom `u8 len 3`, :355 Matrix_TruncatedUtf8, :325-327 Matrix_ByteFF.
#[test]
fn a_utf8_read_with_a_length_skips_a_mark_drops_a_cut_sequence_and_rejects_ff() {
    assert_eq!(read_len("239, 187, 191, 97", "UTF8", 3), "2 [97] pos 5");
    assert_eq!(read_len("97, 195", "UTF8", 3), "2 [97] pos 3");
    assert_eq!(read_len("255", "UTF8", 3), "err InvalidStreamData");
}

// StreamReadMatrix.al:533 Matrix_Utf8Bom `u8 len -1`: the count over a mark leaves the mark out.
#[test]
fn a_utf8_read_without_a_length_over_a_mark_counts_the_text_only() {
    assert_eq!(
        read_with("239, 187, 191, 97", "UTF8", "N := InS.Read(T);"),
        "2 [97] pos 5"
    );
}

// W7StreamRead.al:298-299 (`Windows 5 bytes with a zero byte` 3, 97 98); StreamReadMatrix.al:189 Matrix_AbcZero `win len 3`, :222
// Matrix_AbcZeroDef `win len 3` (rest `def`), :249-250 Matrix_TwoZeros `win len 0` and `win len 3`.
#[test]
fn a_windows_read_with_a_length_stops_at_a_zero_byte_and_counts_it() {
    assert_eq!(read_len("97, 98, 0, 99, 100", "Windows", 5), "3 [97 98] pos 4");
    assert_eq!(read_len("97, 98, 99, 0, 100", "Windows", 3), "4 [97 98 99] pos 5");
    assert_eq!(read_len("0, 0", "Windows", 0), "1 [] pos 2");
    assert_eq!(read_len("0, 0", "MSDos", 3), "1 [] pos 2");
    assert_eq!(read_len("97, 98, 99", "Windows", 0), "0 [] pos 1");
}

// W7StreamRead.al:237-249 ReadTextWithLength_BytesFrom128To159 (Windows lines): code page 1252, and the count past the end.
#[test]
fn a_windows_read_with_a_length_reads_bytes_128_to_159_through_code_page_1252() {
    let bytes = "65, 128, 142, 150, 159, 160, 233, 66";
    assert_eq!(read_len(bytes, "Windows", 3), "3 [65 8364 381] pos 4");
    assert_eq!(
        read_len(bytes, "Windows", 100),
        "8 [65 8364 381 8211 376 160 233 66] pos 9"
    );
}

// StreamUtf8CharsRead.al:196-197 Utf8_Euro_ReadVariants `ReadText(Text,1)`; W7StreamRead.al issue 7113 point 2.
#[test]
fn a_utf8_read_text_with_a_length_counts_characters() {
    assert_eq!(
        read_with("226, 130, 172", "UTF8", "InS.ReadText(T, 1); N := 0;"),
        "0 [8364] pos 4"
    );
    assert_eq!(
        read_with("240, 159, 152, 128", "UTF8", "InS.ReadText(T, 1); N := 0;"),
        "0 [55357 56832] pos 5"
    );
}
