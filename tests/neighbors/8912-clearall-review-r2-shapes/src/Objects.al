table 50600 "PS Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

enum 50601 "PS Kind"
{
    value(0; None) { }
    value(1; Alpha) { }
    value(2; Beta) { }
}

interface "PS Line"
{
    procedure SetLine();
}

codeunit 50602 "PS Store"
{
    var
        T1: Record "PS Row" temporary;
        Kind: Enum "PS Kind";
        Int: Integer;
        LblC: Label 'store label', Comment = 'a comment';

    procedure Mark()
    begin
        Kind := Kind::Beta;
        Int := 7;
    end;

    procedure WriteT1(V: Integer)
    begin
        T1.No := V;
        T1.Insert();
    end;

    procedure EmptyT1()
    begin
        T1.DeleteAll();
    end;

    procedure ShareT1(var R: Record "PS Row" temporary)
    begin
        R.Copy(T1, true);
    end;

    procedure SetInt(V: Integer)
    begin
        Int := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('t1=%1 kind=%2 int=%3 lc=%4', T1.Count(), Kind, Int, LblC));
    end;
}

codeunit 50603 "PS Plain Label Store"
{
    var
        T1: Record "PS Row" temporary;
        Kind: Enum "PS Kind";
        Int: Integer;
        Lbl: Label 'store label';

    procedure WriteT1(V: Integer)
    begin
        T1.No := V;
        T1.Insert();
    end;
}

codeunit 50604 "PS More Store"
{
    var
        T1: Record "PS Row" temporary;
        Kind: Enum "PS Kind";
        Int: Integer;
        LblC: Label 'store label', Comment = 'a comment';
        D: Decimal;

    procedure WriteT1(V: Integer)
    begin
        T1.No := V;
        T1.Insert();
    end;
}

codeunit 50605 "PS Owner" implements "PS Line"
{
    var
        S: Codeunit "PS Store";
        Cnt: Integer;
        LineNo: Integer;

    procedure SetLine()
    begin
        ClearAll();
        LineNo := 1;
    end;

    procedure Prepare()
    begin
        S.Mark();
        S.WriteT1(1);
        S.WriteT1(2);
        Cnt := 5;
    end;

    procedure State(): Text
    begin
        exit(S.State() + ' cnt=' + Format(Cnt) + ' line=' + Format(LineNo));
    end;

    procedure SetInt(V: Integer)
    begin
        S.SetInt(V);
    end;

    procedure GetStore(var X: Codeunit "PS Store")
    begin
        X := S;
    end;

    procedure WriteT1(V: Integer)
    begin
        S.WriteT1(V);
    end;

    procedure EmptyT1()
    begin
        S.EmptyT1();
    end;

    procedure ShareT1(var R: Record "PS Row" temporary)
    begin
        S.ShareT1(R);
    end;
}

codeunit 50606 "PS SI Owner"
{
    SingleInstance = true;

    var
        S: Codeunit "PS Store";

    procedure WriteAndClear()
    begin
        S.WriteT1(1);
        ClearAll();
    end;
}

codeunit 50607 "PS TableNo Owner"
{
    TableNo = "PS Row";

    var
        S: Codeunit "PS Store";

    trigger OnRun()
    begin
        S.WriteT1(1);
        ClearAll();
    end;
}

codeunit 50608 "PS Other Owner"
{
    var
        A: Codeunit "PS Plain Label Store";
        B: Codeunit "PS More Store";

    procedure PlainLabel()
    begin
        A.WriteT1(1);
        ClearAll();
    end;

    procedure More()
    begin
        B.WriteT1(1);
        ClearAll();
    end;
}
