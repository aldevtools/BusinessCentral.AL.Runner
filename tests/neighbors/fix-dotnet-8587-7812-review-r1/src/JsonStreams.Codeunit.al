// JsonValue.ReadFrom(InStream) of a stream that holds one whole string, result used (json/json/P101Streams.al:114-116).
codeunit 50321 "Review R1 Json Pinned Tests"
{
    Subtype = Test;

    [Test]
    procedure ValueInStreamPlain()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        V: JsonValue;
        InS: InStream;
    begin
        H.Stream('"hello"', Buf, InS);
        if not V.ReadFrom(InS) then
            Error('false');
        if V.AsText() <> 'hello' then
            Error('read %1', V.AsText());
    end;
}

// Shapes no pin covers: refused. One codeunit each, so one refusal does not stop another test.
codeunit 50322 "Review R1 Value Unused Tests"
{
    Subtype = Test;

    // The statement form of ValueInStreamNak.
    [Test]
    procedure ValueInStreamNakUnused()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        V: JsonValue;
        InS: InStream;
    begin
        H.Stream('"x' + H.Ch(21) + 'y"', Buf, InS);
        V.ReadFrom(InS);
    end;
}

codeunit 50323 "Review R1 Array String Tests"
{
    Subtype = Test;

    [Test]
    procedure ArrayInStreamString()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        A: JsonArray;
        InS: InStream;
    begin
        H.Stream('"hello"', Buf, InS);
        A.ReadFrom(InS);
    end;
}

codeunit 50324 "Review R1 Token Unused Tests"
{
    Subtype = Test;

    [Test]
    procedure TokenInStreamStringUnused()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        T: JsonToken;
        InS: InStream;
    begin
        H.Stream('"hello"', Buf, InS);
        T.ReadFrom(InS);
    end;
}

codeunit 50325 "Review R1 Token Rest Tests"
{
    Subtype = Test;

    // A string followed by more text, result used: read (json/json/P101Streams.al:162-167). Main reads it too.
    [Test]
    procedure TokenInStreamStringThenRest()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        T: JsonToken;
        InS: InStream;
    begin
        H.Stream('"a":2}', Buf, InS);
        if not T.ReadFrom(InS) then
            Error('false');
    end;
}

codeunit 50326 "Review R1 Code0 Tests"
{
    Subtype = Test;

    // Code 0 in a member value from an InStream: the stream reader refuses a NUL byte, on main and on head.
    [Test]
    procedure ObjectInStreamCode0()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        O: JsonObject;
        InS: InStream;
    begin
        H.Stream('{"a":"x' + H.Ch(0) + 'y"}', Buf, InS);
        if not O.ReadFrom(InS) then
            Error('false');
    end;
}

codeunit 50330 "Review R1 Token Alone Tests"
{
    Subtype = Test;

    // A JsonToken, result used, of a stream that holds one string and nothing else: no pin, refused.
    [Test]
    procedure TokenInStreamStringAlone()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        T: JsonToken;
        InS: InStream;
    begin
        H.Stream('"hello"', Buf, InS);
        if not T.ReadFrom(InS) then
            Error('false');
    end;
}

codeunit 50331 "Review R1 Value Rest Tests"
{
    Subtype = Test;

    // A JsonValue, result used, of a stream that holds a string followed by more text: no pin, refused.
    [Test]
    procedure ValueInStreamStringThenRest()
    var
        H: Codeunit "Review R1 Help";
        Buf: Record "Review R1 Buffer" temporary;
        V: JsonValue;
        InS: InStream;
    begin
        H.Stream('"a":2}', Buf, InS);
        if not V.ReadFrom(InS) then
            Error('false');
    end;
}

codeunit 50332 "Review R1 Nested Nak Tests"
{
    Subtype = Test;

    // The Sales Lines Suggestions dataset shape: U+0015 in an element of an array of an object in an array of a member
    // object. Nested deeper than the probe measured: refused on main and on head (corpus PR 1582 now asserts it).
    [Test]
    procedure ObjectNestedElementNak()
    var
        H: Codeunit "Review R1 Help";
        O: JsonObject;
    begin
        if not O.ReadFrom('{"q":"x","e":{"s":[{"f":["x' + H.Ch(21) + 'y"]}]}}') then
            Error('false');
    end;
}
