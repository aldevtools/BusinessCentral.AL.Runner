// Neighbors of the tree of corpus PR 1620 (ClearSharedCodeunitWhseTree.al, merged, not pinned) that bulk-write rows: a
// node's temporary record that inserts a row and then deletes all rows, and a node's RecordRef that deletes all rows of
// its table. The gate of clear-all-register-tree reads only the state of each variable, which these writes leave as it
// is, so both run flagged. AL has no ModifyAll on a RecordRef, so DeleteAll is its only bulk write.

codeunit 86797 "CWR Node Temp DeleteAll"
{
    var
        T: Record "CKK Row" temporary;

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        T.DeleteAll();
    end;

    procedure TempCount(): Integer
    begin
        exit(T.Count());
    end;
}

codeunit 86798 "CWR Reg Temp DeleteAll"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        N: Codeunit "CWR Node Temp DeleteAll";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        N.Set();
    end;

    procedure NodeTempCount(): Integer
    begin
        exit(N.TempCount());
    end;
}

codeunit 86799 "CWR Owner Temp DeleteAll"
{
    var
        Reg: Codeunit "CWR Reg Temp DeleteAll";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure NodeTempCount(): Integer
    begin
        exit(Reg.NodeTempCount());
    end;
}

codeunit 86800 "CWR Node RR DeleteAll"
{
    var
        RR: RecordRef;

    procedure Set()
    begin
        RR.Open(Database::"CKK Row");
        RR.DeleteAll();
    end;
}

codeunit 86801 "CWR Reg RR DeleteAll"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        N: Codeunit "CWR Node RR DeleteAll";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        N.Set();
    end;
}

codeunit 86802 "CWR Owner RR DeleteAll"
{
    var
        Reg: Codeunit "CWR Reg RR DeleteAll";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86803 "CWR Bulk Test"
{
    Subtype = Test;

    [Test]
    procedure Assumed_TempDeleteAll()
    var
        Owner: Codeunit "CWR Owner Temp DeleteAll";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        if Owner.NodeTempCount() <> 0 then
            Error('temp rows %1', Owner.NodeTempCount());
    end;

    [Test]
    procedure Assumed_RecordRefDeleteAll()
    var
        Owner: Codeunit "CWR Owner RR DeleteAll";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Row.Insert();
        Owner.Prime();
        Owner.DoClearAll();
        if not Row.IsEmpty() then
            Error('rows left');
    end;
}
