// Neighbors of the TableNo register shape of corpus PR 1596 (ClearSharedCodeunitTableNoRegister.al): each changes one
// fact of the measured register, or holds it in a cell the probe does not measure. Every test expects the refusal.

codeunit 86413 "NKT TNo Leaf"
{
    TableNo = "CKK Row";

    var
        N: Integer;

    trigger OnRun()
    begin
    end;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;
}

// The register holds a nested codeunit with a TableNo of its own.
codeunit 86414 "NKT Reg Nested TNo"
{
    TableNo = "CKK Row";

    var
        Leaf: Codeunit "NKT TNo Leaf";
        Entry: Integer;

    trigger OnRun()
    begin
    end;

    procedure Prime()
    begin
        Leaf.SetN(3);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;
}

// The register's record read a row before the share.
codeunit 86415 "NKT Reg Row IO"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Entry: Integer;

    trigger OnRun()
    begin
    end;

    procedure Prime()
    begin
        if Location.FindFirst() then;
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;
}

// The register holds a temporary record, a kind neither probe measures in a register with a TableNo.
codeunit 86416 "NKT Reg Temp"
{
    TableNo = "CKK Row";

    var
        Buffer: Record "CKK Row" temporary;
        Entry: Integer;

    trigger OnRun()
    begin
    end;

    procedure Prime()
    begin
        Buffer."No." := 4;
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;
}

