//! Attribute edits unmasked by the pinned XML name cells. XmlAttribute.ReplaceWith on an attribute of an element by new attributes
//! (xml/XmlAttributes.al:463, xml/xml/P97AttrEdit.al:249 `t6`, :294 `t11`): the shapes the pins measured replace in place; their
//! neighbors keep main's refusal. XmlElement.AddFirst of a new attribute (xml/XmlElement.al:384). XmlAttribute.Remove of an
//! attribute whose element is no longer referenced keeps main's refusal.

use al_builtins::native::Signal;

const SRC: &str = r#"
codeunit 69979 C
{
    procedure Pinned(): Text
    var
        E: XmlElement;
        A: XmlAttribute;
        T: Text;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '1');
        E.SetAttribute('b', '2');
        E.Attributes().Get('a', A);
        A.ReplaceWith(XmlAttribute.Create('c', '3'), XmlAttribute.Create('d', '4'));
        E.WriteTo(T);
        exit(T);
    end;

    procedure SameName()
    var
        E: XmlElement;
        A: XmlAttribute;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '1');
        E.SetAttribute('b', '2');
        E.Attributes().Get('a', A);
        A.ReplaceWith(XmlAttribute.Create('b', '3'));
    end;

    procedure ResultUsed(): Boolean
    var
        E: XmlElement;
        A: XmlAttribute;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '1');
        E.Attributes().Get('a', A);
        exit(A.ReplaceWith(XmlAttribute.Create('c', '3')));
    end;

    procedure Namespaced()
    var
        E: XmlElement;
        A: XmlAttribute;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '1');
        E.Attributes().Get('a', A);
        A.ReplaceWith(XmlAttribute.Create('c', 'urn:x', '3'));
    end;
}
"#;

#[test]
fn new_attributes_take_the_place_of_the_replaced_one() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Pinned")),
        r#"<e c="3" d="4" b="2" />"#
    );
}

#[test]
fn unmeasured_neighbors_keep_the_refusal() {
    for p in ["SameName", "ResultUsed", "Namespaced"] {
        let r = al_testkit::run_in(SRC, "C", p);
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m == "a sibling operation on an attribute or declaration"),
            "{p}: {r:?}"
        );
    }
}

const ADD_FIRST: &str = r#"
codeunit 69980 D
{
    procedure Lone(): Text
    var
        E: XmlElement;
        T: Text;
    begin
        E := XmlElement.Create('a');
        E.Add(XmlElement.Create('b'));
        if not TryAddFirst(E) then
            T := GetLastErrorCode();
        exit(T);
    end;

    [TryFunction]
    local procedure TryAddFirst(E: XmlElement)
    begin
        E.AddFirst(XmlAttribute.Create('k', 'v'));
    end;

    procedure InDocument(): Text
    var
        Doc: XmlDocument;
        E: XmlElement;
        T: Text;
    begin
        Doc := XmlDocument.Create();
        E := XmlElement.Create('a');
        Doc.Add(E);
        Doc.GetRoot(E);
        E.AddFirst(XmlAttribute.Create('k', 'v'));
        E.WriteTo(T);
        exit(T);
    end;
}
"#;

/// AddFirst of a new attribute to an element without a parent raises the Argument error (xml/XmlElement.al:384 `af296`).
#[test]
fn add_first_of_an_attribute_to_a_lone_element_raises() {
    assert_eq!(al_testkit::text(al_testkit::run_in(ADD_FIRST, "D", "Lone")), "Argument");
}

/// An element in a document is not measured: it keeps main's result (the attribute is added).
#[test]
fn add_first_of_an_attribute_to_an_element_in_a_document_keeps_main() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(ADD_FIRST, "D", "InDocument")),
        r#"<a k="v" />"#
    );
}

const DROPPED: &str = r#"
codeunit 69981 G
{
    local procedure Take(var A: XmlAttribute)
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '1');
        E.Attributes().Get('a', A);
    end;

    procedure RemoveDropped()
    var
        A: XmlAttribute;
    begin
        Take(A);
        A.Remove();
    end;
}
"#;

/// The runtime holds parents weakly: an attribute whose element went out of scope is not a lone attribute, and its Remove
/// keeps main's refusal.
#[test]
fn remove_of_an_attribute_whose_element_is_gone_keeps_the_refusal() {
    let r = al_testkit::run_in(DROPPED, "G", "RemoveDropped");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.starts_with("Remove of a node without a parent")),
        "{r:?}"
    );
}
