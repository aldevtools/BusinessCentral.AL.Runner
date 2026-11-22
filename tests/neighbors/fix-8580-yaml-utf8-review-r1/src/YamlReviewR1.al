// Review round 1 of al-runner issue 8580, second round: the shapes the reviewer named, the punctuation matrix of
// corpus PR 1519 (JsonYamlPunctMatrix.al) at its places, and what the matrix does not measure.
// `~` stands for a line feed, `^` for CR LF, {L} for U+2028, {S} for U+00A7.
codeunit 61300 "YR Yaml Review R1 Tests"
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
        exit(Src.Replace('{L}', Chr(8232)).Replace('{S}', Chr(167)));
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

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    // The reviewer's shapes.
    [Test]
    procedure Refused_LineSeparatorBeforeKeyInFolded()
    begin
        ReadRet('q: >-~  a{L}b: c~n: 1');
    end;

    [Test]
    procedure Refused_ThreeSpacesAfterLoneHyphen()
    begin
        ReadStmt('o:~  q: a -   b~  r: 2~n: 1');
    end;

    [Test]
    procedure Refused_FoldedPunctuationAsLaterListItemPair()
    begin
        ReadStmt('k:~  - a: x~    b: >-~      c d!~    e: 1');
    end;

    [Test]
    procedure Refused_NonAsciiNextToPunctuationInDoubleQuotes()
    begin
        ReadStmt('q: "a{S},b"~n: 1');
    end;

    [Test]
    procedure Refused_NonAsciiValueSpaceHashComment()
    begin
        ReadStmt('o:~  q: a{S} #b~  r: 2~n: 1');
    end;

    [Test]
    procedure Refused_DigitValueSpaceHashComment()
    begin
        ReadStmt('o:~  q: 1 #b~  r: 2~n: 1');
    end;

    // The reporter's two shapes.
    [Test]
    procedure Matrix_FoldedLineEndsInDots()
    begin
        Check('{"name":"x","description":"T1 § 19.11 ask-order test ... next line","n":"1"}',
            ReadStmt('name: x~description: >-~  T1 {S} 19.11 ask-order test ...~  next line~n: 1'));
    end;

    [Test]
    procedure Matrix_CommaInListItemMappingValue()
    begin
        Check('{"k":[{"m":"a, b","r":"2"},"c"]}', ReadStmt('k:~  - m: a, b~    r: 2~  - c'));
    end;

    // Matrix cells.
    [Test]
    procedure Matrix_BadStartInNestedValueUsedResult()
    begin
        Check('No|{}', ReadRet('o:~  q: !ab~  r: 2~n: 1'));
    end;

    [Test]
    procedure Matrix_ColonAtEndInListMapValueUsedResult()
    begin
        Check('No|{}', ReadRet('k:~  - m: ab:~    r: 2~  - c'));
    end;

    [Test]
    procedure Matrix_HashStartIsNullInListItem()
    begin
        Check('{"k":[null,"c"]}', ReadStmt('k:~  - #ab~  - c'));
    end;

    [Test]
    procedure Matrix_AnchorIsNullInNestedValue()
    begin
        Check('{"o":{"q":null,"r":"2"},"n":"1"}', ReadStmt('o:~  q: &ab~  r: 2~n: 1'));
    end;

    [Test]
    procedure Matrix_TrailingHyphenSpace()
    begin
        Check('{"o":{"q":"ab-","r":"2"},"n":"1"}', ReadStmt('o:~  q: ab- ~  r: 2~n: 1'));
    end;

    [Test]
    procedure Matrix_StartMarksInNestedList()
    begin
        Check('{"o":{"k":["$ab","...ab","a#b","ab}"]},"n":"1"}', ReadStmt('o:~  k:~    - $ab~    - ...ab~    - a#b~    - ab}~n: 1'));
    end;

    [Test]
    procedure Matrix_DoubleQuotedBellEscape()
    var
        Tok: JsonToken;
        V: Text;
        Code: Integer;
    begin
        ReadStmt('q: "\ab"~n: 1');
        JO.Get('q', Tok);
        V := Tok.AsValue().AsText();
        Code := V[1];
        Check('2', Format(StrLen(V)));
        Check('7', Format(Code));
    end;

    [Test]
    procedure Matrix_LoneSingleQuoteUsedResult()
    begin
        Check('No|{}', ReadRet('q: ''a''b''~n: 1'));
    end;

    [Test]
    procedure Matrix_HashLineInLiteral()
    begin
        Check('{"q":"#ab\n#ab\n","n":"1"}', ReadStmt('q: |~  #ab~  #ab~n: 1'));
    end;

    // New refusals.
    [Test]
    procedure Refused_BadYamlStatement()
    begin
        ReadStmt('o:~  q: !ab~n: 1');
    end;

    [Test]
    procedure Refused_BadYamlNonEmptyReceiver()
    begin
        JO.ReadFrom('{"z":1}');
        Ok := JO.ReadFromYaml(Doc('o:~  q: !ab~n: 1'));
    end;

    [Test]
    procedure Refused_ListItemTurnsIntoMapping()
    begin
        ReadStmt('k:~  - a : b~  - c');
    end;

    [Test]
    procedure Refused_ListItemNestedList()
    begin
        ReadStmt('k:~  - - ab~  - c');
    end;

    [Test]
    procedure Refused_PunctuationInDeepListMapValue()
    begin
        ReadStmt('o:~  k:~    - m: a, b~n: 1');
    end;

    [Test]
    procedure Refused_HashLineInNestedLiteral()
    begin
        ReadStmt('o:~  q: |~    #ab~n: 1');
    end;

    [Test]
    procedure Refused_SpaceHashFirstInFolded()
    begin
        ReadStmt('q: >-~   #ab~   #ab~n: 1');
    end;

    [Test]
    procedure Refused_MatrixValueCrLf()
    begin
        ReadStmt('o:^  q: a!^n: 1');
    end;

    [Test]
    procedure Refused_PunctuationInNestedKey()
    begin
        ReadStmt('o:~  a,b: 1~n: 2');
    end;

    [Test]
    procedure Refused_PunctuationInFlow()
    begin
        ReadStmt('o:~  q: [a!]~n: 1');
    end;

    [Test]
    procedure Refused_OnlyPunctuationValue()
    begin
        ReadStmt('o:~  q: ...~n: 1');
    end;

    [Test]
    procedure Refused_BackspaceEscape()
    begin
        ReadStmt('q: "a\bb"~n: 1');
    end;

    [Test]
    procedure Refused_QuoteClosedLater()
    begin
        ReadRet('q: "ab~r: "c"~n: 1');
    end;

    [Test]
    procedure Refused_DocumentMarker()
    begin
        ReadStmt('---~o:~  q: a!~n: 1');
    end;

    [Test]
    procedure Refused_PunctuationWithLineSeparator()
    begin
        ReadRet('q: |~  a, b~r: x{L}y~n: 1');
    end;

    [Test]
    procedure Refused_DotsLastInNestedFolded()
    begin
        ReadStmt('o:~  q: >-~    a b...~n: 1');
    end;
}
