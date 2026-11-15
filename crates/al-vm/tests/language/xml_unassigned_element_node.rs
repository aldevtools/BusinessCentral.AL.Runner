//! `XmlElement.AsXmlNode()` on an XmlElement nothing assigned (al-runner issue 8157): it returns, `IsXmlElement` of the node
//! answers Yes (corpus PR 1421) and `WriteTo(Text)` of it raises (xml/UhUnassignedXmlElement.al:134). Every other use of that
//! node is refused. Neighbors keep main's outcome.

use al_builtins::native::Signal;
use al_builtins::value::Value;

/// The outcome of `F` of codeunit `C` and the ids of the assumptions the run marked.
fn run(vars: &str, body: &str) -> (String, Vec<&'static str>) {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    al_builtins::assumptions::clear_hits();
    let out = match al_testkit::run_in(&src, "C", "F") {
        Ok(Value::Text(s)) => format!("ok:{s}"),
        Ok(Value::Boolean(b)) => format!("ok:{b}"),
        Err(Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    };
    let hits = al_builtins::assumptions::take_hits()
        .iter()
        .map(|a| a.info().id)
        .collect();
    (out, hits)
}

const ID: &str = "xml-element-unassigned-as-xml-node";
const NOT_INIT: &str = "error:Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized.|NotInitialized";

/// The reporter's repro: `N := E.AsXmlNode()` returns, through the assumption.
#[test]
fn as_xml_node_of_an_unassigned_element_returns() {
    let (out, hits) = run("E: XmlElement; N: XmlNode;", "N := E.AsXmlNode(); exit('done');");
    assert_eq!(out, "ok:done");
    assert!(hits.contains(&ID), "{hits:?}");
}

/// A cleared element is an unassigned one (xml/xml/UlClearXmlRef.al:90-102).
#[test]
fn as_xml_node_of_a_cleared_element_returns() {
    let (out, hits) = run(
        "E: XmlElement; N: XmlNode;",
        "E := XmlElement.Create('a'); Clear(E); N := E.AsXmlNode(); exit('done');",
    );
    assert_eq!(out, "ok:done");
    assert!(hits.contains(&ID), "{hits:?}");
}

/// The probe's step: WriteTo(Text) on the node raises (xml/UhUnassignedXmlElement.al:134), also on a copy of it.
#[test]
fn write_to_text_of_the_node_raises_not_initialized() {
    let (out, _) = run(
        "E: XmlElement; N: XmlNode; T: Text;",
        "N := E.AsXmlNode(); N.WriteTo(T); exit(T);",
    );
    assert_eq!(out, NOT_INIT);
    let (out, _) = run(
        "E: XmlElement; N: XmlNode; N2: XmlNode; T: Text;",
        "N := E.AsXmlNode(); N2 := N; N2.WriteTo(T); exit(T);",
    );
    assert_eq!(out, NOT_INIT);
}

/// Every other use of the node is refused: no corpus case measures it yet.
#[test]
fn other_uses_of_the_node_are_refused() {
    for body in [
        "N := E.AsXmlNode(); N.IsXmlElement(); exit('done');",
        "N := E.AsXmlNode(); E2 := N.AsXmlElement(); exit('done');",
        "N := E.AsXmlNode(); N.WriteTo(O, T); exit(T);",
        "N := E.AsXmlNode(); if N.WriteTo(T) then exit('w'); exit('x');",
        "N := E.AsXmlNode(); N2 := N; exit(Format(N2.IsXmlText()));",
        "N := E.AsXmlNode(); D := XmlDocument.Create(); D.Add(N); exit('done');",
    ] {
        let (out, _) = run(
            "E: XmlElement; E2: XmlElement; N: XmlNode; N2: XmlNode; T: Text; O: XmlWriteOptions; D: XmlDocument;",
            body,
        );
        assert!(out.starts_with("refused:"), "{body}: {out}");
    }
}

/// `N.IsXmlElement()` with its result used answers Yes on the node (corpus PR 1421,
/// xml/xml/UhAsXmlNodeUnassignedElement.al:33-37, test :95-101), through the assumption; also on a copy.
#[test]
fn is_xml_element_of_the_node_answers_yes() {
    let (out, hits) = run(
        "E: XmlElement; N: XmlNode;",
        "N := E.AsXmlNode(); exit(Format(N.IsXmlElement()));",
    );
    assert_eq!(out, "ok:Yes");
    assert!(hits.contains(&ID), "{hits:?}");
    let (out, _) = run(
        "E: XmlElement; N: XmlNode; N2: XmlNode;",
        "N := E.AsXmlNode(); N2 := N; exit(Format(N2.IsXmlElement()));",
    );
    assert_eq!(out, "ok:Yes");
}

/// The Variant Is* checks on the node are refused: not measured (a real element node in a Variant answers IsXmlElement,
/// types/variant/LTP89IsXmlNodes.al, but this node is not measured there).
#[test]
fn variant_is_checks_on_the_node_are_refused() {
    for check in ["IsXmlNode", "IsXmlElement", "IsXmlText", "IsJsonObject"] {
        let (out, _) = run(
            "E: XmlElement; N: XmlNode; V: Variant;",
            &format!("N := E.AsXmlNode(); V := N; exit(Format(V.{check}()));"),
        );
        assert!(out.starts_with("refused:"), "{check}: {out}");
    }
}

/// List Contains, IndexOf, LastIndexOf and Remove are refused when the searched value or an element of the list is the node:
/// not measured.
#[test]
fn list_searches_with_the_node_are_refused() {
    for search in [
        "Format(L.Contains(X))",
        "Format(L.IndexOf(X))",
        "Format(L.LastIndexOf(X))",
        "Format(L.Remove(X))",
    ] {
        // The node is searched for.
        let (out, _) = run(
            "E: XmlElement; N: XmlNode; X: XmlNode; L: List of [XmlNode];",
            &format!("N := E.AsXmlNode(); L.Add(N); X := N; exit({search});"),
        );
        assert!(out.starts_with("refused:"), "{search} (searched): {out}");
        // The node is an element of the list, another node is searched for.
        let (out, _) = run(
            "E: XmlElement; N: XmlNode; X: XmlNode; L: List of [XmlNode];",
            &format!("N := E.AsXmlNode(); L.Add(N); X := XmlElement.Create('a').AsXmlNode(); exit({search});"),
        );
        assert!(out.starts_with("refused:"), "{search} (element): {out}");
    }
}

/// Neighbors keep main's outcome and mark nothing: an assigned element, AsXmlNode as a statement, an unassigned XmlNode and
/// an unassigned XmlDocumentType.
#[test]
fn neighbors_keep_mains_outcome() {
    let (out, hits) = run(
        "E: XmlElement; N: XmlNode; T: Text;",
        "E := XmlElement.Create('a'); N := E.AsXmlNode(); N.WriteTo(T); exit(T);",
    );
    assert_eq!(out, "ok:<a />");
    assert!(!hits.contains(&ID), "{hits:?}");
    let (out, hits) = run("E: XmlElement;", "E.AsXmlNode(); exit('done');");
    assert!(out.starts_with("refused:"), "{out}");
    assert!(!hits.contains(&ID), "{hits:?}");
    let (out, _) = run("N: XmlNode;", "exit(Format(N.IsXmlElement()));");
    assert_eq!(out, "ok:No");
    let (out, hits) = run(
        "D: XmlDocumentType; N: XmlNode; T: Text;",
        "N := D.AsXmlNode(); N.WriteTo(T); exit(T);",
    );
    assert_eq!(out, NOT_INIT);
    assert!(!hits.contains(&ID), "{hits:?}");
    // A Variant holding an unassigned XmlNode and a list search of Integers keep main's answers.
    let (out, _) = run("N: XmlNode; V: Variant;", "V := N; exit(Format(V.IsXmlNode()));");
    assert_eq!(out, "ok:Yes");
    let (out, _) = run("L: List of [Integer];", "L.Add(3); exit(Format(L.IndexOf(3)));");
    assert_eq!(out, "ok:1");
}
