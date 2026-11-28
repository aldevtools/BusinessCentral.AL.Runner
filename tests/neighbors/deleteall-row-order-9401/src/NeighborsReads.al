// Neighbors of the DeleteAll row order (al-runner issue 9401, corpus PR 1659), third set: row code that reads rows of a
// table. Corpus PR 1659 measures no such cell (its row code reads only its own row's field values), so a descending
// DeleteAll refuses with main's text once a row's code read a table: by a record read (Count, Get of the row itself, a read
// of another table) or without a record operation (a FlowField). Control: the same read on an ascending record.

// A FlowField that counts the rows of "DRN Row".
table 94025 "DRN Row Sum"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Rows; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("DRN Row");
        }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 94026 "DRN Neighbors Reads"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Ctl: Codeunit "DRN Ctl";
        Assert: Codeunit "DRN Assert";

    local procedure Seed(Rows: Integer)
    var
        Row: Record "DRN Row";
        Other: Record "DRN Other";
        i: Integer;
    begin
        Row.DeleteAll(false);
        Other.DeleteAll(false);
        for i := 1 to Rows do begin
            Row.Init();
            Row.Id := i;
            Row.Insert(false);
        end;
        Other.Id := 1;
        Other.Insert(false);
        Ctl.Take();
    end;

    // Row code gets the row being deleted.
    [Test]
    procedure RowCodeGetsOwnRowDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('read-get');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,gtrue,T2,gtrue,T1,gtrue', Ctl.Take(), 'order');
    end;

    // Row code reads the table through a FlowField of another table.
    [Test]
    procedure RowCodeFlowFieldOverTableDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('read-flow');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,f3,T2,f2,T1,f1', Ctl.Take(), 'order');
    end;

    // Row code reads another table only: not measured either, refused with main's text.
    [Test]
    procedure RowCodeReadsOtherTableDescending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('read-other-table');
        Row.Ascending(false);
        Row.DeleteAll(true);
        Assert.AreEqual('T3,o1,T2,o1,T1,o1', Ctl.Take(), 'order');
    end;

    // Control: the same read of the table on an ascending record (main's row-by-row path, no watch).
    [Test]
    procedure RowCodeReadsTableAscending()
    var
        Row: Record "DRN Row";
    begin
        Seed(3);
        Ctl.SetMode('read-count');
        Row.DeleteAll(true);
        Assert.AreEqual('T1,c3,T2,c2,T3,c1', Ctl.Take(), 'order');
    end;
}
