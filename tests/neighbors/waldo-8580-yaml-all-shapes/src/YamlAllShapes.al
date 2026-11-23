// al-runner issue 8580, third round: ReadFromYaml has one YAML reader. The shapes the pinned reader read before (now
// flagged unless the text is a pinned one), shapes of corpus PR 1530 the earlier readers refused, and each refusal of the
// new reader. `~` stands for a line feed, {T} for a tab, {L} for U+2028, {D} for U+00DF, {\} for a backslash.
codeunit 61310 "YA Yaml All Shapes Tests"
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
        Src := Src.Replace('~', Chr(10)).Replace('{T}', Chr(9)).Replace('{L}', Chr(8232)).Replace('{D}', Chr(223));
        exit(Src.Replace('{\}', Chr(92)));
    end;

    local procedure Rep(Unit: Text; Count: Integer): Text
    var
        i: Integer;
        R: Text;
    begin
        for i := 1 to Count do
            R += Unit;
        exit(R);
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
        if Ok then
            exit('Yes|' + Out);
        exit('No|' + GetLastErrorCode() + '|' + Out);
    end;

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    // What the pinned reader read clean: a pinned text as a statement stays clean, every other read is flagged.
    [Test]
    procedure Clean_PinnedTextStatement()
    begin
        Check('{"a":{"b":"1"}}', ReadStmt('a:~  b: 1'));
    end;

    [Test]
    procedure Flagged_FlatLineOtherValue()
    begin
        Check('{"a":"2","b":"x y"}', ReadStmt('a: 2~b: x y'));
    end;

    [Test]
    procedure Flagged_PinnedTextResultUsed()
    begin
        Check('Yes|{"a":"1"}', ReadRet('a: 1'));
    end;

    [Test]
    procedure Flagged_TopListStatementRaises()
    begin
        asserterror ReadStmt('- 1~- 2~- 3');
        Check('InvalidJsonType', GetLastErrorCode());
    end;

    // Shapes of corpus PR 1530 that the earlier readers refused.
    [Test]
    procedure Read_DashLineOpensBlockMapping()
    begin
        Check('{"k":[{"t":{"a":"1","b":"2"},"q":"x","e":"y"}]}', ReadStmt('k:~  - t:~      a: 1~      b: 2~    q: x~    e: y'));
    end;

    [Test]
    procedure Read_AnchorAliasMergeKeyKept()
    begin
        Check('{"a":{"c":"1"},"b":{"<<":{"c":"1"},"d":"2"}}', ReadStmt('a: &x~  c: 1~b:~  <<: *x~  d: 2'));
    end;

    [Test]
    procedure Read_DuplicateKeyLastWins()
    begin
        Check('{"a":"3","b":"2"}', ReadStmt('a: 1~b: 2~a: 3'));
    end;

    [Test]
    procedure Read_TaggedInt()
    begin
        Check('Yes|{"q":1}', ReadRet('q: !!int "1"'));
    end;

    [Test]
    procedure Read_TopListResultUsed()
    begin
        Check('No|InvalidJsonType|{}', ReadRet('- 1~- 2'));
    end;

    [Test]
    procedure Read_YamlErrorResultUsed()
    begin
        Check('No|YamlBadFormat|{}', ReadRet('q: [a, b'));
    end;

    [Test]
    procedure Read_Depth101ResultUsed()
    begin
        Check('No|InvalidJsonData|{}', ReadRet('q: ' + Rep('[', 100) + 'x' + Rep(']', 100)));
    end;

    [Test]
    procedure Read_SurrogatePairEscape()
    begin
        Check('Yes|{"q":"a' + Chr(55357) + Chr(56832) + 'b"}', ReadRet('q: "a{\}uD83D{\}uDE00b"'));
    end;

    // The refusals of the reader.
    [Test]
    procedure Refused_UnmeasuredCharacter()
    begin
        ReadStmt('q: a{D}b');
    end;

    [Test]
    procedure Refused_DepthBetweenMeasured()
    begin
        ReadRet('q: ' + Rep('[', 60) + 'x' + Rep(']', 60));
    end;

    [Test]
    procedure Refused_DeeperThanMeasured()
    begin
        ReadRet('q: ' + Rep('[', 310) + 'x' + Rep(']', 310));
    end;

    [Test]
    procedure Refused_TooDeepStatement()
    begin
        ReadStmt('q: ' + Rep('[', 100) + 'x' + Rep(']', 100));
    end;

    [Test]
    procedure Refused_TaggedTopScalar()
    begin
        ReadRet('!!int 1');
    end;

    [Test]
    procedure Refused_LoneSurrogateEscape()
    begin
        ReadRet('q: "a{\}uD83Db"');
    end;

    [Test]
    procedure Refused_LineSeparatorThatReads()
    begin
        ReadRet('q: "a{L}  b"');
    end;

    [Test]
    procedure Refused_TabAfterIndentSpaces()
    begin
        ReadRet('o:~  {T}q: a');
    end;

    [Test]
    procedure Refused_IntTagOnDecimal()
    begin
        ReadRet('q: !!int 1.5');
    end;

    [Test]
    procedure Refused_BoolTagOnOn()
    begin
        ReadRet('q: !!bool on');
    end;

    [Test]
    procedure Refused_TimestampTagOnTime()
    begin
        ReadRet('q: !!timestamp 2001-12-14 21:59:43');
    end;

    [Test]
    procedure Refused_YamlErrorIntoFilledObject()
    begin
        JO.ReadFrom('{"z":1}');
        Ok := JO.ReadFromYaml('q: [a');
    end;

    [Test]
    procedure Refused_TopListIntoFilledObjectResultUsed()
    begin
        JO.ReadFrom('{"z":1}');
        Ok := JO.ReadFromYaml(Doc('- 1~- 2'));
    end;

    [Test]
    procedure Refused_IntTagOnList()
    begin
        ReadRet('q: !!int [a]');
    end;

    [Test]
    procedure Refused_MapTagOnScalar()
    begin
        ReadRet('q: !!map x');
    end;

    [Test]
    procedure Refused_BangTagOnList()
    begin
        ReadRet('q: ! [a]');
    end;

    [Test]
    procedure Refused_EmptyMapTag()
    begin
        ReadRet('q: !!map');
    end;

    [Test]
    procedure Refused_IntLeadingZero()
    begin
        ReadRet('q: !!int 007');
    end;

    [Test]
    procedure Refused_FloatLarge()
    begin
        ReadRet('q: !!float 100000000000000000000');
    end;

    [Test]
    procedure Refused_BoolNo()
    begin
        ReadRet('q: !!bool no');
    end;

    [Test]
    procedure Refused_TaggedIntKey()
    begin
        ReadRet('!!int 1: a');
    end;

    [Test]
    procedure Refused_TaggedTopStr()
    begin
        ReadRet('!!str x');
    end;

    [Test]
    procedure Refused_TagDirectiveToCore()
    begin
        ReadRet('%TAG !e! tag:yaml.org,2002:~---~q: !e!int "1"');
    end;

    [Test]
    procedure Refused_TwoYamlDirectives()
    begin
        ReadRet('%YAML 1.2~%YAML 1.2~---~q: a');
    end;

    [Test]
    procedure Refused_YamlVersion2()
    begin
        ReadRet('%YAML 2.0~---~q: a');
    end;

    [Test]
    procedure Refused_TabAfterSpacesFirstLine()
    begin
        ReadRet('  {T}q: a');
    end;

    [Test]
    procedure Refused_TabInFlowLine()
    begin
        ReadRet('q: [a,~{T}b]');
    end;

    [Test]
    procedure Refused_AnchorWithDot()
    begin
        ReadRet('a: &a.b 1');
    end;

    [Test]
    procedure Refused_Key1500Chars()
    begin
        ReadRet(Rep('k', 1500) + ': 1');
    end;

    [Test]
    procedure Refused_AliasExpansionBomb()
    var
        Src: Text;
        i: Integer;
    begin
        Src := 'a0: &a0 [x, x, x, x, x, x, x, x, x, x]~';
        for i := 1 to 8 do
            Src += 'a' + Format(i) + ': &a' + Format(i) + ' [' + Rep('*a' + Format(i - 1) + ', ', 9) + '*a' + Format(i - 1) + ']~';
        ReadRet(Src);
    end;

    [Test]
    procedure Refused_Utf16InStream()
    begin
        ReadStream('q: a', TextEncoding::UTF16);
    end;

    [Test]
    procedure Refused_WindowsInStream()
    begin
        ReadStream('q: a', TextEncoding::Windows);
    end;

    // Neighbors that keep reading: the measured forms of the same families.
    [Test]
    procedure Read_TaggedFloatBoolStrKey()
    begin
        Check('Yes|{"f":1.0,"b":true,"s":"1"}', ReadRet('f: !!float 1~b: !!bool yes~!!str s: 1'));
    end;

    [Test]
    procedure Read_Utf8InStream()
    begin
        ReadStream('q: a', TextEncoding::UTF8);
    end;

    local procedure ReadStream(Src: Text; Enc: TextEncoding)
    var
        Store: Record "YA Stream Store" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        Store.Id := 1;
        Store.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Src);
        Store.Insert();
        Store.Data.CreateInStream(InS, Enc);
        Clear(JO);
        Ok := JO.ReadFromYaml(InS);
    end;
}

table 61311 "YA Stream Store"
{
    TableType = Temporary;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}
