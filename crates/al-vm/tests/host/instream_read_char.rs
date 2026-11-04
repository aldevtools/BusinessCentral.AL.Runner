//! InStream.Read into a Char: a stream of the default or MS-DOS encoding reads one byte and gives the Char the Byte to Char table
//! maps it to; a Windows stream gives the byte as it is; a UTF-8 or UTF-16 stream reads two bytes as one little-endian code
//! unit, and one byte left is read as 1 with the Char 0 (corpus xmlports/shapes-properties/AyXmlPortAttributeWidth.al:295-420
//! ReadChar_*; set 1 = FF FE 41 00 E9 00 0D 00 0A 00 42, set 2 = 41 C3 A9 42 0A 43).

use al_testkit::text;

fn src(enc: &str, set: i32) -> String {
    format!(
        r#"
table 50195 B {{ TableType = Temporary; fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50195 C
{{
    local procedure SetNo(): Integer begin exit({set}); end;

    procedure F(): Text
    var
        Rec: Record B temporary; OutS: OutStream; InS: InStream; Bytes: List of [Integer]; V: Integer; By: Byte; Ch: Char; N: Integer; R: Text; I: Integer; Code: Integer;
    begin
        if SetNo() = 1 then Bytes.AddRange(255, 254, 65, 0, 233, 0, 13, 0, 10, 0, 66) else Bytes.AddRange(65, 195, 169, 66, 10, 67);
        Rec.Data.CreateOutStream(OutS);
        foreach V in Bytes do begin By := V; OutS.Write(By); end;
        Rec.Data.CreateInStream(InS{enc});
        while (not InS.EOS()) and (I < 12) do begin
            I += 1; Ch := 0; N := InS.Read(Ch); Code := Ch;
            R += Format(N) + ':' + Format(Code) + ';';
        end;
        exit(R);
    end;
}}
"#
    )
}

fn reads(enc: &str, set: i32) -> String {
    text(al_testkit::run_in(&src(enc, set), "C", "F"))
}

#[test]
fn a_default_or_msdos_stream_reads_a_byte_through_the_byte_to_char_table() {
    let set1 = "1:160;1:254;1:65;1:0;1:210;1:0;1:13;1:0;1:10;1:0;1:66;";
    let set2 = "1:65;1:8220;1:20;1:66;1:10;1:67;";
    assert_eq!(reads("", 1), set1);
    assert_eq!(reads("", 2), set2);
    assert_eq!(reads(", TextEncoding::MSDos", 1), set1);
    assert_eq!(reads(", TextEncoding::MSDos", 2), set2);
}

#[test]
fn a_windows_stream_reads_a_byte_as_it_is() {
    assert_eq!(reads(", TextEncoding::Windows", 2), "1:65;1:195;1:169;1:66;1:10;1:67;");
}

#[test]
fn a_unicode_stream_reads_two_bytes_as_one_code_unit() {
    for enc in [", TextEncoding::UTF8", ", TextEncoding::UTF16"] {
        assert_eq!(reads(enc, 1), "2:65279;2:65;2:233;2:13;2:10;1:0;");
        assert_eq!(reads(enc, 2), "2:49985;2:17065;2:17162;");
    }
}

// StTStreams.al:235 ReadPastEnd: a Char read after the data ended raises like the other types.
#[test]
fn a_char_read_past_the_end_raises_the_stream_data_error() {
    let src = r#"
table 50197 B { TableType = Temporary; fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50197 C
{
    procedure F(): Text
    var Rec: Record B temporary; OutS: OutStream; InS: InStream; T: Text; Int: Integer; Ch: Char;
    begin
        Rec.Data.CreateOutStream(OutS);
        OutS.WriteText('x');
        Rec.Data.CreateInStream(InS);
        InS.ReadText(T);
        InS.ReadText(T);
        asserterror InS.Read(Int);
        asserterror InS.Read(Ch);
        exit(GetLastErrorCode());
    end;
}
"#;
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "InvalidStreamData");
}
