//! InStream.Read(Text) and Read(Text, Length) on every encoding, as streams/instream-outstream/StreamReadMatrix.al (codeunit
//! 67081) observes them: the count, the character codes, EOS and what a second Read(Text) gives for the rest.

use al_testkit::text;

/// Runs the corpus `Run` procedure of StreamReadMatrix.al:56-92 over `bytes` written one Byte at a time through a stream of the
/// default encoding. `enc` is `def` (no encoding) or a TextEncoding member; `len` -1 reads without a length.
fn run(bytes: &str, enc: &str, len: i32) -> String {
    let open = if enc == "def" {
        "Rec.Data.CreateInStream(InS);".to_string()
    } else {
        format!("Rec.Data.CreateInStream(InS, TextEncoding::{enc});")
    };
    let add = if bytes.is_empty() {
        String::new()
    } else {
        format!("Bytes.AddRange({bytes});")
    };
    let src = format!(
        r#"
table 50197 B {{ TableType = Temporary; fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50197 C
{{
    local procedure Codes(Value: Text): Text
    var
        i: Integer; Code: Integer; Result: Text;
    begin
        for i := 1 to StrLen(Value) do begin
            if i > 1 then Result += ' ';
            Code := Value[i];
            Result += Format(Code);
        end;
        exit(Result);
    end;

    [TryFunction]
    local procedure TryRead(var InS: InStream; var Result: Text; var Count: Integer; Length: Integer)
    begin
        if Length < 0 then
            Count := InS.Read(Result)
        else
            Count := InS.Read(Result, Length);
    end;

    procedure F(): Text
    var
        Rec: Record B temporary; OutS: OutStream; InS: InStream; Bytes: List of [Integer]; V: Integer; By: Byte;
        Result: Text; Rest: Text; Count: Integer; RestCount: Integer; Info: Text;
    begin
        {add}
        Rec.Data.CreateOutStream(OutS);
        foreach V in Bytes do begin By := V; OutS.Write(By); end;
        {open}
        Result := 'unset';
        if not TryRead(InS, Result, Count, {len}) then
            exit('error ' + GetLastErrorCode());
        Info := Format(Count) + ' [' + Codes(Result) + '] eos ' + Format(InS.EOS());
        if not InS.EOS() then begin
            Rest := 'unset';
            if TryRead(InS, Rest, RestCount, -1) then
                Info += ' rest ' + Format(RestCount) + ' [' + Codes(Rest) + ']'
            else
                Info += ' rest error';
        end;
        exit(Info);
    end;
}}
"#
    );
    text(al_testkit::run_in(&src, "C", "F"))
}

/// The bytes of one matrix test and the fifteen lines it expects, in the order def, u8, u16, dos, win and -1, 0, 3.
fn check(bytes: &str, expected: [&str; 15]) {
    let encs = ["def", "UTF8", "UTF16", "MSDos", "Windows"];
    let lens = [-1, 0, 3];
    for (i, want) in expected.iter().enumerate() {
        let (enc, len) = (encs[i / 3], lens[i % 3]);
        assert_eq!(run(bytes, enc, len), *want, "{enc} len {len} of [{bytes}]");
    }
}

// StreamReadMatrix.al:145-159 Matrix_Abc.
#[test]
fn abc_without_a_terminator() {
    check(
        "97, 98, 99",
        [
            "3 [97 98 99] eos Yes",
            "0 [] eos No rest 3 [97 98 99]",
            "3 [97 98 99] eos Yes",
            "4 [97 98 99] eos Yes",
            "1 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos Yes",
            "4 [25185] eos Yes",
            "2 [] eos No rest 4 [25185]",
            "4 [25185] eos Yes",
            "3 [97 98 99] eos Yes",
            "0 [] eos No rest 3 [97 98 99]",
            "3 [97 98 99] eos Yes",
            "3 [97 98 99] eos Yes",
            "0 [] eos No rest 3 [97 98 99]",
            "3 [97 98 99] eos Yes",
        ],
    );
}

// StreamReadMatrix.al:116-130 Matrix_Empty.
#[test]
fn an_empty_stream() {
    check(
        "",
        [
            "0 [] eos Yes",
            "0 [] eos Yes",
            "0 [] eos Yes",
            "1 [] eos Yes",
            "1 [] eos Yes",
            "1 [] eos Yes",
            "2 [] eos Yes",
            "2 [] eos Yes",
            "2 [] eos Yes",
            "0 [] eos Yes",
            "0 [] eos Yes",
            "0 [] eos Yes",
            "0 [] eos Yes",
            "0 [] eos Yes",
            "0 [] eos Yes",
        ],
    );
}

// StreamReadMatrix.al:175-189 Matrix_AbcZero.
#[test]
fn abc_and_a_zero_byte() {
    check(
        "97, 98, 99, 0",
        [
            "4 [97 98 99] eos Yes",
            "0 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos Yes",
            "4 [97 98 99] eos Yes",
            "1 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos No rest 1 []",
            "6 [25185 99] eos Yes",
            "2 [] eos No rest 6 [25185 99]",
            "6 [25185 99] eos Yes",
            "4 [97 98 99] eos Yes",
            "0 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos Yes",
            "4 [97 98 99] eos Yes",
            "0 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos Yes",
        ],
    );
}

// StreamReadMatrix.al:208-222 Matrix_AbcZeroDef.
#[test]
fn abc_a_zero_byte_and_def() {
    check(
        "97, 98, 99, 0, 100, 101, 102",
        [
            "4 [97 98 99] eos No rest 3 [100 101 102]",
            "0 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos No rest 3 [100 101 102]",
            "4 [97 98 99] eos No rest 4 [100 101 102]",
            "1 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos No rest 1 []",
            "8 [25185 99 25956] eos Yes",
            "2 [] eos No rest 8 [25185 99 25956]",
            "8 [25185 99 25956] eos No rest 2 []",
            "4 [97 98 99] eos No rest 3 [100 101 102]",
            "0 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos No rest 3 [100 101 102]",
            "4 [97 98 99] eos No rest 3 [100 101 102]",
            "0 [] eos No rest 4 [97 98 99]",
            "4 [97 98 99] eos No rest 3 [100 101 102]",
        ],
    );
}

// StreamReadMatrix.al:236-250 Matrix_TwoZeros.
#[test]
fn two_zero_bytes() {
    check(
        "0, 0",
        [
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "2 [] eos Yes",
            "2 [] eos No rest 2 []",
            "2 [] eos Yes",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
            "1 [] eos No rest 1 []",
        ],
    );
}

// StreamReadMatrix.al:264-278 Matrix_EacuteUtf8.
#[test]
fn e_acute_in_utf8() {
    check(
        "195, 169",
        [
            "2 [8220 20] eos Yes",
            "0 [] eos No rest 2 [8220 20]",
            "2 [8220 20] eos Yes",
            "3 [233] eos Yes",
            "1 [] eos No rest 3 [233]",
            "3 [233] eos Yes",
            "4 [43459] eos Yes",
            "2 [] eos No rest 4 [43459]",
            "4 [43459] eos Yes",
            "2 [8220 20] eos Yes",
            "0 [] eos No rest 2 [8220 20]",
            "2 [8220 20] eos Yes",
            "2 [195 169] eos Yes",
            "0 [] eos No rest 2 [195 169]",
            "2 [195 169] eos Yes",
        ],
    );
}

// StreamReadMatrix.al:291-305 Matrix_EacuteLatin1.
#[test]
fn e_acute_in_latin1() {
    check(
        "233",
        [
            "1 [210] eos Yes",
            "0 [] eos No rest 1 [210]",
            "1 [210] eos Yes",
            "1 [] eos Yes",
            "1 [] eos No rest 1 []",
            "1 [] eos Yes",
            "2 [] eos Yes",
            "2 [] eos No rest 2 []",
            "2 [] eos Yes",
            "1 [210] eos Yes",
            "0 [] eos No rest 1 [210]",
            "1 [210] eos Yes",
            "1 [233] eos Yes",
            "0 [] eos No rest 1 [233]",
            "1 [233] eos Yes",
        ],
    );
}

// StreamReadMatrix.al:318-336 Matrix_ByteFF.
#[test]
fn the_byte_ff() {
    check(
        "255",
        [
            "1 [160] eos Yes",
            "0 [] eos No rest 1 [160]",
            "1 [160] eos Yes",
            "error InvalidStreamData",
            "1 [] eos No rest error",
            "error InvalidStreamData",
            "2 [] eos Yes",
            "2 [] eos No rest 2 []",
            "2 [] eos Yes",
            "1 [160] eos Yes",
            "0 [] eos No rest 1 [160]",
            "1 [160] eos Yes",
            "1 [255] eos Yes",
            "0 [] eos No rest 1 [255]",
            "1 [255] eos Yes",
        ],
    );
}

// StreamReadMatrix.al:350-364 Matrix_TruncatedUtf8.
#[test]
fn a_truncated_utf8_sequence() {
    check(
        "97, 195",
        [
            "2 [97 8220] eos Yes",
            "0 [] eos No rest 2 [97 8220]",
            "2 [97 8220] eos Yes",
            "2 [97] eos Yes",
            "1 [] eos No rest 2 [97]",
            "2 [97] eos Yes",
            "4 [50017] eos Yes",
            "2 [] eos No rest 4 [50017]",
            "4 [50017] eos Yes",
            "2 [97 8220] eos Yes",
            "0 [] eos No rest 2 [97 8220]",
            "2 [97 8220] eos Yes",
            "2 [97 195] eos Yes",
            "0 [] eos No rest 2 [97 195]",
            "2 [97 195] eos Yes",
        ],
    );
}

// StreamReadMatrix.al:380-394 Matrix_Crlf.
#[test]
fn cr_lf() {
    check(
        "97, 13, 10, 98",
        [
            "4 [97 13 10 98] eos Yes",
            "0 [] eos No rest 4 [97 13 10 98]",
            "3 [97 13 10] eos No rest 1 [98]",
            "5 [97 13 10 98] eos Yes",
            "1 [] eos No rest 5 [97 13 10 98]",
            "4 [97 13 10] eos No rest 2 [98]",
            "6 [3425 25098] eos Yes",
            "2 [] eos No rest 6 [3425 25098]",
            "6 [3425 25098] eos Yes",
            "4 [97 13 10 98] eos Yes",
            "0 [] eos No rest 4 [97 13 10 98]",
            "3 [97 13 10] eos No rest 1 [98]",
            "4 [97 13 10 98] eos Yes",
            "0 [] eos No rest 4 [97 13 10 98]",
            "3 [97 13 10] eos No rest 1 [98]",
        ],
    );
}

// StreamReadMatrix.al:410-424 Matrix_Utf16Ab.
#[test]
fn utf16_ab() {
    check(
        "97, 0, 98, 0",
        [
            "2 [97] eos No rest 2 [98]",
            "0 [] eos No rest 2 [97]",
            "2 [97] eos No rest 2 [98]",
            "2 [97] eos No rest 2 [98]",
            "1 [] eos No rest 2 [97]",
            "2 [97] eos No rest 2 [98]",
            "6 [97 98] eos Yes",
            "2 [] eos No rest 6 [97 98]",
            "6 [97 98] eos Yes",
            "2 [97] eos No rest 2 [98]",
            "0 [] eos No rest 2 [97]",
            "2 [97] eos No rest 2 [98]",
            "2 [97] eos No rest 2 [98]",
            "0 [] eos No rest 2 [97]",
            "2 [97] eos No rest 2 [98]",
        ],
    );
}

// StreamReadMatrix.al:439-453 Matrix_Utf16Odd.
#[test]
fn utf16_with_an_odd_byte() {
    check(
        "97, 0, 98",
        [
            "2 [97] eos No rest 1 [98]",
            "0 [] eos No rest 2 [97]",
            "2 [97] eos No rest 1 [98]",
            "2 [97] eos No rest 2 [98]",
            "1 [] eos No rest 2 [97]",
            "2 [97] eos No rest 2 [98]",
            "4 [97] eos Yes",
            "2 [] eos No rest 4 [97]",
            "4 [97] eos Yes",
            "2 [97] eos No rest 1 [98]",
            "0 [] eos No rest 2 [97]",
            "2 [97] eos No rest 1 [98]",
            "2 [97] eos No rest 1 [98]",
            "0 [] eos No rest 2 [97]",
            "2 [97] eos No rest 1 [98]",
        ],
    );
}

// StreamReadMatrix.al:469-483 Matrix_EuroUtf8.
#[test]
fn the_euro_sign_in_utf8() {
    check(
        "226, 130, 172, 98",
        [
            "4 [203 233 188 98] eos Yes",
            "0 [] eos No rest 4 [203 233 188 98]",
            "3 [203 233 188] eos No rest 1 [98]",
            "5 [8364 98] eos Yes",
            "1 [] eos No rest 5 [8364 98]",
            "5 [8364 98] eos Yes",
            "6 [33506 25260] eos Yes",
            "2 [] eos No rest 6 [33506 25260]",
            "6 [33506 25260] eos Yes",
            "4 [203 233 188 98] eos Yes",
            "0 [] eos No rest 4 [203 233 188 98]",
            "3 [203 233 188] eos No rest 1 [98]",
            "4 [226 8218 172 98] eos Yes",
            "0 [] eos No rest 4 [226 8218 172 98]",
            "3 [226 8218 172] eos No rest 1 [98]",
        ],
    );
}

// StreamReadMatrix.al:499-513 Matrix_Utf8Bom.
#[test]
fn a_utf8_byte_order_mark() {
    check(
        "239, 187, 191, 97",
        [
            "4 [217 8249 143 97] eos Yes",
            "0 [] eos No rest 4 [217 8249 143 97]",
            "3 [217 8249 143] eos No rest 1 [97]",
            "2 [97] eos Yes",
            "1 [] eos No rest 2 [97]",
            "2 [97] eos Yes",
            "6 [48111 25023] eos Yes",
            "2 [] eos No rest 6 [48111 25023]",
            "6 [48111 25023] eos Yes",
            "4 [217 8249 143 97] eos Yes",
            "0 [] eos No rest 4 [217 8249 143 97]",
            "3 [217 8249 143] eos No rest 1 [97]",
            "4 [239 187 191 97] eos Yes",
            "0 [] eos No rest 4 [239 187 191 97]",
            "3 [239 187 191] eos No rest 1 [97]",
        ],
    );
}

// StreamReadMatrix.al:529-547 Matrix_Utf16Bom.
#[test]
fn a_utf16_byte_order_mark() {
    check(
        "255, 254, 97, 0",
        [
            "4 [160 254 97] eos Yes",
            "0 [] eos No rest 4 [160 254 97]",
            "4 [160 254 97] eos Yes",
            "error InvalidStreamData",
            "1 [] eos No rest error",
            "error InvalidStreamData",
            "4 [97] eos Yes",
            "2 [] eos No rest 4 [97]",
            "4 [97] eos Yes",
            "4 [160 254 97] eos Yes",
            "0 [] eos No rest 4 [160 254 97]",
            "4 [160 254 97] eos Yes",
            "4 [255 254 97] eos Yes",
            "0 [] eos No rest 4 [255 254 97]",
            "4 [255 254 97] eos Yes",
        ],
    );
}

// streams/PnE04Streams.al:162-183 (`utf16` reads `4|w` of 77 00 F6 00 72 00) and :237-250 (`utf16len4` reads `4|h` of `hey!`):
// bytes written one Byte at a time through a UTF-16 OutStream read otherwise than the matrix shows; that stays refused.
#[test]
fn a_utf16_read_of_bytes_written_through_a_utf16_out_stream_is_refused() {
    let src = r#"
table 50198 B { TableType = Temporary; fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50198 C
{
    procedure F(): Integer
    var
        Rec: Record B temporary; OutS: OutStream; InS: InStream; By: Byte; T: Text;
    begin
        Rec.Data.CreateOutStream(OutS, TextEncoding::UTF16);
        By := 119; OutS.Write(By);
        By := 0; OutS.Write(By);
        Rec.Data.CreateInStream(InS, TextEncoding::UTF16);
        exit(InS.Read(T));
    end;
}
"#;
    let r = al_testkit::run_in(src, "C", "F");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
