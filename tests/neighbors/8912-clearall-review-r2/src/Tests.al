codeunit 50520 "RR ClearAll Tests"
{
    Subtype = Test;

    // The register a setter hands to a TableNo owner whose OnRun runs ClearAll: one instance, reset in place, rows kept
    // (corpus PR 1564 ClearSharedCodeunitSetterOwner.al TableNoRun_VarSetter_Rows :318).
    [Test]
    procedure RegisterSharedThroughSetter()
    var
        C: Codeunit "RR Caller";
    begin
        C.Prepare(2);
        C.RunOwner();
        Check(C.CallerState(), 'rows=2 dec=0 flag=No code= filter= no=0 nested=0');
    end;

    // The register's table shared with a record of the test: it stays shared (:397).
    [Test]
    procedure RegisterCopySharedTable()
    var
        C: Codeunit "RR Caller";
        R: Record "RR Row" temporary;
    begin
        C.Prepare(2);
        C.ShareRows(R);
        C.RunOwner();
        Check(Format(R.Count()), '2');
        R.No := 13;
        R.Insert();
        Check(C.CallerState(), 'rows=3 dec=0 flag=No code= filter= no=0 nested=0');
    end;

    // Not read: which side of the sharing runs ClearAll. The caller, whose global made the instance, runs it here; corpus PR
    // 1564 ClearSharedCodeunitStoreShapes.al measures both sides (:1508 owner, :1682 to :1856 caller). Runs flagged.
    [Test]
    procedure RegisterCallerSideClearAll()
    var
        C: Codeunit "RR Caller";
        O: Codeunit "RR TableNo Owner";
    begin
        C.MarkAndWrite(2);
        C.HandTo(O);
        C.ClearAllHere();
        Check(O.StateReg(), 'rows=2 dec=0 flag=No code= filter= no=0 nested=0');
    end;

    // Not measured: ClearAll in a codeunit whose global holds an owner that holds the shared register (the walk reaches the
    // register's rows through the owner). Refused.
    [Test]
    procedure RegisterOwnerHeldByClearingCodeunit()
    var
        C: Codeunit "RR Caller";
    begin
        C.Prepare(2);
        C.ClearAllHere();
        Check(C.OwnerState(), 'rows=2 dec=1.5 flag=Yes code=MARK filter=No: 7 no=9 nested=4');
    end;

    // Not read: the kind of the other holder. A local variable shares the owner's register here; corpus PR 1564
    // ClearSharedCodeunitStoreShapes.al measures another codeunit's global and a local alias (:1963). Runs flagged.
    [Test]
    procedure RegisterSharedByLocalAlias()
    var
        O: Codeunit "RR TableNo Owner";
        Y: Codeunit "RR Register";
    begin
        O.ShareReg(Y);
        Y.Mark();
        Y.WriteRow(1);
        O.ClearAllGlobals();
        Check(Y.State(), 'rows=1 dec=0 flag=No code= filter= no=0 nested=0');
    end;

    // Not measured: the register under a SingleInstance owner. Refused.
    [Test]
    procedure RegisterSingleInstanceOwner()
    var
        C: Codeunit "RR Caller";
        S: Codeunit "RR SI Owner";
    begin
        C.Prepare(1);
        C.HandToSi(S);
        S.ClearAllGlobals();
        Check(C.CallerState(), 'rows=1 dec=1.5 flag=Yes code=MARK filter=No: 7 no=9 nested=4');
    end;

    // Not measured: the register's record read a row before the ClearAll (the probe sets a filter and a field only).
    [Test]
    procedure RegisterRecordReadRow()
    var
        C: Codeunit "RR Caller";
        Stored: Record "RR Row";
    begin
        Stored.No := 7;
        Stored.Insert();
        C.Prepare(1);
        C.ReadRec();
        C.RunOwner();
        Check(C.OwnerState(), 'rows=0 dec=0 flag=No code= filter= no=0 nested=0');
    end;

    // Not measured: the owner's own register, nothing sharing it, whose table a record outside shares.
    [Test]
    procedure RegisterUnsharedCopySharedTable()
    var
        O: Codeunit "RR TableNo Owner";
        R: Record "RR Row" temporary;
    begin
        O.WriteReg(1);
        O.ShareRegRows(R);
        O.ClearAllGlobals();
        Check(O.StateReg(), 'rows=0 dec=0 flag=No code= filter= no=0 nested=0');
    end;

    // Not measured: the register's nested codeunit instance shared by another variable.
    [Test]
    procedure RegisterNestedShared()
    var
        C: Codeunit "RR Caller";
        N: Codeunit "RR Nested";
    begin
        C.Prepare(1);
        C.ShareNested(N);
        C.RunOwner();
        Check(Format(N.Get()), '4');
    end;

    // Not read: which element a record copied. Element 2 here (the elements read one table); corpus PR 1564
    // ClearSharedCodeunitCopyHolder.al measures elements 1 and 2 (:278). Runs flagged.
    [Test]
    procedure ArrayElement2SharedByCopy()
    var
        O: Codeunit "RR Arr Owner";
        R: Record "RR Row" temporary;
    begin
        O.ShareA2(R);
        O.ClearAllGlobals();
        R.No := 1;
        R.Insert();
        Check(Format(O.CountA1()), '1');
    end;

    // Not read: where the record that shares the table lives. Another codeunit's global here; corpus PR 1564
    // ClearSharedCodeunitCopyHolder.al measures a local of the test and a holder global (:337). Runs flagged.
    [Test]
    procedure ArraySharerInOtherCodeunitGlobal()
    var
        O: Codeunit "RR Arr Owner";
        K: Codeunit "RR Keeper";
    begin
        K.Keep(O);
        O.ClearAllGlobals();
        K.InsertKept(1);
        Check(Format(O.CountA1()), '1');
    end;

    // Not measured: an array of two dimensions shared by Copy. Refused.
    [Test]
    procedure TwoDimSharedByCopy()
    var
        O: Codeunit "RR Arr Owner";
        R: Record "RR Row" temporary;
    begin
        O.WriteB(1);
        O.ShareB11(R);
        O.ClearAllGlobals();
        Check(Format(R.Count()), '1');
    end;

    // Not measured: an array of three dimensions with rows. Refused.
    [Test]
    procedure ThreeDimWritten()
    var
        O: Codeunit "RR Arr Owner";
    begin
        O.WriteC(1);
        O.ClearAllGlobals();
        Check(Format(O.CountC()), '0');
    end;

    // Not measured: a Subtype = Normal owner of an array shared by Copy. Refused.
    [Test]
    procedure ArrayNormalOwnerShared()
    var
        O: Codeunit "RR Arr Normal Owner";
    begin
        Check(Format(O.SharedClear()), '0');
    end;

    // Not measured: a SingleInstance owner of an array of one dimension with rows that nothing shares. Refused.
    [Test]
    procedure ArraySingleInstanceWritten()
    var
        O: Codeunit "RR Arr SI Owner";
    begin
        Check(Format(O.WrittenClear()), '0');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
