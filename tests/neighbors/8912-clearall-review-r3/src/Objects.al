// Neighbors of al-runner issue 8912, review round 3 of the ClearAll change: a temporary table that a record outside shares
// by something other than Copy(x, true) (a RecordRef's SetTable or Copy with ShareTable, two Copy targets, a Copy of a Copy target), next to the Copy(x, true) share corpus PR
// 1564 measures. Stand-in objects, invented code.
table 50600 "R3 Row"
{
    fields
    {
        field(1; No; Integer) { }
    }
    keys
    {
        key(PK; No) { Clustered = true; }
    }
}

codeunit 50601 "R3 Nested"
{
    var
        N: Integer;

    procedure Set(V: Integer)
    begin
        N := V;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

// The register shape: TableNo, then a temporary record, a record, a Decimal, a Boolean, a Code and a one-Integer codeunit.
codeunit 50602 "R3 Register"
{
    TableNo = "R3 Row";

    var
        Rows: Record "R3 Row" temporary;
        Rec: Record "R3 Row";
        Dec: Decimal;
        Flag: Boolean;
        Cd: Code[20];
        Nested: Codeunit "R3 Nested";

    trigger OnRun()
    begin
    end;

    procedure Mark()
    begin
        Dec := 1.5;
        Flag := true;
        Cd := 'MARK';
        Nested.Set(4);
    end;

    procedure WriteRow(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
    end;

    procedure ShareRows(var R: Record "R3 Row" temporary)
    begin
        R.Copy(Rows, true);
    end;

    procedure ShareRowsByRef(var R: Record "R3 Row" temporary)
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(Rows);
        RecRef.SetTable(R, true);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 dec=%2 flag=%3 code=%4 nested=%5',
            Rows.Count(), Format(Dec, 0, 9), Flag, Cd, Nested.Get()));
    end;
}

// The owner: TableNo, a setter takes the caller's register, OnRun runs ClearAll.
codeunit 50603 "R3 TableNo Owner"
{
    TableNo = "R3 Row";

    var
        Reg: Codeunit "R3 Register";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetRegister(var NewReg: Codeunit "R3 Register")
    begin
        Reg := NewReg;
    end;
}

codeunit 50604 "R3 Caller"
{
    TableNo = "R3 Row";

    var
        Reg: Codeunit "R3 Register";
        Owner: Codeunit "R3 TableNo Owner";

    trigger OnRun()
    begin
    end;

    procedure Prepare(Rows: Integer)
    var
        I: Integer;
    begin
        Reg.Mark();
        for I := 1 to Rows do
            Reg.WriteRow(I);
        Owner.SetRegister(Reg);
    end;

    procedure RunOwner()
    var
        Row: Record "R3 Row";
    begin
        Owner.Run(Row);
    end;

    procedure ShareRows(var R: Record "R3 Row" temporary)
    begin
        Reg.ShareRows(R);
    end;

    procedure ShareRowsByRef(var R: Record "R3 Row" temporary)
    begin
        Reg.ShareRowsByRef(R);
    end;

    procedure CallerState(): Text
    begin
        exit(Reg.State());
    end;
}

// A SingleInstance owner of an array of temporary records of one dimension.
codeunit 50610 "R3 Arr SI Owner"
{
    SingleInstance = true;

    var
        A: array[2] of Record "R3 Row" temporary;
        Cnt: Integer;

    procedure Write(V: Integer)
    begin
        A[1].No := V;
        A[1].Insert();
        Cnt := V;
    end;

    procedure ShareByCopy(var R: Record "R3 Row" temporary)
    begin
        R.Copy(A[1], true);
    end;

    procedure ShareByRef(var R: Record "R3 Row" temporary)
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(A[1]);
        RecRef.SetTable(R, true);
    end;

    procedure ShareByTwoCopies(var R1: Record "R3 Row" temporary; var R2: Record "R3 Row" temporary)
    begin
        R1.Copy(A[1], true);
        R2.Copy(A[1], true);
    end;

    procedure ShareByChainedCopy(var R2: Record "R3 Row" temporary)
    var
        R1: Record "R3 Row" temporary;
    begin
        R1.Copy(A[1], true);
        R2.Copy(R1, true);
    end;

    procedure ShareByRecordRefCopy(var Other: RecordRef)
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(A[1]);
        Other.Open(Database::"R3 Row", true);
        Other.Copy(RecRef, true);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('a1=%1 a2=%2 cnt=%3', A[1].Count(), A[2].Count(), Cnt));
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;

    procedure Reset()
    begin
        A[1].DeleteAll();
        Cnt := 0;
    end;
}
