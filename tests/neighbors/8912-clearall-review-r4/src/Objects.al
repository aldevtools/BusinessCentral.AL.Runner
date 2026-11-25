// Stores of corpus PR 1564 (tests/al-language/codeunits/clear-instance/), each shared by a test local through the plain
// owner's getter (or setter, for the register) before any call made the instance.
table 50700 "U4 Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

enum 50701 "U4 Kind"
{
    value(0; None) { }
    value(1; Beta) { }
}

codeunit 50702 "U4 Int"
{
    var
        N: Integer;

    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}

codeunit 50703 "U4 Clb"
{
    var
        Rows: Record "U4 Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;

    procedure Set(V: Integer) begin Ints[1] := V; end;
    procedure Get(): Integer begin exit(Ints[1]); end;
}

codeunit 50704 "U4 Plus"
{
    var
        Rows: Record "U4 Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;
        N: Integer;

    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}

codeunit 50705 "U4 Wrap"
{
    var
        Inner: Codeunit "U4 Clb";
        N: Integer;

    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}

codeunit 50706 "U4 Arr"
{
    var
        Rows: array[2] of Record "U4 Row" temporary;
        Refs: array[2] of RecordRef;
        Ints: array[3] of Integer;

    procedure Set(V: Integer) begin Ints[1] := V; end;
    procedure Get(): Integer begin exit(Ints[1]); end;
}

codeunit 50707 "U4 TwoDim"
{
    var
        Rows: array[2, 2] of Record "U4 Row" temporary;
        Ints: array[2] of Integer;

    procedure Set(V: Integer) begin Ints[1] := V; end;
    procedure Get(): Integer begin exit(Ints[1]); end;
}

codeunit 50708 "U4 Reg"
{
    TableNo = "U4 Row";

    var
        Rows: Record "U4 Row" temporary;
        Rec: Record "U4 Row";
        Dec: Decimal;
        Flag: Boolean;
        Cd: Code[20];
        Nested: Codeunit "U4 Int";

    procedure Set(V: Integer) begin Dec := V; end;
    procedure Get(): Integer begin exit(Round(Dec, 1)); end;
}

codeunit 50709 "U4 Price"
{
    var
        T1: Record "U4 Row" temporary;
        Kind: Enum "U4 Kind";
        Int: Integer;
        LblC: Label 'store label', Comment = 'a comment';

    procedure Set(V: Integer) begin Int := V; end;
    procedure Get(): Integer begin exit(Int); end;
}

codeunit 50710 "U4 Owner"
{
    var
        I: Codeunit "U4 Int";
        C: Codeunit "U4 Clb";
        P: Codeunit "U4 Plus";
        W: Codeunit "U4 Wrap";
        A: Codeunit "U4 Arr";
        D: Codeunit "U4 TwoDim";
        R: Codeunit "U4 Reg";
        Pr: Codeunit "U4 Price";

    procedure ShareInt(var Y: Codeunit "U4 Int") begin Y := I; end;
    procedure ShareClb(var Y: Codeunit "U4 Clb") begin Y := C; end;
    procedure SharePlus(var Y: Codeunit "U4 Plus") begin Y := P; end;
    procedure ShareWrap(var Y: Codeunit "U4 Wrap") begin Y := W; end;
    procedure ShareArr(var Y: Codeunit "U4 Arr") begin Y := A; end;
    procedure ShareTwoDim(var Y: Codeunit "U4 TwoDim") begin Y := D; end;
    procedure SetReg(var Y: Codeunit "U4 Reg") begin R := Y; end;
    procedure SharePrice(var Y: Codeunit "U4 Price") begin Y := Pr; end;
    procedure PriceGet(): Integer begin exit(Pr.Get()); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
}
