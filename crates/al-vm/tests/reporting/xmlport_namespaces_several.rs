//! XmlPort export with several Namespaces entries, one of them the empty prefix `""` (gap G15, the PEPPOL BIS 3.0 ports): the
//! root declares the prefixed entries in declaration order, then `xmlns=` with the uri of `""` last; an element without a
//! prefix is written without one; UseDefaultNamespace with a DefaultNamespace replaces the uri of `""`. Pinned by corpus
//! tests/al-language/xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespacesTests.al:60, :71, :82, :93 (ports in
//! Y17XmlPortNamespaces.al). The AL below is my own smaller version of those ports (a table of
//! its own with two rows in place of Integer, which the test workspace lacks).

use al_builtins::native::Signal;
use al_testkit::{run, text};

fn src(props: &str) -> String {
    format!(
        r#"
table 69950 "XS Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69954 "XS Hdr" {{ fields {{ field(1; Number; Integer) {{ }} }} keys {{ key(PK; Number) {{ }} }} }}
table 69951 "XS Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69952 "XS Port"
{{
    Format = Xml; Direction = Export; Encoding = UTF8; UseRequestPage = false; {props}
    schema
    {{
        tableelement(Hdr; "XS Hdr")
        {{
            XmlName = 'Doc';
            MaxOccurs = Once;
            SourceTableView = where(Number = filter(1 ..));
            textelement(DocId) {{ NamespacePrefix = 'b'; }}
            textelement(Party)
            {{
                NamespacePrefix = 'a';
                textelement(Plain) {{ trigger OnBeforePassVariable() begin Plain := 'p'; end; }}
            }}
            tableelement(Line; "XS Row")
            {{
                NamespacePrefix = 'a';
                fieldelement(Id; Line.Id) {{ NamespacePrefix = 'b'; }}
                fieldelement(Name; Line.Name) {{ }}
            }}
            trigger OnAfterGetRecord() begin DocId := 'D1'; end;
        }}
    }}
}}
codeunit 69953 C
{{
    procedure Export(): Text
    var B: Record "XS Blob" temporary; Row: Record "XS Row"; H: Record "XS Hdr"; OutS: OutStream; InS: InStream; Line: Text; Result: Text;
    begin
        H.Number := 1; H.Insert(); H.Number := 2; H.Insert();
        Row.Id := 1; Row.Name := 'one'; Row.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XS Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Result += Line + '|';
        end;
        exit(Result);
    end;
}}
"#
    )
}

fn export(props: &str) -> String {
    text(run(&src(props), "Export"))
}

const BODY: &str = "|  <b:DocId>D1</b:DocId>|  <a:Party>|    <Plain>p</Plain>|  </a:Party>|  <a:Line>|    <b:Id>1</b:Id>|    \
    <Name>one</Name>|  </a:Line>|</Doc>|";
const DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|";

#[test]
fn the_empty_prefix_is_declared_last_whether_listed_first_or_last() {
    let want = format!(
        "{DECL}<Doc xmlns:a=\"urn:lt-a\" xmlns:b=\"urn:lt-b\" xmlns:u=\"urn:lt-unused\" xmlns=\"urn:lt-d\">{BODY}"
    );
    // Y17XmlPortNamespacesTests.al:60 XpNs_EmptyPrefixAmongSeveral_Export.
    assert_eq!(
        export("Namespaces = \"\" = 'urn:lt-d', a = 'urn:lt-a', b = 'urn:lt-b', u = 'urn:lt-unused';"),
        want
    );
    // :71 XpNs_EmptyPrefixLast_Export.
    assert_eq!(
        export("Namespaces = a = 'urn:lt-a', b = 'urn:lt-b', u = 'urn:lt-unused', \"\" = 'urn:lt-d';"),
        want
    );
}

#[test]
fn a_used_default_namespace_replaces_the_uri_of_the_empty_prefix() {
    // Y17XmlPortNamespacesTests.al:82 XpNs_EmptyPrefixAmongSeveral_UseDefault.
    assert_eq!(
        export(
            "Namespaces = \"\" = 'urn:lt-d', a = 'urn:lt-a', b = 'urn:lt-b', u = 'urn:lt-unused'; \
             UseDefaultNamespace = true; DefaultNamespace = 'urn:lt-dflt';"
        ),
        format!(
            "{DECL}<Doc xmlns:a=\"urn:lt-a\" xmlns:b=\"urn:lt-b\" xmlns:u=\"urn:lt-unused\" xmlns=\"urn:lt-dflt\">{BODY}"
        )
    );
}

#[test]
fn prefixed_entries_only_declare_no_default_namespace() {
    // Y17XmlPortNamespacesTests.al:93 XpNs_PrefixedOnly_UnprefixedRoot.
    assert_eq!(
        export("Namespaces = a = 'urn:lt-a', b = 'urn:lt-b', u = 'urn:lt-unused';"),
        format!("{DECL}<Doc xmlns:a=\"urn:lt-a\" xmlns:b=\"urn:lt-b\" xmlns:u=\"urn:lt-unused\">{BODY}")
    );
}

#[test]
fn an_import_with_the_empty_prefix_is_refused() {
    // The import fails on BC with a schema error (Y17XmlPortNamespacesTests.al:178); which element it names is not modelled.
    let s = src("Namespaces = \"\" = 'urn:lt-d', a = 'urn:lt-a', b = 'urn:lt-b';")
        .replace("Direction = Export", "Direction = Import")
        .replace(
            "procedure Export(): Text",
            "procedure Import(): Text\n    var B: Record \"XS Blob\" temporary; OutS: OutStream; InS: InStream;\n    begin\n        B.Data.CreateOutStream(OutS); OutS.WriteText('<Doc xmlns=\"urn:lt-d\"/>'); B.Data.CreateInStream(InS);\n        Xmlport.Import(Xmlport::\"XS Port\", InS); exit('');\n    end;\n    procedure Export(): Text",
        );
    let r = run(&s, "Import");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_text_element_with_only_text_attributes_below_a_text_element_runs_its_triggers() {
    // Y17XmlPortNamespacesTests.al:60: `<b:EndpointID schemeID="0088">E1</b:EndpointID>` inside `<a:Party>`.
    let s = src("Namespaces = \"\" = 'urn:lt-d', a = 'urn:lt-a', b = 'urn:lt-b';").replace(
        "textelement(Plain)",
        "textelement(EndpointID) { NamespacePrefix = 'b'; \
         textattribute(schemeID) { trigger OnBeforePassVariable() begin schemeID := '0088'; end; } \
         trigger OnBeforePassVariable() begin EndpointID := 'E1'; end; }\n                textelement(Plain)",
    );
    let out = text(run(&s, "Export"));
    assert!(
        out.contains(
            "|  <a:Party>|    <b:EndpointID schemeID=\"0088\">E1</b:EndpointID>|    <Plain>p</Plain>|  </a:Party>|"
        ),
        "{out}"
    );
}

#[test]
fn skip_in_on_before_pass_variable_leaves_the_text_element_out() {
    // Y17XmlPortNamespaces.al `Opt` (Y17XmlPortNamespacesTests.al:60): no `<b:Opt />` in the export.
    let s = src("Namespaces = a = 'urn:lt-a', b = 'urn:lt-b';").replace(
        "textelement(Party)",
        "textelement(Opt) { NamespacePrefix = 'b'; trigger OnBeforePassVariable() begin currXMLport.Skip(); end; }\n            textelement(Party)",
    );
    let out = text(run(&s, "Export"));
    assert!(out.contains("<b:DocId>D1</b:DocId>|  <a:Party>"), "{out}");
}
