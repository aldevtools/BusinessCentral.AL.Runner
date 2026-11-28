// Neighbors of the DeleteAll row order (al-runner issue 9401, corpus PR 1659): one test per fact the served shape does not
// read, per new refusal, and controls that keep main's result. Rows 1..5 with Grp 2,1,2,1,3 and Sub = Id mod 2.
table 94011 "DRN Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Grp; Integer) { }
        field(3; Sub; Integer) { }
        field(4; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByGrp; Grp) { }
        key(ByGrpSub; Grp, Sub) { }
    }

    trigger OnDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.RowCode(Id);
    end;
}

// A primary key of two fields.
table 94012 "DRN Pk2 Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; A; Integer) { }
        field(2; B; Integer) { }
    }

    keys
    {
        key(PK; A, B) { Clustered = true; }
    }

    trigger OnDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('T' + Format(A) + '.' + Format(B));
    end;
}

// Another table the row code writes to.
table 94013 "DRN Other"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// No OnDelete of its own; a table extension of the same app has OnDelete only.
table 94015 "DRN Ext Only Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

tableextension 94016 "DRN Ext Only Ext" extends "DRN Ext Only Row"
{
    trigger OnDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('xT' + Format(Id));
    end;
}

// A table extension of a table from another app (deps/xapp-base).
tableextension 94014 "DRN Xapp Ext" extends "DRN Xapp Row"
{
    trigger OnBeforeDelete()
    var
        Ctl: Codeunit "DRN Ctl";
    begin
        Ctl.Add('xB' + Format(Id));
    end;
}

codeunit 94017 "DRN Ctl"
{
    SingleInstance = true;

    var
        Seen: Text;
        Mode: Text;

    procedure SetMode(NewMode: Text)
    begin
        Mode := NewMode;
    end;

    procedure Add(Entry: Text)
    begin
        if Seen <> '' then
            Seen += ',';
        Seen += Entry;
    end;

    procedure CurrentMode(): Text
    begin
        exit(Mode);
    end;

    procedure Take() Result: Text
    begin
        Result := Seen;
        Seen := '';
        Mode := '';
    end;

    // The OnDelete of "DRN Row": logs the row, then does what the mode asks when the row is the first one deleted (Id 3 or 4).
    procedure RowCode(Id: Integer)
    var
        Other: Record "DRN Row";
        OtherTable: Record "DRN Other";
        RowSum: Record "DRN Row Sum";
    begin
        Add('T' + Format(Id));
        case Mode of
            'delete-later':
                if Id = 3 then begin
                    Other.Get(1);
                    Other.Delete(false);
                end;
            'modify-later':
                if Id = 3 then begin
                    Other.Get(1);
                    Other.Name := 'changed';
                    Other.Modify(false);
                end;
            'insert-ahead':
                if Id = 3 then begin
                    Other.Id := 0;
                    Other.Insert(false);
                end;
            'insert-behind':
                if Id = 3 then begin
                    Other.Id := 10;
                    Other.Insert(false);
                end;
            'modify-outside':
                if Id = 4 then begin
                    Other.Get(5);
                    Other.Name := 'changed';
                    Other.Modify(false);
                end;
            'insert-other-table':
                begin
                    OtherTable.Id := Id;
                    OtherTable.Insert(false);
                end;
            'error-first':
                if Id = 3 then
                    Error('stop at %1', Id);
            'error-second':
                if Id = 2 then
                    Error('stop at %1', Id);
            'delete-later-asc':
                if Id = 1 then begin
                    Other.Get(3);
                    Other.Delete(false);
                end;
            'commit':
                if Id = 3 then
                    Commit();
            'insert-outside':
                if Id = 4 then begin
                    Other.Id := 20;
                    Other.Insert(false);
                end;
            'modify-self':
                if Id = 3 then begin
                    Other.Get(Id);
                    Other.Name := 'changed';
                    Other.Modify(false);
                end;
            'read-count':
                Add('c' + Format(Other.Count()));
            'read-get':
                Add('g' + Format(Other.Get(Id)));
            'read-flow':
                begin
                    RowSum.CalcFields(Rows);
                    Add('f' + Format(RowSum.Rows));
                end;
            'read-other-table':
                Add('o' + Format(OtherTable.Count()));
        end;
    end;
}

