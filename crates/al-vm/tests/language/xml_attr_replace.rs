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

    procedure ByRemoved()
    var
        E: XmlElement;
        F: XmlElement;
        A: XmlAttribute;
        B: XmlAttribute;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '1');
        E.SetAttribute('b', '2');
        F := XmlElement.Create('f');
        F.SetAttribute('z', '9');
        F.Attributes().Get('z', B);
        B.Remove();
        E.Attributes().Get('a', A);
        A.ReplaceWith(B);
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
    // ByRemoved: an attribute taken off another element (not measured; every pin's new attribute comes from Create).
    for p in ["SameName", "ResultUsed", "Namespaced", "ByRemoved"] {
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
    begin
        E := XmlElement.Create('a');
        E.Add(XmlElement.Create('b'));
        E.Add(XmlText.Create('tail'));
        E.Add(XmlAttribute.Create('k', 'v'));
        exit(Try(E, XmlAttribute.Create('k2', 'v2')));
    end;

    procedure EmptyLone(): Text
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        exit(Try(E, XmlAttribute.Create('k', 'v')));
    end;

    procedure DuplicateName(): Text
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        E.Add(XmlElement.Create('b'));
        E.SetAttribute('k', '1');
        exit(Try(E, XmlAttribute.Create('k', 'v')));
    end;

    procedure Removed(): Text
    var
        E: XmlElement;
        F: XmlElement;
        A: XmlAttribute;
    begin
        E := XmlElement.Create('a');
        E.Add(XmlElement.Create('b'));
        E.SetAttribute('m', '1');
        F := XmlElement.Create('f');
        A := XmlAttribute.Create('k', 'v');
        F.Add(A);
        A.Remove();
        exit(Try(E, A));
    end;

    local procedure Try(E: XmlElement; A: XmlAttribute): Text
    var
        T: Text;
    begin
        if TryAddFirst(E, A) then
            T := 'ok'
        else
            T := GetLastErrorCode();
        E.WriteTo(A2);
        exit(T + ' ' + A2);
    end;

    [TryFunction]
    local procedure TryAddFirst(E: XmlElement; A: XmlAttribute)
    begin
        E.AddFirst(A);
    end;

    var
        A2: Text;

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

/// AddFirst of a new attribute to an element without a parent, with child nodes and an attribute, raises the Argument error
/// and leaves the element (xml/XmlElement.al:384 `af296`, `addfirstattr`).
#[test]
fn add_first_of_an_attribute_to_a_lone_element_raises() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(ADD_FIRST, "D", "Lone")),
        "Argument <a k=\"v\">\n  <b />tail</a>".replace("\n", "\r\n")
    );
}

/// Element states the pin does not have keep main's result: an empty element and an attribute taken off another element are
/// added; a name the element already has raises InvalidOperation.
#[test]
fn add_first_of_an_attribute_outside_the_pinned_cell_keeps_main() {
    let r = |p| al_testkit::text(al_testkit::run_in(ADD_FIRST, "D", p));
    assert_eq!(r("EmptyLone"), r#"ok <a k="v" />"#);
    assert_eq!(
        r("DuplicateName"),
        "InvalidOperation <a k=\"1\">\n  <b />\n</a>".replace("\n", "\r\n")
    );
    assert_eq!(
        r("Removed"),
        "ok <a m=\"1\" k=\"v\">\n  <b />\n</a>".replace("\n", "\r\n")
    );
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

    procedure RemoveLoneNamespaced()
    var
        A: XmlAttribute;
    begin
        A := XmlAttribute.Create('a', 'urn:x', '1');
        A.Remove();
    end;

    procedure RemoveLoneDeclaration()
    var
        A: XmlAttribute;
    begin
        A := XmlAttribute.CreateNamespaceDeclaration('p', 'urn:p');
        A.Remove();
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

/// Every pin of Remove on a lone attribute has a plain one (xml/xml/P97AttrMisc.al:98, :125; xml/XmlAttributes.al:422): an
/// attribute in a namespace and a namespace declaration keep main's refusal.
#[test]
fn remove_of_a_lone_namespaced_attribute_keeps_the_refusal() {
    for p in ["RemoveLoneNamespaced", "RemoveLoneDeclaration"] {
        let r = al_testkit::run_in(DROPPED, "G", p);
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.starts_with("Remove of a node without a parent")),
            "{p}: {r:?}"
        );
    }
}
