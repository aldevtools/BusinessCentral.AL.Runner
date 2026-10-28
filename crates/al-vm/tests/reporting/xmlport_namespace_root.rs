//! The import of a port without namespaces: a root element in a namespace is not found, a prefix the root declares and no
//! element uses is ignored (corpus xmlport/lt-xmlport-query-datatransfer/XpTNamespaces.al:Import_NoNamespaceXmlportWithNamespacedInput,
//! Import_NoNamespaceXmlportWithUnusedPrefixDecl, Import_DefaultNs*). A DefaultNamespace without UseDefaultNamespace changes
//! nothing on import; its export is refused.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(props: &str, doc: &str) -> String {
    format!(
        r#"
table 69940 "XN Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69941 "XN Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69942 "XN Port"
{{
    Direction = Both; Format = Xml; UseRequestPage = false; {props}
    schema {{ textelement(Root) {{ tableelement(R; "XN Row") {{ fieldelement(Id; R.Id) {{ }} }} }} }}
}}
codeunit 69943 C
{{
    procedure Import(): Text
    var B: Record "XN Blob" temporary; Row: Record "XN Row"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('{doc}');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XN Port", InS);
        exit(Format(Row.Count()));
    end;

    procedure Export(): Text
    var B: Record "XN Blob" temporary; OutS: OutStream;
    begin
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XN Port", OutS);
        exit('');
    end;
}}
"#
    )
}

fn import(props: &str, doc: &str) -> Result<String, String> {
    match run(&src(props, doc), "Import") {
        Ok(v) => Ok(al_testkit::text(Ok(v))),
        Err(Signal::Error(e)) => Err(format!("{}|{}", e.message, e.code)),
        Err(other) => panic!("{other:?}"),
    }
}

const NOT_FOUND: &str = "The element <Root> cannot be found in the object description.|XmlPortElementNotFound";

#[test]
fn a_root_in_a_namespace_is_not_found() {
    assert_eq!(
        import("", "<Root xmlns=\"urn:x\"><R><Id>1</Id></R></Root>"),
        Err(NOT_FOUND.into())
    );
    assert_eq!(
        import("", "<a:Root xmlns:a=\"urn:x\"><a:R><a:Id>1</a:Id></a:R></a:Root>"),
        Err(NOT_FOUND.into())
    );
    assert_eq!(
        import("", "<Root xmlns:z=\"urn:z\"><R><Id>1</Id></R></Root>"),
        Ok("1".into())
    );
}

#[test]
fn an_unused_default_namespace_changes_nothing_on_import() {
    let p = "DefaultNamespace = 'urn:x';";
    assert_eq!(import(p, "<Root><R><Id>1</Id></R></Root>"), Ok("1".into()));
    assert_eq!(
        import(p, "<Root xmlns=\"urn:x\"><R><Id>1</Id></R></Root>"),
        Err(NOT_FOUND.into())
    );
    assert!(matches!(run(&src(p, ""), "Export"), Err(Signal::Unsupported(_))));
    let used = "DefaultNamespace = 'urn:x'; UseDefaultNamespace = true;";
    assert!(matches!(
        run(&src(used, "<Root/>"), "Import"),
        Err(Signal::Unsupported(_))
    ));
}

/// UseDefaultNamespace = true without DefaultNamespace puts the port in BC's own default namespace
/// `urn:microsoft-dynamics-nav/xmlports/x<id>` (xmlport/lt-bd-xmlport/BdXmlPort.al:499 SqUseDefaultNamespace_NoDefaultNamespace,
/// xmlport 89580). The import takes a root in that namespace, by default or by prefix; a root in another namespace or in none
/// is not found; a child reset to no namespace fails the schema check.
const UDN: &str = "UseDefaultNamespace = true;";
const BC_NS: &str = "urn:microsoft-dynamics-nav/xmlports/x69942";

#[test]
fn use_default_namespace_imports_a_root_in_bcs_namespace() {
    assert_eq!(
        import(
            UDN,
            &format!("<Root xmlns=\"{BC_NS}\"><R><Id>1</Id></R><R><Id>2</Id></R></Root>")
        ),
        Ok("2".into())
    );
    assert_eq!(
        import(
            UDN,
            &format!("<p:Root xmlns:p=\"{BC_NS}\"><p:R><p:Id>1</p:Id></p:R></p:Root>")
        ),
        Ok("1".into())
    );
}

#[test]
fn use_default_namespace_does_not_find_a_root_elsewhere() {
    for doc in [
        "<Root xmlns=\"urn:other\"><R><Id>1</Id></R></Root>",
        "<Root><R><Id>1</Id></R></Root>",
        "<Root xmlns=\"urn:microsoft-dynamics-nav/xmlports/x81085\"><R><Id>1</Id></R></Root>",
    ] {
        assert_eq!(import(UDN, doc), Err(NOT_FOUND.into()), "{doc}");
    }
}

#[test]
fn use_default_namespace_rejects_a_child_in_no_namespace() {
    assert_eq!(
        import(
            UDN,
            &format!("<Root xmlns=\"{BC_NS}\"><R xmlns=\"\"><Id>1</Id></R></Root>")
        ),
        Err(format!(
            "Imported XML cannot validate with the schema: The element 'Root' in namespace '{BC_NS}' has invalid child element 'R'. \
             List of possible elements expected: 'R' in namespace '{BC_NS}'.|XmlPortInvalidXml"
        ))
    );
}

#[test]
fn use_default_namespace_exports_bcs_namespace_on_the_root() {
    let src = src(UDN, "").replace(
        "exit('');\n    end;",
        "B.Data.CreateInStream(InS, TextEncoding::UTF16);\n        while not InS.EOS() do begin InS.ReadText(Line); R += Line + '|'; end;\n        exit(R);\n    end;",
    ).replace(
        "var B: Record \"XN Blob\" temporary; OutS: OutStream;\n",
        "var B: Record \"XN Blob\" temporary; OutS: OutStream; InS: InStream; Line: Text; R: Text; Row: Record \"XN Row\";\n",
    ).replace(
        "B.Data.CreateOutStream(OutS);\n        Xmlport.Export",
        "Row.Id := 1; Row.Insert(); B.Data.CreateOutStream(OutS);\n        Xmlport.Export",
    );
    assert_eq!(
        al_testkit::text(run(&src, "Export")),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root xmlns=\"{BC_NS}\">|  <R>|    <Id>1</Id>|  </R>|</Root>|"
        )
    );
}

/// Shapes the corpus does not pin stay refused: an attribute, a Namespaces entry, a text format.
#[test]
fn use_default_namespace_refuses_unpinned_shapes() {
    let doc = format!("<Root xmlns=\"{BC_NS}\"><R><Id>1</Id></R></Root>");
    for p in [
        "UseDefaultNamespace = true; Namespaces = p = 'urn:p';",
        "UseDefaultNamespace = true; UseLax = true;",
    ] {
        assert!(
            matches!(run(&src(p, &doc), "Import"), Err(Signal::Unsupported(_))),
            "{p}"
        );
    }
}
