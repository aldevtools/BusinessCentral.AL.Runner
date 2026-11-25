// Neighbors of al-runner issue 8912, review round 2 of the ClearAll change: the holder of a shared instance or of a shared
// temporary table runs ClearAll. Stand-in objects, invented code.
table 50650 "RH Row"
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

interface "RH Face"
{
    procedure Get(): Integer;
}

// The one-Integer store shape, behind an interface.
codeunit 50651 "RH Store" implements "RH Face"
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

// A plain owner whose global store an interface holder shares.
codeunit 50652 "RH Owner"
{
    var
        S: Codeunit "RH Store";

    procedure SetN(V: Integer)
    begin
        S.Set(V);
    end;

    procedure ShareFace(var H: Codeunit "RH Face Holder")
    begin
        H.Take(S);
    end;

    procedure GetN(): Integer
    begin
        exit(S.Get());
    end;
}

// A plain codeunit that holds the owner's store in an interface global.
codeunit 50653 "RH Face Holder"
{
    var
        F: Interface "RH Face";

    procedure Take(NewF: Interface "RH Face")
    begin
        F := NewF;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

// A plain owner whose global array of temporary records a record holder shares through Copy(x, true).
codeunit 50654 "RH Arr Owner"
{
    var
        A: array[2] of Record "RH Row" temporary;
        Cnt: Integer;

    procedure SetCnt(V: Integer)
    begin
        Cnt := V;
    end;

    procedure WriteA(V: Integer)
    begin
        A[1].No := V;
        A[1].Insert();
    end;

    procedure ShareA1(var R: Record "RH Row" temporary)
    begin
        R.Copy(A[1], true);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('a1=%1 cnt=%2', A[1].Count(), Cnt));
    end;
}

// A plain codeunit whose global temporary record shares the owner's table, with an Integer.
codeunit 50655 "RH Rec Holder"
{
    var
        R: Record "RH Row" temporary;
        HCnt: Integer;

    procedure Take(var O: Codeunit "RH Arr Owner")
    begin
        O.ShareA1(R);
        HCnt := 1;
    end;

    procedure Write(V: Integer)
    begin
        R.No := V;
        R.Insert();
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('r=%1 hcnt=%2', R.Count(), HCnt));
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}
