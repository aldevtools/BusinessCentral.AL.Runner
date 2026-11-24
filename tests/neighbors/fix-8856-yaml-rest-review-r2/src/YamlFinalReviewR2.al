// Neighbors of al-runner issue 8856, review round 2: a quoted empty tagged scalar as the document and a low surrogate
// escape before a `\u` escape that is not a high surrogate. No cell of corpus PR 1549 measures them; each refuses.
codeunit 61370 "YN Yaml Review R2 Tests"
{
    Subtype = Test;

    var
        JO: JsonObject;
        Ok: Boolean;

    local procedure ReadRet(Src: Text): Text
    var
        Out: Text;
    begin
        Clear(JO);
        Ok := JO.ReadFromYaml(Src);
        JO.WriteTo(Out);
        exit(Format(Ok) + '|' + Out);
    end;

    [Test]
    procedure Refused_Tag_TopStrEmptyQuoted()
    begin
        ReadRet('!!str ""');
    end;

    [Test]
    procedure Refused_Tag_TopStrEmptySingleQuoted()
    begin
        ReadRet('!!str ''''');
    end;

    [Test]
    procedure Refused_Tag_TopIntEmptyQuoted()
    begin
        ReadRet('!!int ""');
    end;

    [Test]
    procedure Refused_Escape_LowSurrogateThenOtherU()
    begin
        ReadRet('q: "\uDE00A"');
    end;

    [Test]
    procedure Refused_Escape_LowSurrogateThenLowU()
    begin
        ReadRet('q: "\uDE00\uDE00"');
    end;

    [Test]
    procedure Assumed_Escape_LowSurrogateThenHighU()
    begin
        // The measured cell next to the two rows above (corpus PR 1549 EscapeSurrogates1_8): the YAML error.
        if ReadRet('q: "\uDE00\uD83D"') <> 'No|{}' then
            Error('Expected No|{}, got <%1>', ReadRet('q: "\uDE00\uD83D"'));
    end;
}
