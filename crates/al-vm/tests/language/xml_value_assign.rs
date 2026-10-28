//! The assignment form of the Xml node properties (al-runner issue 6045): `Text.Value := x` is the call `Text.Value(x)`,
//! on a variable and on the result of `AsXmlText()`, as `Attr.Value := x` is (xml/XmlAttributes.al:549). alc compiles the
//! assignment form for XmlText, XmlCData and XmlComment `Value` and XmlDeclaration `Version`, `Encoding` and `Standalone`;
//! the call forms are pinned (XmlDocReadWrite.al:622-624 for the declaration).

use al_testkit::text;

fn run(body: &str) -> String {
    let src = format!("codeunit 50100 C {{ {body} }}");
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

/// The reporter's repro: through AsXmlText() and on a variable.
#[test]
fn xml_text_value_assigns_like_the_call() {
    let r = run(
        "procedure F(): Text var Doc: XmlDocument; Element: XmlElement; Node: XmlNode; T: XmlText; R: Text;
        begin
            XmlDocument.ReadFrom('<a>old</a>', Doc);
            Doc.GetRoot(Element);
            Element.GetChildNodes().Get(1, Node);
            Node.AsXmlText().Value := 'new';
            R := Element.InnerText();
            T := Node.AsXmlText();
            T.Value := 'newer';
            exit(R + ',' + Element.InnerText() + ',' + T.Value());
        end;",
    );
    assert_eq!(r, "new,newer,newer");
}

#[test]
fn cdata_comment_and_declaration_properties_assign_like_the_call() {
    let r = run("procedure F(): Text var C: XmlCData; M: XmlComment; D: XmlDeclaration;
        begin
            C := XmlCData.Create('a'); M := XmlComment.Create('b'); D := XmlDeclaration.Create('1.0', 'utf-8', 'no');
            C.Value := 'c'; M.Value := 'd';
            D.Version := '1.0'; D.Encoding := 'utf-16'; D.Standalone := 'yes';
            exit(C.Value() + ',' + M.Value() + ',' + D.Version() + ',' + D.Encoding() + ',' + D.Standalone());
        end;");
    assert_eq!(r, "c,d,1.0,utf-16,yes");
}
