//! An `xmlns` attribute made with `XmlAttribute.Create('xmlns', Uri)` on an element (xml/DeXmlWriteLeads.al:246-310).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 50310 X
{
    [TryFunction]
    local procedure TryWrite(E: XmlElement; var T: Text)
    begin
        E.WriteTo(T);
    end;

    local procedure Write(E: XmlElement): Text
    var
        T: Text;
    begin
        if TryWrite(E, T) then
            exit('ok ' + T);
        exit('error ' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure NoNamespace(): Text
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('Root');
        E.Add(XmlAttribute.Create('xmlns', 'urn:a'));
        exit(Write(E));
    end;

    procedure SameNamespace(): Text
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('Root', 'urn:a');
        E.Add(XmlAttribute.Create('xmlns', 'urn:a'));
        exit(Write(E));
    end;

    procedure OtherNamespace(): Text
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('Root', 'urn:b');
        E.SetAttribute('xmlns', 'urn:a');
        exit(Write(E));
    end;
}
"#;

#[test]
fn an_xmlns_attribute_is_written_once_or_cannot_redefine_the_default_namespace() {
    assert_eq!(
        text(run_in(SRC, "X", "NoNamespace")),
        "error The following exception was encountered when processing XML data: 'The prefix '' cannot be redefined from '' to 'urn:a' within the same start element tag.' at line 0 and position 0.|Xml:Xml"
    );
    assert_eq!(text(run_in(SRC, "X", "SameNamespace")), "ok <Root xmlns=\"urn:a\" />");
    assert_eq!(
        text(run_in(SRC, "X", "OtherNamespace")),
        "error The following exception was encountered when processing XML data: 'The prefix '' cannot be redefined from 'urn:b' to 'urn:a' within the same start element tag.' at line 0 and position 0.|Xml:Xml"
    );
}

const ZERO: &str = r#"
table 50311 B { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }

codeunit 50312 Z
{
    [TryFunction]
    local procedure TryRead(var R: Record B)
    var
        InS: InStream;
        Doc: XmlDocument;
    begin
        R.Data.CreateInStream(InS);
        XmlDocument.ReadFrom(InS, Doc);
    end;

    procedure F(): Text
    var
        R: Record B;
        OutS: OutStream;
    begin
        R.Data.CreateOutStream(OutS);
        OutS.Write('<Envelope><Id>1</Id></Envelope>');
        if TryRead(R) then
            exit('ok');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

// DeXmlWriteLeads.al:85-89: the zero byte `Write(Text)` adds is an invalid character after the root element.
#[test]
fn the_zero_byte_after_the_root_element_is_an_invalid_character() {
    assert_eq!(
        text(run_in(ZERO, "Z", "F")),
        "The following exception was encountered when processing XML data: ''.', hexadecimal value 0x00, is an invalid character. Line 1, position 32.' at line 1 and position 32.|Xml:Xml"
    );
}
