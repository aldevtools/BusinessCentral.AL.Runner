codeunit 50220 "GH ClearAll Tests"
{
    Subtype = Test;

    // A TableNo owner whose setter took the caller's register runs ClearAll in its OnRun. Want: main's answer (the caller's
    // register kept its state).
    [Test]
    procedure TableNoOwnerSharedRegisterThroughSetter()
    var
        Rec: Record "GH Row" temporary;
        A: Codeunit "GH Activity Post";
    begin
        A.Run(Rec);
        Check(A.RegisterState(), 'amount=1 rows=0');
    end;

    // The holder's nested codeunit wrote its temporary table and emptied it again: no row now, nothing outside shares it.
    [Test]
    procedure NestedTableWrittenThenEmptied()
    var
        H: Codeunit "GH Holder";
    begin
        H.WriteG(3);
        H.EmptyG();
        H.ClearAllGlobals();
        Check(H.StateG(), 'rows=0 d=0 n=0');
    end;

    // The same with the row still there. Want: main's answer (a fresh instance).
    [Test]
    procedure NestedTableWithRowsNow()
    var
        H: Codeunit "GH Holder";
    begin
        H.WriteG(3);
        H.ClearAllGlobals();
        Check(H.StateG(), 'rows=0 d=0 n=0');
    end;

    // A record outside shares the nested codeunit's empty temporary table (Copy(x, true)). Want: main's answer.
    [Test]
    procedure NestedTableSharedByCopy()
    var
        H: Codeunit "GH Holder";
        R: Record "GH Row" temporary;
    begin
        H.ShareG(R);
        H.ClearAllGlobals();
        R.No := 1;
        R.Insert();
        Check(H.StateG(), 'rows=0 d=0 n=0');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