codeunit 86417 "NKT Shared Leaf"
{
    var
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

// The register's nested codeunit is shared by a variable of the caller too.
codeunit 86418 "NKT Reg Shared Nested"
{
    TableNo = "CKK Row";

    var
        Leaf: Codeunit "NKT Shared Leaf";
        Entry: Integer;

    trigger OnRun()
    begin
    end;

    procedure Prime(var Out: Codeunit "NKT Shared Leaf")
    begin
        Leaf.SetN(3);
        Out := Leaf;
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;
}

// A chain one level deeper than the probe's four: "CKT M1" holds three more below it.
codeunit 86419 "NKT Chain Top"
{
    var
        Chain: Codeunit "CKT M1";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Chain.SetN(V);
    end;
}

codeunit 86420 "NKT Reg Deep5"
{
    TableNo = "CKK Row";

    var
        Top: Codeunit "NKT Chain Top";
        Entry: Integer;

    trigger OnRun()
    begin
    end;

    procedure Prime()
    begin
        Top.SetN(2);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;
}

// A back link two levels below the register: the probe's cycle has it one level below.
codeunit 86421 "NKT Cyc Inner"
{
    var
        Back: Codeunit "NKT Reg Cyc2";
        I: Integer;

    procedure Link(var R: Codeunit "NKT Reg Cyc2")
    begin
        Back := R;
        I := 5;
    end;
}

codeunit 86422 "NKT Cyc Outer"
{
    var
        Inner: Codeunit "NKT Cyc Inner";
        I: Integer;

    procedure Link(var R: Codeunit "NKT Reg Cyc2")
    begin
        Inner.Link(R);
        I := 4;
    end;
}

codeunit 86423 "NKT Reg Cyc2"
{
    TableNo = "CKK Row";

    var
        Outer: Codeunit "NKT Cyc Outer";
        Entry: Integer;

    trigger OnRun()
    begin
    end;

    procedure Prime(var Self: Codeunit "NKT Reg Cyc2")
    begin
        Outer.Link(Self);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;
}

// One owner per register: a setter, then ClearAll and a read of the register.
codeunit 86424 "NKT Owner"
{
    TableNo = "CKK Row";

    var
        Nested: Codeunit "NKT Reg Nested TNo";
        RowIO: Codeunit "NKT Reg Row IO";
        Temp: Codeunit "NKT Reg Temp";
        SharedNested: Codeunit "NKT Reg Shared Nested";
        Deep5: Codeunit "NKT Reg Deep5";
        Cyc2: Codeunit "NKT Reg Cyc2";
        Which: Integer;
        After: Integer;

    trigger OnRun()
    var
        W: Integer;
    begin
        W := Which;
        ClearAll();
        case W of
            1:
                After := Nested.Peek();
            2:
                After := RowIO.Peek();
            3:
                After := Temp.Peek();
            4:
                After := SharedNested.Peek();
            5:
                After := Deep5.Peek();
            6:
                After := Cyc2.Peek();
        end;
    end;

    procedure SetNested(var R: Codeunit "NKT Reg Nested TNo")
    begin
        Nested := R;
        Which := 1;
    end;

    procedure SetRowIO(var R: Codeunit "NKT Reg Row IO")
    begin
        RowIO := R;
        Which := 2;
    end;

    procedure SetTemp(var R: Codeunit "NKT Reg Temp")
    begin
        Temp := R;
        Which := 3;
    end;

    procedure SetSharedNested(var R: Codeunit "NKT Reg Shared Nested")
    begin
        SharedNested := R;
        Which := 4;
    end;

    procedure SetDeep5(var R: Codeunit "NKT Reg Deep5")
    begin
        Deep5 := R;
        Which := 5;
    end;

    procedure SetCyc2(var R: Codeunit "NKT Reg Cyc2")
    begin
        Cyc2 := R;
        Which := 6;
    end;

    procedure GetAfter(): Integer
    begin
        exit(After);
    end;
}

codeunit 86425 "NKT Neighbor Tests"
{
    Subtype = Test;

    [Test]
    procedure NestedTableNo_Primed()
    var
        Reg: Codeunit "NKT Reg Nested TNo";
        Owner: Codeunit "NKT Owner";
        Row: Record "CKK Row";
    begin
        Reg.Prime();
        Owner.SetNested(Reg);
        Owner.Run(Row);
        if Owner.GetAfter() <> 0 then Error('owner after %1', Owner.GetAfter());
        if Reg.Peek() <> 0 then Error('caller after %1', Reg.Peek());
    end;

    [Test]
    procedure RecordReadRow_Primed()
    var
        Reg: Codeunit "NKT Reg Row IO";
        Owner: Codeunit "NKT Owner";
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Row."No." := 1;
        Row.Insert();
        Reg.Prime();
        Owner.SetRowIO(Reg);
        Owner.Run(Row);
        if Reg.Peek() <> 0 then Error('caller after %1', Reg.Peek());
    end;

    [Test]
    procedure TempRecord_Primed()
    var
        Reg: Codeunit "NKT Reg Temp";
        Owner: Codeunit "NKT Owner";
        Row: Record "CKK Row";
    begin
        Reg.Prime();
        Owner.SetTemp(Reg);
        Owner.Run(Row);
        if Reg.Peek() <> 0 then Error('caller after %1', Reg.Peek());
    end;

    [Test]
    procedure NestedSharedByCaller_Primed()
    var
        Reg: Codeunit "NKT Reg Shared Nested";
        Leaf: Codeunit "NKT Shared Leaf";
        Owner: Codeunit "NKT Owner";
        Row: Record "CKK Row";
    begin
        Reg.Prime(Leaf);
        Owner.SetSharedNested(Reg);
        Owner.Run(Row);
        if Leaf.Get() <> 0 then Error('leaf after %1', Leaf.Get());
    end;

    [Test]
    procedure ChainDepthFive_Primed()
    var
        Reg: Codeunit "NKT Reg Deep5";
        Owner: Codeunit "NKT Owner";
        Row: Record "CKK Row";
    begin
        Reg.Prime();
        Owner.SetDeep5(Reg);
        Owner.Run(Row);
        if Reg.Peek() <> 0 then Error('caller after %1', Reg.Peek());
    end;

    [Test]
    procedure BackLinkDepthTwo_Primed()
    var
        Reg: Codeunit "NKT Reg Cyc2";
        Owner: Codeunit "NKT Owner";
        Row: Record "CKK Row";
    begin
        Reg.Prime(Reg);
        Owner.SetCyc2(Reg);
        Owner.Run(Row);
        if Reg.Peek() <> 0 then Error('caller after %1', Reg.Peek());
    end;

    [Test]
    procedure PlainOwner_Peeked()
    var
        Reg: Codeunit "CKT Reg";
        Owner: Codeunit "CKT Plain Owner";
        Row: Record "CKK Row";
    begin
        if Reg.Peek() <> 0 then Error('peek');
        Owner.SetReg(Reg);
        Owner.SetSuppress(true);
        Owner.Run(Row);
        if Reg.Peek() <> 1 then Error('caller after %1', Reg.Peek());
    end;
}
