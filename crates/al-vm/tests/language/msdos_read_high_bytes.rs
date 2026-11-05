//! InStream.ReadText and BigText.Read of bytes 128 to 255 from a stream created without an encoding (AL Runner issue 6968;
//! corpus streams/instream-outstream/ReadTextMsDosHighBytes.al on branch al-runtime-rs-staging-r6963).

const SRC: &str = r#"
table 69995 "MD Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
codeunit 69996 C
{
    procedure ReadLines(): Text
    var B: Record "MD Blob" temporary; OutS: OutStream; InS: InStream; Line: Text; T: Text; Bytes: array[3] of Char;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('urn:a');
        OutS.WriteText();
        Bytes[1] := 195; Bytes[2] := 188;
        OutS.Write(Bytes[1]); OutS.Write(Bytes[2]);
        OutS.WriteText(' urn:b');
        B.Data.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('urn:') then
                T += CopyStr(Line, StrLen(Line) - 4) + ';';
        end;
        exit(T);
    end;
}
"#;

/// AL Runner issue 6968: C3 BC read without an encoding (the corpus test ReadText_NoEncoding_Utf8BytesBetweenLines reads them as
/// 8220 338).
#[test]
fn utf8_bytes_read_without_an_encoding() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ReadLines")),
        "urn:a;urn:b;"
    );
}

fn codes(first: i32, last: i32, big: bool) -> String {
    let read = if big {
        "BigT.Read(InS); BigT.GetSubText(Line, 1, BigT.Length());"
    } else {
        "InS.ReadText(Line);"
    };
    let src = format!(
        r#"
table 69998 "MD Blob 2" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
codeunit 69997 T
{{
    procedure Codes(): Text
    var B: Record "MD Blob 2" temporary; OutS: OutStream; InS: InStream; Byte: Byte; V: Integer; Line: Text; BigT: BigText; i: Integer; C: Integer; R: Text;
    begin
        B.Data.CreateOutStream(OutS);
        for V := {first} to {last} do begin Byte := V; OutS.Write(Byte); end;
        B.Data.CreateInStream(InS);
        {read}
        for i := 1 to StrLen(Line) do begin C := Line[i]; if R <> '' then R += ' '; R += Format(C); end;
        exit(R);
    end;
}}
"#
    );
    al_testkit::text(al_testkit::run_in(&src, "T", "Codes"))
}

/// Every byte 128 to 255 through ReadText and BigText.Read of a stream without an encoding (corpus branch
/// al-runtime-rs-staging-r6963, streams/instream-outstream/ReadTextMsDosHighBytes.al ReadText_NoEncoding_Bytes128To159 ... _224To255).
#[test]
fn every_high_byte_reads_as_the_probe_measured() {
    let cases = [
        (
            128,
            159,
            "199 252 233 226 228 224 229 231 234 235 232 239 238 236 196 197 201 230 198 244 246 242 251 249 255 214 220 162 163 165 15 402",
        ),
        (
            160,
            191,
            "225 237 243 250 241 209 170 186 191 20 172 189 188 161 171 187 21 8364 129 8218 8222 8230 8224 8225 710 8240 352 8249 338 141 381 143",
        ),
        (
            192,
            223,
            "144 8216 8217 8220 8221 8226 8211 8212 732 8482 353 8250 339 157 382 376 168 169 173 174 175 179 180 184 185 190 192 193 194 166 195 200",
        ),
        (
            224,
            255,
            "202 223 203 204 205 206 181 207 208 210 211 212 213 215 216 217 218 177 219 221 222 227 247 240 176 245 183 248 253 178 254 160",
        ),
    ];
    for (first, last, want) in cases {
        for big in [false, true] {
            assert_eq!(codes(first, last, big), want, "{first}-{last} big {big}");
        }
    }
}
