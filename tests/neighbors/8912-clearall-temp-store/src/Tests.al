codeunit 50120 "CT ClearAll Tests"
{
    Subtype = Test;

    var
        GX: Codeunit "CT Store";

    [Test]
    procedure SiOwnerKeepsRows()
    var
        S: Codeunit "CT SI Owner";
    begin
        S.WriteX(5);
        S.SetCnt(4);
        S.ClearAllGlobals();
        Check(S.StateX(), 'rows=1 ref11=0 int1=0 cnt=0');
        S.WriteX(6);
        Check(S.StateX(), 'rows=2 ref11=50100 int1=6 cnt=0');
    end;

    [Test]
    procedure PlainHolderKeepsRows()
    var
        H: Codeunit "CT Holder";
    begin
        H.WriteX(3);
        H.ClearAllGlobals();
        Check(H.StateX(), 'rows=1 ref11=0 int1=0');
    end;

    [Test]
    procedure TestCodeunitOwnerKeepsRows()
    begin
        GX.Write(3);
        ClearAll();
        Check(GX.State(), 'rows=1 ref11=0 int1=0');
    end;

    [Test]
    procedure TempTableNeverWritten()
    var
        H: Codeunit "CT Holder";
    begin
        H.SetIntX(3);
        H.ClearAllGlobals();
        Check(H.StateX(), 'rows=0 ref11=0 int1=0');
    end;

    [Test]
    procedure ArrayOfTempRecordsInPlainCodeunit()
    var
        C: Codeunit "CT Array Owner";
    begin
        Check(Format(C.ArrayWritten()), '1');
    end;

    [Test]
    procedure ArrayOfTempRecordsInReport()
    var
        R: Report "CT Report Owner";
    begin
        Check(Format(R.ArrayWritten()), '1');
    end;

    [Test]
    procedure NormalSubtypeOwner()
    var
        C: Codeunit "CT Normal Owner";
    begin
        Check(C.GlobalWritten(), 'rows=1 ref11=0 int1=0');
    end;

    [Test]
    procedure ReportOwner()
    var
        R: Report "CT Report Owner";
    begin
        Check(R.GlobalWritten(), 'rows=1 ref11=0 int1=0');
    end;

    [Test]
    procedure StoreWithOneMoreGlobal()
    var
        H: Codeunit "CT Holder";
    begin
        H.WriteE();
        H.ClearAllGlobals();
        Check(Format(H.CountE()), '1');
    end;

    [Test]
    procedure StoreOneLevelDeeper()
    var
        H: Codeunit "CT Holder";
    begin
        H.WriteW();
        H.ClearAllGlobals();
        Check(H.StateW(), 'rows=1 ref11=0 int1=0');
    end;

    [Test]
    procedure ArrayOfStores()
    var
        H: Codeunit "CT Holder";
    begin
        H.WriteA();
        H.ClearAllGlobals();
        Check(H.StateA(), 'rows=1 ref11=0 int1=0');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
