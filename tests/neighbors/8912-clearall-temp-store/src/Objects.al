// Neighbors of al-runner issue 8912: ClearAll of a store codeunit whose temporary table holds rows, and of an array of
// temporary records (the shapes of corpus PR 1561 and the ones next to them). Each test expects the rows to stay, as BC
// does in the measured shapes.
table 50100 "CT Row"
{
    fields
    {
        field(1; No; Integer) { }
    }
    keys
    {
        key(PK; No) { }
    }
}

codeunit 50101 "CT Store"
{
    var
        Rows: Record "CT Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;

    procedure Write(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
        if Refs[1, 1].Number = 0 then
            Refs[1, 1].Open(Database::"CT Row", true);
        Refs[1, 1].Init();
        Refs[1, 1].Field(1).Value := V;
        Refs[1, 1].Insert();
        Ints[1] := V;
    end;

    procedure SetInt(V: Integer)
    begin
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 ref11=%2 int1=%3', Rows.Count(), Refs[1, 1].Number, Ints[1]));
    end;
}

codeunit 50102 "CT Store Extra"
{
    var
        Rows: Record "CT Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;
        N: Integer;

    procedure Write(V: Integer)
    begin
        Rows.No := V;
        Rows.Insert();
    end;

    procedure RowCount(): Integer
    begin
        exit(Rows.Count());
    end;
}

codeunit 50103 "CT Wrap"
{
    var
        Inner: Codeunit "CT Store";

    procedure Write(V: Integer)
    begin
        Inner.Write(V);
    end;

    procedure State(): Text
    begin
        exit(Inner.State());
    end;
}

codeunit 50104 "CT SI Owner"
{
    SingleInstance = true;

    var
        X: Codeunit "CT Store";
        Cnt: Integer;

    procedure WriteX(V: Integer)
    begin
        X.Write(V);
    end;

    procedure StateX(): Text
    begin
        exit(X.State() + ' cnt=' + Format(Cnt));
    end;

    procedure SetCnt(V: Integer)
    begin
        Cnt := V;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50105 "CT Holder"
{
    var
        X: Codeunit "CT Store";
        E: Codeunit "CT Store Extra";
        W: Codeunit "CT Wrap";
        A: array[2] of Codeunit "CT Store";

    procedure WriteX(V: Integer)
    begin
        X.Write(V);
    end;

    procedure SetIntX(V: Integer)
    begin
        X.SetInt(V);
    end;

    procedure StateX(): Text
    begin
        exit(X.State());
    end;

    procedure WriteE()
    begin
        E.Write(1);
    end;

    procedure CountE(): Integer
    begin
        exit(E.RowCount());
    end;

    procedure WriteW()
    begin
        W.Write(1);
    end;

    procedure StateW(): Text
    begin
        exit(W.State());
    end;

    procedure WriteA()
    begin
        A[1].Write(1);
    end;

    procedure StateA(): Text
    begin
        exit(A[1].State());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50106 "CT Normal Owner"
{
    Subtype = Normal;

    var
        GX: Codeunit "CT Store";

    procedure GlobalWritten(): Text
    begin
        GX.Write(3);
        ClearAll();
        exit(GX.State());
    end;
}

report 50107 "CT Report Owner"
{
    ProcessingOnly = true;

    var
        GX: Codeunit "CT Store";
        Rows: array[2] of Record "CT Row" temporary;

    procedure GlobalWritten(): Text
    begin
        GX.Write(3);
        ClearAll();
        exit(GX.State());
    end;

    procedure ArrayWritten(): Integer
    begin
        Rows[1].No := 1;
        Rows[1].Insert();
        ClearAll();
        exit(Rows[2].Count());
    end;
}

codeunit 50108 "CT Array Owner"
{
    var
        Rows: array[2] of Record "CT Row" temporary;

    procedure ArrayWritten(): Integer
    begin
        Rows[1].No := 1;
        Rows[1].Insert();
        ClearAll();
        exit(Rows[2].Count());
    end;
}
