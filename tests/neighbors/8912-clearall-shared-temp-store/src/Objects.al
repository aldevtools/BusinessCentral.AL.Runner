// Neighbors of al-runner issue 8912: ClearAll in the owner of a store codeunit with temporary records whose instance another
// variable shares (the (owner, store) pairs of corpus PR 1561 and the ones next to them). Each test expects what BC does in
// the measured pairs: the owner's variable and the sharer stay on one instance, reset in place (rows kept, RecordRef
// closed, Integers 0).
table 50100 "CS Row"
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

codeunit 50101 "CS Store"
{
    var
        Rows: Record "CS Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;

    procedure Write(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
        if Refs[1, 1].Number = 0 then
            Refs[1, 1].Open(Database::"CS Row", true);
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 ref11=%2 int1=%3', Rows.Count(), Refs[1, 1].Number, Ints[1]));
    end;
}

codeunit 50102 "CS Arr Store"
{
    var
        Rows: array[2] of Record "CS Row" temporary;
        Refs: array[2] of RecordRef;
        Ints: array[3] of Integer;

    procedure Write(E: Integer; V: Integer)
    begin
        Rows[E].Init();
        Rows[E].No := V;
        Rows[E].Insert();
        if Refs[1].Number = 0 then
            Refs[1].Open(Database::"CS Row", true);
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows1=%1 rows2=%2 ref1=%3 int1=%4', Rows[1].Count(), Rows[2].Count(), Refs[1].Number, Ints[1]));
    end;
}

codeunit 50103 "CS Label Store"
{
    var
        Rows: Record "CS Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;
        Caption: Label 'Store';

    procedure Write(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 int1=%2 %3', Rows.Count(), Ints[1], Caption));
    end;
}

codeunit 50104 "CS Int Store"
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

codeunit 50105 "CS Holder"
{
    var
        X: Codeunit "CS Store";
        Count: Integer;

    procedure Share(var Y: Codeunit "CS Store")
    begin
        Y := X;
    end;

    procedure SetCount(V: Integer)
    begin
        Count := V;
    end;

    procedure StateX(): Text
    begin
        exit(X.State() + ' count=' + Format(Count));
    end;

    procedure ValueParam(): Text
    begin
        exit(ClearThenWrite(X));
    end;

    local procedure ClearThenWrite(P: Codeunit "CS Store"): Text
    begin
        ClearAll();
        P.Write(5);
        exit(X.State());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50106 "CS SI Owner"
{
    SingleInstance = true;

    var
        X: Codeunit "CS Store";
        Cnt: Integer;

    procedure Share(var Y: Codeunit "CS Store")
    begin
        Y := X;
    end;

    procedure SetCnt(V: Integer)
    begin
        Cnt := V;
    end;

    procedure StateX(): Text
    begin
        exit(X.State() + ' cnt=' + Format(Cnt));
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50107 "CS Arr Holder"
{
    var
        X: Codeunit "CS Arr Store";

    procedure Share(var Y: Codeunit "CS Arr Store")
    begin
        Y := X;
    end;

    procedure WriteX(E: Integer; V: Integer)
    begin
        X.Write(E, V);
    end;

    procedure StateX(): Text
    begin
        exit(X.State());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50108 "CS Arr SI Owner"
{
    SingleInstance = true;

    var
        X: Codeunit "CS Arr Store";

    procedure Share(var Y: Codeunit "CS Arr Store")
    begin
        Y := X;
    end;

    procedure StateX(): Text
    begin
        exit(X.State());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50109 "CS Label Holder"
{
    var
        X: Codeunit "CS Label Store";

    procedure Share(var Y: Codeunit "CS Label Store")
    begin
        Y := X;
    end;

    procedure WriteX(V: Integer)
    begin
        X.Write(V);
    end;

    procedure StateX(): Text
    begin
        exit(X.State());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}
