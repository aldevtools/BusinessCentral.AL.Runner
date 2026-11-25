codeunit 50620 "R3 ClearAll Tests"
{
    Subtype = Test;

    // Measured: the array's table shared with a record of the test through Copy(A[1], true) stays shared (corpus PR 1564
    // ClearSharedCodeunitTempArrayShare.al SIArrWritten_SharedByCopy_ClearAll :772). Runs flagged.
    [Test]
    procedure TempArraySharedByCopySingleInstance()
    var
        O: Codeunit "R3 Arr SI Owner";
        R: Record "R3 Row" temporary;
    begin
        O.Reset();
        O.Write(1);
        O.ShareByCopy(R);
        O.ClearAllGlobals();
        Check(O.State(), 'a1=1 a2=1 cnt=0');
        R.No := 2;
        R.Insert();
        Check(O.State(), 'a1=2 a2=2 cnt=0');
    end;

    // Not measured: the same table shared through RecordRef.SetTable(R, true). Refused.
    [Test]
    procedure TempArraySharedByRecordRefSingleInstance()
    var
        O: Codeunit "R3 Arr SI Owner";
        R: Record "R3 Row" temporary;
    begin
        O.Reset();
        O.Write(1);
        O.ShareByRef(R);
        O.ClearAllGlobals();
        Check(O.State(), 'a1=1 a2=1 cnt=0');
        R.No := 2;
        R.Insert();
        Check(O.State(), 'a1=2 a2=2 cnt=0');
    end;

    // Not measured: the same table shared through RecordRef.Copy(RecRef, true), a RecordRef and not Record.Copy. Refused.
    [Test]
    procedure TempArraySharedByRecordRefCopySingleInstance()
    var
        O: Codeunit "R3 Arr SI Owner";
        Other: RecordRef;
    begin
        O.Reset();
        O.Write(1);
        O.ShareByRecordRefCopy(Other);
        O.ClearAllGlobals();
        Check(O.State(), 'a1=1 a2=1 cnt=0');
        Check(Format(Other.Count()), '1');
    end;

    // Not measured: two records of the test share the table through Copy(A[1], true). Refused.
    [Test]
    procedure TempArraySharedByTwoCopiesSingleInstance()
    var
        O: Codeunit "R3 Arr SI Owner";
        R1: Record "R3 Row" temporary;
        R2: Record "R3 Row" temporary;
    begin
        O.Reset();
        O.Write(1);
        O.ShareByTwoCopies(R1, R2);
        O.ClearAllGlobals();
        Check(O.State(), 'a1=1 a2=1 cnt=0');
        Check(Format(R2.Count()), '1');
    end;

    // Not measured: the record of the test copied a record that copied A[1] (R2.Copy(R1, true)). Refused.
    [Test]
    procedure TempArraySharedByChainedCopySingleInstance()
    var
        O: Codeunit "R3 Arr SI Owner";
        R2: Record "R3 Row" temporary;
    begin
        O.Reset();
        O.Write(1);
        O.ShareByChainedCopy(R2);
        O.ClearAllGlobals();
        Check(O.State(), 'a1=1 a2=1 cnt=0');
        Check(Format(R2.Count()), '1');
    end;

    // Measured: the register's table shared with a record of the test through Copy(Rows, true) stays shared (corpus PR 1564
    // ClearSharedCodeunitSetterOwner.al :397). Runs flagged.
    [Test]
    procedure RegisterTableSharedByCopy()
    var
        C: Codeunit "R3 Caller";
        R: Record "R3 Row" temporary;
    begin
        C.Prepare(2);
        C.ShareRows(R);
        C.RunOwner();
        Check(Format(R.Count()), '2');
        R.No := 13;
        R.Insert();
        Check(C.CallerState(), 'rows=3 dec=0 flag=No code= nested=0');
    end;

    // Not measured: the register's table shared through RecordRef.SetTable(R, true). Refused.
    [Test]
    procedure RegisterTableSharedByRecordRef()
    var
        C: Codeunit "R3 Caller";
        R: Record "R3 Row" temporary;
    begin
        C.Prepare(2);
        C.ShareRowsByRef(R);
        C.RunOwner();
        Check(Format(R.Count()), '2');
        R.No := 13;
        R.Insert();
        Check(C.CallerState(), 'rows=3 dec=0 flag=No code= nested=0');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
