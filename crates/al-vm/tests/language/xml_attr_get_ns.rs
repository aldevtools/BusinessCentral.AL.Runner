//! XmlAttributeCollection.Get by a plain name when the attribute was set with a namespace and no prefix (xml/XmlAttributes.al:220
//! with :230 SetGet_Attribute_Namespace `get_no_ns`: the answer is No). The statement form is not pinned and refuses; an
//! attribute without a namespace is still found.

use al_builtins::native::Signal;

const SRC: &str = r#"
codeunit 69978 C
{
    procedure Answer(): Text
    var
        E: XmlElement;
        A: XmlAttribute;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', 'urn:x', '1');
        exit(Format(E.Attributes().Get('a', A)));
    end;

    procedure Plain(): Text
    var
        E: XmlElement;
        A: XmlAttribute;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '2');
        E.Attributes().Get('a', A);
        exit(A.Value());
    end;

    procedure DefaultDecl(): Text
    var
        Doc: XmlDocument;
        E: XmlElement;
        A: XmlAttribute;
    begin
        XmlDocument.ReadFrom('<e xmlns="urn:d" a="1"/>', Doc);
        Doc.GetRoot(E);
        if E.Attributes().Get('xmlns', A) then
            exit(A.Value());
        exit('not found');
    end;

    procedure Statement()
    var
        E: XmlElement;
        A: XmlAttribute;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', 'urn:x', '1');
        E.Attributes().Get('a', A);
    end;
}
"#;

#[test]
fn a_namespaced_attribute_is_not_found_by_its_plain_name() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Answer")), "No");
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Plain")), "2");
}

/// A default namespace declaration has no prefix and is found by `xmlns` (xml/XmlAttributes.al:493 `get_xmlns`: Yes).
#[test]
fn a_default_namespace_declaration_is_found_by_xmlns() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "DefaultDecl")), "urn:d");
}

#[test]
fn the_statement_form_refuses() {
    let r = al_testkit::run_in(SRC, "C", "Statement");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("in a namespace")),
        "{r:?}"
    );
}
