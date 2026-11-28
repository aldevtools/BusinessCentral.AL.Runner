// al-runner issue 8560 and the XML cells of the same branch: a node holds its parent strongly; E.Add(E) and Remove of a
// lone created element; GetParent, IsXmlText and IsXmlDocument of an unassigned XmlNode; IsEmpty of a parsed <a></a>.
// One test per neighbor shape. A test that expects a value fails or is not run where the runner answers otherwise.
codeunit 61940 "XN Xml Node Tests"
{
    Subtype = Test;

    local procedure Load(Src: Text; var Root: XmlElement)
    var
        Doc: XmlDocument;
    begin
        XmlDocument.ReadFrom(Src, Doc);
        Doc.GetRoot(Root);
    end;

    local procedure Str(E: XmlElement): Text
    var
        T: Text;
    begin
        E.WriteTo(T);
        exit(T);
    end;

    local procedure Expect(Actual: Text; Wanted: Text)
    begin
        if Actual <> Wanted then
            Error('got [%1], wanted [%2]', Actual, Wanted);
    end;

    local procedure ExpectInvalidOperation()
    begin
        Expect(GetLastErrorCode(), 'InvalidOperation');
    end;

    // ----- a node keeps its document alive

    [Test]
    procedure GetParent_DocumentVariableGone()
    var
        E: XmlElement;
        C: XmlElement;
        Par: XmlElement;
        N: XmlNode;
    begin
        Load('<a><b><c/></b></a>', E);
        E.GetDescendantElements('c').Get(1, N);
        C := N.AsXmlElement();
        Expect(Format(C.GetParent(Par)) + Par.Name(), 'Yesb');
        Expect(Format(E.GetParent(Par)), 'No');
    end;

    [Test]
    procedure GetDocument_DocumentVariableGone()
    var
        E: XmlElement;
        Doc: XmlDocument;
        T: Text;
    begin
        Load('<a><b/></a>', E);
        Expect(Format(E.GetDocument(Doc)), 'Yes');
        Doc.GetRoot(E);
        Expect(E.Name(), 'a');
    end;

    [Test]
    procedure GetDocument_ElementVariableAndDocumentGone()
    var
        E: XmlElement;
        C: XmlElement;
        Doc: XmlDocument;
        N: XmlNode;
    begin
        Load('<a><b/></a>', E);
        E.SelectSingleNode('b', N);
        C := N.AsXmlElement();
        Clear(E);
        Expect(Format(C.GetDocument(Doc)), 'Yes');
    end;

    [Test]
    procedure SelectSingleNode_AbsoluteFromRoot_DocumentVariableGone()
    var
        E: XmlElement;
        N: XmlNode;
    begin
        Load('<a><b>1</b></a>', E);
        Expect(Format(E.SelectSingleNode('/a/b', N)), 'Yes');
        Expect(N.AsXmlElement().InnerText(), '1');
    end;

    [Test]
    procedure SelectSingleNode_ParentAxisFromChild_ElementVariableGone()
    var
        E: XmlElement;
        N: XmlNode;
        C: XmlNode;
    begin
        Load('<a><b/></a>', E);
        E.SelectSingleNode('b', C);
        Clear(E);
        Expect(Format(C.SelectSingleNode('..', N)), 'Yes');
        Expect(N.AsXmlElement().Name(), 'a');
    end;

    [Test]
    procedure AddAfterSelf_Comment_RootDocumentVariableGone()
    var
        E: XmlElement;
        Doc: XmlDocument;
        T: Text;
    begin
        Load('<a/>', E);
        E.AddAfterSelf(XmlComment.Create('c'));
        E.GetDocument(Doc);
        Doc.WriteTo(T);
        Expect(T, '<?xml version="1.0" encoding="utf-16"?>' + CRLF() + '<a />' + CRLF() + '<!--c-->');
    end;

    [Test]
    procedure AddBeforeSelf_Comment_RootDocumentVariableGone()
    var
        E: XmlElement;
        Doc: XmlDocument;
        T: Text;
    begin
        Load('<a/>', E);
        E.AddBeforeSelf(XmlComment.Create('c'));
        E.GetDocument(Doc);
        Doc.WriteTo(T);
        Expect(T, '<?xml version="1.0" encoding="utf-16"?>' + CRLF() + '<!--c-->' + CRLF() + '<a />');
    end;

    [Test]
    procedure AttributeRemove_ElementVariableGone()
    var
        E: XmlElement;
        A: XmlAttribute;
        Par: XmlElement;
    begin
        Load('<e a="1" b="2"/>', E);
        E.Attributes().Get('a', A);
        Clear(E);
        A.GetParent(Par);
        A.Remove();
        Expect(Str(Par), '<e b="2" />');
    end;

    [Test]
    procedure AttributeGetParent_ElementVariableGone()
    var
        E: XmlElement;
        A: XmlAttribute;
        Par: XmlElement;
    begin
        Load('<e a="1"/>', E);
        E.Attributes().Get('a', A);
        Clear(E);
        Expect(Format(A.GetParent(Par)) + Par.Name(), 'Yese');
    end;

    [Test]
    procedure AttributeWriteTo_NamespacedLone()
    var
        A: XmlAttribute;
        T: Text;
    begin
        A := XmlAttribute.Create('x', 'urn:u', '1');
        A.WriteTo(T);
        Expect(T, 'p0:x="1"');
    end;

    [Test]
    procedure AttributeWriteTo_NamespacedElementVariableGone()
    var
        E: XmlElement;
        A: XmlAttribute;
        T: Text;
    begin
        Load('<e xmlns:p="urn:u" p:x="1"/>', E);
        E.Attributes().Get('x', 'urn:u', A);
        Clear(E);
        A.WriteTo(T);
        Expect(T, 'p:x="1"');
    end;

    // ----- E.Add(E)

    [Test]
    procedure SelfAdd_CreatedLone()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        E.Add(E);
        Expect(Str(E), '<a>' + CRLF() + '  <a />' + CRLF() + '</a>');
    end;

    [Test]
    procedure SelfAdd_WithChild()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        E.Add(XmlElement.Create('b'));
        E.Add(E);
        Expect(Str(E), '<a>' + CRLF() + '  <b />' + CRLF() + '  <a>' + CRLF() + '    <b />' + CRLF() + '  </a>' + CRLF() + '</a>');
    end;

    [Test]
    procedure SelfAdd_WithAttribute()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        E.SetAttribute('x', '1');
        E.Add(E);
        Expect(Str(E), '<a x="1">' + CRLF() + '  <a x="1" />' + CRLF() + '</a>');
    end;

    [Test]
    procedure SelfAdd_Namespaced()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a', 'urn:u');
        E.Add(E);
        Expect(Str(E), '<a xmlns="urn:u">' + CRLF() + '  <a />' + CRLF() + '</a>');
    end;

    [Test]
    procedure SelfAdd_TakenOffParent()
    var
        P: XmlElement;
        E: XmlElement;
    begin
        P := XmlElement.Create('p');
        E := XmlElement.Create('a');
        P.Add(E);
        E.Remove();
        E.Add(E);
        Expect(Str(E), '<a>' + CRLF() + '  <a />' + CRLF() + '</a>');
    end;

    [Test]
    procedure SelfAdd_TwoArguments()
    var
        E: XmlElement;
        X: XmlElement;
    begin
        E := XmlElement.Create('a');
        X := XmlElement.Create('x');
        E.Add(X, E);
        Expect(Str(E), '<a>' + CRLF() + '  <x />' + CRLF() + '  <a />' + CRLF() + '</a>');
    end;

    [Test]
    procedure SelfAdd_ResultUsed()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        Expect(Format(E.Add(E)), 'Yes');
    end;

    [Test]
    procedure SelfAddFirst_CreatedLone()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        E.AddFirst(E);
        Expect(Str(E), '<a>' + CRLF() + '  <a />' + CRLF() + '</a>');
    end;

    [Test]
    procedure SelfAdd_ArgumentAsXmlNode()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        E.Add(E.AsXmlNode());
        Expect(Str(E), '<a>' + CRLF() + '  <a />' + CRLF() + '</a>');
    end;

    [Test]
    procedure AddToOwnChild_Parentless()
    var
        E: XmlElement;
        C: XmlElement;
    begin
        E := XmlElement.Create('a');
        C := XmlElement.Create('c');
        E.Add(C);
        asserterror C.Add(E);
        ExpectInvalidOperation();
    end;

    // ----- Remove of a lone element

    [Test]
    procedure LoneRemove_CreatedLone()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('lone');
        asserterror E.Remove();
        ExpectInvalidOperation();
    end;

    [Test]
    procedure LoneRemove_WithChild()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('lone');
        E.Add(XmlElement.Create('b'));
        asserterror E.Remove();
        ExpectInvalidOperation();
    end;

    [Test]
    procedure LoneRemove_WithAttribute()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('lone');
        E.SetAttribute('x', '1');
        asserterror E.Remove();
        ExpectInvalidOperation();
    end;

    [Test]
    procedure LoneRemove_Namespaced()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('lone', 'urn:u');
        asserterror E.Remove();
        ExpectInvalidOperation();
    end;

    [Test]
    procedure LoneRemove_TakenOffParent()
    var
        P: XmlElement;
        E: XmlElement;
    begin
        P := XmlElement.Create('p');
        E := XmlElement.Create('lone');
        P.Add(E);
        E.Remove();
        asserterror E.Remove();
        ExpectInvalidOperation();
    end;

    [Test]
    procedure LoneRemove_ThroughXmlNode()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('lone');
        asserterror E.AsXmlNode().Remove();
        ExpectInvalidOperation();
    end;

    [Test]
    procedure LoneRemove_ResultUsed()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('lone');
        Expect(Format(E.Remove()), 'No');
    end;

    // ----- unassigned XmlNode

    [Test]
    procedure Unassigned_GetParent_ResultUsed()
    var
        N: XmlNode;
        Par: XmlElement;
    begin
        asserterror Expect(Format(N.GetParent(Par)), 'No');
        Expect(GetLastErrorCode(), 'NotInitialized');
    end;

    [Test]
    procedure Unassigned_GetParent_Statement()
    var
        N: XmlNode;
        Par: XmlElement;
    begin
        asserterror N.GetParent(Par);
        Expect(GetLastErrorCode(), 'NotInitialized');
    end;

    [Test]
    procedure Unassigned_IsXmlText_IsXmlDocument()
    var
        N: XmlNode;
    begin
        Expect(Format(N.IsXmlText()) + Format(N.IsXmlDocument()), 'NoNo');
    end;

    [Test]
    procedure Unassigned_IsXmlComment()
    var
        N: XmlNode;
    begin
        Expect(Format(N.IsXmlComment()), 'No');
    end;

    [Test]
    procedure Cleared_IsXmlText()
    var
        N: XmlNode;
    begin
        N := XmlText.Create('t').AsXmlNode();
        Clear(N);
        Expect(Format(N.IsXmlText()), 'No');
    end;

    [Test]
    procedure NotFound_IsXmlText()
    var
        E: XmlElement;
        N: XmlNode;
    begin
        Load('<a/>', E);
        E.SelectSingleNode('missing', N);
        Expect(Format(N.IsXmlText()) + Format(N.IsXmlDocument()), 'NoNo');
    end;

    // ----- IsEmpty of a parsed <a></a>

    [Test]
    procedure IsEmpty_ParsedOpenRoot()
    var
        E: XmlElement;
    begin
        Load('<a></a>', E);
        Expect(Format(E.IsEmpty()), 'No');
    end;

    [Test]
    procedure IsEmpty_ParsedOpenRoot_ChildAddedAndRemoved()
    var
        E: XmlElement;
        C: XmlElement;
    begin
        Load('<a></a>', E);
        C := XmlElement.Create('b');
        E.Add(C);
        C.Remove();
        Expect(Format(E.IsEmpty()) + Str(E), 'Yes<a></a>');
    end;

    [Test]
    procedure IsEmpty_CopyOfParsedOpenRootThatGotAChild()
    var
        E: XmlElement;
        C: XmlElement;
        P: XmlElement;
        N: XmlNode;
    begin
        Load('<a></a>', E);
        C := XmlElement.Create('b');
        E.Add(C);
        C.Remove();
        P := XmlElement.Create('p');
        P.Add(E);
        P.GetChildElements().Get(1, N);
        Expect(Format(N.AsXmlElement().IsEmpty()), 'Yes');
    end;

    [Test]
    procedure IsEmpty_ParsedOpenNested()
    var
        E: XmlElement;
        N: XmlNode;
    begin
        Load('<r><a></a></r>', E);
        E.SelectSingleNode('a', N);
        Expect(Format(N.AsXmlElement().IsEmpty()), 'Yes');
    end;

    [Test]
    procedure IsEmpty_ParsedOpenRootWithAttribute()
    var
        E: XmlElement;
    begin
        Load('<a x="1"></a>', E);
        Expect(Format(E.IsEmpty()), 'Yes');
    end;

    [Test]
    procedure IsEmpty_ParsedOpenRootNamespaced()
    var
        E: XmlElement;
    begin
        Load('<p:a xmlns:p="urn:u"></p:a>', E);
        Expect(Format(E.IsEmpty()), 'Yes');
    end;

    [Test]
    procedure IsEmpty_ParsedOpenRoot_AttributeAdded()
    var
        E: XmlElement;
    begin
        Load('<a></a>', E);
        E.SetAttribute('x', '1');
        Expect(Format(E.IsEmpty()), 'Yes');
    end;

    [Test]
    procedure IsEmpty_ParsedOpenRoot_DocumentKept()
    var
        Doc: XmlDocument;
        E: XmlElement;
    begin
        XmlDocument.ReadFrom('<a></a>', Doc);
        Doc.GetRoot(E);
        Expect(Format(E.IsEmpty()), 'No');
    end;

    [Test]
    procedure IsEmpty_CreatedWithEmptyText()
    var
        E: XmlElement;
    begin
        E := XmlElement.Create('x', '', '');
        Expect(Format(E.IsEmpty()) + Str(E), 'No<x></x>');
    end;

    local procedure CRLF(): Text
    var
        T: Text[2];
    begin
        T[1] := 13;
        T[2] := 10;
        exit(T);
    end;
}
