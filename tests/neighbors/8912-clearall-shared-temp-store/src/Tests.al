codeunit 50120 "CS Shared ClearAll Tests"
{
    Subtype = Test;

    [Test]
    procedure PlainHolderSharedAliasWroteBefore()
    var
        H: Codeunit "CS Holder";
        Y: Codeunit "CS Store";
    begin
        H.Share(Y);
        Y.Write(3);
        H.SetCount(4);
        H.ClearAllGlobals();
        Check(H.StateX(), 'rows=1 ref11=0 int1=0 count=0');
        Check(Y.State(), 'rows=1 ref11=0 int1=0');
        Y.Write(5);
        Check(H.StateX(), 'rows=2 ref11=50100 int1=5 count=0');
    end;

    [Test]
    procedure PlainHolderSharedUnused()
    var
        H: Codeunit "CS Holder";
        Y: Codeunit "CS Store";
    begin
        H.Share(Y);
        H.ClearAllGlobals();
        Y.Write(5);
        Check(H.StateX(), 'rows=1 ref11=50100 int1=5 count=0');
    end;

    [Test]
    procedure PlainHolderValueParam()
    var
        H: Codeunit "CS Holder";
    begin
        Check(H.ValueParam(), 'rows=1 ref11=50100 int1=5');
    end;

    [Test]
    procedure SiOwnerSharedAlias()
    var
        S: Codeunit "CS SI Owner";
        Y: Codeunit "CS Store";
    begin
        S.Share(Y);
        Y.Write(3);
        S.SetCnt(4);
        S.ClearAllGlobals();
        Check(S.StateX(), 'rows=1 ref11=0 int1=0 cnt=0');
        Y.Write(5);
        Check(S.StateX(), 'rows=2 ref11=50100 int1=5 cnt=0');
    end;

    [Test]
    procedure ArrHolderSharedWroteBefore()
    var
        H: Codeunit "CS Arr Holder";
        Y: Codeunit "CS Arr Store";
    begin
        H.Share(Y);
        Y.Write(1, 3);
        Y.Write(2, 4);
        H.ClearAllGlobals();
        Check(H.StateX(), 'rows1=2 rows2=2 ref1=0 int1=0');
        Y.Write(2, 5);
        Check(H.StateX(), 'rows1=3 rows2=3 ref1=50100 int1=5');
    end;

    [Test]
    procedure ArrHolderSharedUnused()
    var
        H: Codeunit "CS Arr Holder";
        Y: Codeunit "CS Arr Store";
    begin
        H.Share(Y);
        H.ClearAllGlobals();
        Y.Write(1, 5);
        Check(H.StateX(), 'rows1=1 rows2=1 ref1=50100 int1=5');
    end;

    [Test]
    procedure ArrHolderUnsharedWritten()
    var
        H: Codeunit "CS Arr Holder";
    begin
        H.WriteX(1, 3);
        H.ClearAllGlobals();
        Check(H.StateX(), 'rows1=1 rows2=1 ref1=0 int1=0');
    end;

    [Test]
    procedure ArrSiOwnerShared()
    var
        S: Codeunit "CS Arr SI Owner";
        Y: Codeunit "CS Arr Store";
    begin
        S.Share(Y);
        Y.Write(1, 3);
        S.ClearAllGlobals();
        Check(S.StateX(), 'rows1=1 rows2=1 ref1=0 int1=0');
    end;

    [Test]
    procedure LabelStoreWritten()
    var
        H: Codeunit "CS Label Holder";
    begin
        H.WriteX(3);
        H.ClearAllGlobals();
        Check(H.StateX(), 'rows=1 int1=0 Store');
    end;

    [Test]
    procedure LabelStoreShared()
    var
        H: Codeunit "CS Label Holder";
        Y: Codeunit "CS Label Store";
    begin
        H.Share(Y);
        Y.Write(3);
        H.ClearAllGlobals();
        Check(H.StateX(), 'rows=1 int1=0 Store');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}

codeunit 50121 "CS Test Owner Tests"
{
    Subtype = Test;

    var
        GX: Codeunit "CS Store";

    [Test]
    procedure TestOwnerSharedLocalAlias()
    var
        Y: Codeunit "CS Store";
    begin
        Y := GX;
        Y.Write(3);
        ClearAll();
        Check(GX.State(), 'rows=1 ref11=0 int1=0');
        Y.Write(5);
        Check(GX.State(), 'rows=2 ref11=50100 int1=5');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}

codeunit 50122 "CS Int Test Owner Tests"
{
    Subtype = Test;

    var
        GN: Codeunit "CS Int Store";

    [Test]
    procedure IntegerStoreTestOwnerShared()
    var
        Y: Codeunit "CS Int Store";
    begin
        Y := GN;
        Y.Set(3);
        ClearAll();
        Y.Set(5);
        Check(Format(GN.Get()), '5');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
