//! Clear of an XmlElement, XmlNode and XmlDocument variable (corpus branch al-runtime-rs-staging-ul at b753d3fb7a,
//! xml/lt-ul-clear-ref/UlClearXmlRef.al, codeunit 90420): the cleared variable reads as an unassigned one, and another
//! variable that holds the same node or document keeps it.

use al_testkit::text;

const HELPERS: &str = "
    local procedure Err(): Text
    begin
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    [TryFunction]
    local procedure TryName(E: XmlElement; var Result: Text)
    begin
        Result := E.Name();
    end;

    local procedure Name(E: XmlElement): Text
    var
        T: Text;
    begin
        if TryName(E, T) then
            exit(T);
        exit(Err());
    end;

    [TryFunction]
    local procedure TryNodeIsElement(N: XmlNode; var Result: Boolean)
    begin
        Result := N.IsXmlElement();
    end;

    [TryFunction]
    local procedure TryNodeAsElementName(N: XmlNode; var Result: Text)
    var
        E: XmlElement;
    begin
        E := N.AsXmlElement();
        Result := E.Name();
    end;

    local procedure NodeInfo(N: XmlNode): Text
    var
        B: Boolean;
        T: Text;
        R: Text;
    begin
        if TryNodeIsElement(N, B) then
            R := Format(B)
        else
            R := Err();
        if TryNodeAsElementName(N, T) then
            R += ';' + T
        else
            R += ';' + Err();
        exit(R);
    end;

    [TryFunction]
    local procedure TryDocRoot(D: XmlDocument; var Result: Text)
    var
        R: XmlElement;
    begin
        D.GetRoot(R);
        Result := R.Name();
    end;

    local procedure Root(D: XmlDocument): Text
    var
        T: Text;
    begin
        if TryDocRoot(D, T) then
            exit(T);
        exit(Err());
    end;

    [TryFunction]
    local procedure TryWrite(D: XmlDocument; var Result: Text)
    begin
        D.WriteTo(Result);
    end;

    local procedure Write(D: XmlDocument): Text
    var
        T: Text;
    begin
        if TryWrite(D, T) then
            exit(T.Substring(T.LastIndexOf('<')));
        exit(Err());
    end;

    [TryFunction]
    local procedure TryAddFresh(D: XmlDocument)
    begin
        D.Add(XmlElement.Create('fresh'));
    end;
";

fn run(body: &str) -> String {
    let src = format!("codeunit 50100 C {{ {HELPERS} {body} }}");
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

const NOT_INIT: &str = "Microsoft.Dynamics.Nav.Runtime.NavXmlElement variable not initialized.|NotInitialized";

// UlClearXmlRef.al ClearXmlElement_SecondVariable.
#[test]
fn a_cleared_element_is_unassigned_and_another_variable_keeps_the_node() {
    let r = run("procedure F(): Text
        var
            E: XmlElement;
            Alias: XmlElement;
            Doc: XmlDocument;
            Root: XmlElement;
            Child: XmlElement;
            R: Text;
        begin
            Doc := XmlDocument.Create();
            Root := XmlElement.Create('root');
            Doc.Add(Root);
            Child := XmlElement.Create('child');
            Root.Add(Child);
            E := Child;
            Alias := Child;
            Clear(E);
            R := Name(E) + '#' + Name(Alias) + '#' + Name(Child) + '#';
            Doc.GetRoot(Root);
            R += Root.AsXmlNode().AsXmlElement().InnerXml() + '#' + Format(Root.GetChildElements().Count()) + '#';
            E := XmlElement.Create('again');
            R += Name(E);
            exit(R);
        end;");
    assert_eq!(r, format!("{NOT_INIT}#child#child#<child />#1#again"));
}

// UlClearXmlRef.al ClearXmlElement_NeverAssigned.
#[test]
fn a_cleared_unassigned_element_stays_unassigned() {
    let r = run("procedure F(): Text
        var
            E: XmlElement;
            R: Text;
        begin
            R := Name(E) + '#';
            Clear(E);
            exit(R + Name(E));
        end;");
    assert_eq!(r, format!("{NOT_INIT}#{NOT_INIT}"));
}

// UlClearXmlRef.al ClearXmlNode_SecondVariable.
#[test]
fn a_cleared_node_is_empty_and_another_variable_keeps_the_node() {
    let r = run("procedure F(): Text
        var
            E: XmlElement;
            N: XmlNode;
            Alias: XmlNode;
        begin
            E := XmlElement.Create('elem');
            N := E.AsXmlNode();
            Alias := N;
            Clear(N);
            exit(NodeInfo(N) + '#' + NodeInfo(Alias) + '#' + E.Name());
        end;");
    assert_eq!(
        r,
        "No;Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavXmlNode to Microsoft.Dynamics.Nav.Runtime.NavXmlElement.|Conversion#Yes;elem#elem"
    );
}

// UlClearXmlRef.al ClearXmlDocument_SecondVariable.
#[test]
fn a_cleared_document_is_unassigned_and_another_variable_keeps_the_document() {
    let r = run("procedure F(): Text
        var
            D: XmlDocument;
            Alias: XmlDocument;
            R: Text;
        begin
            D := XmlDocument.Create();
            D.Add(XmlElement.Create('root'));
            Alias := D;
            Clear(D);
            R := Root(D) + '#' + Root(Alias) + '#' + Write(D) + '#' + Write(Alias) + '#';
            if TryAddFresh(D) then
                R += 'ok#'
            else
                R += Err() + '#';
            exit(R + Write(D) + '#' + Write(Alias));
        end;");
    assert_eq!(
        r,
        "The XML node is not set.|XmlNodeNotFound#root#The requested operation cannot be performed in this context.|InvalidOperation#<root />#ok#<fresh />#<root />"
    );
}

// xml/XmlElement.al:815-836 Default_Element_Uninitialized: Name, Add, InnerText and WriteTo of an unassigned element.
#[test]
fn an_unassigned_element_raises_not_initialized() {
    let r = run("procedure F(): Text
        var
            E: XmlElement;
            Txt: Text;
            R: Text;
        begin
            asserterror Txt := E.Name();
            R := Err() + '#';
            asserterror E.Add(XmlText.Create('x'));
            R += Err() + '#';
            asserterror Txt := E.InnerText();
            R += Err() + '#';
            asserterror E.WriteTo(Txt);
            exit(R + Err());
        end;");
    assert_eq!(
        r,
        format!(
            "{NOT_INIT}#{NOT_INIT}#{NOT_INIT}#Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized.|NotInitialized"
        )
    );
}
