// Neighbors of al-runner issue 8580, second round: JsonObject.ReadFromYaml of text outside printable ASCII in the forms
// corpus PR 1515 measured (JsonYamlUtf8.al), the shapes main's pinned reader keeps, and the shapes no cell measures.
// `~` stands for a line feed, `^` for CR LF, {N} {L} {P} {A} {B} for U+0085, U+2028, U+2029, U+00A0, U+FEFF.
codeunit 60935 "YV Yaml Utf8 Tests"
{
    Subtype = Test;

    var
        JO: JsonObject;
        Ok: Boolean;

    local procedure Chr(Code: Integer): Text
    var
        C: Text[1];
    begin
        C[1] := Code;
        exit(C);
    end;

    local procedure Doc(Src: Text): Text
    begin
        Src := Src.Replace('^', Chr(13) + Chr(10)).Replace('~', Chr(10));
        Src := Src.Replace('{N}', Chr(133)).Replace('{L}', Chr(8232)).Replace('{P}', Chr(8233));
        exit(Src.Replace('{A}', Chr(160)).Replace('{B}', Chr(65279)));
    end;

    local procedure ReadStmt(Src: Text): Text
    var
        Out: Text;
    begin
        Clear(JO);
        JO.ReadFromYaml(Doc(Src));
        JO.WriteTo(Out);
        exit(Out);
    end;

    local procedure ReadRet(Src: Text): Text
    var
        Out: Text;
    begin
        Clear(JO);
        Ok := JO.ReadFromYaml(Doc(Src));
        JO.WriteTo(Out);
        exit(Format(Ok) + '|' + Out);
    end;

    local procedure Q(): Text
    var
        Tok: JsonToken;
    begin
        JO.Get('q', Tok);
        exit(Tok.AsValue().AsText());
    end;

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    local procedure IssueDoc(): Text
    begin
        exit('name: x~description: >-~  T1 § 19.11 ask-order test, two turns. Turn 1: a clear item reference but no~' +
            '  shipment/invoice number anywhere - the agent must ask~tests:~  - name: t~    description: Ask-order primary question.~');
    end;

    [Test]
    procedure IssueDocumentLf()
    begin
        Check('{"name":"x","description":"T1 § 19.11 ask-order test, two turns. Turn 1: a clear item reference but no shipment/invoice number anywhere - the agent must ask","tests":[{"name":"t","description":"Ask-order primary question."}]}',
            ReadStmt(IssueDoc()));
    end;

    [Test]
    procedure IssueDocumentCrLfUsedResult()
    var
        Tok: JsonToken;
    begin
        Check('Yes', CopyStr(ReadRet(IssueDoc().Replace('~', '^')), 1, 3));
        JO.Get('description', Tok);
        Check('129', Format(StrLen(Tok.AsValue().AsText())));
    end;

    [Test]
    procedure FoldedBlockInListItemMapping()
    begin
        Check('{"k":[{"name":"t","description":"T1 § 19.11 ask next € line","n":"1"},{"name":"u"}]}',
            ReadStmt('k:~  - name: t~    description: >-~      T1 § 19.11 ask~      next € line~    n: 1~  - name: u'));
    end;

    [Test]
    procedure FoldedStripSection()
    begin
        Check('{"q":"a§b","n":"1"}', ReadStmt('q: >-~  a§b~n: 1'));
    end;

    [Test]
    procedure FoldedKeepEuro()
    begin
        Check('{"q":"a€b\n","n":"1"}', ReadStmt('q: >~  a€b~n: 1'));
    end;

    [Test]
    procedure LiteralEmojiLength()
    begin
        ReadStmt('q: |~  a😀b~n: 1');
        Check('a😀b' + Chr(10), Q());
        Check('5', Format(StrLen(Q())));
    end;

    [Test]
    procedure LiteralStripCombiningMark()
    begin
        ReadStmt('q: |-~  ae' + Chr(769) + 'b~n: 1');
        Check('4', Format(StrLen(Q())));
    end;

    [Test]
    procedure FoldedLineEdges()
    begin
        Check('{"q":"øone twoø øthreeø","n":"1"}', ReadStmt('q: >-~  øone twoø~  øthreeø~n: 1'));
    end;

    [Test]
    procedure FoldedCharOnly()
    begin
        Check('{"q":"ü","n":"1"}', ReadStmt('q: >-~  ü~n: 1'));
    end;

    [Test]
    procedure SingleQuotedValue()
    begin
        Check('Yes|{"q":"a§b","n":"1"}', ReadRet('q: ''a§b''~n: 1'));
    end;

    [Test]
    procedure DoubleQuotedValue()
    begin
        Check('{"q":"a€b","n":"1"}', ReadStmt('q: "a€b"~n: 1'));
    end;

    [Test]
    procedure ListItemWord()
    begin
        Check('{"k":["aéb","c"]}', ReadStmt('k:~  - aéb~  - c'));
    end;

    [Test]
    procedure FlowListAndMapping()
    begin
        Check('{"k":["a§b","c€d","e😀f"]}', ReadStmt('k: [a§b, "c€d", ''e😀f'']'));
        Check('{"q":{"xé":"y😀"}}', ReadStmt('q: {xé: y😀}'));
    end;

    [Test]
    procedure CommentLineAndTailComment()
    begin
        Check('{"q":"v"}', ReadStmt('# c§€😀~q: v'));
        Check('{"q":"vü"}', ReadStmt('q: vü # c§😀'));
    end;

    [Test]
    procedure KeyInBlockDocument()
    begin
        Check('{"a§b":"1","k":["c"]}', ReadStmt('a§b: 1~k:~  - c'));
    end;

    [Test]
    procedure NelInPlainValueUsedResult()
    begin
        ClearLastError();
        Check('No|{}', ReadRet('q: a{N}b~n: 1'));
        Check('The data does not represent valid YAML.', GetLastErrorText());
    end;

    [Test]
    procedure LineSeparatorInFoldedUsedResult()
    begin
        ClearLastError();
        Check('No|{}', ReadRet('q: >-~  a{L}b~n: 1'));
        Check('The data does not represent valid YAML.', GetLastErrorText());
    end;

    [Test]
    procedure ParagraphSeparatorInDoubleQuotedUsedResult()
    begin
        ClearLastError();
        Check('No|{}', ReadRet('q: "a{P}b"~n: 1'));
        Check('The data does not represent valid YAML.', GetLastErrorText());
    end;

    // Main's pinned reader keeps these (flat top lines).
    [Test]
    procedure MainKept_FlatPlainValue()
    begin
        Check('{"q":"a§b","n":"1"}', ReadStmt('q: a§b~n: 1'));
    end;

    [Test]
    procedure MainKept_FlatKey()
    begin
        Check('{"a€b":"1","n":"2"}', ReadStmt('a€b: 1~n: 2'));
    end;

    // No cell measures these: each refuses.
    [Test]
    procedure Refused_UnmeasuredLetterInFolded()
    begin
        ReadStmt('q: >-~  aäb~n: 1');
    end;

    [Test]
    procedure Refused_NoBreakSpaceInFolded()
    begin
        ReadStmt('q: >-~  a{A}b~n: 1');
    end;

    [Test]
    procedure Refused_LeadingByteOrderMark()
    begin
        ReadStmt('{B}q: >-~  a§b~n: 1');
    end;

    [Test]
    procedure Refused_NelStatement()
    begin
        ReadStmt('q: a{N}b~n: 1');
    end;

    [Test]
    procedure Refused_NelInNestedValue()
    begin
        ReadRet('k:~  q: a{N}b');
    end;

    [Test]
    procedure Refused_NelInLiteral()
    begin
        ReadRet('q: |~  a{N}b~n: 1');
    end;

    [Test]
    procedure Refused_NelInComment()
    begin
        ReadRet('# a{N}b~q: >-~  a~n: 1');
    end;

    [Test]
    procedure Refused_NelNonEmptyReceiver()
    begin
        JO.ReadFrom('{"z":1}');
        Ok := JO.ReadFromYaml(Doc('q: a{N}b~n: 1'));
    end;

    [Test]
    procedure Refused_PunctuationInLiteral()
    begin
        ReadStmt('q: |~  a, b.~n: 1');
    end;

    [Test]
    procedure Refused_PunctuationFirstInFolded()
    begin
        ReadStmt('q: >-~  - a~n: 1');
    end;

    [Test]
    procedure Refused_CommaInListItemMappingValue()
    begin
        ReadStmt('tests:~  - name: t~    description: Ask-order primary question, then more.');
    end;

    [Test]
    procedure Refused_SectionInQuotedKey()
    begin
        ReadStmt('"a§b": 1~k:~  - c');
    end;

    [Test]
    procedure Refused_SectionFirstInListItem()
    begin
        ReadStmt('k:~  - §b~  - c');
    end;

    // Punctuation in plain scalars (JsonYamlUtf8.al ReadFromYaml_Plain*_Read, the four places of PlainAll).
    [Test]
    procedure Punct_CommaInNestedValue()
    begin
        Check('{"o":{"q":"a, b","r":"2"},"n":"1"}', ReadStmt('o:~  q: a, b~  r: 2~n: 1'));
    end;

    [Test]
    procedure Punct_IssueLineInListItemUsedResult()
    begin
        Check('Yes|{"k":["Ask-order primary question, then fallthrough ...","c"]}',
            ReadRet('k:~  - Ask-order primary question, then fallthrough ...~  - c'));
    end;

    [Test]
    procedure Punct_SymbolsInNestedListItem()
    begin
        Check('{"o":{"k":["a @ b & c","c"]}}', ReadStmt('o:~  k:~    - a @ b & c~    - c'));
    end;

    [Test]
    procedure Punct_LeadingHyphenInListItem()
    begin
        Check('{"k":["-a b","c"]}', ReadStmt('k:~  - -a b~  - c'));
    end;

    [Test]
    procedure Punct_SpaceHashCommentInNestedValue()
    begin
        Check('{"o":{"q":"a","r":"2"},"n":"1"}', ReadStmt('o:~  q: a #b~  r: 2~n: 1'));
    end;

    [Test]
    procedure Refused_QuotedColonInNestedValue()
    begin
        ReadRet('o:~  q: say "a: b" end~n: 1');
    end;

    [Test]
    procedure Refused_QuotedColonInListItem()
    begin
        ReadStmt('k:~  - say ''a: b'' end~  - c');
    end;

    [Test]
    procedure Refused_CommaTwoLevelsDown()
    begin
        ReadStmt('o:~  p:~    q: a, b~n: 1');
    end;

    [Test]
    procedure Refused_CommaWithLineSeparatorUsedResult()
    begin
        ReadRet('q: a{L}b, c~n: 1');
    end;
}
