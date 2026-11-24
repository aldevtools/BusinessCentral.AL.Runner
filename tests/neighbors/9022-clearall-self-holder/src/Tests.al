codeunit 50950 "SN Tests"
{
    Subtype = Test;

    // The repro with a Decimal: nothing else holds the instance, so a fresh instance (main's answer).
    [Test]
    procedure DecimalSelfHolderUnshared()
    var
        H: Codeunit "SN Holder";
    begin
        if H.DecUnshared() <> 0 then
            Error('Count after ClearAll: %1', H.DecUnshared());
    end;

    // The repro with a Decimal, shared: no measured cell, refused.
    [Test]
    procedure DecimalSelfHolderShared()
    var
        H: Codeunit "SN Holder";
    begin
        H.DecShared();
    end;

    // Clear of the repro's codeunit reads no store shape.
    [Test]
    procedure ClearOfSelfHolder()
    var
        H: Codeunit "SN Self Owner";
    begin
        if H.ClearSelf() <> 0 then
            Error('Count after Clear: %1', H.ClearSelf());
    end;

    // The measured wrapper still reads as the wrapper: its unwritten inner store shared refuses.
    [Test]
    procedure WrapperInnerShared()
    var
        H: Codeunit "SN Holder";
        Y: Codeunit "SN Store";
    begin
        H.ShareWrapperInner(Y);
        H.ClearAllGlobals();
        Y.Write(5);
        Check(H.StateWrapper(), 'rows=1 int1=5 wcnt=0');
    end;

    // A wrapper of a wrapper is no store: main's answer, the inner store keeps its alias.
    [Test]
    procedure OuterInnerShared()
    var
        H: Codeunit "SN Holder";
        Y: Codeunit "SN Store";
    begin
        H.ShareOuterInner(Y);
        H.ClearAllGlobals();
        Y.Write(5);
        Check(H.StateOuter(), 'rows=0 int1=0 wcnt=0');
    end;

    // A codeunit of an Integer store then an Integer is no store: main's answer.
    [Test]
    procedure IntHolderInnerShared()
    var
        H: Codeunit "SN Holder";
        Y: Codeunit "SN Int Store";
    begin
        H.ShareIntInner(Y);
        H.ClearAllGlobals();
        Y.SetN(5);
        if H.IntN() <> 0 then
            Error('N after ClearAll: %1', H.IntN());
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('Got %1, want %2', Got, Want);
    end;
}
