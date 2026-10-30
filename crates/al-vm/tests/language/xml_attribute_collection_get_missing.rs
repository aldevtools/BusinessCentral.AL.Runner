//! `XmlAttributeCollection.Get` of an attribute that is not there (corpus master, corpus PR 886:
//! xml/lt-xml-attr-doc-http-members/AttrCollectionGetMissing.al:68-83, :109-113).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 50321 X
{
    local procedure Root(var E: XmlElement)
    var
        Doc: XmlDocument;
    begin
        XmlDocument.ReadFrom('<root a="1" xmlns:p="urn:p" p:b="2" />', Doc);
        Doc.GetRoot(E);
    end;

    [TryFunction]
    local procedure TryShow(A: XmlAttribute; var Shown: Text)
    begin
        Shown := A.Name() + '=' + A.Value();
    end;

    local procedure Show(A: XmlAttribute): Text
    var
        Shown: Text;
    begin
        if TryShow(A, Shown) then
            exit(Shown);
        exit('E[' + GetLastErrorText() + '|' + GetLastErrorCode() + ']');
    end;

    [TryFunction]
    local procedure TryGet3(Attrs: XmlAttributeCollection; Name: Text; Ns: Text; var A: XmlAttribute)
    begin
        Attrs.Get(Name, Ns, A);
    end;

    [TryFunction]
    local procedure TryGet2(Attrs: XmlAttributeCollection; Name: Text; var A: XmlAttribute)
    begin
        Attrs.Get(Name, A);
    end;

    local procedure Stmt3(Attrs: XmlAttributeCollection; Name: Text; Ns: Text): Text
    var
        A: XmlAttribute;
    begin
        if TryGet3(Attrs, Name, Ns, A) then
            exit('ok|' + Show(A));
        exit('E[' + GetLastErrorText() + '|' + GetLastErrorCode() + ']');
    end;

    procedure Expression(): Text
    var
        E: XmlElement;
        Attrs: XmlAttributeCollection;
        Fresh: XmlAttribute;
        Held: XmlAttribute;
        P: XmlAttribute;
        Out: Text;
    begin
        Root(E);
        Attrs := E.Attributes();
        Out := Format(Attrs.Get('zz', '', Fresh)) + ' ' + Show(Fresh);
        Attrs.Get('a', Held);
        Out += ';' + Format(Attrs.Get('zz', '', Held)) + ' ' + Show(Held);
        Out += ';' + Format(Attrs.Get('b', '', P)) + ' ' + Show(P);
        Out += ';' + Format(Attrs.Get('b', 'urn:p', P)) + ' ' + Show(P);
        exit(Out);
    end;

    procedure Statement(): Text
    var
        E: XmlElement;
        Attrs: XmlAttributeCollection;
        A: XmlAttribute;
        Out: Text;
    begin
        Root(E);
        Attrs := E.Attributes();
        Out := Stmt3(Attrs, 'zz', '') + ';' + Stmt3(Attrs, 'a', '') + ';' + Stmt3(Attrs, 'b', '') + ';' + Stmt3(Attrs, 'zz', 'urn:x');
        if TryGet2(Attrs, 'zz', A) then
            exit(Out + ';ok');
        exit(Out + ';E[' + GetLastErrorText() + '|' + GetLastErrorCode() + ']');
    end;
}
"#;

#[test]
fn a_missing_attribute_answers_false_and_leaves_the_variable() {
    assert_eq!(
        text(run_in(SRC, "X", "Expression")),
        "No E[Microsoft.Dynamics.Nav.Runtime.NavXmlAttribute variable not initialized.|NotInitialized];No a=1;\
         No E[Microsoft.Dynamics.Nav.Runtime.NavXmlAttribute variable not initialized.|NotInitialized];Yes p:b=2"
    );
}

#[test]
fn a_get_statement_of_a_missing_attribute_raises_xml_node_not_found() {
    assert_eq!(
        text(run_in(SRC, "X", "Statement")),
        "E[The XML node with the name '{}zz' could not be found.|XmlNodeNotFound];ok|a=1;\
         E[The XML node with the name '{}b' could not be found.|XmlNodeNotFound];\
         E[The XML node with the name '{urn:x}zz' could not be found.|XmlNodeNotFound];\
         E[The XML node with the name 'zz' could not be found.|XmlNodeNotFound]"
    );
}
