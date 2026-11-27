// Neighbors of the tree of corpus PR 1620 (ClearSharedCodeunitWhseTree.al, merged, not pinned) that the probe does not
// measure: a register List another variable shares, a temporary record of a node that read a row. The probe's nodes
// only insert into their temporary records and never hand out a List.

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

codeunit 86790 "CWR Ref Test"
{
    Subtype = Test;


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
    procedure Refused_TempRecordReadRow()
    var
        Owner: Codeunit "CWR Owner Temp Read";
    begin
        Owner.Prime();
        Owner.DoClearAll();
    end;
}
