//! Sibling operations (`AddAfterSelf`, `AddBeforeSelf`, `ReplaceWith`) on an element, attribute or document without a
//! parent (corpus xml/XmlElement.al, xml/xml/P97AttrEdit.al, P97DocEdit.al, P97DocMisc.al).

use al_testkit::text;

fn run(body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var E: XmlElement; A: XmlAttribute; Doc: XmlDocument; N: XmlNode; I: Integer; S: Text; begin {body} end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

const MSG: &str = "The requested operation cannot be performed in this context.";

#[test]
fn a_lone_element_raises_invalid_operation_for_an_element_argument() {
    // xml/XmlElement.al:AddAfterSelf_Root_Error `lone_after`, `lone_before`; ReplaceWith_Root_Unparented `lone`.
    let r = run("E := XmlElement.Create('lone');
         asserterror E.AddAfterSelf(XmlElement.Create('b')); S := GetLastErrorText() + '/' + GetLastErrorCode();
         asserterror E.AddBeforeSelf(XmlElement.Create('b')); S += '|' + GetLastErrorText();
         asserterror E.ReplaceWith(XmlElement.Create('b')); S += '|' + GetLastErrorText(); exit(S);");
    assert_eq!(r, format!("{MSG}/InvalidOperation|{MSG}|{MSG}"));
}

#[test]
fn a_detached_attribute_raises_invalid_operation_for_any_argument() {
    // P97AttrEdit.al:AddAfterSelf_Detached_Kinds `k14` (an unassigned XmlNode), *_Detached_Typed `t10` (an Integer).
    let r = run("A := XmlAttribute.Create('x', '2');
         asserterror A.AddAfterSelf(N); S := GetLastErrorText() + '/' + GetLastErrorCode();
         I := 42; asserterror A.AddBeforeSelf(I); S += '|' + GetLastErrorText();
         asserterror A.ReplaceWith('plain text'); S += '|' + GetLastErrorText(); exit(S);");
    assert_eq!(r, format!("{MSG}/InvalidOperation|{MSG}|{MSG}"));
}

#[test]
fn a_document_raises_invalid_operation_but_an_unassigned_node_argument_first() {
    // P97DocEdit.al:AddAfterSelf_RootOnly_Kinds `k1`, `k14`; AddBeforeSelf_RootOnly_Typed `t10`.
    let r = run("Doc := XmlDocument.Create(); Doc.Add(XmlElement.Create('r'));
         asserterror Doc.AddAfterSelf(XmlElement.Create('e')); S := GetLastErrorText() + '/' + GetLastErrorCode();
         I := 42; asserterror Doc.AddBeforeSelf(I); S += '|' + GetLastErrorText();
         asserterror Doc.ReplaceWith(N); S += '|' + GetLastErrorText() + '/' + GetLastErrorCode(); exit(S);");
    assert_eq!(
        r,
        format!(
            "{MSG}/InvalidOperation|{MSG}|Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized./NotInitialized"
        )
    );
}

#[test]
fn an_element_whose_document_is_no_longer_referenced_is_refused() {
    // xml/XmlElement.al:530: BC keeps the document of a root alive after the local document variable is gone; the
    // runtime holds parents weakly, so the root would look parentless.
    let src = "codeunit 50100 C {
        local procedure Load(var Root: XmlElement) var Doc: XmlDocument; begin XmlDocument.ReadFrom('<a/>', Doc); Doc.GetRoot(Root); end;
        procedure F(): Text var E: XmlElement; begin Load(E); E.AddAfterSelf(XmlComment.Create('c')); exit('ok'); end; }";
    let r = al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v);
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("no longer referenced")),
        "{r:?}"
    );
}

#[test]
fn an_integer_argument_of_a_sibling_operation_becomes_its_text() {
    // xml/xml/P104TreeACData.al:198 `c8` (AddAfterSelf(42)), :309 (AddBeforeSelf(42)),
    // P104TreeBText.al:321 (ReplaceWith(42)).
    let r = run(
        "Doc := XmlDocument.Create(); E := XmlElement.Create('r'); E.Add(XmlComment.Create('a')); Doc.Add(E);
         E.GetChildNodes().Get(1, N); N.AddAfterSelf(42); N.AddBeforeSelf(7); N.ReplaceWith(5); E.WriteTo(S); exit(S);",
    );
    assert_eq!(r, "<r>7542</r>");
}

