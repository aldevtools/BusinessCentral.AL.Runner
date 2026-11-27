table 64900 "N94 Row"
{
    fields
    {
        field(1; "No."; Integer) { }
    }
    keys
    {
        key(PK; "No.") { }
    }
}

codeunit 64901 "N94 Impl"
{
    SingleInstance = true;

    var
        Handled: Boolean;
        N: Integer;

    procedure Prime(V: Integer)
    begin
        Handled := true;
        N := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('impl[%1,%2]', Handled, N));
    end;
}

codeunit 64902 "N94 Confirm"
{
    SingleInstance = true;

    var
        Impl: Codeunit "N94 Impl";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        Impl.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('confirm[%1]', N) + Impl.State());
    end;
}

codeunit 64903 "N94 Mid"
{
    var
        Rows: Record "N94 Row";
        Confirm: Codeunit "N94 Confirm";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        Rows.SetRange("No.", V);
        Confirm.Prime(V + 10);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('mid[%1,%2]', N, Rows.GetFilter("No.")) + Confirm.State());
    end;
}

codeunit 64904 "N94 Reg"
{
    TableNo = "N94 Row";

    var
        Loc: Record "N94 Row";
        Mid: Codeunit "N94 Mid";
        EntryNo: Integer;
        Text000: Label 'Qty %1 short', Comment = '%1 = quantity';
        Text001: Label 'Serial %1 found';
        OnMove: Boolean;

    trigger OnRun()
    begin
        EntryNo := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        EntryNo := V;
        OnMove := true;
        Loc.SetRange("No.", V);
        Mid.Prime(V + 100);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('reg[%1,%2,%3,%4,%5]', EntryNo, OnMove, Loc.GetFilter("No."), StrSubstNo(Text000, 1), StrSubstNo(Text001, 2)) + Mid.State());
    end;
}

codeunit 64905 "N94 Owner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 Reg";
        Posted: Integer;

    trigger OnRun()
    begin
        Posted := Rec."No.";
    end;

    procedure SetReg(var R: Codeunit "N94 Reg")
    begin
        Reg := R;
        Posted := 7;
    end;

    procedure PrimeOwn(V: Integer)
    begin
        Reg.Prime(V);
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('owner[%1]', Posted) + Reg.State());
    end;
}

codeunit 64906 "N94 PlainOwner"
{
    var
        Reg: Codeunit "N94 Reg";

    procedure SetReg(var R: Codeunit "N94 Reg")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64907 "N94 KSi"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('ksi[%1]', N));
    end;
}

codeunit 64908 "N94 KReg"
{
    TableNo = "N94 Row";

    var
        Si: Codeunit "N94 KSi";
        N: Integer;
        T: Text;
        T30: Text[30];
        C20: Code[20];
        Dec: Decimal;
        Dt: Date;
        Big: BigInteger;
        Opt: Option A,B,C;
        Ch: Char;
        Flag: Boolean;
        LblC: Label 'comment %1', Comment = '%1 = n';
        LblP: Label 'plain %1';
        LblL: Label 'locked', Locked = true;

    trigger OnRun()
    begin
        N := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        T := 't' + Format(V);
        T30 := 'u' + Format(V);
        C20 := 'C' + Format(V);
        Dec := V + 0.5;
        Dt := DMY2Date(1, 2, 2020);
        Big := V * 1000;
        Opt := Opt::C;
        Ch := 65;
        Flag := true;
        Si.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('kreg[%1,%2,%3,%4,%5,%6,%7,%8,%9,', N, T, T30, C20, Format(Dec, 0, 9), Format(Dt, 0, 9), Big, Opt, Format(Ch))
            + StrSubstNo('%1,%2,%3,%4]', Flag, StrSubstNo(LblC, 1), StrSubstNo(LblP, 2), LblL) + Si.State());
    end;
}

codeunit 64909 "N94 KOwner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 KReg";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 KReg")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64910 "N94 D3"
{
    var
        Rows: Record "N94 Row";
        N: Integer;
        LblC: Label 'deep %1', Comment = '%1 = n';

    procedure Prime(V: Integer)
    begin
        N := V;
        Rows.SetRange("No.", V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d3[%1,%2,%3]', N, Rows.GetFilter("No."), StrSubstNo(LblC, 3)));
    end;
}

codeunit 64911 "N94 D2Si"
{
    SingleInstance = true;

    var
        D3: Codeunit "N94 D3";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        D3.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d2si[%1]', N) + D3.State());
    end;
}

codeunit 64912 "N94 D2P"
{
    var
        D3: Codeunit "N94 D3";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        D3.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d2p[%1]', N) + D3.State());
    end;
}

codeunit 64913 "N94 D1"
{
    var
        Si: Codeunit "N94 D2Si";
        P: Codeunit "N94 D2P";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        Si.Prime(V + 10);
        P.Prime(V + 20);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('d1[%1]', N) + Si.State() + P.State());
    end;
}

codeunit 64914 "N94 DReg"
{
    TableNo = "N94 Row";

    var
        D1: Codeunit "N94 D1";
        N: Integer;

    trigger OnRun()
    begin
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        D1.Prime(V + 100);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('dreg[%1]', N) + D1.State());
    end;
}

codeunit 64915 "N94 DOwner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 DReg";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 DReg")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}
