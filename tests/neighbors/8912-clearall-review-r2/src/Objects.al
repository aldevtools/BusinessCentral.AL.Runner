// Neighbors of al-runner issue 8912, review round 2 of the ClearAll change: the register a setter shares, arrays of
// temporary records shared by Copy(x, true), arrays of two and three dimensions. Stand-in objects, invented code.
table 50500 "RR Row"
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

codeunit 50501 "RR Nested"
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
codeunit 50502 "RR Register"
{
    TableNo = "RR Row";

    var
        Rows: Record "RR Row" temporary;
        Rec: Record "RR Row";
        Dec: Decimal;
        Flag: Boolean;
        Cd: Code[20];
        Nested: Codeunit "RR Nested";

    trigger OnRun()
    begin
    end;

    procedure Mark()
    begin
        Dec := 1.5;
        Flag := true;
        Cd := 'MARK';
        Rec.SetRange(No, 7);
        Rec.No := 9;
        Nested.Set(4);
    end;

    procedure WriteRow(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
    end;

    procedure ReadRec()
    begin
        if Rec.FindFirst() then;
    end;

    procedure ShareRows(var R: Record "RR Row" temporary)
    begin
        R.Copy(Rows, true);
    end;

    procedure ShareNested(var N: Codeunit "RR Nested")
    begin
        N := Nested;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 dec=%2 flag=%3 code=%4 filter=%5 no=%6 nested=%7',
            Rows.Count(), Format(Dec, 0, 9), Flag, Cd, Rec.GetFilters(), Rec.No, Nested.Get()));
    end;
}

// The owner: TableNo, a setter takes the caller's register, OnRun runs ClearAll.
codeunit 50503 "RR TableNo Owner"
{
    TableNo = "RR Row";

    var
        Reg: Codeunit "RR Register";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetRegister(var NewReg: Codeunit "RR Register")
    begin
        Reg := NewReg;
    end;

    procedure ShareReg(var Y: Codeunit "RR Register")
    begin
        Y := Reg;
    end;

    procedure WriteReg(V: Integer)
    begin
        Reg.WriteRow(V);
    end;

    procedure ShareRegRows(var R: Record "RR Row" temporary)
    begin
        Reg.ShareRows(R);
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;

    procedure StateReg(): Text
    begin
        exit(Reg.State());
    end;
}

codeunit 50504 "RR SI Owner"
{
    SingleInstance = true;

    var
        Reg: Codeunit "RR Register";

    procedure SetRegister(var NewReg: Codeunit "RR Register")
    begin
        Reg := NewReg;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

// The caller: holds the register, hands it to the owner, runs the owner, or runs ClearAll itself.
codeunit 50505 "RR Caller"
{
    TableNo = "RR Row";

    var
        Reg: Codeunit "RR Register";
        Owner: Codeunit "RR TableNo Owner";

    trigger OnRun()
    begin
    end;

    procedure Prepare(Rows: Integer)
    begin
        MarkAndWrite(Rows);
        Owner.SetRegister(Reg);
    end;

    procedure MarkAndWrite(Rows: Integer)
    var
        I: Integer;
    begin
        Reg.Mark();
        for I := 1 to Rows do
            Reg.WriteRow(I);
    end;

    procedure HandTo(var O: Codeunit "RR TableNo Owner")
    begin
        O.SetRegister(Reg);
    end;

    procedure RunOwner()
    var
        Row: Record "RR Row";
    begin
        Owner.Run(Row);
    end;

    procedure ClearAllHere()
    begin
        ClearAll();
    end;

    procedure ShareRows(var R: Record "RR Row" temporary)
    begin
        Reg.ShareRows(R);
    end;

    procedure ShareNested(var N: Codeunit "RR Nested")
    begin
        Reg.ShareNested(N);
    end;

    procedure ReadRec()
    begin
        Reg.ReadRec();
    end;

    procedure HandToSi(var S: Codeunit "RR SI Owner")
    begin
        S.SetRegister(Reg);
    end;

    procedure CallerState(): Text
    begin
        exit(Reg.State());
    end;

    procedure OwnerState(): Text
    begin
        exit(Owner.StateReg());
    end;
}

// Arrays of temporary records as direct globals.
codeunit 50510 "RR Arr Owner"
{
    var
        A: array[2] of Record "RR Row" temporary;
        B: array[2, 2] of Record "RR Row" temporary;
        C: array[2, 2, 2] of Record "RR Row" temporary;

    procedure ShareA1(var R: Record "RR Row" temporary)
    begin
        R.Copy(A[1], true);
    end;

    procedure ShareA2(var R: Record "RR Row" temporary)
    begin
        R.Copy(A[2], true);
    end;

    procedure ShareB11(var R: Record "RR Row" temporary)
    begin
        R.Copy(B[1, 1], true);
    end;

    procedure WriteB(V: Integer)
    begin
        B[1, 1].No := V;
        B[1, 1].Insert();
    end;

    procedure WriteC(V: Integer)
    begin
        C[1, 1, 1].No := V;
        C[1, 1, 1].Insert();
    end;

    procedure CountA1(): Integer
    begin
        exit(A[1].Count());
    end;

    procedure CountC(): Integer
    begin
        exit(C[2, 2, 2].Count());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50511 "RR Arr Normal Owner"
{
    Subtype = Normal;

    var
        A: array[2] of Record "RR Row" temporary;

    procedure SharedClear(): Integer
    var
        R: Record "RR Row" temporary;
    begin
        R.Copy(A[1], true);
        ClearAll();
        R.No := 1;
        R.Insert();
        exit(A[1].Count());
    end;
}

codeunit 50512 "RR Arr SI Owner"
{
    SingleInstance = true;

    var
        A: array[2] of Record "RR Row" temporary;

    procedure WrittenClear(): Integer
    begin
        A[1].No := 1;
        A[1].Insert();
        ClearAll();
        exit(A[2].Count());
    end;
}

// A codeunit that keeps a record sharing another codeunit's array table in its own global.
codeunit 50513 "RR Keeper"
{
    var
        Kept: Record "RR Row" temporary;

    procedure Keep(var O: Codeunit "RR Arr Owner")
    begin
        O.ShareA1(Kept);
    end;

    procedure InsertKept(V: Integer)
    begin
        Kept.No := V;
        Kept.Insert();
    end;
}
