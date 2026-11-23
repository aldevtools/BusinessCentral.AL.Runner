// Neighbors of al-runner issue 8856: the ReadFromYaml shapes of corpus PR 1549 (tags, directives, anchors, key
// length, tabs, escapes, line separators, characters, depth, failed reads, streams), the shapes next to them that
// still refuse, and the readers of OutStream.WriteText's `§` on an MS-DOS stream.
// `~` stands for a line feed, {T} for a tab, {L} for U+2028, {S} for U+00A7, {A} for U+00E4.
codeunit 61351 "YN Yaml Final Shapes Tests"
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

    local procedure Rep(S: Text; N: Integer): Text
    var
        B: TextBuilder;
        i: Integer;
    begin
        for i := 1 to N do
            B.Append(S);
        exit(B.ToText());
    end;

    local procedure Doc(Src: Text): Text
    begin
        Src := Src.Replace('~', Chr(10)).Replace('{T}', Chr(9)).Replace('{L}', Chr(8232));
        exit(Src.Replace('{S}', Chr(167)).Replace('{A}', Chr(228)));
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

    local procedure ValueOf(Name: Text): Text
    var
        T: JsonToken;
    begin
        JO.Get(Name, T);
        exit(T.AsValue().AsText());
    end;

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    local procedure Laughs(Levels: Integer): Text
    var
        B: TextBuilder;
        i: Integer;
        Prev: Text;
    begin
        B.Append('l0: &l0 [x, x, x, x, x, x, x, x, x]~');
        for i := 1 to Levels - 1 do begin
            Prev := '*l' + Format(i - 1);
            B.Append('l' + Format(i) + ': &l' + Format(i) + ' [' + Rep(Prev + ', ', 8) + Prev + ']~');
        end;
        exit(B.ToText());
    end;

    local procedure StreamRead(Src: Text; OutEnc: TextEncoding; InEnc: TextEncoding): Text
    var
        Blob: Record "YN Yaml Final Blob";
        OutS: OutStream;
        InS: InStream;
        Out: Text;
    begin
        Blob.DeleteAll();
        Blob.Id := 1;
        Blob.Data.CreateOutStream(OutS, OutEnc);
        OutS.WriteText(Doc(Src));
        Blob.Insert();
        Blob.CalcFields(Data);
        Blob.Data.CreateInStream(InS, InEnc);
        Clear(JO);
        Ok := JO.ReadFromYaml(InS);
        JO.WriteTo(Out);
        exit(Format(Ok) + '|' + Out);
    end;

    // Main's pinned read stays clean.
    [Test]
    procedure Kept_PinnedText()
    begin
        Check('{"a":"1"}', ReadStmt('a: 1'));
    end;

    // Tags and directives.
    [Test]
    procedure Assumed_Tag_IntLeadingZero()
    begin
        Check('Yes|{"q":7}', ReadRet('q: !!int 007'));
    end;

    [Test]
    procedure Assumed_Tag_KeyBool()
    begin
        Check('Yes|{"True":"a"}', ReadRet('!!bool yes: a'));
    end;

    [Test]
    procedure Assumed_Tag_LocalOnList()
    begin
        Check('No|{}', ReadRet('q: !custom [a]'));
    end;

    [Test]
    procedure Refused_Tag_TimestampOtherDate()
    begin
        ReadRet('q: !!timestamp 2001-12-15');
    end;

    [Test]
    procedure Refused_Tag_NonUriCharacter()
    begin
        ReadRet('a: !{A} x');
    end;

    [Test]
    procedure Assumed_Directive_Yaml10()
    begin
        Check('Yes|{"q":"1"}', ReadRet('%YAML 1.0~---~q: 1'));
    end;

    [Test]
    procedure Refused_Directive_Yaml14()
    begin
        ReadRet('%YAML 1.4~---~q: 1');
    end;

    [Test]
    procedure Assumed_TagDirective_CorePrefixUsed()
    begin
        Check('Yes|{"q":1}', ReadRet('%TAG !e! tag:yaml.org,2002:~---~q: !e!int 1'));
    end;

    [Test]
    procedure Assumed_TagDirective_OtherPrefixUsed()
    begin
        Check('No|{}', ReadRet('%TAG !e! tag:example.com,2000:~---~q: !e!x 1'));
    end;

    [Test]
    procedure Refused_TagDirective_OtherPrefixUnused()
    begin
        ReadRet('%TAG !e! tag:example.com,2000:~---~q: a');
    end;

    // Anchors and aliases.
    [Test]
    procedure Assumed_AnchorName_Dot()
    begin
        Check('Yes|{"a":"1","b":"1"}', ReadRet('a: &a.b 1~b: *a.b'));
    end;

    [Test]
    procedure Refused_AnchorName_UnmeasuredLetter()
    begin
        ReadRet('a: &' + Chr(252) + ' 1');
    end;

    [Test]
    procedure Kept_Expansion_FiveLevels()
    var
        Out: Text;
    begin
        Clear(JO);
        Check('Yes', Format(JO.ReadFromYaml(Laughs(5).Replace('~', Chr(10)))));
        JO.WriteTo(Out);
        Check('282348', Format(StrLen(Out)));
    end;

    [Test]
    procedure Refused_Expansion_SixLevels()
    begin
        ReadRet(Laughs(6));
    end;

    // Key length.
    [Test]
    procedure Assumed_KeyLength_Plain1025()
    begin
        Check('No|{}', ReadRet(Rep('k', 1025) + ': 1'));
    end;

    [Test]
    procedure Assumed_KeyLength_FlowMapping1025()
    begin
        Check('Yes', CopyStr(ReadRet('{' + Rep('k', 1025) + ': 1}'), 1, 3));
    end;

    [Test]
    procedure Refused_KeyLength_FlowCollectionKey()
    begin
        ReadRet('[' + Rep('a, ', 500) + 'b]: 1');
    end;

    [Test]
    procedure Refused_KeyLength_InFlowList()
    begin
        ReadRet('q: [' + Rep('k', 1025) + ': 1]');
    end;

    // Tabs.
    [Test]
    procedure Assumed_Tab_BeforeFirstKey()
    begin
        Check('Yes|{"q":"a"}', ReadRet('  {T}q: a'));
    end;

    [Test]
    procedure Assumed_Tab_InFlowIndentation()
    begin
        Check('Yes|{"q":["a","b"]}', ReadRet('q: [a,~  {T}b]'));
    end;

    [Test]
    procedure Assumed_Tab_LaterBlockIndentation()
    begin
        Check('No|{}', ReadRet('o:~  q: a~  {T}r: b'));
    end;

    // Escapes, line separators, characters.
    [Test]
    procedure Assumed_Escape_LoneSurrogate()
    begin
        Check('No|{}', ReadRet('q: "\uD800"'));
    end;

    [Test]
    procedure Refused_Escape_CallStackMark()
    begin
        ReadRet('q: "\uFDD0"');
    end;

    [Test]
    procedure Assumed_LineSeparator_InSingleQuotes()
    begin
        ReadRet('q: ''a{L}b''');
        Check('Yes', Format(Ok));
        Check(Doc('a{L}b'), ValueOf('q'));
    end;

    [Test]
    procedure Refused_LineSeparator_AfterBlankInFlow()
    begin
        ReadRet('q: [a {L}b]');
    end;

    [Test]
    procedure Assumed_Character_Auml()
    begin
        ReadRet('q: x{A}y');
        Check('Yes', Format(Ok));
        Check(Doc('x{A}y'), ValueOf('q'));
    end;

    // Depth.
    [Test]
    procedure Assumed_Depth_65Containers()
    begin
        Check('No|{}', ReadRet('q: ' + Rep('[', 64) + 'x' + Rep(']', 64)));
        Check('InvalidJsonData', GetLastErrorCode());
    end;

    [Test]
    procedure Refused_Depth_AsStatement()
    begin
        ReadStmt('q: ' + Rep('[', 64) + 'x' + Rep(']', 64));
    end;

    [Test]
    procedure Refused_Depth_402Containers()
    begin
        ReadRet('q: ' + Rep('[', 401) + 'x' + Rep(']', 401));
    end;

    // A failed read into an object with members.
    [Test]
    procedure Assumed_FailedRead_KeepsMembers()
    var
        Out: Text;
    begin
        Clear(JO);
        JO.ReadFrom('{"a":1}');
        Ok := JO.ReadFromYaml('q: [a');
        JO.WriteTo(Out);
        Check('No|{"a":1}', Format(Ok) + '|' + Out);
    end;

    // Streams.
    [Test]
    procedure Assumed_Stream_WindowsBytesReadAsUtf8()
    begin
        StreamRead('q: x{A}y', TextEncoding::Windows, TextEncoding::UTF8);
        Check('Yes', Format(Ok));
        Check('x' + Chr(65533) + 'y', ValueOf('q'));
    end;

    [Test]
    procedure Assumed_Stream_Utf16Written()
    begin
        Check('No|{}', StreamRead('q: abc', TextEncoding::UTF16, TextEncoding::UTF8));
    end;

    [Test]
    procedure Assumed_WriteText_SectionMsDos_ReadFromYaml()
    begin
        StreamRead('q: a{S}b', TextEncoding::MSDos, TextEncoding::UTF8);
        Check('Yes', Format(Ok));
        Check('a' + Chr(21) + 'b', ValueOf('q'));
    end;

    // Readers of the bytes OutStream.WriteText gives `§` on an MS-DOS stream.
    [Test]
    procedure Assumed_WriteText_SectionMsDos_ReadText()
    var
        Blob: Record "YN Yaml Final Blob";
        OutS: OutStream;
        InS: InStream;
        T: Text;
    begin
        Blob.Data.CreateOutStream(OutS, TextEncoding::MSDos);
        OutS.WriteText(Doc('a{S}b'));
        Blob.Data.CreateInStream(InS, TextEncoding::MSDos);
        InS.ReadText(T);
        Check('3', Format(StrLen(T)));
    end;

    [Test]
    procedure Kept_Write_SectionMsDos()
    var
        Blob: Record "YN Yaml Final Blob";
        OutS: OutStream;
    begin
        Blob.Data.CreateOutStream(OutS, TextEncoding::MSDos);
        OutS.Write(Doc('a{S}b'));
    end;
}
