codeunit 50309 "Raw Control Help"
{
    procedure Ch(Code: Integer): Text
    var
        C: Text[1];
    begin
        C[1] := Code;
        exit(C);
    end;

    procedure Stream(Src: Text; var Buf: Record "Raw Control Buffer" temporary; var InS: InStream)
    var
        OutS: OutStream;
    begin
        Buf.Data.CreateOutStream(OutS);
        OutS.WriteText(Src);
        Buf.Data.CreateInStream(InS);
    end;
}

// Measured places that the first codeunit does not reach (corpus pull request 1582).
codeunit 50301 "Raw Control Place Tests"
{
    Subtype = Test;

    var
        H: Codeunit "Raw Control Help";

    [Test]
    procedure TokenKeyNak()
    var
        T: JsonToken;
    begin
        if not T.ReadFrom('{"k' + H.Ch(21) + 'z":"v"}') then
            Error('false');
    end;

    [Test]
    procedure TokenSingleQuotedNak()
    var
        T: JsonToken;
    begin
        if not T.ReadFrom('{''a'':''x' + H.Ch(21) + 'y''}') then
            Error('false');
    end;

    [Test]
    procedure ArraySingleQuotedNak()
    var
        A: JsonArray;
    begin
        if not A.ReadFrom('[''x' + H.Ch(21) + 'y'']') then
            Error('false');
    end;

    [Test]
    procedure ValueSingleQuotedNak()
    var
        V: JsonValue;
    begin
        if not V.ReadFrom('''x' + H.Ch(21) + 'y''') then
            Error('false');
    end;

    [Test]
    procedure TokenInStreamNak()
    var
        Buf: Record "Raw Control Buffer" temporary;
        T: JsonToken;
        InS: InStream;
    begin
        H.Stream('{"a":"x' + H.Ch(21) + 'y"}', Buf, InS);
        if not T.ReadFrom(InS) then
            Error('false');
    end;

    [Test]
    procedure ArrayInStreamNak()
    var
        Buf: Record "Raw Control Buffer" temporary;
        A: JsonArray;
        InS: InStream;
    begin
        H.Stream('["x' + H.Ch(21) + 'y"]', Buf, InS);
        if not A.ReadFrom(InS) then
            Error('false');
    end;

    [Test]
    procedure ValueInStreamNak()
    var
        Buf: Record "Raw Control Buffer" temporary;
        V: JsonValue;
        InS: InStream;
    begin
        H.Stream('"x' + H.Ch(21) + 'y"', Buf, InS);
        if not V.ReadFrom(InS) then
            Error('false');
    end;

}

// Places the probe did not measure: refused. One codeunit each, so one refusal does not stop another test.
codeunit 50302 "Raw Control SqKey Tests"
{
    Subtype = Test;

    [Test]
    procedure ObjectSingleQuotedKeyNak()
    var
        H: Codeunit "Raw Control Help";
        O: JsonObject;
    begin
        if not O.ReadFrom('{''k' + H.Ch(21) + 'z'':''v''}') then
            Error('false');
    end;
}

codeunit 50303 "Raw Control StreamKey Tests"
{
    Subtype = Test;

    [Test]
    procedure ObjectInStreamKeyNak()
    var
        H: Codeunit "Raw Control Help";
        Buf: Record "Raw Control Buffer" temporary;
        O: JsonObject;
        InS: InStream;
    begin
        H.Stream('{"k' + H.Ch(21) + 'z":"v"}', Buf, InS);
        if not O.ReadFrom(InS) then
            Error('false');
    end;
}

codeunit 50304 "Raw Control StreamSq Tests"
{
    Subtype = Test;

    [Test]
    procedure ObjectInStreamSingleQuotedNak()
    var
        H: Codeunit "Raw Control Help";
        Buf: Record "Raw Control Buffer" temporary;
        O: JsonObject;
        InS: InStream;
    begin
        H.Stream('{''a'':''x' + H.Ch(21) + 'y''}', Buf, InS);
        if not O.ReadFrom(InS) then
            Error('false');
    end;
}

codeunit 50305 "Raw Control ObjElement Tests"
{
    Subtype = Test;

    // An array element inside an object that JsonObject reads.
    [Test]
    procedure ObjectArrayElementNak()
    var
        H: Codeunit "Raw Control Help";
        O: JsonObject;
    begin
        if not O.ReadFrom('{"a":["x' + H.Ch(21) + 'y"]}') then
            Error('false');
    end;
}

codeunit 50306 "Raw Control ArrKey Tests"
{
    Subtype = Test;

    // A key inside an array that JsonArray reads.
    [Test]
    procedure ArrayObjectKeyNak()
    var
        H: Codeunit "Raw Control Help";
        A: JsonArray;
    begin
        if not A.ReadFrom('[{"k' + H.Ch(21) + 'z":1}]') then
            Error('false');
    end;
}

codeunit 50307 "Raw Control TokRoot Tests"
{
    Subtype = Test;

    // A string that is the whole text, read by JsonToken.
    [Test]
    procedure TokenWholeStringNak()
    var
        H: Codeunit "Raw Control Help";
        T: JsonToken;
    begin
        if not T.ReadFrom('"x' + H.Ch(21) + 'y"') then
            Error('false');
    end;
}

codeunit 50308 "Raw Control Evaluate Tests"
{
    Subtype = Test;

    // Evaluate of a JsonArray: another reader.
    [Test]
    procedure EvaluateArrayNak()
    var
        H: Codeunit "Raw Control Help";
        A: JsonArray;
    begin
        if not Evaluate(A, '["x' + H.Ch(21) + 'y"]') then
            Error('false');
    end;
}

codeunit 50310 "Raw Control Nested Tests"
{
    Subtype = Test;

    // A member value of a nested object: one level deeper than the probe measured.
    [Test]
    procedure ObjectNestedMemberNak()
    var
        H: Codeunit "Raw Control Help";
        O: JsonObject;
    begin
        if not O.ReadFrom('{"o":{"a":"x' + H.Ch(21) + 'y"}}') then
            Error('false');
    end;
}
