//! `OutStream.Write(Text)` and `BigText.Write(OutStream)` of characters a code page lacks and of characters outside the Basic
//! Multilingual Plane. Pinned by streams/P132OutStreamUnicode.al:67-112 (the default stream), streams/P129StreamWriteText.al:126-141
//! (a surrogate pair on each encoding) and streams/AuEdgeTests.al:250-318 (BigText.Write on each encoding).

const SRC: &str = r#"
table 60009 "OWT Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50100 C
{
    local procedure Hex(var B: Record "OWT Blob" temporary): Text
    var InS: InStream; Byte1: Byte; Code: Integer; R: Text;
    begin
        B.Data.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(Byte1);
            Code := Byte1;
            R += Format(Code) + ' ';
        end;
        exit(R);
    end;

    procedure Default(Value: Text): Text
    var B: Record "OWT Blob" temporary; OutS: OutStream; Ret: Integer;
    begin
        B.Data.CreateOutStream(OutS);
        Ret := OutS.Write(Value);
        exit(Format(Ret) + ':' + Hex(B));
    end;

    procedure WithEnc(Enc: TextEncoding; Value: Text): Text
    var B: Record "OWT Blob" temporary; OutS: OutStream; Ret: Integer;
    begin
        B.Data.CreateOutStream(OutS, Enc);
        Ret := OutS.Write(Value);
        exit(Format(Ret) + ':' + Hex(B));
    end;

    procedure Smile(): Text
    var Hi: Text[1]; Lo: Text[1];
    begin
        Hi[1] := 55357;
        Lo[1] := 56832;
        exit('a' + Hi + Lo + 'b');
    end;

    procedure Other(): Text
    var T: Text;
    begin
        T[1] := 233; T[2] := 223; T[3] := 8220; T[4] := 937; T[5] := 20013; T[6] := 55357; T[7] := 56832; T[8] := 160;
        exit(T);
    end;

    procedure Big(Enc: Integer): Text
    var B: Record "OWT Blob" temporary; OutS: OutStream; BT: BigText;
    begin
        BT.AddText(Other());
        case Enc of
            0: B.Data.CreateOutStream(OutS);
            1: B.Data.CreateOutStream(OutS, TextEncoding::Windows);
            2: B.Data.CreateOutStream(OutS, TextEncoding::MSDos);
            3: B.Data.CreateOutStream(OutS, TextEncoding::UTF16);
        end;
        BT.Write(OutS);
        exit(Format(BT.Length()) + ':' + Hex(B));
    end;

    procedure BigDefault(): Text begin exit(Big(0)); end;
    procedure BigWindows(): Text begin exit(Big(1)); end;
    procedure BigMsDos(): Text begin exit(Big(2)); end;
    procedure BigUtf16(): Text begin exit(Big(3)); end;
    procedure WriteLen(Enc: TextEncoding; Value: Text; Len: Integer): Text
    var B: Record "OWT Blob" temporary; OutS: OutStream;
    begin
        B.Data.CreateOutStream(OutS, Enc);
        OutS.WriteText(Value, Len);
        exit(Hex(B));
    end;

    procedure LenUtf8(): Text begin exit(WriteLen(TextEncoding::UTF8, 'abcdef', 3) + '/' + WriteLen(TextEncoding::UTF8, 'abc', -1)); end;
    procedure LenUtf16(): Text begin exit(WriteLen(TextEncoding::UTF16, 'ab', 1)); end;
    procedure LenWindows(): Text begin exit(WriteLen(TextEncoding::Windows, 'äöü€', 3)); end;
    procedure LenMsDos(): Text begin exit(WriteLen(TextEncoding::MSDos, 'äöü€', 3)); end;
    procedure LenWindowsLonger(): Text begin exit(WriteLen(TextEncoding::Windows, 'abc', 5)); end;
    procedure Greek(): Text begin exit(Default('αβγ')); end;
    procedure Macron(): Text begin exit(Default('Āā')); end;
    procedure Ligature(): Text begin exit(Default('Œœ')); end;
    procedure Emoji(): Text begin exit(Default(Smile())); end;
    procedure EmojiUtf8(): Text begin exit(WithEnc(TextEncoding::UTF8, Smile())); end;
    procedure EmojiUtf16(): Text begin exit(WithEnc(TextEncoding::UTF16, Smile())); end;
    procedure EmojiWindows(): Text begin exit(WithEnc(TextEncoding::Windows, Smile())); end;
    procedure EmojiMsDos(): Text begin exit(WithEnc(TextEncoding::MSDos, Smile())); end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn characters_the_ms_dos_code_page_lacks_take_their_best_fit_byte() {
    // P132OutStreamUnicode.al:84-89: greek `61E13F00`, u0100 `416100`, oe_ligature `BCCC00`.
    assert_eq!(run("Greek"), "4:97 225 63 0 ");
    assert_eq!(run("Macron"), "3:65 97 0 ");
    assert_eq!(run("Ligature"), "3:188 204 0 ");
}

#[test]
fn a_surrogate_pair_is_two_question_marks_on_a_code_page_and_one_character_on_unicode() {
    // P132OutStreamUnicode.al:109 `613F3F6200`; P129StreamWriteText.al:135-139 (base64 YT8/YgA=, YfCfmIBiAA==, YQA92ADeYgAAAA==).
    assert_eq!(run("Emoji"), "5:97 63 63 98 0 ");
    assert_eq!(run("EmojiMsDos"), "5:97 63 63 98 0 ");
    assert_eq!(run("EmojiWindows"), "5:97 63 63 98 0 ");
    assert_eq!(run("EmojiUtf8"), "7:97 240 159 152 128 98 0 ");
    assert_eq!(run("EmojiUtf16"), "10:97 0 61 216 0 222 98 0 0 0 ");
}

#[test]
fn big_text_write_of_characters_a_code_page_lacks() {
    // AuEdgeTests.al:278-279 `82E1C34F3F3F3FFF`, :295 `E9DF934F3F3F3FA0`, :315-316.
    assert_eq!(run("BigDefault"), "8:130 225 195 79 63 63 63 255 ");
    assert_eq!(run("BigMsDos"), "8:130 225 195 79 63 63 63 255 ");
    assert_eq!(run("BigWindows"), "8:233 223 147 79 63 63 63 160 ");
    assert_eq!(run("BigUtf16"), "8:233 0 223 0 28 32 169 3 45 78 61 216 0 222 160 0 ");
}

#[test]
fn write_text_with_a_length_ignores_it_on_unicode_and_cuts_the_text_on_a_code_page() {
    // streams/P130WriteText.al:54-110: UTF-8 and UTF-16 write the whole text whatever the length; Windows and MS-DOS write
    // `äöü` of `äöü€` for the length 3 (`ok bytes=3 read=äöü/`).
    assert_eq!(run("LenUtf8"), "97 98 99 100 101 102 /97 98 99 ");
    assert_eq!(run("LenUtf16"), "97 0 98 0 ");
    assert_eq!(run("LenWindows"), "228 246 252 ");
    assert_eq!(run("LenMsDos"), "132 148 129 ");
    // A length beyond the text on a code page is not pinned.
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "LenWindowsLonger"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
