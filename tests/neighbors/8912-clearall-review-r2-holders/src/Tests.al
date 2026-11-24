codeunit 50670 "RH ClearAll Tests"
{
    Subtype = Test;

    // Not measured: ClearAll in a codeunit whose interface global shares the instance of a store of the runtime's cells. The
    // corpus measures it for another store only (corpus PR 1564 ClearSharedCodeunitStoreShapes.al
    // Whse_InterfaceHolder_HolderClearAll :2001: BC resets the shared instance in place). Main clears the interface and
    // leaves the instance as it was. Refused.
    [Test]
    procedure InterfaceHolderClearAll()
    var
        O: Codeunit "RH Owner";
        H: Codeunit "RH Face Holder";
    begin
        O.SetN(7);
        O.ShareFace(H);
        H.ClearAllGlobals();
        Check(Format(O.GetN()), '0');
    end;

    // The sharer side: ClearAll in the codeunit whose global temporary record shares the owner's table. The owner keeps its
    // state, the holder's record still shares the table and its rows, the holder's Integer is 0 (corpus PR 1564
    // ClearSharedCodeunitCopyHolder.al ArrEl1_HolderGlobal_Written_HolderClears :484).
    [Test]
    procedure RecordHolderClearAll()
    var
        O: Codeunit "RH Arr Owner";
        H: Codeunit "RH Rec Holder";
    begin
        O.SetCnt(5);
        O.WriteA(1);
        H.Take(O);
        H.ClearAllGlobals();
        Check(O.State(), 'a1=1 cnt=5');
        Check(H.State(), 'r=1 hcnt=0');
        H.Write(10);
        Check(O.State(), 'a1=2 cnt=5');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
