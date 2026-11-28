codeunit 94119 "DRR Review Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Ctl: Codeunit "DRR Ctl";

    local procedure Seed()
    var
        Row: Record "DRR Row";
        Other: Record "DRR Other";
        i: Integer;
    begin
        Row.DeleteAll(false);
        Other.DeleteAll(false);
        for i := 1 to 3 do begin
            Row.Init();
            Row.Id := i;
            Row.Insert(false);
        end;
        Row.Id := 9;
        Row.Insert(false);
        Other.Id := 1;
        Other.Insert(false);
        Ctl.Take();
    end;

    local procedure RunDescending(Mode: Text) Result: Text
    var
        Row: Record "DRR Row";
    begin
        Ctl.SetMode(Mode);
        Row.SetRange(Id, 1, 3);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Result := Ctl.Take();
    end;

    local procedure AreEqual(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected:<%1>. Actual:<%2>.', Expected, Actual);
    end;

    // Reads without a record read.
    [Test]
    procedure RowCodeFieldRefCalcFieldBlobDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('blob-fieldref'));
    end;

    [Test]
    procedure RowCodeXmlPortExportTempDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('xmlport-export-temp'));
    end;

    [Test]
    procedure RowCodeXmlPortImportDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('xmlport-import'));
    end;

    [Test]
    procedure RowCodeRecordRefIsDirtyDescending()
    begin
        Seed();
        AreEqual('T3,No,T2,T1', RunDescending('recref-isdirty'));
    end;

    [Test]
    procedure RowCodeLoadsUnloadedFieldDescending()
    begin
        Seed();
        Ctl.Hold(9);
        AreEqual('T3,,T2,T1', RunDescending('held-unloaded-field'));
    end;

    [Test]
    procedure RowCodeValidatesUnloadedFieldDescending()
    begin
        Seed();
        Ctl.Hold(9);
        AreEqual('T3,T2,T1', RunDescending('held-validate'));
    end;

    [Test]
    procedure RowCodeStartSessionTempDescending()
    begin
        Seed();
        Commit();
        AreEqual('T3,T2,T1', RunDescending('start-session-temp'));
    end;

    // Writes of the table in another company.
    [Test]
    procedure RowCodeInsertsSameTableOtherCompanyDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('insert-same-other-company'));
    end;

    [Test]
    procedure RowCodeInsertsSharedTableOtherCompanyDescending()
    var
        Shared: Record "DRR Shared";
        i: Integer;
    begin
        Shared.DeleteAll(false);
        for i := 1 to 3 do begin
            Shared.Id := i;
            Shared.Insert(false);
        end;
        Ctl.Take();
        Shared.Ascending(false);
        Shared.DeleteAll(true);
        AreEqual('T3,T2,T1', Ctl.Take());
    end;

    // Writes of another table other than an Insert in the session's company.
    [Test]
    procedure RowCodeDeletesOtherTableByKeyDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('delete-other-by-key'));
    end;

    [Test]
    procedure RowCodeModifiesOtherTableByKeyDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('modify-other-by-key'));
    end;

    [Test]
    procedure RowCodeRenamesOtherTableByKeyDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('rename-other-by-key'));
    end;

    [Test]
    procedure RowCodeModifyAllOtherTableDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('modifyall-other'));
    end;

    [Test]
    procedure RowCodeInsertsOtherTableOtherCompanyDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('insert-other-other-company'));
    end;

    [Test]
    procedure RowCodeInsertsTempTableDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('insert-temp-other'));
    end;

    // ChangeCompany('') leaves the record in the session's company; no cell measures it.
    [Test]
    procedure ChangeCompanyEmptyNameDescending()
    var
        Row: Record "DRR Row";
    begin
        Seed();
        Row.ChangeCompany('');
        Row.SetRange(Id, 1, 3);
        Row.Ascending(false);
        Row.DeleteAll(true);
        AreEqual('T3,T2,T1', Ctl.Take());
    end;

    // Control: the measured write, an Insert into another table in the session's company, runs.
    [Test]
    procedure RowCodeInsertsOtherTableControlDescending()
    begin
        Seed();
        AreEqual('T3,T2,T1', RunDescending('insert-other'));
    end;
}
