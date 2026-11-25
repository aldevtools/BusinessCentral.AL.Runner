codeunit 50804 "U6 ClearAll Tests"
{
    Subtype = Test;

    // The call only shares the register's table (no row, no scalar written), then ClearAll in the plain owner. Every
    // register cell of corpus PR 1564 calls MarkReg() first (ClearSharedCodeunitSetterOwner.al TableNoRun_VarSetter_SharedNever
    // :437): refused.
    [Test]
    procedure RegCopySharedReadOnlyCall()
    var
        O: Codeunit "U6 Owner";
        Y: Codeunit "U6 Reg";
        T: Record "U6 Row" temporary;
    begin
        O.SetReg(Y);
        Y.ShareRows(T);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.RegGet() <> 5 then
            Error('owner reads %1', O.RegGet());
    end;

    // The call shares the table and writes a scalar: runs flagged, as before.
    [Test]
    procedure RegCopySharedWritingCall()
    var
        O: Codeunit "U6 Owner";
        Y: Codeunit "U6 Reg";
        T: Record "U6 Row" temporary;
    begin
        O.SetReg(Y);
        Y.ShareRows(T);
        Y.Set(3);
        O.ClearAllGlobals();
        Y.Set(5);
        if O.RegGet() <> 5 then
            Error('owner reads %1', O.RegGet());
    end;
}
