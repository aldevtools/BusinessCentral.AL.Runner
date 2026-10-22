//! `XmlAttributeCollection.Set` (xml/XmlAttributes.al:342-361 Collection_Set; xml/lt-xml-remaining-node-methods/
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

/// Only the first generated prefix on an element with no namespace in scope is pinned.
#[test]
fn a_second_generated_prefix_or_a_namespace_in_scope_is_refused() {
    for routine in ["SecondNamespace", "ElementInANamespace"] {
        match run_in(SRC, "X", routine) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("prefix"), "{routine}: {why}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}
