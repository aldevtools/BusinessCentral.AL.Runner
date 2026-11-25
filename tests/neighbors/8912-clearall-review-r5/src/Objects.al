// Stores of corpus PR 1564 (tests/al-language/codeunits/clear-instance/), each shared by a test local through the plain
// owner's getter (or setter, for the register). "U5 PlusFace" is the store with one more Integer that implements an
// interface; the measured one ("CLQ Store Plus", ClearSharedCodeunitOwnerKinds.al:183) implements none.
table 50750 "U5 Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

enum 50751 "U5 Kind"
{
    value(0; None) { }
    value(1; Beta) { }
}

codeunit 50752 "U5 Int"
{
    var
        N: Integer;

    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}

codeunit 50753 "U5 Clb"
{
    var
        Rows: Record "U5 Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;

    procedure Set(V: Integer) begin Ints[1] := V; end;
    procedure Get(): Integer begin exit(Ints[1]); end;
}

codeunit 50754 "U5 Plus"
{
    var
        Rows: Record "U5 Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;
        N: Integer;

    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}

codeunit 50755 "U5 Wrap"
{
    var
        Inner: Codeunit "U5 Clb";
        N: Integer;

    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}

codeunit 50756 "U5 Arr"
{
    var
        Rows: array[2] of Record "U5 Row" temporary;
        Refs: array[2] of RecordRef;
        Ints: array[3] of Integer;

    procedure Set(V: Integer) begin Ints[1] := V; end;
    procedure Get(): Integer begin exit(Ints[1]); end;
}

codeunit 50757 "U5 TwoDim"
{
    var
        Rows: array[2, 2] of Record "U5 Row" temporary;
        Ints: array[2] of Integer;

    procedure Set(V: Integer) begin Ints[1] := V; end;
    procedure Get(): Integer begin exit(Ints[1]); end;
}

codeunit 50758 "U5 Reg"
{
    TableNo = "U5 Row";

    var
        Rows: Record "U5 Row" temporary;
        Rec: Record "U5 Row";
        Dec: Decimal;
        Flag: Boolean;
        Cd: Code[20];
        Nested: Codeunit "U5 Int";

    procedure Set(V: Integer) begin Dec := V; end;
    procedure Get(): Integer begin exit(Round(Dec, 1)); end;
}

codeunit 50759 "U5 Price"
{
    var
        T1: Record "U5 Row" temporary;
        Kind: Enum "U5 Kind";
        Int: Integer;
        LblC: Label 'store label', Comment = 'a comment';

    procedure Set(V: Integer) begin Int := V; end;
    procedure Get(): Integer begin exit(Int); end;
}

codeunit 50760 "U5 Owner"
{
    var
        I: Codeunit "U5 Int";
        C: Codeunit "U5 Clb";
        P: Codeunit "U5 Plus";
        W: Codeunit "U5 Wrap";
        A: Codeunit "U5 Arr";
        D: Codeunit "U5 TwoDim";
        R: Codeunit "U5 Reg";
        Pr: Codeunit "U5 Price";

    procedure ShareInt(var Y: Codeunit "U5 Int") begin Y := I; end;
    procedure ShareClb(var Y: Codeunit "U5 Clb") begin Y := C; end;
    procedure SharePlus(var Y: Codeunit "U5 Plus") begin Y := P; end;
    procedure ShareWrap(var Y: Codeunit "U5 Wrap") begin Y := W; end;
    procedure ShareArr(var Y: Codeunit "U5 Arr") begin Y := A; end;
    procedure ShareTwoDim(var Y: Codeunit "U5 TwoDim") begin Y := D; end;
    procedure SetReg(var Y: Codeunit "U5 Reg") begin R := Y; end;
    procedure SharePrice(var Y: Codeunit "U5 Price") begin Y := Pr; end;
    procedure IntGet(): Integer begin exit(I.Get()); end;
    procedure PlusGet(): Integer begin exit(P.Get()); end;
    procedure WrapGet(): Integer begin exit(W.Get()); end;
    procedure TwoDimGet(): Integer begin exit(D.Get()); end;
    procedure RegGet(): Integer begin exit(R.Get()); end;
    procedure PriceGet(): Integer begin exit(Pr.Get()); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
}
interface "U5 Face"
{
    procedure Set(V: Integer);
    procedure Get(): Integer;
}
codeunit 50772 "U5 PlusFace" implements "U5 Face"
{
    var
        Rows: Record "U5 Row" temporary;
        Refs: array[2, 3] of RecordRef;
        Ints: array[3] of Integer;
        N: Integer;
    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
}
codeunit 50773 "U5 FaceOwner"
{
    var
        P: Codeunit "U5 PlusFace";
    procedure SharePlus(var Y: Codeunit "U5 PlusFace") begin Y := P; end;
    procedure PlusGet(): Integer begin exit(P.Get()); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
}