#[test]
fn an_integer_of_four_digits_as_sibling_content_is_refused() {
    // Only 42 is pinned; Format would group the digits of 1000 and .NET would not, so 0..=999 is all that runs.
    let src = "codeunit 50100 C { procedure F(): Text var E: XmlElement; N: XmlNode; begin
        E := XmlElement.Create('r'); E.Add(XmlComment.Create('a')); E.GetChildNodes().Get(1, N); N.AddAfterSelf(1000); exit('ok'); end; }";
    let r = al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v);
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
    // An element receiver is not pinned for an Integer or an empty text.
    for arg in ["42", "''"] {
        let src = format!("codeunit 50100 C {{ procedure F(): Text var E: XmlElement; N: XmlNode; begin
            E := XmlElement.Create('r'); E.Add(XmlElement.Create('a')); E.GetChildNodes().Get(1, N); N.AddAfterSelf({arg}); exit('ok'); end; }}");
        let r = al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v);
        assert!(
            matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{arg}: {r:?}"
        );
    }
}

#[test]
fn an_unassigned_node_argument_of_a_text_like_node_raises_not_initialized() {
    // xml/xml/P90CDataEdit.al:404 AfterSelf_UninitialisedArgument (attached `j0`, detached `j1`).
    let src =
        "codeunit 50100 C { procedure F(): Text var E: XmlElement; C: XmlCData; N: XmlNode; S: Text; T: Text; begin
        asserterror XmlCData.Create('x').AddAfterSelf(N); S := GetLastErrorText() + '/' + GetLastErrorCode();
        C := XmlCData.Create('x'); E := XmlElement.Create('r'); E.Add(C);
        asserterror C.ReplaceWith(N); S += '|' + GetLastErrorCode(); E.WriteTo(T); exit(S + '|' + T); end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(
        r,
        "Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized./NotInitialized|NotInitialized|<r><![CDATA[x]]></r>"
    );
}

fn run_cdata(body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var R: XmlElement; X: XmlCData; P: XmlElement; S: Text; T: Text; begin
         R := XmlElement.Create('r'); R.Add(XmlElement.Create('a')); X := XmlCData.Create('x'); R.Add(X); R.Add(XmlElement.Create('b'));
         {body} end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn an_attribute_among_sibling_content_stops_after_the_nodes_before_it() {
    // P90CDataEdit.al:887 AfterSelf_ThreeNodes_OneInvalid `t1_6_3`.
    let r = run_cdata(
        "asserterror X.AddAfterSelf(XmlElement.Create('e1'), XmlAttribute.Create('a6', '1'), XmlComment.Create('c3'));
         S := GetLastErrorCode(); R.WriteTo(T); exit(S + '|' + T);",
    );
    assert_eq!(r, "Argument|<r>\r\n  <a /><![CDATA[x]]><e1 /><b /></r>");
}

#[test]
fn replace_with_removes_a_text_like_node_first() {
    // P90CDataEdit.al:681 ReplaceWith_Attached_InvalidKinds `k6`; ReplaceWith_Attached_ExistingNodes `k11`;
    // P104TreeBComment.al:331 `c15` (an empty text adds nothing).
    let r = run_cdata(
        "asserterror X.ReplaceWith(XmlAttribute.Create('a6', '1')); S := GetLastErrorCode() + '|' + Format(X.GetParent(P));
         R.WriteTo(T); S += '|' + T; R.Add(X); X.ReplaceWith(X); S += '|' + Format(X.GetParent(P));
         X.ReplaceWith(''); R.WriteTo(T); exit(S + '|' + T);",
    );
    assert_eq!(
        r,
        "Argument|No|<r>\r\n  <a />\r\n  <b />\r\n</r>|Yes|<r>\r\n  <a />\r\n  <b />\r\n</r>"
    );
}

#[test]
fn a_sibling_operation_on_an_attribute_is_refused_with_its_text() {
    // xml/edit.rs `position_in_parent`: an attribute has an owner element but is no child of it; no corpus case
    // pins a sibling operation there. ADR-071 step 1 keeps this text while the site gets a key.
    let src = "codeunit 50100 C { procedure F(): Text var E: XmlElement; A: XmlAttribute; begin
        E := XmlElement.Create('r'); A := XmlAttribute.Create('x', '2'); E.Add(A);
        A.AddAfterSelf(XmlElement.Create('b')); exit('ok'); end; }";
    let r = al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v);
    match r {
        Err(al_builtins::native::Signal::Unsupported(why)) => {
            assert_eq!(why, "a sibling operation on an attribute or declaration")
        }
        other => panic!("{other:?}"),
    }
}
