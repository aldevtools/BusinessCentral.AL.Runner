//! A Byte into a Char maps through a fixed table (code page 437 read as Windows-1252), and a Char into a Byte maps back
//! (types/lt-byte-char-high/ByteCharHigh.al, writer Y57).

use al_testkit::text;

const SRC: &str = r#"
table 69971 "BCT Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 69970 C
{
    local procedure Code(C: Char): Integer
    var I: Integer;
    begin
        I := C;
        exit(I);
    end;

    procedure Range(From: Integer; UpTo: Integer): Text
    var B: Byte; B2: Byte; C: Char; N: Integer; Back: Integer; R: Text;
    begin
        for N := From to UpTo do begin
            B := N;
            C := B;
            B2 := C;
            Back := B2;
            R += Format(N) + ':' + Format(Code(C));
            if Back <> N then R += ' b' + Format(Back);
            R += ',';
        end;
        exit(R);
    end;

    procedure Low(): Text
    var B: Byte; C: Char; N: Integer; R: Text;
    begin
        for N := 0 to 127 do begin
            B := N;
            C := B;
            if Code(C) <> N then R += Format(N) + ':' + Format(Code(C)) + ',';
        end;
        exit(R);
    end;

    procedure Written(N: Integer): Text
    var Blob: Record "BCT Blob" temporary; OutS: OutStream; InS: InStream; B: Byte; C: Char; I: Integer; R: Text;
    begin
        B := N;
        C := B;
        Blob.Data.CreateOutStream(OutS);
        OutS.Write(C);
        OutS.WriteText(Format(C));
        Blob.Data.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(B);
            I := B;
            R += Format(I) + ',';
        end;
        exit(R);
    end;

    procedure Back(): Text
    var B: Byte; C: Char; I: Integer;
    begin
        I := 8364;
        C := I;
        B := C;
        I := B;
        exit(Format(I));
    end;
}
"#;

fn range(from: i32, to: i32) -> String {
    let mut b = al_testkit::Built::from_source(SRC);
    let (o, r) = b.routine_in("C", "Range");
    let (v, _) = b
        .call(
            o,
            r,
            vec![
                al_builtins::value::Value::Integer(from),
                al_builtins::value::Value::Integer(to),
            ],
        )
        .unwrap();
    text(Ok(v))
}

#[test]
fn bytes_from_128_map_through_the_table_and_back() {
    // ByteCharHigh.al:113-118 (ByteChar_Codes_128_159), :155-160 (ByteChar_Codes_224_255).
    assert_eq!(
        range(128, 143),
        "128:199,129:252,130:233,131:226,132:228,133:224,134:229,135:231,136:234,137:235,138:232,139:239,140:238,141:236,142:196,143:197,"
    );
    assert_eq!(
        range(176, 191),
        "176:21,177:8364,178:129,179:8218,180:8222,181:8230,182:8224,183:8225,184:710,185:8240,186:352,187:8249,188:338,189:141,190:381,191:143,"
    );
    assert_eq!(
        range(240, 255),
        "240:218,241:177,242:219,243:221,244:222,245:227,246:247,247:240,248:176,249:245,250:183,251:248,252:253,253:178,254:254,255:160,"
    );
}

#[test]
fn three_bytes_below_128_differ_and_the_euro_goes_back_to_177() {
    // ByteCharHigh.al:163-180 (ByteChar_Codes_0_127), :236-256 (ByteChar_CharAbove255_ToByte `char8364`).
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Low")), "15:164,20:182,21:167,");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Back")), "177");
}

#[test]
fn a_char_of_the_table_is_written_as_its_byte() {
    // ByteCharHigh.al:213-228 (ByteChar_139_Details `stream_write_char*`, `stream_writetext_char*`).
    for n in [139, 200, 255] {
        let mut b = al_testkit::Built::from_source(SRC);
        let (o, r) = b.routine_in("C", "Written");
        let (v, _) = b.call(o, r, vec![al_builtins::value::Value::Integer(n)]).unwrap();
        assert_eq!(text(Ok(v)), format!("{n},{n},"));
    }
}
