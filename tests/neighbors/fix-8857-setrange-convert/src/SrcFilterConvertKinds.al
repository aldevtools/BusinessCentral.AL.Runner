// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/fieldref/fieldref-setrange-method
// Scope: in-scope
// Fixtures used: Probe Observer (69160); own table 57406, helper codeunit 57406, test codeunits 57407..57409
// Probe: every literal asserted below was observed by the corpus CI on BC 27.5, 28.5 and 29.0; #if BC28PLUS marks the results BC 27.5
//   gives differently.
// Facts: FieldRef.SetRange, FieldRef.SetFilter('%1', ...) and Record.SetRange on a Code[20], Text[50], Integer, BigInteger, Decimal,
//   Boolean, Guid and Date field with a value of another type, and FieldRef.Value := of the same value for comparison.
//   Sources into Code and Text: Integer 5 and 0, BigInteger 5, Decimal 1.5, 1000.5 and -1.25, Boolean true.
//   Into Integer, BigInteger and Decimal: Code '123', 'ABC' and '1,000', Text 'XY', '1.5' and '' (empty).
//   Into Boolean: Code '123', '0' and 'ABC', Text 'Yes' and 'XY', Integer 1 and 0. Into Date: Code 'ABC', Text 'XY'.
//   Into Guid: Code 'ABC' and '123', Text 'XY' and Text '{1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}'.
//   Routes: a FieldRef.SetRange(X) typed; b FieldRef.SetRange(V), V a Variant holding X; c FieldRef.SetRange(X, X);
//   e FieldRef.SetFilter('%1', X); f FieldRef.SetFilter('%1', V); gr Record.SetRange(Rec.<field>, V); v FieldRef.Value := X.
//   Each cell runs in a TryFunction on a fresh RecordRef or Record and records "ok: <GetFilter of the field>" (route v:
//   "ok: " + Format(F.Value, 0, 9)) or "E: <GetLastErrorText> | <GetLastErrorCode>". A test is <Source>_<Field>; the Date tests of
//   Code 'ABC' and Text 'XY' run route v in a test of its own (<Source>_D_Value), as the results of all routes do not fit one observation.
//   Companion of SrcSetRangeConvert.al (Date, Time and DateTime fields).
//   Asked by al-runner issues 8857, 8867, 8868, 8869 and 8870.
// BC versions: 27, 28, 29 by CI
table 57406 "SFK Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Cod; Code[20]) { }
        field(3; Txt; Text[50]) { }
        field(4; Int; Integer) { }
        field(5; Big; BigInteger) { }
        field(6; Dec; Decimal) { }
        field(7; Bool; Boolean) { }
        field(8; G; Guid) { }
        field(9; D; Date) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 57406 "SFK Probe"
{
    // Each test declares its own "SFK Probe" variable, so every test starts with an empty observer.
    var
        Obs: Codeunit "Probe Observer";

    procedure Finish()
    begin
        Obs.Finish();
    end;

    procedure Expect(Name: Text; Value: Text)
    begin
        Obs.Expect(Name, Value);
    end;

    procedure Verify()
    begin
        Obs.Verify();
    end;

    local procedure Outcome(Name: Text; Ok: Boolean; Filter: Text)
    begin
        if Ok then
            Obs.Observe(Name, 'ok: ' + Filter)
        else
            Obs.Observe(Name, 'E: ' + GetLastErrorText() + ' | ' + GetLastErrorCode());
        ClearLastError();
    end;

    // Route v records the field value; every other route records the filter of the field.
    local procedure Result(var F: FieldRef; Route: Text; Ok: Boolean): Text
    begin
        if not Ok then
            exit('');
        if Route = 'v' then
            exit(Format(F.Value, 0, 9));
        exit(F.GetFilter());
    end;

    local procedure RecFilter(var R: Record "SFK Row"; FieldNo: Integer): Text
    begin
        case FieldNo of
            2:
                exit(R.GetFilter(Cod));
            3:
                exit(R.GetFilter(Txt));
            4:
                exit(R.GetFilter(Int));
            5:
                exit(R.GetFilter(Big));
            6:
                exit(R.GetFilter(Dec));
            7:
                exit(R.GetFilter(Bool));
            8:
                exit(R.GetFilter(G));
            9:
                exit(R.GetFilter(D));
        end;
    end;

    [TryFunction]
    local procedure TryInt(var F: FieldRef; Route: Text; X: Integer)
    begin
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'e':
                F.SetFilter('%1', X);
            'v':
                F.Value := X;
        end;
    end;

    local procedure CellInt(Route: Text; FieldNo: Integer; X: Integer)
    var
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SFK Row");
        F := RecRef.Field(FieldNo);
        Ok := TryInt(F, Route, X);
        Outcome(Route, Ok, Result(F, Route, Ok));
        RecRef.Close();
    end;

    // All routes for one Integer source on one field, in the order a, b, c, e, f, gr, v.
    procedure CellsInt(FieldNo: Integer; X: Integer)
    var
        V: Variant;
    begin
        V := X;
        CellInt('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellInt('c', FieldNo, X);
        CellInt('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
        CellInt('v', FieldNo, X);
    end;

    // Neighbor copy only (not in corpus PR 1552): routes a, b, c, e, f and gr of an Integer source, like FiltersCode.
    procedure FiltersInt(FieldNo: Integer; X: Integer)
    var
        V: Variant;
    begin
        V := X;
        CellInt('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellInt('c', FieldNo, X);
        CellInt('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
    end;

    [TryFunction]
    local procedure TryBig(var F: FieldRef; Route: Text; X: BigInteger)
    begin
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'e':
                F.SetFilter('%1', X);
            'v':
                F.Value := X;
        end;
    end;

    local procedure CellBig(Route: Text; FieldNo: Integer; X: BigInteger)
    var
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SFK Row");
        F := RecRef.Field(FieldNo);
        Ok := TryBig(F, Route, X);
        Outcome(Route, Ok, Result(F, Route, Ok));
        RecRef.Close();
    end;

    // All routes for one BigInteger source on one field, in the order a, b, c, e, f, gr, v.
    procedure CellsBig(FieldNo: Integer; X: BigInteger)
    var
        V: Variant;
    begin
        V := X;
        CellBig('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellBig('c', FieldNo, X);
        CellBig('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
        CellBig('v', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryDec(var F: FieldRef; Route: Text; X: Decimal)
    begin
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'e':
                F.SetFilter('%1', X);
            'v':
                F.Value := X;
        end;
    end;

    local procedure CellDec(Route: Text; FieldNo: Integer; X: Decimal)
    var
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SFK Row");
        F := RecRef.Field(FieldNo);
        Ok := TryDec(F, Route, X);
        Outcome(Route, Ok, Result(F, Route, Ok));
        RecRef.Close();
    end;

    // All routes for one Decimal source on one field, in the order a, b, c, e, f, gr, v.
    procedure CellsDec(FieldNo: Integer; X: Decimal)
    var
        V: Variant;
    begin
        V := X;
        CellDec('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellDec('c', FieldNo, X);
        CellDec('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
        CellDec('v', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryBool(var F: FieldRef; Route: Text; X: Boolean)
    begin
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'e':
                F.SetFilter('%1', X);
            'v':
                F.Value := X;
        end;
    end;

    local procedure CellBool(Route: Text; FieldNo: Integer; X: Boolean)
    var
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SFK Row");
        F := RecRef.Field(FieldNo);
        Ok := TryBool(F, Route, X);
        Outcome(Route, Ok, Result(F, Route, Ok));
        RecRef.Close();
    end;

    // All routes for one Boolean source on one field, in the order a, b, c, e, f, gr, v.
    procedure CellsBool(FieldNo: Integer; X: Boolean)
    var
        V: Variant;
    begin
        V := X;
        CellBool('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellBool('c', FieldNo, X);
        CellBool('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
        CellBool('v', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryCode(var F: FieldRef; Route: Text; X: Code[20])
    begin
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'e':
                F.SetFilter('%1', X);
            'v':
                F.Value := X;
        end;
    end;

    local procedure CellCode(Route: Text; FieldNo: Integer; X: Code[20])
    var
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SFK Row");
        F := RecRef.Field(FieldNo);
        Ok := TryCode(F, Route, X);
        Outcome(Route, Ok, Result(F, Route, Ok));
        RecRef.Close();
    end;

    // All routes for one Code[20] source on one field, in the order a, b, c, e, f, gr, v.
    procedure CellsCode(FieldNo: Integer; X: Code[20])
    var
        V: Variant;
    begin
        V := X;
        CellCode('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellCode('c', FieldNo, X);
        CellCode('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
        CellCode('v', FieldNo, X);
    end;

    // Routes a, b, c, e, f and gr only: a test whose results do not fit one observation (about 1800 characters) tests route v apart.
    procedure FiltersCode(FieldNo: Integer; X: Code[20])
    var
        V: Variant;
    begin
        V := X;
        CellCode('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellCode('c', FieldNo, X);
        CellCode('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
    end;

    procedure ValueCode(FieldNo: Integer; X: Code[20])
    begin
        CellCode('v', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryTxt(var F: FieldRef; Route: Text; X: Text)
    begin
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'e':
                F.SetFilter('%1', X);
            'v':
                F.Value := X;
        end;
    end;

    local procedure CellTxt(Route: Text; FieldNo: Integer; X: Text)
    var
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SFK Row");
        F := RecRef.Field(FieldNo);
        Ok := TryTxt(F, Route, X);
        Outcome(Route, Ok, Result(F, Route, Ok));
        RecRef.Close();
    end;

    // All routes for one Text source on one field, in the order a, b, c, e, f, gr, v.
    procedure CellsTxt(FieldNo: Integer; X: Text)
    var
        V: Variant;
    begin
        V := X;
        CellTxt('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellTxt('c', FieldNo, X);
        CellTxt('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
        CellTxt('v', FieldNo, X);
    end;

    // Routes a, b, c, e, f and gr only: a test whose results do not fit one observation (about 1800 characters) tests route v apart.
    procedure FiltersTxt(FieldNo: Integer; X: Text)
    var
        V: Variant;
    begin
        V := X;
        CellTxt('a', FieldNo, X);
        CellVar('b', FieldNo, V);
        CellTxt('c', FieldNo, X);
        CellTxt('e', FieldNo, X);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
    end;

    procedure ValueTxt(FieldNo: Integer; X: Text)
    begin
        CellTxt('v', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryVar(var R: Record "SFK Row"; var F: FieldRef; Route: Text; V: Variant)
    begin
        case Route of
            'b':
                F.SetRange(V);
            'f':
                F.SetFilter('%1', V);
            'gr':
                case F.Number() of
                    2:
                        R.SetRange(Cod, V);
                    3:
                        R.SetRange(Txt, V);
                    4:
                        R.SetRange(Int, V);
                    5:
                        R.SetRange(Big, V);
                    6:
                        R.SetRange(Dec, V);
                    7:
                        R.SetRange(Bool, V);
                    8:
                        R.SetRange(G, V);
                    9:
                        R.SetRange(D, V);
                end;
        end;
    end;

    local procedure CellVar(Route: Text; FieldNo: Integer; V: Variant)
    var
        R: Record "SFK Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SFK Row");
        F := RecRef.Field(FieldNo);
        Ok := TryVar(R, F, Route, V);
        if Route = 'gr' then
            Outcome(Route, Ok, RecFilter(R, FieldNo))
        else
            Outcome(Route, Ok, Result(F, Route, Ok));
        RecRef.Close();
    end;
}

codeunit 57407 "SFK Number To Text Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Int5_Cod()
    var
        Probe: Codeunit "SFK Probe";
        X: Integer;
    begin
        X := 5;
        Probe.CellsInt(2, X);
        Probe.Expect('a', 'ok: 5');
        Probe.Expect('b', 'ok: 5');
        Probe.Expect('c', 'ok: 5');
        Probe.Expect('e', 'ok: 5');
        Probe.Expect('f', 'ok: 5');
        Probe.Expect('gr', 'ok: 5');
        Probe.Expect('v', 'ok: 5');
        Probe.Verify();
    end;

    [Test]
    procedure Int0_Cod()
    var
        Probe: Codeunit "SFK Probe";
        X: Integer;
    begin
        X := 0;
        Probe.CellsInt(2, X);
        Probe.Expect('a', 'ok: 0');
        Probe.Expect('b', 'ok: 0');
        Probe.Expect('c', 'ok: 0');
        Probe.Expect('e', 'ok: 0');
        Probe.Expect('f', 'ok: 0');
        Probe.Expect('gr', 'ok: 0');
        Probe.Expect('v', 'ok: 0');
        Probe.Verify();
    end;

    [Test]
    procedure Big5_Cod()
    var
        Probe: Codeunit "SFK Probe";
        X: BigInteger;
    begin
        X := 5;
        Probe.CellsBig(2, X);
        Probe.Expect('a', 'ok: 5');
        Probe.Expect('b', 'ok: 5');
        Probe.Expect('c', 'ok: 5');
        Probe.Expect('e', 'ok: 5');
        Probe.Expect('f', 'ok: 5');
        Probe.Expect('gr', 'ok: 5');
        Probe.Expect('v', 'ok: 5');
        Probe.Verify();
    end;

    [Test]
    procedure Dec15_Cod()
    var
        Probe: Codeunit "SFK Probe";
        X: Decimal;
    begin
        X := 1.5;
        Probe.CellsDec(2, X);
        Probe.Expect('a', 'ok: 1.5');
        Probe.Expect('b', 'ok: 1.5');
        Probe.Expect('c', 'ok: 1.5');
        Probe.Expect('e', 'ok: 1.5');
        Probe.Expect('f', 'ok: 1.5');
        Probe.Expect('gr', 'ok: 1.5');
        Probe.Expect('v', 'ok: 1.5');
        Probe.Verify();
    end;

    [Test]
    procedure Dec10005_Cod()
    var
        Probe: Codeunit "SFK Probe";
        X: Decimal;
    begin
        X := 1000.5;
        Probe.CellsDec(2, X);
        Probe.Expect('a', 'ok: 1,000.5');
        Probe.Expect('b', 'ok: 1,000.5');
        Probe.Expect('c', 'ok: 1,000.5');
        Probe.Expect('e', 'ok: 1,000.5');
        Probe.Expect('f', 'ok: 1,000.5');
        Probe.Expect('gr', 'ok: 1,000.5');
        Probe.Expect('v', 'ok: 1,000.5');
        Probe.Verify();
    end;

    [Test]
    procedure DecNeg125_Cod()
    var
        Probe: Codeunit "SFK Probe";
        X: Decimal;
    begin
        X := -1.25;
        Probe.CellsDec(2, X);
        Probe.Expect('a', 'ok: -1.25');
        Probe.Expect('b', 'ok: -1.25');
        Probe.Expect('c', 'ok: -1.25');
        Probe.Expect('e', 'ok: -1.25');
        Probe.Expect('f', 'ok: -1.25');
        Probe.Expect('gr', 'ok: -1.25');
        Probe.Expect('v', 'ok: -1.25');
        Probe.Verify();
    end;

    [Test]
    procedure BoolTrue_Cod()
    var
        Probe: Codeunit "SFK Probe";
        X: Boolean;
    begin
        X := true;
        Probe.CellsBool(2, X);
        Probe.Expect('a', 'ok: YES');
        Probe.Expect('b', 'ok: YES');
        Probe.Expect('c', 'ok: YES');
        Probe.Expect('e', 'ok: YES');
        Probe.Expect('f', 'ok: YES');
        Probe.Expect('gr', 'ok: YES');
        Probe.Expect('v', 'ok: YES');
        Probe.Verify();
    end;

    [Test]
    procedure Int5_Txt()
    var
        Probe: Codeunit "SFK Probe";
        X: Integer;
    begin
        X := 5;
        Probe.CellsInt(3, X);
        Probe.Expect('a', 'ok: 5');
        Probe.Expect('b', 'ok: 5');
        Probe.Expect('c', 'ok: 5');
        Probe.Expect('e', 'ok: 5');
        Probe.Expect('f', 'ok: 5');
        Probe.Expect('gr', 'ok: 5');
        Probe.Expect('v', 'ok: 5');
        Probe.Verify();
    end;

    [Test]
    procedure Int0_Txt()
    var
        Probe: Codeunit "SFK Probe";
        X: Integer;
    begin
        X := 0;
        Probe.CellsInt(3, X);
        Probe.Expect('a', 'ok: 0');
        Probe.Expect('b', 'ok: 0');
        Probe.Expect('c', 'ok: 0');
        Probe.Expect('e', 'ok: 0');
        Probe.Expect('f', 'ok: 0');
        Probe.Expect('gr', 'ok: 0');
        Probe.Expect('v', 'ok: 0');
        Probe.Verify();
    end;

    [Test]
    procedure Big5_Txt()
    var
        Probe: Codeunit "SFK Probe";
        X: BigInteger;
    begin
        X := 5;
        Probe.CellsBig(3, X);
        Probe.Expect('a', 'ok: 5');
        Probe.Expect('b', 'ok: 5');
        Probe.Expect('c', 'ok: 5');
        Probe.Expect('e', 'ok: 5');
        Probe.Expect('f', 'ok: 5');
        Probe.Expect('gr', 'ok: 5');
        Probe.Expect('v', 'ok: 5');
        Probe.Verify();
    end;

    [Test]
    procedure Dec15_Txt()
    var
        Probe: Codeunit "SFK Probe";
        X: Decimal;
    begin
        X := 1.5;
        Probe.CellsDec(3, X);
        Probe.Expect('a', 'ok: 1.5');
        Probe.Expect('b', 'ok: 1.5');
        Probe.Expect('c', 'ok: 1.5');
        Probe.Expect('e', 'ok: 1.5');
        Probe.Expect('f', 'ok: 1.5');
        Probe.Expect('gr', 'ok: 1.5');
        Probe.Expect('v', 'ok: 1.5');
        Probe.Verify();
    end;

    [Test]
    procedure Dec10005_Txt()
    var
        Probe: Codeunit "SFK Probe";
        X: Decimal;
    begin
        X := 1000.5;
        Probe.CellsDec(3, X);
        Probe.Expect('a', 'ok: 1,000.5');
        Probe.Expect('b', 'ok: 1,000.5');
        Probe.Expect('c', 'ok: 1,000.5');
        Probe.Expect('e', 'ok: 1,000.5');
        Probe.Expect('f', 'ok: 1,000.5');
        Probe.Expect('gr', 'ok: 1,000.5');
        Probe.Expect('v', 'ok: 1,000.5');
        Probe.Verify();
    end;

    [Test]
    procedure DecNeg125_Txt()
    var
        Probe: Codeunit "SFK Probe";
        X: Decimal;
    begin
        X := -1.25;
        Probe.CellsDec(3, X);
        Probe.Expect('a', 'ok: -1.25');
        Probe.Expect('b', 'ok: -1.25');
        Probe.Expect('c', 'ok: -1.25');
        Probe.Expect('e', 'ok: -1.25');
        Probe.Expect('f', 'ok: -1.25');
        Probe.Expect('gr', 'ok: -1.25');
        Probe.Expect('v', 'ok: -1.25');
        Probe.Verify();
    end;

    [Test]
    procedure BoolTrue_Txt()
    var
        Probe: Codeunit "SFK Probe";
        X: Boolean;
    begin
        X := true;
        Probe.CellsBool(3, X);
        Probe.Expect('a', 'ok: Yes');
        Probe.Expect('b', 'ok: Yes');
        Probe.Expect('c', 'ok: Yes');
        Probe.Expect('e', 'ok: Yes');
        Probe.Expect('f', 'ok: Yes');
        Probe.Expect('gr', 'ok: Yes');
        Probe.Expect('v', 'ok: Yes');
        Probe.Verify();
    end;

}

codeunit 57408 "SFK Text To Number Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Code123_Int()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '123';
        Probe.CellsCode(4, X);
        Probe.Expect('a', 'ok: 123');
        Probe.Expect('b', 'ok: 123');
        Probe.Expect('c', 'ok: 123');
        Probe.Expect('e', 'ok: 123');
        Probe.Expect('f', 'ok: 123');
        Probe.Expect('gr', 'ok: 123');
        Probe.Expect('v', 'ok: 123');
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_Int()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.CellsCode(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "ABC" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "ABC" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "ABC" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "ABC" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "ABC" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "ABC" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "ABC" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_Int()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.CellsTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "XY" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "XY" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "XY" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "XY" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "XY" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "XY" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "XY" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_Int()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '';
        Probe.CellsTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code123_Big()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '123';
        Probe.CellsCode(5, X);
        Probe.Expect('a', 'ok: 123');
        Probe.Expect('b', 'ok: 123');
        Probe.Expect('c', 'ok: 123');
        Probe.Expect('e', 'ok: 123');
        Probe.Expect('f', 'ok: 123');
        Probe.Expect('gr', 'ok: 123');
        Probe.Expect('v', 'ok: 123');
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_Big()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.CellsCode(5, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "ABC" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "ABC" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "ABC" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "ABC" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "ABC" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "ABC" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "ABC" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_Big()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.CellsTxt(5, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "XY" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "XY" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "XY" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "XY" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "XY" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "XY" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "XY" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_Big()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '';
        Probe.CellsTxt(5, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code123_Dec()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '123';
        Probe.CellsCode(6, X);
        Probe.Expect('a', 'ok: 123');
        Probe.Expect('b', 'ok: 123');
        Probe.Expect('c', 'ok: 123');
        Probe.Expect('e', 'ok: 123');
        Probe.Expect('f', 'ok: 123');
        Probe.Expect('gr', 'ok: 123');
        Probe.Expect('v', 'ok: 123');
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_Dec()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.CellsCode(6, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "ABC" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "ABC" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "ABC" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "ABC" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "ABC" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "ABC" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "ABC" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_Dec()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.CellsTxt(6, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "XY" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "XY" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "XY" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "XY" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "XY" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "XY" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "XY" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Txt15_Dec()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '1.5';
        Probe.CellsTxt(6, X);
        Probe.Expect('a', 'ok: 1.5');
        Probe.Expect('b', 'ok: 1.5');
        Probe.Expect('c', 'ok: 1.5');
        Probe.Expect('e', 'ok: 1.5');
        Probe.Expect('f', 'ok: 1.5');
        Probe.Expect('gr', 'ok: 1.5');
        Probe.Expect('v', 'ok: 1.5');
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_Dec()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '';
        Probe.CellsTxt(6, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "" can''t be evaluated into type Decimal. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavDecimal is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code1000_Dec()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '1,000';
        Probe.CellsCode(6, X);
        Probe.Expect('a', 'ok: 1,000');
        Probe.Expect('b', 'ok: 1,000');
        Probe.Expect('c', 'ok: 1,000');
        Probe.Expect('e', 'ok: 1,000');
        Probe.Expect('f', 'ok: 1,000');
        Probe.Expect('gr', 'ok: 1,000');
        Probe.Expect('v', 'ok: 1000');
        Probe.Verify();
    end;

}

codeunit 57409 "SFK Other Kinds Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Code123_Bool()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '123';
        Probe.CellsCode(7, X);
        Probe.Expect('a', 'ok: Yes');
        Probe.Expect('b', 'ok: Yes');
        Probe.Expect('c', 'ok: Yes');
        Probe.Expect('e', 'ok: Yes');
        Probe.Expect('f', 'ok: Yes');
        Probe.Expect('gr', 'ok: Yes');
        Probe.Expect('v', 'ok: true');
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_Bool()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.CellsCode(7, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "ABC" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "ABC" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "ABC" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "ABC" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "ABC" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "ABC" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "ABC" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtYes_Bool()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'Yes';
        Probe.CellsTxt(7, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "Yes" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "Yes" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "Yes" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "Yes" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "Yes" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "Yes" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "Yes" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_Bool()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.CellsTxt(7, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "XY" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "XY" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "XY" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "XY" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "XY" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "XY" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "XY" can''t be evaluated into type Boolean. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBoolean is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int1_Bool()
    var
        Probe: Codeunit "SFK Probe";
        X: Integer;
    begin
        X := 1;
        Probe.CellsInt(7, X);
        Probe.Expect('a', 'ok: Yes');
        Probe.Expect('b', 'ok: Yes');
        Probe.Expect('c', 'ok: Yes');
        Probe.Expect('e', 'ok: Yes');
        Probe.Expect('f', 'ok: Yes');
        Probe.Expect('gr', 'ok: Yes');
        Probe.Expect('v', 'ok: true');
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_D()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.FiltersCode(9, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''ABC'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''ABC'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''ABC'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''ABC'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''ABC'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''ABC'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_D_Value()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.ValueCode(9, X);
#if BC28PLUS
        Probe.Expect('v', 'E: The format of the Date ''ABC'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_D()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.FiltersTxt(9, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''XY'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''XY'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''XY'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''XY'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''XY'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''XY'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_D_Value()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.ValueTxt(9, X);
#if BC28PLUS
        Probe.Expect('v', 'E: The format of the Date ''XY'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure CodeABC_G()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := 'ABC';
        Probe.CellsCode(8, X);
        Probe.Expect('a', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('b', 'E: The value "ABC" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('c', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Expect('e', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('f', 'E: The value "ABC" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "ABC" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('v', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Verify();
    end;

    [Test]
    procedure Code123_G()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '123';
        Probe.CellsCode(8, X);
        Probe.Expect('a', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('b', 'E: The value "123" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('c', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Expect('e', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('f', 'E: The value "123" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "123" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('v', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Verify();
    end;

    [Test]
    procedure TxtXY_G()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := 'XY';
        Probe.CellsTxt(8, X);
        Probe.Expect('a', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('b', 'E: The value "XY" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('c', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Expect('e', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
#if BC28PLUS
        Probe.Expect('f', 'E: The value "XY" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "XY" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('v', 'E: Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits. | InvalidGuidFormat');
        Probe.Verify();
    end;

    [Test]
    procedure TxtGuid_G()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '{1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}';
        Probe.CellsTxt(8, X);
        Probe.Expect('a', 'ok: {1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}');
#if BC28PLUS
        Probe.Expect('b', 'E: The value "{1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('c', 'ok: {1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}');
        Probe.Expect('e', 'ok: {1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}');
#if BC28PLUS
        Probe.Expect('f', 'E: The value "{1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "{1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}" can''t be evaluated into type GUID. | Evaluate:Argument');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Invalid Conversion. Source: NavVariant, Destination: NavGuid. | Argument');
#endif
        Probe.Expect('v', 'ok: {1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}');
        Probe.Verify();
    end;

}

// Neighbor copy only (not in corpus PR 1552): the probe tests that run route v on a value al-runner refuses there, moved
// into a codeunit of their own so that a refusal does not hide the failure of another test in the same codeunit.
codeunit 57412 "SFK Route V Refused Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Txt15_Int()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '1.5';
        Probe.CellsTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "1.5" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code1000_Int()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '1,000';
        Probe.CellsCode(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "1,000" can''t be evaluated into type Integer. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Txt15_Big()
    var
        Probe: Codeunit "SFK Probe";
        X: Text;
    begin
        X := '1.5';
        Probe.CellsTxt(5, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "1.5" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code1000_Big()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '1,000';
        Probe.CellsCode(5, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
#if BC28PLUS
        Probe.Expect('v', 'E: The value "1,000" can''t be evaluated into type BigInteger. | Evaluate:Format');
#else // BC 27.5 gives another result
        Probe.Expect('v', 'E: The type NavBigInteger is unknown. | NotSupportedType');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code0_Bool()
    var
        Probe: Codeunit "SFK Probe";
        X: Code[20];
    begin
        X := '0';
        Probe.CellsCode(7, X);
        Probe.Expect('a', 'ok: No');
        Probe.Expect('b', 'ok: No');
        Probe.Expect('c', 'ok: No');
        Probe.Expect('e', 'ok: No');
        Probe.Expect('f', 'ok: No');
        Probe.Expect('gr', 'ok: No');
        Probe.Expect('v', 'ok: false');
        Probe.Verify();
    end;

    [Test]
    procedure Int0_Bool()
    var
        Probe: Codeunit "SFK Probe";
        X: Integer;
    begin
        X := 0;
        Probe.CellsInt(7, X);
        Probe.Expect('a', 'ok: No');
        Probe.Expect('b', 'ok: No');
        Probe.Expect('c', 'ok: No');
        Probe.Expect('e', 'ok: No');
        Probe.Expect('f', 'ok: No');
        Probe.Expect('gr', 'ok: No');
        Probe.Expect('v', 'ok: false');
        Probe.Verify();
    end;
}
