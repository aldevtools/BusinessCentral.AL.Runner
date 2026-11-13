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

#[test]
fn the_statement_form_refuses() {
    let r = al_testkit::run_in(SRC, "C", "Statement");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("in a namespace")),
        "{r:?}"
    );
}
