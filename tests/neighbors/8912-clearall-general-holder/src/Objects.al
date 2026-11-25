// Neighbors of al-runner issue 8912 (review round 1, H1): ClearAll of codeunit globals that are not one of the store shapes
// the corpus measures. Stand-in objects, invented code.
table 50200 "GH Row"
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

// A codeunit with a TableNo and globals of mixed kinds, as a journal register codeunit has.
codeunit 50201 "GH Register"
{
    TableNo = "GH Row";

    var
        Saved: Record "GH Row";
        TempRows: Record "GH Row" temporary;
        Amount: Decimal;
        Posted: Boolean;
        LastCode: Code[20];

    trigger OnRun()
    begin
        Posted := true;
    end;

    procedure Bump()
    begin
        Amount += 1;
        LastCode := 'X';
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('amount=%1 rows=%2', Amount, TempRows.Count()));
    end;
}

// The posting codeunit: a setter hands it the caller's register, its OnRun calls ClearAll first.
codeunit 50202 "GH Poster"
{
    TableNo = "GH Row";

    var
        Register: Codeunit "GH Register";
        Lines: Integer;

    trigger OnRun()
    begin
        ClearAll();
        Lines += 1;
    end;

    procedure SetRegisterCU(var NewRegister: Codeunit "GH Register")
    begin
        Register := NewRegister;
    end;
}

// The activity posting codeunit: it uses its register, hands it to the poster, and runs the poster.
codeunit 50203 "GH Activity Post"
{
    TableNo = "GH Row";

    var
        Register: Codeunit "GH Register";
        Poster: Codeunit "GH Poster";

    trigger OnRun()
    begin
        Register.Bump();
        Poster.SetRegisterCU(Register);
        Poster.Run(Rec);
    end;

    procedure RegisterState(): Text
    begin
        exit(Register.State());
    end;
}

// A codeunit with a temporary record and a Decimal: not a store shape the corpus measures.
codeunit 50204 "GH Gen"
{
    var
        Rows: Record "GH Row" temporary;
        D: Decimal;

    procedure Write(V: Integer)
    begin
        Rows.No := V;
        Rows.Insert();
        D := V;
    end;

    procedure Empty()
    begin
        Rows.DeleteAll();
    end;

    procedure ShareRows(var R: Record "GH Row" temporary)
    begin
        R.Copy(Rows, true);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 d=%2', Rows.Count(), D));
    end;
}

// A plain holder, reused across calls.
codeunit 50205 "GH Holder"
{
    var
        G: Codeunit "GH Gen";
        N: Integer;

    procedure WriteG(V: Integer)
    begin
        G.Write(V);
        N := V;
    end;

    procedure EmptyG()
    begin
        G.Empty();
    end;

    procedure ShareG(var R: Record "GH Row" temporary)
    begin
        G.ShareRows(R);
    end;

    procedure StateG(): Text
    begin
        exit(G.State() + ' n=' + Format(N));
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}
