// Neighbors of the tree of open corpus PR 1620 (ClearSharedCodeunitWhseTree.al) that the probe does not measure: a Time
// global in a node, a node four levels below the register, a TableNo below the register, a register another variable
// shares, an owner with a TableNo, a plain owner with a second global, a plain owner holding a node that no register
// declares. One reduced tree inside the measured shape ("CWR Owner Ok").
codeunit 86760 "CWR Leaf"
{
    var
        T: Record "CKK Row" temporary;
        I: Integer;

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        I := 1;
    end;

    procedure State(): Integer
    begin
        exit(T.Count() * 10 + I);
    end;
}

codeunit 86761 "CWR Reg Time"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        Tm: Time;
        P: Codeunit "CWR Leaf";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        Tm := 120000T;
        P.Set();
    end;

    procedure State(): Integer
    begin
        exit(T.Count() * 100 + P.State());
    end;
}

codeunit 86762 "CWR Owner Time"
{
    var
        Reg: Codeunit "CWR Reg Time";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Reg.State());
    end;
}

codeunit 86763 "CWR D3"
{
    var
        I: Integer;
        P: Codeunit "CWR Leaf";

    procedure Set()
    begin
        I := 1;
        P.Set();
    end;

    procedure State(): Integer
    begin
        exit(I * 100 + P.State());
    end;
}

codeunit 86764 "CWR D2"
{
    var
        P: Codeunit "CWR D3";

    procedure Set()
    begin
        P.Set();
    end;

    procedure State(): Integer
    begin
        exit(P.State());
    end;
}

codeunit 86765 "CWR D1"
{
    var
        P: Codeunit "CWR D2";

    procedure Set()
    begin
        P.Set();
    end;

    procedure State(): Integer
    begin
        exit(P.State());
    end;
}

codeunit 86766 "CWR Reg Deep"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        P: Codeunit "CWR D1";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        P.Set();
    end;

    procedure State(): Integer
    begin
        exit(T.Count() * 1000 + P.State());
    end;
}

codeunit 86767 "CWR Owner Deep"
{
    var
        Reg: Codeunit "CWR Reg Deep";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Reg.State());
    end;
}

codeunit 86768 "CWR Node TableNo"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        I: Integer;

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        I := 1;
    end;

    procedure State(): Integer
    begin
        exit(T.Count() * 10 + I);
    end;
}

codeunit 86769 "CWR Reg Node TableNo"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        P: Codeunit "CWR Node TableNo";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        P.Set();
    end;

    procedure State(): Integer
    begin
        exit(T.Count() * 100 + P.State());
    end;
}

codeunit 86770 "CWR Owner Node TableNo"
{
    var
        Reg: Codeunit "CWR Reg Node TableNo";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Reg.State());
    end;
}

codeunit 86771 "CWR Reg Ok"
{
    TableNo = "CKK Row";

    var
        T: Record "CKK Row" temporary;
        I: Integer;
        P: Codeunit "CWR Leaf";

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        I := 1;
        P.Set();
    end;

    procedure State(): Integer
    begin
        exit(T.Count() * 1000 + I * 100 + P.State());
    end;
}

codeunit 86772 "CWR Owner Share"
{
    var
        Reg: Codeunit "CWR Reg Ok";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure GetReg(var Other: Codeunit "CWR Reg Ok")
    begin
        Other := Reg;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Reg.State());
    end;
}

codeunit 86773 "CWR Owner TableNo"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CWR Reg Ok";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Reg.State());
    end;
}

codeunit 86774 "CWR Owner Ok"
{
    var
        Reg: Codeunit "CWR Reg Ok";

    procedure Prime()
    begin
        Reg.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Reg.State());
    end;
}

codeunit 86776 "CWR Owner Second"
{
    var
        Reg: Codeunit "CWR Reg Ok";
        N: Integer;

    procedure Prime()
    begin
        Reg.Set();
        N := 1;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Reg.State() + N);
    end;
}

codeunit 86777 "CWR Loose Node"
{
    var
        T: Record "CKK Row" temporary;
        I: Integer;
        L: Label 'll', Locked = true;

    procedure Set()
    begin
        T."No." := 1;
        T.Insert();
        I := 1;
    end;

    procedure State(): Integer
    begin
        exit(T.Count() * 10 + I);
    end;
}

codeunit 86778 "CWR Owner Loose"
{
    var
        Node: Codeunit "CWR Loose Node";

    procedure Prime()
    begin
        Node.Set();
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Integer
    begin
        exit(Node.State());
    end;
}

codeunit 86775 "CWR Test"
{
    Subtype = Test;

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_TimeGlobal()
    var
        Owner: Codeunit "CWR Owner Time";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        Check(Owner.State(), 110);
    end;

    [Test]
    procedure Refused_DepthFour()
    var
        Owner: Codeunit "CWR Owner Deep";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        Check(Owner.State(), 1010);
    end;

    [Test]
    procedure Refused_TableNoBelowRegister()
    var
        Owner: Codeunit "CWR Owner Node TableNo";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        Check(Owner.State(), 110);
    end;

    [Test]
    procedure Refused_SharedRegister()
    var
        Owner: Codeunit "CWR Owner Share";
        Other: Codeunit "CWR Reg Ok";
    begin
        Owner.Prime();
        Owner.GetReg(Other);
        Owner.DoClearAll();
        Check(Other.State(), 1010);
    end;

    [Test]
    procedure Refused_TableNoOwner()
    var
        Owner: Codeunit "CWR Owner TableNo";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        Check(Owner.State(), 1010);
    end;

    [Test]
    procedure Refused_OwnerSecondGlobal()
    var
        Owner: Codeunit "CWR Owner Second";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        Check(Owner.State(), 1010);
    end;

    [Test]
    procedure Refused_NodeNoRegisterDeclares()
    var
        Owner: Codeunit "CWR Owner Loose";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        Check(Owner.State(), 10);
    end;

    [Test]
    procedure Assumed_ReducedTree()
    var
        Owner: Codeunit "CWR Owner Ok";
    begin
        Owner.Prime();
        Owner.DoClearAll();
        Check(Owner.State(), 1010);
    end;
}
