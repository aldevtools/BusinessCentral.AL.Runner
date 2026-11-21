// Neighbors of al-runner issue 8580: JsonObject.ReadFromYaml of the shapes corpus PR 1493 measured (blank lines, CRLF,
// block scalars, comments, quotes, flow forms, an AI eval dataset), the shapes main's pinned reader keeps, and the
// shapes no cell measures. `~` stands for the line break.
codeunit 60920 "YN Yaml Tests"
{
    Subtype = Test;

    var
        JO: JsonObject;
        Ok: Boolean;

    local procedure Lf(): Text
    var
        C: Text[1];
    begin
        C[1] := 10;
        exit(C);
    end;

    local procedure CrLf(): Text
    var
        C: Text[2];
    begin
        C[1] := 13;
        C[2] := 10;
        exit(C);
    end;

    local procedure Tab(): Text
    var
        C: Text[1];
    begin
        C[1] := 9;
        exit(C);
    end;

    local procedure ReadStmt(Src: Text): Text
    var
        Out: Text;
    begin
        Clear(JO);
        JO.ReadFromYaml(Src);
        JO.WriteTo(Out);
        exit(Out);
    end;

    local procedure ReadRet(Src: Text): Text
    var
        Out: Text;
    begin
        Clear(JO);
        Ok := JO.ReadFromYaml(Src);
        JO.WriteTo(Out);
        exit(Format(Ok) + '|' + Out);
    end;

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    [Test]
    procedure BlankLineBetweenTopListItems_Statement()
    begin
        asserterror ReadStmt('- a~~- b'.Replace('~', Lf()));
        Check('JSON tokens of type ''Array'' are not supported by this implementation.', GetLastErrorText());
        Check('InvalidJsonType', GetLastErrorCode());
    end;

    [Test]
    procedure BlankLineBetweenTopListItems_UsedResult()
    begin
        ClearLastError();
        Check('No|{}', ReadRet('- a~~- b'.Replace('~', Lf())));
        Check('JSON tokens of type ''Array'' are not supported by this implementation.', GetLastErrorText());
    end;

    [Test]
    procedure CrLfTopList_UsedResult()
    begin
        ClearLastError();
        Check('No|{}', ReadRet('- a~- b'.Replace('~', CrLf())));
        Check('InvalidJsonType', GetLastErrorCode());
    end;

    [Test]
    procedure BlankLineInListAtTopKeyIndent()
    begin
        Check('{"k":["a","b","c"]}', ReadStmt('k:~- a~~- b~~- c'.Replace('~', Lf())));
    end;

    [Test]
    procedure BlankLineInNestedListCrLf()
    begin
        Check('Yes|{"k":["a","b"]}', ReadRet('k:~  - a~~  - b'.Replace('~', CrLf())));
    end;

    [Test]
    procedure BlankLineBetweenNestedAndTopKey()
    begin
        Check('{"a":{"b":"1"},"c":"2"}', ReadStmt('a:~  b: 1~~c: 2'.Replace('~', Lf())));
    end;

    [Test]
    procedure SpacesOnlyLineInDeepMapping()
    begin
        Check('{"a":{"b":{"c":"1","d":"2"}}}', ReadStmt('a:~  b:~    c: 1~      ~    d: 2'.Replace('~', Lf())));
    end;

    [Test]
    procedure BlankLineBetweenListItemMappings()
    begin
        Check('{"k":[{"x":"1","y":"2"},{"x":"3","y":"4"}]}', ReadStmt('k:~  - x: 1~    y: 2~~  - x: 3~    y: 4'.Replace('~', Lf())));
    end;

    [Test]
    procedure TrailingLineFeeds()
    begin
        Check('{"a":{"b":"1"}}', ReadStmt('a:~  b: 1~~~'.Replace('~', Lf())));
    end;

    [Test]
    procedure FlowFormsInListItemMapping()
    begin
        Check('{"k":[{"tags":["a","b c"],"opts":{"x":"1"}},{"tags":[]}]}',
            ReadStmt('k:~  - tags: [a, "b c"]~    opts: {x: 1}~~  - tags: []'.Replace('~', Lf())));
    end;

    [Test]
    procedure LiteralAndFoldedBlockScalars()
    begin
        Check('{"q":"line one\nline two\n","n":"1"}', ReadStmt('q: |~  line one~  line two~n: 1'.Replace('~', Lf())));
        Check('{"q":"line one line two","n":"1"}', ReadStmt('q: >-~  line one~  line two~n: 1'.Replace('~', Lf())));
        Check('{"q":"line one\nline three\n","n":"1"}', ReadStmt('q: >~  line one~~  line three~n: 1'.Replace('~', Lf())));
    end;

    [Test]
    procedure LiteralBlockScalarAtEnd()
    begin
        Check('{"q":"line one\nline two"}', ReadStmt('q: |~  line one~  line two'.Replace('~', Lf())));
    end;

    [Test]
    procedure CommentsAndQuotedKeys()
    begin
        Check('{"a":"1","b":"2"}', ReadStmt('# head~a: 1 # tail~~# between~b: 2'.Replace('~', Lf())));
        Check('{"Some Key":"two words","Other Key":"x"}', ReadStmt('"Some Key": two words~''Other Key'': "x"'.Replace('~', Lf())));
    end;

    [Test]
    procedure ScalarForms()
    begin
        Check('{"a":"1.5","b":"-3","c":"0010","d":"1e3"}', ReadStmt('a: 1.5~b: -3~c: 0010~d: 1e3'.Replace('~', Lf())));
        Check('{"a":"two words here","b":"yes","c":null,"d":null}', ReadStmt('a: two words here~b: yes~c: ~~d: Null'.Replace('~', Lf())));
    end;

    [Test]
    procedure EvalDatasetDocument()
    begin
        Check('{"name":"Demo","tests":[{"name":"First case","question":"Line one\nLine two\n","Expected Amount":"10","expected_data":{"lines":[{"Item No.":"1000","quantity":"2"}]}},{"name":"Second case","description":"plain words","tags":["alpha","beta"]}]}',
            ReadStmt(DatasetDoc().Replace('~', Lf())));
    end;

    [Test]
    procedure EvalDatasetDocumentCrLf()
    begin
        Check('Yes|{"name":"Demo","tests":[{"name":"First case","question":"Line one\nLine two\n","Expected Amount":"10","expected_data":{"lines":[{"Item No.":"1000","quantity":"2"}]}},{"name":"Second case","description":"plain words","tags":["alpha","beta"]}]}',
            ReadRet(DatasetDoc().Replace('~', CrLf())));
    end;

    local procedure DatasetDoc(): Text
    begin
        exit('# Own dataset~name: Demo~tests:~  - name: "First case"~    question: |~      Line one~      Line two~' +
            '    "Expected Amount": 10~    expected_data:~      lines:~        - "Item No.": "1000"~          quantity: 2~' +
            '~  - name: Second case~    description: plain words # a comment~    tags: [alpha, beta]~');
    end;

    // Main's pinned reader keeps these.
    [Test]
    procedure MainKept_FlatLinesWithBlankLine()
    begin
        Check('{"a":"1","b":"hello world"}', ReadStmt('a: 1~~b: hello world'.Replace('~', Lf())));
    end;

    [Test]
    procedure MainKept_TopListLfUsedResult()
    begin
        Check('No|{}', ReadRet('- a~- b'.Replace('~', Lf())));
    end;

    // No cell measures these: each refuses.
    [Test]
    procedure Refused_QuotedNullValue()
    begin
        ReadStmt('a: ''null''~~b:~  c: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_QuotedTrueValue()
    begin
        ReadStmt('a: "true"~~b:~  c: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_KeyNull()
    begin
        ReadStmt('null: a~~b:~  c: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_KeyTrue()
    begin
        ReadStmt('true: a~~b:~  c: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_KeyDigit()
    begin
        ReadStmt('1: a~~b:~  c: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_KeyUnderscore()
    begin
        ReadStmt('_a: 1~~b:~  c: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_NestedNull()
    begin
        ReadStmt('a:~  b: null~~  c: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_ItemMappingNull()
    begin
        ReadStmt('k:~  - x: Null~~  - x: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_KeyNullAfterTopLine()
    begin
        ReadStmt('x: 1~a:~b: 2~~c: 3'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_ItemMappingInListAtKeyIndent()
    begin
        ReadStmt('k:~- x: 1~~- x: 2'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_FlowItemInListAtKeyIndent()
    begin
        ReadStmt('k:~- [a]~~- b'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_Tab()
    begin
        ReadStmt(('k:~' + Tab() + '- a~~- b').Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_KeepIndicator()
    begin
        ReadStmt('q: |+~  x~~n: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_IndentationIndicator()
    begin
        ReadStmt('q: |2~  x~~n: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_AnchorAndAlias()
    begin
        ReadStmt('a: &x 1~~b: *x'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_DocumentMarker()
    begin
        ReadStmt('---~a: 1~~b: 2'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_IndentedComment()
    begin
        ReadStmt('k:~  # c~  - a~~  - b'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_PunctuationInBlockScalar()
    begin
        ReadStmt('q: |~  x, y.~n: 1~~m: 2'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_BlankLineAfterBlockScalar()
    begin
        ReadStmt('q: |~  x~~n: 1'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_MixedLineEnds()
    begin
        ReadStmt(('k:' + CrLf() + '  - a~~  - b').Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_BooleanWordBelowTop()
    begin
        ReadStmt('a:~  b: true~~  c: 2'.Replace('~', Lf()));
    end;

    [Test]
    procedure Refused_ListAtNestedKeyIndent()
    begin
        ReadStmt('a:~  k:~  - x~~  m: 1'.Replace('~', Lf()));
    end;
}
