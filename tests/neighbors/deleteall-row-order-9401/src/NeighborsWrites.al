// Neighbors of the DeleteAll row order (al-runner issue 9401, corpus PR 1659), second set: row code that writes the table
// outside the filter or the row being deleted, through the record of an event subscriber (database and temporary), row code
// that reads the table, a table of TableType Temporary, and ChangeCompany. Rows 1..5 of "DRN Row" as in Neighbors.al.

// No OnDelete; an OnBeforeDeleteEvent subscriber (codeunit 94023) that logs the row and, in mode 'modify-rec', modifies it.
table 94021 "DRN Sub Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// TableType = Temporary, with OnDelete.
table 94022 "DRN Temp Type Row"
{
    TableType = Temporary;

    fields
    {
        field(1; Id; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    trigger OnDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('T' + Format(Id));
    end;
}

codeunit 94023 "DRN Sub Subs"
{
    var
        Mode: Text;

    procedure SetMode(NewMode: Text)
    begin
        Mode := NewMode;
    end;

    [EventSubscriber(ObjectType::Table, Database::"DRN Sub Row", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure OnBeforeDelete(var Rec: Record "DRN Sub Row"; RunTrigger: Boolean)
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('B' + Format(Rec.Id));
        if (Ctl.CurrentMode() = 'modify-rec') and (Rec.Id = 3) then begin
            Rec.Name := 'changed';
            Rec.Modify(false);
        end;
    end;
}

codeunit 94024 "DRN Neighbors Writes"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Ctl: Codeunit "DRN Ctl";
        Assert: Codeunit "DRN Assert";

    local procedure Seed(Rows: Integer)
    var
        Row: Record "DRN Row";
        i: Integer;
    begin
        Row.DeleteAll(false);
        for i := 1 to Rows do begin
            Row.Init();
            Row.Id := i;
            Row.Insert(false);
        end;
        Ctl.Take();
    end;

    local procedure Left() Result: Text
    var
        Row: Record "DRN Row";
    begin
        if Row.FindSet() then
            repeat
                if Result <> '' then
                    Result += ',';
                Result += Format(Row.Id);
            until Row.Next() = 0;
        if Result = '' then
            Result := '<none>';
    end;

    // Row code inserts a row of the same table outside the filter of the descending DeleteAll.
    [Test]
    procedure RowCodeInsertsRowOutsideFilterDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(5);
        Ctl.SetMode('insert-outside');
        Row.SetRange(Id, 2, 4);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T4,T3,T2', Ctl.Take(), 'order');
        Assert.AreEqual('1,5,20', Left(), 'left');
    end;

    // Row code modifies the row being deleted, through another record variable.
    [Test]
    procedure RowCodeModifiesItsOwnRowDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('modify-self');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
        Assert.AreEqual('<none>', Left(), 'left');
    end;

    // An OnBeforeDeleteEvent subscriber modifies the row through its Rec (database record).
    [Test]
    procedure SubscriberModifiesRecDescending()
    var
        Row: Record "DRN Sub Row";
        i: Integer;
    begin
        Row.DeleteAll(false);
        for i := 1 to 3 do begin
            Row.Id := i;
            Row.Insert(false);
        end;
        Ctl.Take();
        Ctl.SetMode('modify-rec');
        Row.Ascending(false);
        Row.DeleteAll(false);
        Assert.AreEqual('B3,B2,B1', Ctl.Take(), 'order');
        Assert.AreEqual(0, Row.Count(), 'left');
    end;

    // An OnBeforeDeleteEvent subscriber modifies the row through its Rec (temporary record).
    [Test]
    procedure SubscriberModifiesRecTemporaryDescending()
    var
        Row: Record "DRN Sub Row" temporary;
        i: Integer;
    begin
        for i := 1 to 3 do begin
            Row.Id := i;
            Row.Insert(false);
        end;
        Ctl.Take();
        Ctl.SetMode('modify-rec');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('B3,B2,B1', Ctl.Take(), 'order');
        Assert.AreEqual(0, Row.Count(), 'left');
    end;

    // Control: an OnBeforeDeleteEvent subscriber that writes nothing, temporary record (served).
    [Test]
    procedure SubscriberOnlyTemporaryDescending()
    var
        Row: Record "DRN Sub Row" temporary;
        i: Integer;
    begin
        for i := 1 to 3 do begin
            Row.Id := i;
            Row.Insert(false);
        end;
        Ctl.Take();
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('B3,B2,B1', Ctl.Take(), 'order');
    end;

    // Row code reads the table (Count): not measured, refused with main's text (records/delete_all_order.rs).
    [Test]
    procedure RowCodeReadsTableDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('read-count');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,c3,T2,c2,T1,c1', Ctl.Take(), 'order');
    end;

    // A table of TableType Temporary, descending.
    [Test]
    procedure TableTypeTemporaryDescending()
    var
        Row: Record "DRN Temp Type Row";
        i: Integer;
    begin
        for i := 1 to 3 do begin
            Row.Id := i;
            Row.Insert(false);
        end;
        Ctl.Take();
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
    end;

    // ChangeCompany to the session's own company, descending: no cell measures it, refused with main's text.
    [Test]
    procedure ChangeCompanyDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Row.ChangeCompany(CompanyName());
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
    end;
}
