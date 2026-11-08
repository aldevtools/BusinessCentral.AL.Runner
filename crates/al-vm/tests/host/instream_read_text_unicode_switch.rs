//! InStream.ReadText through a UTF-8 or UTF-16 stream on a Blob that an earlier ReadText already read with the other of
//! the two encodings, as streams/instream-outstream/StreamWrittenText.al (codeunit 71575) observes it. BC then does not
//! decode as the new stream's encoding says (:88, :127, :165), and no rule the corpus pins explains what it does.

use al_builtins::native::Signal;

/// Writes the corpus `Sample` text (StreamWrittenText.al:15-24) through an OutStream with no encoding, reads it with ReadText
/// through each stream in `reads` (TextEncoding members, each on a new InStream of the same Blob) and describes the last read
/// as the corpus `Describe` does (:51-59).
fn run(reads: &[&str]) -> Result<al_builtins::value::Value, Signal> {
    let mut steps = String::new();
    for r in reads {
        steps += &format!(
            "Rec.Data.CreateInStream(InS, TextEncoding::{r}); Cnt := -1; Clear(T); Last := 'ok'; if not TryRead(InS, T, Cnt) then Last := 'err|' + GetLastErrorText() + '|' + GetLastErrorCode();\n"
        );
    }
    let src = format!(
        r#"
table 50198 B {{ TableType = Temporary; fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50198 C
{{
    local procedure Sample(): Text
    var
        Emoji: Text[2];
    begin
        Emoji[1] := 55357;
        Emoji[2] := 56832;
        exit('a' + 'é' + 'ü' + '€' + Emoji + 'b');
    end;

    [TryFunction]
    local procedure TryRead(var InS: InStream; var T: Text; var Cnt: Integer)
    begin
        Cnt := InS.ReadText(T);
    end;

    procedure F(): Text
    var
        Rec: Record B temporary; OutS: OutStream; InS: InStream; T: Text; Cnt: Integer; Last: Text;
    begin
        Rec.Data.CreateOutStream(OutS);
        OutS.WriteText(Sample());
        {steps}
        if Last <> 'ok' then
            exit(Last);
        exit('ret=' + Format(Cnt) + ';len=' + Format(StrLen(T)));
    end;
}}
"#
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn utf16_read_text_after_a_utf8_read_text_of_the_same_blob_is_refused() {
    // StreamWrittenText.al:88 P175_Written_None: after `readText utf8` raised, `readText utf16` of the 7 bytes raises too,
    // where the same bytes on a Blob read only as UTF-16 decode with the odd byte dropped (streams/StTEncodings.al
    // CrossEncodingWindowsToOthers `windows-as-utf16`).
    match run(&["UTF8", "UTF16"]) {
        Err(Signal::Unsupported(m)) => assert!(m.contains("ReadText"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn utf16_read_text_alone_still_reads() {
    assert_eq!(al_testkit::text(run(&["UTF16"])), "ret=6;len=3");
}
