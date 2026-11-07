//! `XmlDocument.ReadFrom(InStream)` of UTF-16 bytes whose declaration names another encoding
//! (xml/AyXmlReadFromStream.al).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const TABLE: &str = r#"
table 60008 "ALT Blob"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(3; Data; Blob) { }
    }
    keys { key(PK; "Code") { } }
}
"#;

/// Writes a UTF-16 LE byte order mark (when `bom`), then `body` as UTF-16 LE, and reads it with the statement form
/// of `ReadFrom` inside a TryFunction, as the corpus test's `TryRead` does.
fn read(bom: bool, body: &str) -> Result<Value, Signal> {
    let bom = if bom {
        "V := 255; O.Write(V); V := 254; O.Write(V);"
    } else {
        ""
    };
    let src = format!(
        "{TABLE}\ncodeunit 50100 C {{
         [TryFunction] local procedure TryRead(var I: InStream; var D: XmlDocument) begin XmlDocument.ReadFrom(I, D); end;
         procedure F(): Text var B: Record \"ALT Blob\"; O: OutStream; I: InStream; D: XmlDocument; R: XmlElement; V: Byte;
         begin B.Data.CreateOutStream(O, TextEncoding::UTF16); {bom} O.WriteText('{body}'); B.Data.CreateInStream(I);
         ClearLastError(); if TryRead(I, D) then begin D.GetRoot(R); exit('ok ' + R.Name()); end;
         exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(I.EOS())); end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

fn text(bom: bool, body: &str) -> String {
    match read(bom, body) {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn utf16_bom_with_a_utf8_declaration_fails_after_the_declaration() {
    // AyXmlReadFromStream.al:103-112 ReadFrom_le_bom_decl8: the reader switches to UTF-8 after the declaration and
    // meets the zero byte of `<`.
    assert_eq!(
        text(true, "<?xml version=\"1.0\" encoding=\"utf-8\"?><r>a</r>"),
        "The following exception was encountered when processing XML data: 'Name cannot begin with the '.' character, \
         hexadecimal value 0x00. Line 1, position 40.' at line 1 and position 40.|Xml:Xml|No"
    );
}

#[test]
fn utf16_bom_with_a_utf16_declaration_reads() {
    // AyXmlReadFromStream.al:90-98 ReadFrom_le_bom_decl16.
    assert_eq!(
        text(true, "<?xml version=\"1.0\" encoding=\"utf-16\"?><r>a</r>"),
        "ok r"
    );
}

#[test]
fn utf16_with_another_declared_encoding_is_refused() {
    // Not pinned: a UTF-8 declaration without a byte order mark, other text after the declaration, other encodings.
    assert!(matches!(
        read(false, "<?xml version=\"1.0\" encoding=\"utf-8\"?><r>a</r>"),
        Err(Signal::Unsupported(_))
    ));
    assert!(matches!(
        read(true, "<?xml version=\"1.0\" encoding=\"utf-8\"?> <r>a</r>"),
        Err(Signal::Unsupported(_))
    ));
    assert!(matches!(
        read(true, "<?xml version=\"1.0\" encoding=\"iso-8859-1\"?><r>a</r>"),
        Err(Signal::Unsupported(_))
    ));
}
