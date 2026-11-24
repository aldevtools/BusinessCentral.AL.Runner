codeunit 50720 "U4 ClearAll Tests"
{
    Subtype = Test;

    // Shared while no call made the instance, then ClearAll in the plain owner, then a write through the alias. Measured
    // for the one-Integer store (ClearSharedCodeunit.al ClearAll_GlobalUnusedIface), "CLB Store"
    // (ClearSharedCodeunitTempRef.al :382), the array store (ClearSharedCodeunitOwnerKinds.al :418) and the store with one more
    // Integer (:684).
    [Test]
    procedure IntStoreAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Int";
    begin
        O.ShareInt(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    [Test]
    procedure ClbStoreAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Clb";
    begin
        O.ShareClb(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    [Test]
    procedure ArrStoreAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Arr";
    begin
        O.ShareArr(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    [Test]
    procedure PlusStoreAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Plus";
    begin
        O.SharePlus(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    // Not measured for these four stores: every shared cell of corpus PR 1564 made the instance with a call first.
    [Test]
    procedure WrapperAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Wrap";
    begin
        O.ShareWrap(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    [Test]
    procedure TwoDimStoreAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 TwoDim";
    begin
        O.ShareTwoDim(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    [Test]
    procedure RegisterAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Reg";
    begin
        O.SetReg(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    [Test]
    procedure PriceStoreAliasNeverCalled()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Price";
    begin
        O.SharePrice(Y);
        O.ClearAllGlobals();
        Y.Set(5);
    end;

    // A call made the instance but wrote nothing (a read). The cells wrote a scalar (ClearSharedCodeunitHolderKinds.al
    // Price_VarParam_Never :1323); whether the call wrote is not read. Runs flagged: one instance.
    [Test]
    procedure PriceStoreAliasReadOnlyCall()
    var
        O: Codeunit "U4 Owner";
        Y: Codeunit "U4 Price";
    begin
        O.SharePrice(Y);
        if Y.Get() <> 0 then
            Error('read %1', Y.Get());
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PriceGet() <> 5 then
            Error('owner reads %1', O.PriceGet());
    end;
}
