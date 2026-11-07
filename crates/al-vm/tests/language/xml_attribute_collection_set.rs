//! `XmlAttributeCollection.Set` (xml/XmlAttributes.al:342-361 Collection_Set; xml/xml/
//! P104AttrCollection.al:74 `set_after`).

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 50320 X
{
    local procedure Str(E: XmlElement) T: Text
    begin
        E.WriteTo(T);
    end;

    procedure Set(): Text
    var E: XmlElement; C: XmlAttributeCollection; Out: Text;
    begin
        E := XmlElement.Create('e');
        C := E.Attributes();
        C.Set('a', '1');
        Out := Str(E);
        C.Set('a', '2');
        Out += '|' + Str(E);
        C.Set('a', 'urn:x', '3');
        exit(Out + '|' + Str(E) + '|' + Format(C.Count()));
    end;

    procedure SetAfterRemoveAll(): Text
    var E: XmlElement; C: XmlAttributeCollection;
    begin
        E := XmlElement.Create('e');
        E.SetAttribute('a', '1');
        C := E.Attributes();
        C.RemoveAll();
        C.Set('n', 'new');
        exit(Str(E) + '|' + Format(C.Count()));
    end;

    procedure SecondNamespace(): Text
    var E: XmlElement; C: XmlAttributeCollection;
    begin
        E := XmlElement.Create('e');
        C := E.Attributes();
        C.Set('a', 'urn:x', '3');
        C.Set('b', 'urn:y', '4');
        exit(Str(E));
    end;

    procedure ElementInANamespace(): Text
    var E: XmlElement; C: XmlAttributeCollection;
    begin
        E := XmlElement.Create('e', 'urn:e');
        C := E.Attributes();
        C.Set('a', 'urn:x', '3');
        exit(Str(E));
    end;
}
"#;

#[test]
fn set_adds_or_replaces_an_attribute_and_a_namespaced_one_gets_a_generated_prefix() {
    assert_eq!(
        text(run_in(SRC, "X", "Set")),
        r#"<e a="1" />|<e a="2" />|<e a="2" p1:a="3" xmlns:p1="urn:x" />|2"#
    );
    assert_eq!(text(run_in(SRC, "X", "SetAfterRemoveAll")), r#"<e n="new" />|1"#);
}

/// A second generated prefix on the same element is pinned (xml/XmlAttributes.al:648 `two_ns`, XmlNsPrefixes.al
/// Attribute_PrefixNumbering `four_on_element`): the last one made is declared first.
#[test]
fn a_second_generated_prefix_is_numbered_and_declared_last_made_first() {
    assert_eq!(
        text(run_in(SRC, "X", "SecondNamespace")),
        r#"<e p1:a="3" p2:b="4" xmlns:p2="urn:y" xmlns:p1="urn:x" />"#
    );
}

/// A namespace on the element itself is still not pinned for a namespaced attribute.
#[test]
fn a_namespace_in_scope_is_refused() {
    for routine in ["ElementInANamespace"] {
        match run_in(SRC, "X", routine) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("prefix"), "{routine}: {why}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}
