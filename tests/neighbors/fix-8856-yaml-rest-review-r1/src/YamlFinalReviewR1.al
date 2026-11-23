// Neighbors of al-runner issue 8856, review round 1: the shapes next to the ReadFromYaml cells of corpus PR 1549 that
// no cell measures (each refuses), the shapes main answered without a cell, and the two reads now marked.
// `~` stands for a line feed, {T} for a tab, {U} for U+0015.
table 61360 "YN Yaml Review Blob"
{
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

codeunit 61361 "YN Yaml Review R1 Tests"
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
        exit(Src.Replace('~', Chr(10)).Replace('{T}', Chr(9)).Replace('{U}', Chr(21)));
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

    [Test]
    procedure Refused_Tag_FloatTwoSigns()
    begin
        ReadRet('q: !!float --1');
    end;

    [Test]
    procedure Refused_Tag_FloatPlusMinus()
    begin
        ReadRet('q: !!float +-1');
    end;

    [Test]
    procedure Refused_Tag_IntWordUnmeasured()
    begin
        ReadRet('q: !!int hello');
    end;

    [Test]
    procedure Refused_Tag_BoolWordUnmeasured()
    begin
        ReadRet('q: !!bool maybe');
    end;

    [Test]
    procedure Refused_TagDirective_HandleWithoutBang()
    begin
        ReadRet('%TAG e! tag:yaml.org,2002:~---~q: 1');
    end;

    [Test]
    procedure Refused_TagDirective_HandleWord()
    begin
        ReadRet('%TAG foo tag:yaml.org,2002:~---~q: 1');
    end;

    [Test]
    procedure Refused_TagDirective_PrimaryHandleUnused()
    begin
        ReadRet('%TAG ! tag:yaml.org,2002:~---~q: 1');
    end;

    [Test]
    procedure Refused_TagDirective_SecondaryHandleUnused()
    begin
        ReadRet('%TAG !! tag:yaml.org,2002:~---~q: 1');
    end;

    [Test]
    procedure Refused_Escape_HighSurrogateThenOtherEscape()
    begin
        ReadRet('q: "\uD83D\n"');
    end;

    [Test]
    procedure Refused_Escape_HighSurrogateThenLongEscape()
    begin
        ReadRet('q: "\uD83D\U0000DE00"');
    end;

    [Test]
    procedure Refused_Escape_LowSurrogateThenLetter()
    begin
        ReadRet('q: "\uDE00a"');
    end;

    [Test]
    procedure Refused_Escape_OtherMark()
    begin
        ReadRet('q: "﷑"');
    end;

    [Test]
    procedure Refused_Tag_Bool11OtherWord()
    begin
        ReadRet('%YAML 1.1~---~q: !!bool On');
    end;

    [Test]
    procedure Refused_Character_U0015_InTextKey()
    begin
        ReadRet('a{U}b: 1');
    end;

    [Test]
    procedure Refused_Character_U0015_InText()
    begin
        ReadRet('q: a{U}b');
    end;

    [Test]
    procedure Refused_Tag_BeforeComma()
    begin
        ReadRet('q: [!!str, a]');
    end;

    [Test]
    procedure Refused_Alias_ColonAfterName()
    begin
        ReadRet('a: &x 1~*x: 2');
    end;

    [Test]
    procedure Refused_Anchor_ColonAfterName()
    begin
        ReadRet('&x: 1');
    end;

    [Test]
    procedure Refused_Tag_StrQuotedNull()
    begin
        ReadRet('q: !!str "null"');
    end;

    [Test]
    procedure Refused_Tag_IntSignLeadingZero()
    begin
        ReadRet('q: !!int -007');
    end;

    [Test]
    procedure Refused_Tag_IntUnderscoreLeadingZero()
    begin
        ReadRet('q: !!int 0_7');
    end;

    [Test]
    procedure Refused_Tag_IntQuotedBlank()
    begin
        ReadRet('q: !!int " 1"');
    end;

    [Test]
    procedure Refused_Tag_BoolEmptyQuoted()
    begin
        ReadRet('q: !!bool ""');
    end;

    [Test]
    procedure Refused_Tag_FloatEmptyQuoted()
    begin
        ReadRet('q: !!float ""');
    end;

    [Test]
    procedure Refused_Tag_FloatNegativeZeroKey()
    begin
        ReadRet('!!float -0: a');
    end;

    [Test]
    procedure Refused_Tag_TopNullWord()
    begin
        ReadRet('!!int ~');
    end;

    [Test]
    procedure Refused_Tag_NullWordUnderYaml11()
    begin
        ReadRet('%YAML 1.1~---~q: !!int ~');
    end;

    [Test]
    procedure Refused_Tab_AfterValueBeforeListItem()
    begin
        ReadRet('k:~  {T}- a');
    end;

    [Test]
    procedure Assumed_Tab_AfterValueBeforeKey()
    begin
        Check('Yes|{"o":{"q":"a"}}', ReadRet('o:~  {T}q: a'));
    end;

    [Test]
    procedure Refused_KeyLength_StartsWithDigit()
    var
        B: TextBuilder;
        i: Integer;
    begin
        for i := 1 to 1100 do
            B.Append('1');
        ReadRet(B.ToText() + ': 1');
    end;

    [Test]
    procedure Assumed_GoodRead_IntoMembers()
    var
        Out: Text;
    begin
        Clear(JO);
        JO.ReadFrom('{"a":1}');
        Ok := JO.ReadFromYaml('q: 1');
        JO.WriteTo(Out);
        Check('Yes|{"q":"1"}', Format(Ok) + '|' + Out);
    end;

    [Test]
    procedure Assumed_Stream_FromPosition()
    var
        Blob: Record "YN Yaml Review Blob";
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Out: Text;
    begin
        Blob.DeleteAll();
        Blob.Id := 1;
        Blob.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Doc('skip: me~q: abc'));
        Blob.Insert();
        Blob.CalcFields(Data);
        Blob.Data.CreateInStream(InS, TextEncoding::UTF8);
        InS.ReadText(Line);
        Clear(JO);
        Ok := JO.ReadFromYaml(InS);
        JO.WriteTo(Out);
        Check('Yes|{"q":"abc"}', Format(Ok) + '|' + Out);
    end;

    [Test]
    procedure Assumed_Stream_U0015()
    var
        Blob: Record "YN Yaml Review Blob";
        OutS: OutStream;
        InS: InStream;
        T: JsonToken;
    begin
        Blob.DeleteAll();
        Blob.Id := 1;
        Blob.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Doc('q: a{U}b'));
        Blob.Insert();
        Blob.CalcFields(Data);
        Blob.Data.CreateInStream(InS, TextEncoding::UTF8);
        Clear(JO);
        Ok := JO.ReadFromYaml(InS);
        Check('Yes', Format(Ok));
        JO.Get('q', T);
        Check('a' + Chr(21) + 'b', T.AsValue().AsText());
    end;
}
