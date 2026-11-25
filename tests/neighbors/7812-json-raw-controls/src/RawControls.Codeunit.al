codeunit 50300 "Raw Control Char Tests"
{
    Subtype = Test;

    // The reporter's repro (al-runner issue 7812, comment of the re-check on the 2026-11-13 edge), unchanged.
    [Test]
    procedure RawNakInString()
    var
        O: JsonObject;
        Ctl: Text[1];
    begin
        Ctl[1] := 21; // U+0015
        if not O.ReadFrom('{"question": "a' + Ctl + 'b"}') then
            Error('ReadFrom returned false');
    end;

    [Test]
    procedure RawUnitSeparatorInString()
    var
        O: JsonObject;
        Ctl: Text[1];
    begin
        Ctl[1] := 31; // U+001F
        if not O.ReadFrom('{"question": "a' + Ctl + 'b"}') then
            Error('ReadFrom returned false');
    end;

    local procedure Ch(Code: Integer): Text
    var
        C: Text[1];
    begin
        C[1] := Code;
        exit(C);
    end;

    local procedure Codes(T: Text): Text
    var
        I: Integer;
        Code: Integer;
        R: Text;
    begin
        for I := 1 to StrLen(T) do begin
            if I > 1 then
                R += ',';
            Code := T[I];
            R += Format(Code);
        end;
        exit(R);
    end;

    local procedure ObjectValueAt(Src: Text; "Key": Text): Text
    var
        O: JsonObject;
        T: JsonToken;
    begin
        if not O.ReadFrom(Src) then
            exit('false');
        O.Get("Key", T);
        exit(Codes(T.AsValue().AsText()));
    end;

    // Every code 1..31 in a double-quoted value of JsonObject.ReadFrom(Text): the reporter's measurement on BC 28.4.
    [Test]
    procedure ObjectValueEveryCode()
    var
        C: Integer;
    begin
        for C := 1 to 31 do
            if ObjectValueAt('{"a":"x' + Ch(C) + 'y"}', 'a') <> '120,' + Format(C) + ',121' then
                Error('code %1', C);
    end;

    // The same codes through the other readers of a Text: JsonToken, JsonArray, JsonValue.
    [Test]
    procedure TokenValueNak()
    var
        T: JsonToken;
        V: JsonToken;
    begin
        if not T.ReadFrom('{"a":"x' + Ch(21) + 'y"}') then
            Error('false');
        T.AsObject().Get('a', V);
        if Codes(V.AsValue().AsText()) <> '120,21,121' then
            Error('codes');
    end;

    [Test]
    procedure ArrayValueNak()
    var
        A: JsonArray;
        T: JsonToken;
    begin
        if not A.ReadFrom('["x' + Ch(21) + 'y"]') then
            Error('false');
        A.Get(0, T);
        if Codes(T.AsValue().AsText()) <> '120,21,121' then
            Error('codes');
    end;

    [Test]
    procedure ValueNak()
    var
        V: JsonValue;
    begin
        if not V.ReadFrom('"x' + Ch(21) + 'y"') then
            Error('false');
        if Codes(V.AsText()) <> '120,21,121' then
            Error('codes');
    end;

    // A raw control character other than TAB in a key, and in a single-quoted value.
    [Test]
    procedure ObjectKeyNak()
    var
        O: JsonObject;
    begin
        if not O.ReadFrom('{"a' + Ch(21) + 'b":1}') then
            Error('false');
        if not O.Contains('a' + Ch(21) + 'b') then
            Error('key');
    end;

    [Test]
    procedure ObjectSingleQuotedNak()
    begin
        if ObjectValueAt('{''a'':''x' + Ch(21) + 'y''}', 'a') <> '120,21,121' then
            Error('codes');
    end;

    // ReadFrom(InStream) with a raw control character in a value.
    [Test]
    procedure ObjectInStreamNak()
    var
        Buf: Record "Raw Control Buffer" temporary;
        O: JsonObject;
        OutS: OutStream;
        InS: InStream;
    begin
        Buf.Data.CreateOutStream(OutS);
        OutS.WriteText('{"a":"x' + Ch(21) + 'y"}');
        Buf.Data.CreateInStream(InS);
        if not O.ReadFrom(InS) then
            Error('false');
    end;

    // TAB in a value through an InStream.
    [Test]
    procedure ObjectInStreamTab()
    var
        Buf: Record "Raw Control Buffer" temporary;
        O: JsonObject;
        OutS: OutStream;
        InS: InStream;
    begin
        Buf.Data.CreateOutStream(OutS);
        OutS.WriteText('{"a":"x' + Ch(9) + 'y"}');
        Buf.Data.CreateInStream(InS);
        if not O.ReadFrom(InS) then
            Error('false');
    end;
}
