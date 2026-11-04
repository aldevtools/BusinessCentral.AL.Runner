//! An import into a port whose Root text element and table element both carry a NamespacePrefix fails with the empty-prefix
//! error, whatever the document (corpus xmlports/import/XpTNamespaces.al:44-97 Import_Prefixed*, :186
//! Export_PrefixedNamespace_Utf16, :231 Import_PrefixedAsExported). Nothing is inserted.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(doc: &str) -> String {
    format!(
        r#"
table 69980 "XPI Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "XPI Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69982 "XPI Port"
{{
    Direction = Both; Format = Xml; UseRequestPage = false; Namespaces = ns1 = 'urn:lt:xpt';
    schema
    {{
        textelement(Root)
        {{
            NamespacePrefix = 'ns1';
            tableelement(R; "XPI Row") {{ XmlName = 'Item'; NamespacePrefix = 'ns1'; fieldelement(Id; R.Id) {{ }} }}
        }}
    }}
}}
codeunit 69983 C
{{
    procedure Import(): Text
    var B: Record "XPI Blob" temporary; Row: Record "XPI Row"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('{doc}');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XPI Port", InS);
        exit(Format(Row.Count()));
    end;
}}
"#
    )
}

const ERR: &str = "The prefix  is not defined in the Namespaces property of the XMLPort.|XmlPortNodeProcess";

fn import(doc: &str) -> String {
    match run(&src(doc), "Import") {
        Err(Signal::Error(e)) => format!("{}|{}", e.message, e.code),
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_document_fails_with_the_empty_prefix_error() {
    for doc in [
        "<ns1:Root xmlns:ns1=\"urn:lt:xpt\"><ns1:Item><Id>1</Id></ns1:Item></ns1:Root>",
        "<x:Root xmlns:x=\"urn:lt:xpt\"><x:Item><x:Id>1</x:Id></x:Item></x:Root>",
        "<ns1:Root xmlns:ns1=\"urn:other\"><ns1:Item><ns1:Id>1</ns1:Id></ns1:Item></ns1:Root>",
        "<Root><Item><Id>1</Id></Item></Root>",
    ] {
        assert_eq!(import(doc), ERR, "{doc}");
    }
}
