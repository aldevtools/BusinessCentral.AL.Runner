//! ADR-068: a text that holds half of a surrogate pair. Real BC keeps the half (AL texts are UTF-16); this runtime keeps
//! U+FFFD in the string and the real half beside it, and halves that meet merge. Readers the corpus pins see the exact
//! units; any other reader refuses.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50993 "LH Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50994 L
{
    local procedure Codes(T: Text): Text
    var i: Integer; R: Text; C: Integer;
    begin
        for i := 1 to StrLen(T) do begin
            C := T[i];
            if i > 1 then R += ',';
            R += Format(C);
        end;
        exit(R);
    end;

    // types/text-code-char/TextCharIndexAssignEdges.al:100-117: the half is stored, the length stays 3.
    procedure IndexAssign(): Text
    var T: Text;
    begin
        T := 'abc';
        T[2] := 55296;
        exit(Format(StrLen(T)) + '|' + Codes(T));
    end;

    // text/text-methods/ForeachText.al:137-148: two one-unit texts joined give the pair; foreach visits 4 units.
    procedure JoinMerges(): Text
    var T: Text; High: Text[1]; Low: Text[1]; Ch: Char; R: Text;
    begin
        High[1] := 55357;
        Low[1] := 56832;
        T := 'a' + High + Low + 'b';
        foreach Ch in T do
            R += Format(Ch + 0) + ' ';
        exit(R + '|' + Format(StrLen(T)) + '|' + T);
    end;

    // text/CrTextLeads.al:421-443: CopyStr and DelStr cut the pair, StrPos finds the low half.
    procedure Cuts(): Text
    var T: Text;
    begin
        T := 'a';
        T[2] := 55357;
        T[3] := 56832;
        T[4] := 'b';
        exit(Codes(CopyStr(T, 1, 2)) + '|' + Codes(CopyStr(T, 3)) + '|' + Codes(CopyStr(T, 3, 1)) + '|' +
            Codes(DelStr(T, 2, 1)) + '|' + Format(StrPos(T, CopyStr(T, 3, 1))));
    end;

    // streams/StTEncodings.al:238-244: Format of half of a pair is the one-unit text that holds it.
    procedure FormatHalf(): Text
    var C: Char; T: Text;
    begin
        C := 55357;
        T := Format(C);
        exit(Codes(T) + '|' + Format(T <> ''));
    end;

    // The Probe Observer's Replace and IndexOf on an observed text with a half (_fixtures/ProbeObserver.Codeunit.al:32,105).
    procedure ReplaceAndIndexOf(): Text
    var T: Text;
    begin
        T := 'abc';
        T[2] := 55296;
        T := T.Replace('c', 'x');
        exit(Codes(T) + '|' + Format(T.IndexOf('x')) + '|' + Codes(T.Substring(2, 1)));
    end;

    // types/text-code-char/TextCharIndexAssignEdges.al:92,108: the character 0 ends the text.
    procedure CharZero(): Text
    var T: Text[5];
    begin
        T := 'abc';
        T[2] := 0;
        exit(Format(StrLen(T)) + T);
    end;

    // Bytes of a stream: the default, MS-DOS and Windows encodings write a character outside the BMP as two `?`
    // (streams/instream-outstream/StreamWrittenText.al:84,199,238); UTF-8 writes a half as U+FFFD (streams/StTEncodings.al:257-265).
    procedure Bytes(Enc: Integer; Half: Boolean): Text
    var Blob: Record "LH Blob" temporary; OutS: OutStream; InS: InStream; B: Byte; I: Integer; R: Text; T: Text;
    begin
        T := 'a😀b';
        if Half then T[3] := 'x';
        case Enc of
            0: Blob.Data.CreateOutStream(OutS);
            1: Blob.Data.CreateOutStream(OutS, TextEncoding::MSDos);
            2: Blob.Data.CreateOutStream(OutS, TextEncoding::Windows);
            3: Blob.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        end;
        OutS.WriteText(T);
        Blob.Data.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(B);
            I := B;
            R += Format(I) + ',';
        end;
        exit(R);
    end;

    // streams/instream-outstream/StreamUtf8CharsRead.al:245-247: an emoji (2 units) read into a Text[1] raises the length error.
    procedure ReadIntoShort(): Text
    var Blob: Record "LH Blob" temporary; OutS: OutStream; InS: InStream; T: Text[1];
    begin
        Blob.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('😀');
        Blob.Data.CreateInStream(InS, TextEncoding::UTF8);
        if TryRead(InS, T) then exit('ok');
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryRead(var InS: InStream; var T: Text[1])
    begin
        InS.Read(T);
    end;

    procedure Unaudited(): Text
    var T: Text;
    begin
        T := 'abc';
        T[2] := 55296;
        exit(DelChr(T, '=', 'a'));
    end;
}
"#;

#[test]
fn a_char_assigned_into_a_text_keeps_the_half() {
    assert_eq!(text(run_in(SRC, "L", "IndexAssign")), "3|97,55296,99");
}

#[test]
fn halves_joined_merge_into_the_character() {
    assert_eq!(text(run_in(SRC, "L", "JoinMerges")), "97 55357 56832 98 |4|a😀b");
}

#[test]
fn copystr_delstr_and_strpos_see_the_halves() {
    assert_eq!(text(run_in(SRC, "L", "Cuts")), "97,55357|56832,98|56832|97,56832,98|3");
}

#[test]
fn an_unaudited_reader_refuses() {
    match al_testkit::run(SRC, "Unaudited") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("half of a surrogate pair read by"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn format_of_a_half_is_the_half() {
    assert_eq!(text(run_in(SRC, "L", "FormatHalf")), "55357|Yes");
}

#[test]
fn replace_indexof_and_substring_see_the_half() {
    assert_eq!(text(run_in(SRC, "L", "ReplaceAndIndexOf")), "97,55296,120|3|55296");
}

#[test]
fn the_character_zero_ends_the_text() {
    assert_eq!(text(run_in(SRC, "L", "CharZero")), "1a");
}

#[test]
fn stream_bytes_of_a_pair_and_a_half() {
    let call = |enc: i32, half: bool| {
        al_testkit::run_with(SRC, "L", "Bytes", vec![Value::Integer(enc), Value::Boolean(half)]).map(|(v, _)| v)
    };
    for enc in 0..=2 {
        assert_eq!(text(call(enc, false)), "97,63,63,98,", "{enc}");
    }
    assert_eq!(text(call(3, true)), "97,239,191,189,120,98,");
    match call(1, true) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("half of a surrogate pair"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_read_longer_than_the_text_variable_raises_the_length_error() {
    assert_eq!(
        text(run_in(SRC, "L", "ReadIntoShort")),
        "StringLengthExceeded|The length of the string is 2, but it must be less than or equal to 1 characters. Value: 😀"
    );
}
