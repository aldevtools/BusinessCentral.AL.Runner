//! An import of a source that is no UTF-8 or UTF-16 text: a document that declares iso-8859-1 is read as Latin-1 (corpus
//! xmlport/lt-xmlport-query-datatransfer/XpTEncoding.al:235 Encoding_ImportXmlDeclaredLatin1: `Mü` imports), and a text port
//! whose root is a table element fails with the root error before the bytes are decoded (:165 Encoding_ImportCsvWindows, :179
//! Encoding_ImportCsvUtf16).

use al_builtins::native::Signal;
use al_testkit::run;

fn src(port: &str, enc: &str, doc: &str) -> String {
    format!(
        r#"
table 69990 "XIE Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69991 "XIE Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
{port}
codeunit 69993 C
{{
    procedure Import(): Text
    var B: Record "XIE Blob" temporary; Row: Record "XIE Row"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::{enc});
        OutS.WriteText('{doc}');
        B.Data.CreateInStream(InS, TextEncoding::{enc});
        Xmlport.Import(Xmlport::"XIE Port", InS);
        Row.FindFirst();
        exit(Format(Row.Id) + '/' + Row.Name);
    end;
}}
"#
    )
}

const XML_PORT: &str = r#"xmlport 69992 "XIE Port"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; "XIE Row") { XmlName = 'Item'; fieldelement(Id; R.Id) { } fieldelement(Name; R.Name) { } } } }
}"#;

fn csv_port(enc: &str) -> String {
    format!(
        r#"xmlport 69992 "XIE Port"
{{
    Direction = Import; Format = VariableText; FieldSeparator = ','; TextEncoding = {enc}; UseRequestPage = false;
    schema {{ tableelement(R; "XIE Row") {{ fieldelement(Id; R.Id) {{ }} fieldelement(Name; R.Name) {{ }} }} }}
}}"#
    )
}

#[test]
fn a_latin1_document_imports() {
    let doc = "<?xml version=\"1.0\" encoding=\"iso-8859-1\"?><Root><Item><Id>1</Id><Name>Mü</Name></Item></Root>";
    assert_eq!(al_testkit::text(run(&src(XML_PORT, "Windows", doc), "Import")), "1/Mü");
}

#[test]
fn a_text_port_with_a_table_root_fails_with_the_root_error() {
    let msg =
        "If you choose XMLport format Fixed Text or Variable Text, then the root element must have source type Text.";
    for (port_enc, enc, doc) in [("WINDOWS", "Windows", "1,Müller ß"), ("UTF16", "UTF16", "1,Müller €")] {
        match run(&src(&csv_port(port_enc), enc, doc), "Import") {
            Err(Signal::Error(e)) => assert_eq!((e.message.as_str(), e.code.as_str()), (msg, "XmlPortInvalidNodeType")),
            other => panic!("{other:?}"),
        }
    }
}
