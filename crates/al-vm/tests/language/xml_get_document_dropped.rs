//! `GetDocument` of a node whose document no variable references any more (xml/XmlElement.al:349, xml/XmlNodes.al:363):
//! BC keeps that document alive and answers Yes; the runtime holds parents weakly, so it refuses there. A lone node and a
//! node of a live document keep their answers.

use al_builtins::native::Signal;

fn run(body: &str) -> Result<String, Signal> {
    let src = format!(
        "codeunit 50100 C {{
        local procedure Load(var Root: XmlElement) var Doc: XmlDocument; begin XmlDocument.ReadFrom('<a><b/></a>', Doc); Doc.GetRoot(Root); end;
        procedure F(): Text var E: XmlElement; D: XmlDocument; S: Text; begin {body} end; }}"
    );
    al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| al_testkit::text(Ok(v)))
}

fn refused(r: &Result<String, Signal>) -> bool {
    matches!(r, Err(Signal::Unsupported(m)) if m.contains("no longer referenced"))
}

#[test]
fn get_document_of_a_node_whose_document_was_dropped_is_refused() {
    let r = run("Load(E); exit(Format(E.GetDocument(D)));");
    assert!(refused(&r), "{r:?}");
    let r = run("Load(E); E.GetDocument(D); exit('ok');");
    assert!(refused(&r), "{r:?}");
}

#[test]
fn get_document_of_a_lone_node_keeps_no() {
    // xml/XmlElement.al:353 `lone_doc`.
    let r = run("E := XmlElement.Create('x'); exit(Format(E.GetDocument(D)));");
    assert_eq!(r.ok().as_deref(), Some("No"));
}

#[test]
fn get_document_of_a_node_of_a_live_document_keeps_yes() {
    let r = run("XmlDocument.ReadFrom('<a/>', D); D.GetRoot(E); exit(Format(E.GetDocument(D)));");
    assert_eq!(r.ok().as_deref(), Some("Yes"));
}
