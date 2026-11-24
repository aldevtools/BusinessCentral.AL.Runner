// Neighbors of al-runner issue 8912, round of corpus PR 1564: ClearAll in owners and of stores next to the cells 1564
// measures. Each test expects what BC does in the nearest measured cell: the owner's store reset in place (rows kept,
// RecordRef closed, Integers 0), a sharer on the same instance.
table 50300 "CN Row"
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

interface "CN IStore"
{
    procedure Write(V: Integer);
    procedure State(): Text;
}

codeunit 50301 "CN Store"
{
    var
        Rows: Record "CN Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;

    procedure Write(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
        if Refs[1, 1].Number = 0 then
            Refs[1, 1].Open(Database::"CN Row", true);
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 ref11=%2 int1=%3', Rows.Count(), Refs[1, 1].Number, Ints[1]));
    end;
}

codeunit 50302 "CN Label Store" implements "CN IStore"
{
    var
        Rows: Record "CN Row" temporary;
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

codeunit 50303 "CN Label Mid2 Store"
{
    var
        Rows: Record "CN Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Caption: Label 'Store';
        Ints: array[3] of Integer;

    procedure Write(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 int1=%2', Rows.Count(), Ints[1]));
    end;
}

codeunit 50304 "CN Label Locked Store"
{
    var
        Rows: Record "CN Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;
        Caption: Label 'Store', Locked = true;

    procedure Write(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 int1=%2', Rows.Count(), Ints[1]));
    end;
}

codeunit 50305 "CN Arr Label Store"
{
    var
        Rows: array[2] of Record "CN Row" temporary;
        Refs: array[2] of RecordRef;
        Ints: array[3] of Integer;
        Caption: Label 'Store';

    procedure Write(V: Integer)
    begin
        Rows[1].Init();
        Rows[1].No := V;
        Rows[1].Insert();
        Ints[1] := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 int1=%2', Rows[2].Count(), Ints[1]));
    end;
}

codeunit 50306 "CN Int Label Store"
{
    var
        N: Integer;
        Caption: Label 'Store';

    procedure Set(V: Integer)
    begin
        N := V;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 50307 "CN Plus Store"
{
    var
        Rows: Record "CN Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;
        N: Integer;

    procedure Write(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
        Ints[1] := V;
        N := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 int1=%2 n=%3', Rows.Count(), Ints[1], N));
    end;
}

codeunit 50308 "CN Wrapper"
{
    var
        X: Codeunit "CN Store";
        WCnt: Integer;

    procedure WriteX(V: Integer)
    begin
        X.Write(V);
        WCnt := V;
    end;

    procedure StateX(): Text
    begin
        exit(X.State() + ' wcnt=' + Format(WCnt));
    end;

    procedure ShareAlias(var Y: Codeunit "CN Store")
    begin
        Y := X;
    end;
}

// A plain holder of every store kind above.
codeunit 50310 "CN Holder"
{
    var
        L: Codeunit "CN Label Store";
        M2: Codeunit "CN Label Mid2 Store";
        Lk: Codeunit "CN Label Locked Store";
        AL: Codeunit "CN Arr Label Store";
        IL: Codeunit "CN Int Label Store";
        W: Codeunit "CN Wrapper";

    procedure ShareL(var Y: Codeunit "CN Label Store")
    begin
        Y := L;
    end;

    procedure ShareLIface(var I: Interface "CN IStore")
    begin
        I := L;
    end;

    procedure ShareIL(var Y: Codeunit "CN Int Label Store")
    begin
        Y := IL;
    end;

    procedure ShareInner(var Y: Codeunit "CN Store")
    begin
        W.ShareAlias(Y);
    end;

    procedure WriteM2(V: Integer)
    begin
        M2.Write(V);
    end;

    procedure WriteLk(V: Integer)
    begin
        Lk.Write(V);
    end;

    procedure WriteAL(V: Integer)
    begin
        AL.Write(V);
    end;

    procedure StateL(): Text
    begin
        exit(L.State());
    end;

    procedure StateM2(): Text
    begin
        exit(M2.State());
    end;

    procedure StateLk(): Text
    begin
        exit(Lk.State());
    end;

    procedure StateAL(): Text
    begin
        exit(AL.State());
    end;

    procedure StateW(): Text
    begin
        exit(W.StateX());
    end;

    procedure GetIL(): Integer
    begin
        exit(IL.Get());
    end;

    procedure ValueParamL(): Text
    begin
        exit(ClearThenWrite(L));
    end;

    procedure OuterValueParamL(): Text
    begin
        exit(Outer(L));
    end;

    local procedure ClearThenWrite(P: Codeunit "CN Label Store"): Text
    begin
        ClearAll();
        P.Write(5);
        exit(L.State());
    end;

    local procedure Outer(P: Codeunit "CN Label Store"): Text
    begin
        ClearAllGlobals();
        P.Write(5);
        exit(L.State());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 50311 "CN SI Owner"
{
    SingleInstance = true;

    var
        L: Codeunit "CN Label Store";
        P: Codeunit "CN Plus Store";
        W: Codeunit "CN Wrapper";
        A: array[2] of Codeunit "CN Store";

    procedure ShareL(var Y: Codeunit "CN Label Store")
    begin
        Y := L;
    end;

    procedure StateL(): Text
    begin
        exit(L.State());
    end;

    procedure WriteP(V: Integer)
    begin
        P.Write(V);
    end;

    procedure StateP(): Text
    begin
        exit(P.State());
    end;

    procedure WriteW(V: Integer)
    begin
        W.WriteX(V);
    end;

    procedure StateW(): Text
    begin
        exit(W.StateX());
    end;

    procedure WriteA(V: Integer)
    begin
        A[1].Write(V);
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

codeunit 50312 "CN Normal Owner"
{
    Subtype = Normal;

    var
        L: Codeunit "CN Label Store";

    procedure WriteL(V: Integer)
    begin
        L.Write(V);
    end;

    procedure StateL(): Text
    begin
        exit(L.State());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

report 50313 "CN Report RP"
{
    ProcessingOnly = true;

    var
        X: Codeunit "CN Store";

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

page 50314 "CN Page Owner"
{
    PageType = Card;

    var
        X: Codeunit "CN Store";

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

table 50315 "CN Table Owner"
{
    fields
    {
        field(1; No; Integer) { }
    }
    keys
    {
        key(PK; No) { }
    }

    var
        X: Codeunit "CN Store";

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
