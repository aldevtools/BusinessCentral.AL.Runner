// Neighbors of al-runner issue 9022: the shapes next to a codeunit that holds a codeunit global of its own type and an
// Integer. The self-holding shapes themselves crash main, so they are Rust tests (al-vm tests/language/
// clear_shared_codeunit_cycle.rs); a crashed main run reports no outcome. Each test here gives the same result on main
// and on head.
table 50900 "SN Row"
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

// The repro's codeunit with a Decimal in place of the Integer: never a store.
codeunit 50901 "SN Self Dec"
{
    var
        Next: Codeunit "SN Self Dec";
        Count: Decimal;

    procedure Bump()
    begin
        Count += 1;
    end;

    procedure GetCount(): Decimal
    begin
        exit(Count);
    end;
}

// The repro's codeunit itself, reached only through Clear, which reads no store shape.
codeunit 50902 "SN Self"
{
    var
        Next: Codeunit "SN Self";
        Count: Integer;

    procedure Bump()
    begin
        Count += 1;
    end;

    procedure GetCount(): Integer
    begin
        exit(Count);
    end;
}

// "CLB Store": a temporary record, an array of RecordRef of two dimensions, an array of Integer.
codeunit 50903 "SN Store"
{
    var
        Rows: Record "SN Row" temporary;
        Refs: array[2, 3] of RecordRef;
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

// "CLQ Wrapper": a "CLB Store" global, then an Integer. Still the wrapper shape after the fix.
codeunit 50904 "SN Wrapper"
{
    var
        X: Codeunit "SN Store";
        WCnt: Integer;

    procedure ShareAlias(var Y: Codeunit "SN Store")
    begin
        Y := X;
    end;

    procedure StateX(): Text
    begin
        exit(X.State() + ' wcnt=' + Format(WCnt));
    end;
}

// A wrapper of a wrapper: no store shape, on main and on head.
codeunit 50905 "SN Outer"
{
    var
        W: Codeunit "SN Wrapper";
        N: Integer;

    procedure ShareInner(var Y: Codeunit "SN Store")
    begin
        W.ShareAlias(Y);
    end;

    procedure StateW(): Text
    begin
        exit(W.StateX());
    end;
}

// A codeunit global of another codeunit's type that holds one Integer, then an Integer: no store shape.
codeunit 50906 "SN Int Store"
{
    var
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure GetN(): Integer
    begin
        exit(N);
    end;
}

codeunit 50907 "SN Int Holder"
{
    var
        S: Codeunit "SN Int Store";
        Cnt: Integer;

    procedure ShareS(var Y: Codeunit "SN Int Store")
    begin
        Y := S;
    end;

    procedure GetN(): Integer
    begin
        exit(S.GetN());
    end;
}

codeunit 50908 "SN Holder"
{
    var
        Dec: Codeunit "SN Self Dec";
        W: Codeunit "SN Wrapper";
        O: Codeunit "SN Outer";
        IH: Codeunit "SN Int Holder";

    procedure DecUnshared(): Decimal
    begin
        Dec.Bump();
        ClearAll();
        exit(Dec.GetCount());
    end;

    procedure DecShared(): Decimal
    var
        X: Codeunit "SN Self Dec";
    begin
        X := Dec;
        X.Bump();
        ClearAll();
        exit(Dec.GetCount());
    end;

    procedure ShareWrapperInner(var Y: Codeunit "SN Store")
    begin
        W.ShareAlias(Y);
    end;

    procedure ShareOuterInner(var Y: Codeunit "SN Store")
    begin
        O.ShareInner(Y);
    end;

    procedure ShareIntInner(var Y: Codeunit "SN Int Store")
    begin
        IH.ShareS(Y);
    end;

    procedure StateWrapper(): Text
    begin
        exit(W.StateX());
    end;

    procedure StateOuter(): Text
    begin
        exit(O.StateW());
    end;

    procedure IntN(): Integer
    begin
        exit(IH.GetN());
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

// The repro's codeunit in an owner of its own: Clear only, no ClearAll here (ClearAll of it crashes main).
codeunit 50909 "SN Self Owner"
{
    var
        Self: Codeunit "SN Self";

    procedure ClearSelf(): Integer
    begin
        Self.Bump();
        Clear(Self);
        exit(Self.GetCount());
    end;
}
