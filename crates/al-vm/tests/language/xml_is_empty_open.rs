//! `XmlElement.IsEmpty` of an element parsed from `<a></a>` (corpus xml/XmlElement.al:692-711 IsEmpty_States pins No)
//! after a child was added to it: not measured, so it keeps the answer of an element without children.

use al_testkit::text;

fn run(body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var Doc: XmlDocument; R: XmlElement; E: XmlElement; C: XmlElement; N: XmlNode; S: Text; W: Text; begin {body} end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn a_parsed_open_element_is_not_empty() {
    // xml/XmlElement.al:692-711 IsEmpty_States.
    let r = run("XmlDocument.ReadFrom('<a></a>', Doc); Doc.GetRoot(E); exit(Format(E.IsEmpty()));");
    assert_eq!(r, "No");
}

#[test]
fn a_parsed_open_element_after_a_child_was_added_and_removed_keeps_the_answer_of_no_children() {
    let r = run("XmlDocument.ReadFrom('<a></a>', Doc); Doc.GetRoot(E); C := XmlElement.Create('b'); E.Add(C); C.Remove();
         E.WriteTo(W); exit(Format(E.IsEmpty()) + ' ' + W);");
    assert_eq!(r, "Yes <a></a>");
}

#[test]
fn a_copy_of_it_keeps_that_answer() {
    // Add of an element that has a parent adds a copy.
    let r = run("XmlDocument.ReadFrom('<r><a></a></r>', Doc); Doc.GetRoot(R); R.SelectSingleNode('a', N); E := N.AsXmlElement();
         C := XmlElement.Create('b'); E.Add(C); C.Remove(); R.Add(E); R.GetChildElements().Get(2, N);
         exit(Format(N.AsXmlElement().IsEmpty()) + ' ' + Format(R.GetChildElements().Count()));");
    assert_eq!(r, "Yes 2");
}

#[test]
fn only_the_pinned_parsed_root_without_attributes_answers_no() {
    // The pin's element is a parsed root with no attribute and no namespace; a nested one, one with an attribute and a
    // namespaced one are not measured and answer from their children.
    let r = run("XmlDocument.ReadFrom('<r><a></a></r>', Doc); Doc.GetRoot(R); R.SelectSingleNode('a', N); S := Format(N.AsXmlElement().IsEmpty());
         XmlDocument.ReadFrom('<a x=\"1\"></a>', Doc); Doc.GetRoot(E); S += ' ' + Format(E.IsEmpty());
         XmlDocument.ReadFrom('<p:a xmlns:p=\"u\"></p:a>', Doc); Doc.GetRoot(E); S += ' ' + Format(E.IsEmpty()); exit(S);");
    assert_eq!(r, "Yes Yes Yes");
}
