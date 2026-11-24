codeunit 50320 "CN ClearAll Tests"
{
    Subtype = Test;

    // Measured: the Label store shared through a second variable in a plain holder (ClearSharedCodeunitLabelStore.al
    // LabelLast_ClearAllInHolder_AliasWroteBefore :672). This store also implements an interface; 1564's does not.
    [Test]
    procedure LabelHolderSharedAlias()
    var
        H: Codeunit "CN Holder";
        Y: Codeunit "CN Label Store";
    begin
        H.ShareL(Y);
        Y.Write(3);
        H.ClearAllGlobals();
        Check(H.StateL(), 'rows=1 int1=0 Store');
        Y.Write(5);
        Check(H.StateL(), 'rows=2 int1=5 Store');
    end;

    // Not measured: the Label store shared through an interface. The sharer's kind is not read: it runs as the alias.
    [Test]
    procedure LabelHolderSharedIface()
    var
        H: Codeunit "CN Holder";
        I: Interface "CN IStore";
    begin
        H.ShareLIface(I);
        I.Write(3);
        H.ClearAllGlobals();
        Check(H.StateL(), 'rows=1 int1=0 Store');
    end;

    // Not measured: the Label store behind a by-value parameter of the routine that runs ClearAll: refused.
    [Test]
    procedure LabelValueParamInClearingRoutine()
    var
        H: Codeunit "CN Holder";
    begin
        Check(H.ValueParamL(), 'rows=1 int1=5 Store');
    end;

    // Not measured: the by-value parameter belongs to a routine further out than the one that runs ClearAll. Not
    // detected: it runs as the alias.
    [Test]
    procedure LabelValueParamFurtherOut()
    var
        H: Codeunit "CN Holder";
    begin
        Check(H.OuterValueParamL(), 'rows=1 int1=5 Store');
    end;

    // Not measured: a Label between the RecordRef array and the Integer array.
    [Test]
    procedure LabelBetweenArraysWritten()
    var
        H: Codeunit "CN Holder";
    begin
        H.WriteM2(3);
        H.ClearAllGlobals();
        Check(H.StateM2(), 'rows=1 int1=0');
    end;

    // Not measured: a Label with Locked = true.
    [Test]
    procedure LabelLockedWritten()
    var
        H: Codeunit "CN Holder";
    begin
        H.WriteLk(3);
        H.ClearAllGlobals();
        Check(H.StateLk(), 'rows=1 int1=0');
    end;

    // Not measured: a Label on the array store.
    [Test]
    procedure LabelOnArrStoreWritten()
    var
        H: Codeunit "CN Holder";
    begin
        H.WriteAL(3);
        H.ClearAllGlobals();
        Check(H.StateAL(), 'rows=1 int1=0');
    end;

    // Not measured: a Label on the one-Integer store, shared.
    [Test]
    procedure LabelOnIntStoreShared()
    var
        H: Codeunit "CN Holder";
        Y: Codeunit "CN Int Label Store";
    begin
        H.ShareIL(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Y.Set(5);
        Check(Format(H.GetIL()), '5');
    end;

    // Not measured: a wrapper never written whose store an alias shares: refused (the probe's alias wrote).
    [Test]
    procedure WrapperUnwrittenInnerShared()
    var
        H: Codeunit "CN Holder";
        Y: Codeunit "CN Store";
    begin
        H.ShareInner(Y);
        H.ClearAllGlobals();
        Y.Write(5);
        Check(H.StateW(), 'rows=1 ref11=50300 int1=5 wcnt=0');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}

codeunit 50321 "CN Owner Tests"
{
    Subtype = Test;

    // Not measured: the Label store shared under a SingleInstance owner.
    [Test]
    procedure LabelSiShared()
    var
        S: Codeunit "CN SI Owner";
        Y: Codeunit "CN Label Store";
    begin
        S.ShareL(Y);
        Y.Write(3);
        S.ClearAllGlobals();
        Check(S.StateL(), 'rows=1 int1=0 Store');
    end;

    // Not measured: the store with one more Integer under a SingleInstance owner.
    [Test]
    procedure PlusSiWritten()
    begin
        PlusSi();
    end;

    // Not measured: the wrapper under a SingleInstance owner.
    [Test]
    procedure WrapperSiWritten()
    var
        S: Codeunit "CN SI Owner";
    begin
        S.WriteW(3);
        S.ClearAllGlobals();
        Check(S.StateW(), 'rows=1 ref11=0 int1=0 wcnt=0');
    end;

    // Not measured: an array of stores under a SingleInstance owner.
    [Test]
    procedure StoreArraySiWritten()
    var
        S: Codeunit "CN SI Owner";
    begin
        S.WriteA(3);
        S.ClearAllGlobals();
        Check(S.StateA(), 'rows=1 ref11=0 int1=0');
    end;

    // Not measured: the Label store under a codeunit with Subtype = Normal.
    [Test]
    procedure LabelNormalOwnerWritten()
    var
        C: Codeunit "CN Normal Owner";
    begin
        C.WriteL(3);
        C.ClearAllGlobals();
        Check(C.StateL(), 'rows=1 int1=0 Store');
    end;

    // Not measured: a report that does not state UseRequestPage = false.
    [Test]
    procedure ReportWithRequestPageWritten()
    var
        R: Report "CN Report RP";
    begin
        R.WriteX(3);
        R.ClearAllGlobals();
        Check(R.StateX(), 'rows=1 ref11=0 int1=0');
    end;

    // Not measured: a page owner.
    [Test]
    procedure PageOwnerWritten()
    var
        P: Page "CN Page Owner";
    begin
        P.WriteX(3);
        P.ClearAllGlobals();
        Check(P.StateX(), 'rows=1 ref11=0 int1=0');
    end;

    // Not measured: a table owner.
    [Test]
    procedure TableOwnerWritten()
    var
        T: Record "CN Table Owner";
    begin
        T.WriteX(3);
        T.ClearAllGlobals();
        Check(T.StateX(), 'rows=1 ref11=0 int1=0');
    end;

    local procedure PlusSi()
    var
        S: Codeunit "CN SI Owner";
    begin
        S.WriteP(3);
        S.ClearAllGlobals();
        Check(S.StateP(), 'rows=1 int1=0 n=0');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}

codeunit 50322 "CN Test Global Tests"
{
    Subtype = Test;

    var
        GL: Codeunit "CN Label Store";

    // Not measured: the Label store as a test codeunit global.
    [Test]
    procedure LabelTestGlobalWritten()
    begin
        GL.Write(3);
        ClearAll();
        Check(GL.State(), 'rows=1 int1=0 Store');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