codeunit 94020 "DRN Neighbors"
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
            Row.Grp := GrpOf(i);
            Row.Sub := i mod 2;
            Row.Name := 'n' + Format(6 - i);
            Row.Insert(false);
        end;
        Ctl.Take();
    end;

    local procedure GrpOf(i: Integer): Integer
    begin
        case i of
            1, 3:
                exit(2);
            2, 4:
                exit(1);
        end;
        exit(3);
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

    // Direction: SetAscending on two fields of a two-field key (the probe sets one field of a one-field key).
    [Test]
    procedure SetAscendingTwoFields_Descending()
    var
        Row: Record "DRN Row";
    begin
        Seed(5);
        Row.SetCurrentKey(Grp, Sub);
        Row.SetAscending(Grp, false);
        Row.SetAscending(Sub, false);
        Row.DeleteAll(true);
        Assert.AreEqual('T5,T3,T1,T2,T4', Ctl.Take(), 'order');
    end;

    // Direction: Ascending(false), then SetAscending(Grp, false): every field descending through SetAscending.
    [Test]
    procedure AscendingFalseThenSetAscendingFalse()
    var
        Row: Record "DRN Row";
    begin
        Seed(5);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.SetAscending(Grp, false);
        Row.DeleteAll(true);
        Assert.AreEqual('T5,T3,T1,T4,T2', Ctl.Take(), 'order');
    end;

    // Direction: Ascending(false), then SetAscending(Grp, true): only the primary key tie-break descending.
    [Test]
    procedure AscendingFalseThenSetAscendingTrue()
    var
        Row: Record "DRN Row";
    begin
        Seed(5);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.SetAscending(Grp, true);
        Row.DeleteAll(true);
        Assert.AreEqual('T4,T2,T3,T1,T5', Ctl.Take(), 'order');
    end;

    // Direction: SetAscending(Id, false) on the primary key (the probe sets it on a secondary key).
    [Test]
    procedure SetAscendingPrimaryKeyField()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Row.SetAscending(Id, false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
    end;

    // Key: Ascending(false) on a secondary key of two fields.
    [Test]
    procedure AscendingFalseTwoFieldKey()
    var
        Row: Record "DRN Row";
    begin
        Seed(5);
        Row.SetCurrentKey(Grp, Sub);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T5,T3,T1,T4,T2', Ctl.Take(), 'order');
    end;

    // Key: Ascending(false) on a primary key of two fields.
    [Test]
    procedure AscendingFalseTwoFieldPrimaryKey()
    var
        Row: Record "DRN Pk2 Row";
        i: Integer;
    begin
        for i := 1 to 3 do begin
            Row.A := 1;
            Row.B := i;
            Row.Insert(false);
        end;
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T1.3,T1.2,T1.1', Ctl.Take(), 'order');
    end;

    // Key: SetCurrentKey on a field no key starts with (an ad hoc key), descending.
    [Test]
    procedure AdHocKeyDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Row.SetCurrentKey(Name);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T1,T2,T3', Ctl.Take(), 'order');
    end;

    // Row code: a table extension from another app than its table.
    [Test]
    procedure OtherAppExtensionDescending()
    var
        Row: Record "DRN Xapp Row";
        i: Integer;
    begin
        for i := 1 to 3 do begin
            Row.Id := i;
            Row.Insert(false);
        end;
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('xB3,xB2,xB1', Ctl.Take(), 'order');
    end;

    // Row code: a same-app table extension OnDelete alone, without the table's own trigger.
    [Test]
    procedure SameAppExtensionOnDeleteOnlyDescending()
    var
        Row: Record "DRN Ext Only Row";
        i: Integer;
    begin
        for i := 1 to 3 do begin
            Row.Id := i;
            Row.Insert(false);
        end;
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('xT3,xT2,xT1', Ctl.Take(), 'order');
    end;

    // Row code: MarkedOnly on a secondary key, descending.
    [Test]
    procedure MarkedOnlySecondaryKeyDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(5);
        Row.Get(1);
        Row.Mark(true);
        Row.Get(2);
        Row.Mark(true);
        Row.Get(5);
        Row.Mark(true);
        Row.SetCurrentKey(Grp);
        Row.Ascending(false);
        Row.MarkedOnly(true);
        Row.DeleteAll(true);
        Assert.AreEqual('T5,T1,T2', Ctl.Take(), 'order');
        Assert.AreEqual('3,4', Left(), 'left');
    end;

    // Row code deletes a row the descending DeleteAll has still to visit.
    [Test]
    procedure RowCodeDeletesLaterRowDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('delete-later');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2', Ctl.Take(), 'order');
    end;

    // Row code modifies a row the descending DeleteAll has still to visit.
    [Test]
    procedure RowCodeModifiesLaterRowDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('modify-later');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
    end;

    // Row code inserts a row ahead of the descending DeleteAll.
    [Test]
    procedure RowCodeInsertsAheadDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('insert-ahead');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1,T0', Ctl.Take(), 'order');
    end;

    // Row code inserts a row behind the descending DeleteAll.
    [Test]
    procedure RowCodeInsertsBehindDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('insert-behind');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
        Assert.AreEqual('10', Left(), 'left');
    end;

    // Row code modifies a row of the same table outside the filter of the descending DeleteAll.
    [Test]
    procedure RowCodeModifiesRowOutsideFilterDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(5);
        Ctl.SetMode('modify-outside');
        Row.SetRange(Id, 2, 4);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T4,T3,T2', Ctl.Take(), 'order');
        Assert.AreEqual('1,5', Left(), 'left');
        Row.Reset();
        Row.Get(5);
        Assert.AreEqual('changed', Row.Name, 'name');
    end;

    // Row code inserts into another table (record/crud/RecordPathsUnpinned.al on corpus master logs to a table).
    [Test]
    procedure RowCodeInsertsOtherTableDescending()
    var
        Row: Record "DRN Row";
        OtherTable: Record "DRN Other";
    begin
        Seed(3);
        OtherTable.DeleteAll(false);
        Ctl.SetMode('insert-other-table');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
        Assert.AreEqual(3, OtherTable.Count(), 'other rows');
    end;

    // Row code raises an error at the first row of a descending DeleteAll.
    [Test]
    procedure RowCodeErrorFirstRowDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('error-first');
        Row.Ascending(false);
        asserterror Row.DeleteAll(true);
        Assert.AreEqual('stop at 3', GetLastErrorText(), 'error');
    end;

    // Row code raises an error at the second row of a descending DeleteAll.
    [Test]
    procedure RowCodeErrorSecondRowDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('error-second');
        Row.Ascending(false);
        asserterror Row.DeleteAll(true);
        Assert.AreEqual('stop at 2', GetLastErrorText(), 'error');
    end;

    // Row code commits during a descending DeleteAll.
    [Test]
    procedure RowCodeCommitsDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('commit');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,T2,T1', Ctl.Take(), 'order');
        Assert.AreEqual('<none>', Left(), 'left');
    end;

    // Control: one row, descending (no order; main runs it).
    [Test]
    procedure OneRowDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(1);
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T1', Ctl.Take(), 'order');
    end;

    // Control: ascending, row code deletes a row still to visit (main's path, unchanged).
    [Test]
    procedure RowCodeDeletesLaterRowAscending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('delete-later-asc');
        Row.DeleteAll(true);
        Assert.AreEqual('T1,T2', Ctl.Take(), 'order');
    end;
}
