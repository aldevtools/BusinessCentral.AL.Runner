// The register of corpus PR 1564 (ClearSharedCodeunitSetterOwner.al), shared by a test local through the plain owner's
// setter, and a record of the test that shares the register's table through Copy(Rows, true).
table 50800 "U6 Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

codeunit 50801 "U6 Int"
{
    var
        N: Integer;

    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}

codeunit 50802 "U6 Reg"
{
    TableNo = "U6 Row";

    var
        Rows: Record "U6 Row" temporary;
        Rec: Record "U6 Row";
        Dec: Decimal;
        Flag: Boolean;
        Cd: Code[20];
        Nested: Codeunit "U6 Int";

    procedure Set(V: Integer) begin Dec := V; end;
    procedure Get(): Integer begin exit(Round(Dec, 1)); end;
    procedure ShareRows(var T: Record "U6 Row" temporary) begin T.Copy(Rows, true); end;
}

codeunit 50803 "U6 Owner"
{
    var
        R: Codeunit "U6 Reg";

    procedure SetReg(var Y: Codeunit "U6 Reg") begin R := Y; end;
    procedure RegGet(): Integer begin exit(R.Get()); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
}
