//! `E.Add(E)` and `E.Remove()` of an element made by `XmlElement.Create(Name)` that never had a parent (corpus
//! xml/XmlElement.al:455-466 Add_Self_Error, :653-656 Remove_Root_Lone). Only the pinned element shape runs; an element
//! with content, an attribute or a namespace, one taken off a parent, and another call form keep the refusal.

use al_builtins::native::Signal;

fn run(body: &str) -> Result<String, Signal> {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var E: XmlElement; P: XmlElement; X: XmlElement; S: Text; B: Boolean; begin {body} end; }}"
    );
    al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| al_testkit::text(Ok(v)))
}

fn refused(r: &Result<String, Signal>, start: &str) -> bool {
    matches!(r, Err(Signal::Unsupported(m)) if m.starts_with(start))
}

const SELF: &str = "adding an XML node to itself";
const LONE: &str = "Remove of a node without a parent";

#[test]
fn the_pinned_self_add_adds_a_copy() {
    let r = run("E := XmlElement.Create('a'); E.Add(E); E.WriteTo(S); exit(S);");
    assert_eq!(r.ok().as_deref(), Some("<a>\r\n  <a />\r\n</a>"));
}

#[test]
fn a_self_add_of_another_element_shape_keeps_the_refusal() {
    for body in [
        "E := XmlElement.Create('a'); E.Add(XmlElement.Create('b')); E.Add(E);",
        "E := XmlElement.Create('a'); E.SetAttribute('x', '1'); E.Add(E);",
        "E := XmlElement.Create('a', 'urn:u'); E.Add(E);",
        "P := XmlElement.Create('p'); E := XmlElement.Create('a'); P.Add(E); E.Remove(); E.Add(E);",
        "E := XmlElement.Create('a'); X := XmlElement.Create('x'); E.Add(X, E);",
        "E := XmlElement.Create('a'); E.AddFirst(E);",
        "E := XmlElement.Create('a'); E.Add(E.AsXmlNode());",
    ] {
        let r = run(&format!("{body} exit('');"));
        assert!(refused(&r, SELF), "{body}: {r:?}");
    }
}

#[test]
fn a_self_add_with_the_result_used_keeps_the_refusal() {
    let r = run("E := XmlElement.Create('a'); B := E.Add(E); exit(Format(B));");
    assert!(refused(&r, SELF), "{r:?}");
}

#[test]
fn the_pinned_lone_remove_raises() {
    let r = run("E := XmlElement.Create('lone'); asserterror E.Remove(); exit(GetLastErrorCode());");
    assert_eq!(r.ok().as_deref(), Some("InvalidOperation"));
}

#[test]
fn a_lone_remove_of_another_element_shape_keeps_the_refusal() {
    for body in [
        "E := XmlElement.Create('a'); E.Add(XmlElement.Create('b')); E.Remove();",
        "E := XmlElement.Create('a'); E.SetAttribute('x', '1'); E.Remove();",
        "E := XmlElement.Create('a', 'urn:u'); E.Remove();",
        "P := XmlElement.Create('p'); E := XmlElement.Create('a'); P.Add(E); E.Remove(); E.Remove();",
        "E := XmlElement.Create('a'); E.AsXmlNode().Remove();",
    ] {
        let r = run(&format!("{body} exit('');"));
        assert!(refused(&r, LONE), "{body}: {r:?}");
    }
}

#[test]
fn a_lone_remove_with_the_result_used_keeps_main_s_false() {
    let r = run("E := XmlElement.Create('lone'); exit(Format(E.Remove()));");
    assert_eq!(r.ok().as_deref(), Some("No"));
}
