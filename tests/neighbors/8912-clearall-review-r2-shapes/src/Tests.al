codeunit 50620 "PS ClearAll Tests"
{
    Subtype = Test;

    // The price store nothing else holds, rows now, ClearAll in SetLine through an interface: reset in place, T1 keeps its
    // rows (corpus PR 1564 ClearSharedCodeunitStoreShapes.al Price_ViaInterface_Filled_SetLine :1206).
    [Test]
    procedure PriceStoreViaInterface()
    var
        O: Codeunit "PS Owner";
        L: Interface "PS Line";
    begin
        O.Prepare();
        L := O;
        L.SetLine();
        Check(O.State(), 't1=2 kind=None int=0 lc=store label cnt=0 line=1');
        O.WriteT1(10);
        Check(O.State(), 't1=3 kind=None int=0 lc=store label cnt=0 line=1');
    end;

    // Called directly (:1225).
    [Test]
    procedure PriceStoreDirect()
    var
        O: Codeunit "PS Owner";
    begin
        O.Prepare();
        O.SetLine();
        Check(O.State(), 't1=2 kind=None int=0 lc=store label cnt=0 line=1');
    end;

    // SetLine twice, a row and an Integer written between: the rows add up (:1277).
    [Test]
    procedure PriceStoreTwice()
    var
        O: Codeunit "PS Owner";
    begin
        O.Prepare();
        O.SetLine();
        O.WriteT1(10);
        O.SetInt(8);
        O.SetLine();
        Check(O.State(), 't1=3 kind=None int=0 lc=store label cnt=0 line=1');
    end;

    // Never prepared (:1242): main's fresh instance reads the same, no mark.
    [Test]
    procedure PriceStoreFresh()
    var
        O: Codeunit "PS Owner";
    begin
        O.SetLine();
        Check(O.State(), 't1=0 kind=None int=0 lc=store label cnt=0 line=1');
    end;

    // T1 emptied before (:1257): main's answer, no mark.
    [Test]
    procedure PriceStoreEmptied()
    var
        O: Codeunit "PS Owner";
    begin
        O.Prepare();
        O.EmptyT1();
        O.SetLine();
        O.WriteT1(10);
        Check(O.State(), 't1=1 kind=None int=0 lc=store label cnt=0 line=1');
    end;

    // An alias of the store taken before stays on the one reset instance; writes cross (:1296).
    [Test]
    procedure PriceStoreAlias()
    var
        O: Codeunit "PS Owner";
        X: Codeunit "PS Store";
    begin
        O.Prepare();
        O.GetStore(X);
        O.SetLine();
        Check(X.State(), 't1=2 kind=None int=0 lc=store label');
        X.SetInt(10);
        Check(O.State(), 't1=2 kind=None int=10 lc=store label cnt=0 line=1');
    end;

    // T1 shared with a record of the test by R.Copy(T1, true) stays shared (:1322).
    [Test]
    procedure PriceStoreCopyShared()
    var
        O: Codeunit "PS Owner";
        R: Record "PS Row" temporary;
    begin
        O.Prepare();
        O.ShareT1(R);
        O.SetLine();
        Check(Format(R.Count()), '2');
        R.No := 30;
        R.Insert();
        Check(O.State(), 't1=3 kind=None int=0 lc=store label cnt=0 line=1');
    end;

    // Shared with no row after a call made the instance (Mark() through the alias): measured (ClearSharedCodeunitHolderKinds.al
    // Price_VarParam_Never :1323). One instance, reset in place; runs flagged.
    [Test]
    procedure PriceStoreAliasMarkedNoRow()
    var
        O: Codeunit "PS Owner";
        X: Codeunit "PS Store";
    begin
        O.GetStore(X);
        X.Mark();
        O.SetLine();
        Check(X.State(), 't1=0 kind=None int=0 lc=store label');
        X.SetInt(10);
        Check(O.State(), 't1=0 kind=None int=10 lc=store label cnt=0 line=1');
    end;

    // Not measured: shared while no call made the instance (every price store cell calls it first). Refused.
    [Test]
    procedure PriceStoreAliasNeverWritten()
    var
        O: Codeunit "PS Owner";
        X: Codeunit "PS Store";
    begin
        O.GetStore(X);
        O.SetLine();
        Check(X.State(), 't1=0 kind=None int=0 lc=store label');
    end;

    // Not measured: T1 Copy-shared while it holds no row.
    [Test]
    procedure PriceStoreCopySharedEmpty()
    var
        O: Codeunit "PS Owner";
        R: Record "PS Row" temporary;
    begin
        O.ShareT1(R);
        O.SetLine();
        R.No := 1;
        R.Insert();
        Check(O.State(), 't1=1 kind=None int=0 lc=store label cnt=0 line=1');
    end;

    // Not measured: a SingleInstance owner.
    [Test]
    procedure PriceStoreSingleInstanceOwner()
    var
        O: Codeunit "PS SI Owner";
    begin
        O.WriteAndClear();
    end;

    // Not measured: a TableNo owner.
    [Test]
    procedure PriceStoreTableNoOwner()
    var
        O: Codeunit "PS TableNo Owner";
        R: Record "PS Row";
    begin
        O.Run(R);
    end;

    // Not measured: the Label with no Comment.
    [Test]
    procedure PriceStorePlainLabel()
    var
        O: Codeunit "PS Other Owner";
    begin
        O.PlainLabel();
    end;

    // Not measured: one more global (a Decimal).
    [Test]
    procedure PriceStoreMoreGlobals()
    var
        O: Codeunit "PS Other Owner";
    begin
        O.More();
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
