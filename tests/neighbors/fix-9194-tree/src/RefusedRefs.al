// Neighbors of the tree of corpus PR 1620 (ClearSharedCodeunitWhseTree.al, merged, not pinned) that the probe does not
// measure: a RecordRef of a node that read a row, a register List another variable shares, a node RecordRef another
// variable shares, a node FieldRef another variable shares (on the same record as the node's RecordRef), a temporary record of a node that read a row, an array global of a node. The probe's nodes only open their RecordRef and only
// insert into their temporary records, and never hand out a List or a RecordRef.

codeunit 86779 "CWR Node RR Read"
{
    var
        RR: RecordRef;

    procedure Set()
    begin
        RR.Open(Database::"CKK Row");
        if RR.FindFirst() then;
    end;
}

codeunit 86780 "CWR Reg RR Read"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        N: Codeunit "CWR Node RR Read";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        N.Set();
    end;
}

codeunit 86781 "CWR Owner RR Read"
{
    var
        Reg: Codeunit "CWR Reg RR Read";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86782 "CWR Reg List"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        L: List of [Integer];

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        L.Add(1);
    end;

    procedure GetList(var X: List of [Integer])
    begin
        X := L;
    end;
}

codeunit 86783 "CWR Owner List"
{
    var
        Reg: Codeunit "CWR Reg List";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure GetList(var X: List of [Integer])
    begin
        Reg.GetList(X);
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86784 "CWR Node RR Alias"
{
    var
        RR: RecordRef;

    procedure Set()
    begin
        RR.Open(Database::"CKK Row");
    end;

    procedure GetRR(var X: RecordRef)
    begin
        X := RR;
    end;
}

codeunit 86785 "CWR Reg RR Alias"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        N: Codeunit "CWR Node RR Alias";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        N.Set();
    end;

    procedure GetRR(var X: RecordRef)
    begin
        N.GetRR(X);
    end;
}

codeunit 86786 "CWR Owner RR Alias"
{
    var
        Reg: Codeunit "CWR Reg RR Alias";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure GetRR(var X: RecordRef)
    begin
        Reg.GetRR(X);
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86791 "CWR Node FR Alias"
{
    var
        RR: RecordRef;

    procedure Set()
    begin
        RR.Open(Database::"CKK Row");
    end;

    procedure GetFR(var X: FieldRef)
    begin
        X := RR.Field(1);
    end;
}

codeunit 86792 "CWR Reg FR Alias"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        N: Codeunit "CWR Node FR Alias";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        N.Set();
    end;

    procedure GetFR(var X: FieldRef)
    begin
        N.GetFR(X);
    end;
}

codeunit 86793 "CWR Owner FR Alias"
{
    var
        Reg: Codeunit "CWR Reg FR Alias";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure GetFR(var X: FieldRef)
    begin
        Reg.GetFR(X);
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86787 "CWR Node Temp Read"
{
    var
        T: Record "CKK Row" temporary;

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        T.FindFirst();
    end;
}

codeunit 86788 "CWR Reg Temp Read"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        N: Codeunit "CWR Node Temp Read";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        N.Set();
    end;
}

codeunit 86789 "CWR Owner Temp Read"
{
    var
        Reg: Codeunit "CWR Reg Temp Read";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86794 "CWR Node Array"
{
    var
        A: array[2] of Integer;

    procedure Set()
    begin
        A[1] := 5;
    end;
}

codeunit 86795 "CWR Reg Array"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        N: Codeunit "CWR Node Array";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        N.Set();
    end;
}

codeunit 86796 "CWR Owner Array"
{
    var
        Reg: Codeunit "CWR Reg Array";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 86790 "CWR Ref Test"
{
    Subtype = Test;

    [Test]
    procedure Refused_RecordRefReadRow()
    var
        Owner: Codeunit "CWR Owner RR Read";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Row.Insert();
        Owner.Prime();
        Owner.DoClearAll();
    end;

    [Test]
    procedure Refused_RegisterListAliased()
    var
        Owner: Codeunit "CWR Owner List";
        X: List of [Integer];
    begin
        Owner.Prime();
        Owner.GetList(X);
        Owner.DoClearAll();
        if X.Count() <> 1 then
            Error('count %1', X.Count());
    end;

    [Test]
    procedure Refused_NodeRecordRefAliased()
    var
        Owner: Codeunit "CWR Owner RR Alias";
        X: RecordRef;
    begin
        Owner.Prime();
        Owner.GetRR(X);
        Owner.DoClearAll();
        if X.Number() <> Database::"CKK Row" then
            Error('number %1', X.Number());
    end;

    [Test]
    procedure Refused_NodeFieldRefAliased()
    var
        Owner: Codeunit "CWR Owner FR Alias";
        X: FieldRef;
    begin
        Owner.Prime();
        Owner.GetFR(X);
        Owner.DoClearAll();
        if X.Number() <> 1 then
            Error('field %1', X.Number());
    end;

    [Test]
    procedure Refused_TempRecordReadRow()
    var
        Owner: Codeunit "CWR Owner Temp Read";
    begin
        Owner.Prime();
        Owner.DoClearAll();
    end;

    [Test]
    procedure Refused_NodeArrayGlobal()
    var
        Owner: Codeunit "CWR Owner Array";
    begin
        Owner.Prime();
        Owner.DoClearAll();
    end;
}
