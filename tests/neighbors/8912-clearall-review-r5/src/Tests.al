codeunit 50770 "U5 ClearAll Tests"
{
    Subtype = Test;

    // Shared, then a call that reads only (writes no row and no scalar), then ClearAll in the plain owner. Every cell of
    // corpus PR 1564 for the wrapper, the 2-D store, the register and the price store writes a scalar first: refused. The
    // one-Integer store (ClearSharedCodeunit.al) and the store with one more Integer (ClearSharedCodeunitPlusUncalled.al) are
    // measured with no call at all and with a call that writes: runs flagged, as before.
    [Test]
    procedure WrapReadOnlyCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Wrap";
        R: Integer;
    begin
        O.ShareWrap(Y);
        R := Y.Get();
        O.ClearAllGlobals();
        Y.Set(5);
        if O.WrapGet() <> 5 then
            Error('owner reads %1', O.WrapGet());
    end;

    [Test]
    procedure TwoDimReadOnlyCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 TwoDim";
        R: Integer;
    begin
        O.ShareTwoDim(Y);
        R := Y.Get();
        O.ClearAllGlobals();
        Y.Set(5);
        if O.TwoDimGet() <> 5 then
            Error('owner reads %1', O.TwoDimGet());
    end;

    [Test]
    procedure RegReadOnlyCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Reg";
        R: Integer;
    begin
        O.SetReg(Y);
        R := Y.Get();
        O.ClearAllGlobals();
        Y.Set(5);
        if O.RegGet() <> 5 then
            Error('owner reads %1', O.RegGet());
    end;

    [Test]
    procedure PriceReadOnlyCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Price";
        R: Integer;
    begin
        O.SharePrice(Y);
        R := Y.Get();
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PriceGet() <> 5 then
            Error('owner reads %1', O.PriceGet());
    end;

    [Test]
    procedure IntReadOnlyCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Int";
        R: Integer;
    begin
        O.ShareInt(Y);
        R := Y.Get();
        O.ClearAllGlobals();
        Y.Set(5);
        if O.IntGet() <> 5 then
            Error('owner reads %1', O.IntGet());
    end;

    [Test]
    procedure PlusReadOnlyCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Plus";
        R: Integer;
    begin
        O.SharePlus(Y);
        R := Y.Get();
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PlusGet() <> 5 then
            Error('owner reads %1', O.PlusGet());
    end;

    // Shared, then a call that writes a scalar: runs flagged for the four stores (the cells write a scalar first).
    [Test]
    procedure WrapWritingCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Wrap";
        R: Integer;
    begin
        O.ShareWrap(Y);
        Y.Set(3);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.WrapGet() <> 5 then
            Error('owner reads %1', O.WrapGet());
    end;

    [Test]
    procedure TwoDimWritingCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 TwoDim";
        R: Integer;
    begin
        O.ShareTwoDim(Y);
        Y.Set(3);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.TwoDimGet() <> 5 then
            Error('owner reads %1', O.TwoDimGet());
    end;

    [Test]
    procedure RegWritingCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Reg";
        R: Integer;
    begin
        O.SetReg(Y);
        Y.Set(3);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.RegGet() <> 5 then
            Error('owner reads %1', O.RegGet());
    end;

    [Test]
    procedure PriceWritingCall()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Price";
        R: Integer;
    begin
        O.SharePrice(Y);
        Y.Set(3);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PriceGet() <> 5 then
            Error('owner reads %1', O.PriceGet());
    end;

    // The store with one more Integer, shared before any call: measured for "CLQ Store Plus", which implements no
    // interface (ClearSharedCodeunitPlusUncalled.al). One that implements an interface refuses, through an alias, an
    // Interface variable, a List or a Dictionary of the interface.
    [Test]
    procedure PlusNoInterfaceAliasNeverCalled()
    var
        O: Codeunit "U5 Owner";
        Y: Codeunit "U5 Plus";
        R: Integer;
    begin
        O.SharePlus(Y);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PlusGet() <> 5 then
            Error('owner reads %1', O.PlusGet());
    end;

    [Test]
    procedure PlusFaceAliasNeverCalled()
    var
        O: Codeunit "U5 FaceOwner";
        Y: Codeunit "U5 PlusFace";
    begin
        O.SharePlus(Y);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PlusGet() <> 5 then
            Error('owner reads %1', O.PlusGet());
    end;

    [Test]
    procedure PlusFaceInterfaceNeverCalled()
    var
        O: Codeunit "U5 FaceOwner";
        Y: Codeunit "U5 PlusFace";
        I: Interface "U5 Face";
    begin
        O.SharePlus(Y);
        I := Y;
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PlusGet() <> 5 then
            Error('owner reads %1', O.PlusGet());
    end;

    [Test]
    procedure PlusFaceListNeverCalled()
    var
        O: Codeunit "U5 FaceOwner";
        Y: Codeunit "U5 PlusFace";
        I: Interface "U5 Face";
        L: List of [Interface "U5 Face"];
    begin
        O.SharePlus(Y);
        I := Y;
        L.Add(I);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PlusGet() <> 5 then
            Error('owner reads %1', O.PlusGet());
    end;

    [Test]
    procedure PlusFaceDictionaryNeverCalled()
    var
        O: Codeunit "U5 FaceOwner";
        Y: Codeunit "U5 PlusFace";
        I: Interface "U5 Face";
        D: Dictionary of [Integer, Interface "U5 Face"];
    begin
        O.SharePlus(Y);
        I := Y;
        D.Add(1, I);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.PlusGet() <> 5 then
            Error('owner reads %1', O.PlusGet());
    end;
}
