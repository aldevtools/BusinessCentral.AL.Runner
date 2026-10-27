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
/// (`urn:microsoft-dynamics-nav/xmlports/x<id>`, the export of xmlport/lt-dg-xmlport-leads/DgXmlPortLeads.al:290
/// Item108_NamespaceRules). That port has a namespace, so the "root in a namespace is not found" pin of a port without one
/// does not apply. Its import and export are not pinned: refused, both with and without the namespace on the root.
#[test]
fn use_default_namespace_without_default_namespace_is_refused() {
    let p = "UseDefaultNamespace = true;";
    for doc in [
        "<Root xmlns=\"urn:microsoft-dynamics-nav/xmlports/x69942\"><R><Id>1</Id></R></Root>",
        "<Root><R><Id>1</Id></R></Root>",
    ] {
        assert!(
            matches!(run(&src(p, doc), "Import"), Err(Signal::Unsupported(_))),
            "{doc}"
        );
    }
    assert!(matches!(run(&src(p, ""), "Export"), Err(Signal::Unsupported(_))));
}
