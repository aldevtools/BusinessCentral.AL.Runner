// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/fieldref/fieldref-setrange-method
// Scope: in-scope
// Fixtures used: Probe Observer (69160); own enum 57400, table 57400, helper codeunit 57401, test codeunits 57402..57405
// Probe: every literal asserted below was observed by the corpus CI on BC 27.5, 28.5 and 29.0; #if BC28PLUS marks the results BC 27.5 gives
//   differently.
// Facts: FieldRef.SetRange, FieldRef.SetFilter('%1', ...), Record.SetRange and Record.SetFilter('%1', ...) on a Date, Time and DateTime
//   field with a value of another type (and of the field's own type, for comparison). Sources: Integer 20240101 (literal and variable),
//   Integer 0, Integer 120000, BigInteger 20240101L, Decimal 2.5, Boolean true, Text '20240101' (literal and variable), '2024-01-01',
//   '01/01/24', '10:00:00 AM', 'abc', '' (empty), Code 'C1', a fixed Guid, Duration 1500, an Option member (ordinal 1), an Enum value
//   (ordinal 1), Date 20240101D, Time 100000T, DateTime CreateDateTime(20240101D, 100000T), 0D, 0T and 0DT.
//   Routes: a FieldRef.SetRange(X) typed; b FieldRef.SetRange(V), V a Variant holding X; c FieldRef.SetRange(X, X);
//   d1 FieldRef.SetRange(<valid value of the field type>, X); d2 FieldRef.SetRange(X, <valid value>); e FieldRef.SetFilter('%1', X);
//   f FieldRef.SetFilter('%1', V); gr Record.SetRange(Rec.<field>, V); gf Record.SetFilter(Rec.<field>, '%1', X). The Integer and Text
//   literal sources add aLit, cLit and eLit (the literal written in the call). Each cell runs in a TryFunction on a fresh RecordRef or
//   Record and records "ok: <GetFilter of the field>" or "E: <GetLastErrorText> | <GetLastErrorCode>". A test is <Source>_<Field>_<Group>:
//   group Typed (a, c, e), Pair (d1, d2, gf), Var (b, f, gr).
//   Not probed (does not compile, error AL0193 on every leg): Record.SetRange(Rec.<field>, X) and Record.SetFilter(Rec.<field>, '%1', X)
//   with a typed X of another type than the field (also Date into DateTime or Time, and DateTime into Date); route gf runs only for
//   Date into D, Time into T and DateTime into DT.
//   Asked by al-runner issue 8857.
// BC versions: 27, 28, 29 by CI
enum 57400 "SRC Enum"
{
    Extensible = true;
    value(0; Zero) { }
    value(1; One) { }
    value(2; Two) { }
}

table 57400 "SRC Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; D; Date) { }
        field(3; T; Time) { }
        field(4; DT; DateTime) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 57401 "SRC Probe"
{
    // Each test declares its own "SRC Probe" variable, so every test starts with an empty observer.
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

    local procedure RecFilter(var R: Record "SRC Row"; FieldNo: Integer): Text
    begin
        case FieldNo of
            2:
                exit(R.GetFilter(D));
            3:
                exit(R.GetFilter(T));
            4:
                exit(R.GetFilter(DT));
        end;
    end;

    [TryFunction]
    local procedure TryInt(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Integer)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellInt(Route: Text; FieldNo: Integer; X: Integer)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryInt(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedInt(FieldNo: Integer; X: Integer)
    begin
        CellInt('a', FieldNo, X);
        CellInt('c', FieldNo, X);
        CellInt('e', FieldNo, X);
    end;

    procedure PairInt(FieldNo: Integer; X: Integer)
    begin
        CellInt('d1', FieldNo, X);
        CellInt('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryBig(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: BigInteger)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellBig(Route: Text; FieldNo: Integer; X: BigInteger)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryBig(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedBig(FieldNo: Integer; X: BigInteger)
    begin
        CellBig('a', FieldNo, X);
        CellBig('c', FieldNo, X);
        CellBig('e', FieldNo, X);
    end;

    procedure PairBig(FieldNo: Integer; X: BigInteger)
    begin
        CellBig('d1', FieldNo, X);
        CellBig('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryDec(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Decimal)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellDec(Route: Text; FieldNo: Integer; X: Decimal)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryDec(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedDec(FieldNo: Integer; X: Decimal)
    begin
        CellDec('a', FieldNo, X);
        CellDec('c', FieldNo, X);
        CellDec('e', FieldNo, X);
    end;

    procedure PairDec(FieldNo: Integer; X: Decimal)
    begin
        CellDec('d1', FieldNo, X);
        CellDec('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryBool(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Boolean)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellBool(Route: Text; FieldNo: Integer; X: Boolean)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryBool(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedBool(FieldNo: Integer; X: Boolean)
    begin
        CellBool('a', FieldNo, X);
        CellBool('c', FieldNo, X);
        CellBool('e', FieldNo, X);
    end;

    procedure PairBool(FieldNo: Integer; X: Boolean)
    begin
        CellBool('d1', FieldNo, X);
        CellBool('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryTxt(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Text)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellTxt(Route: Text; FieldNo: Integer; X: Text)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryTxt(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedTxt(FieldNo: Integer; X: Text)
    begin
        CellTxt('a', FieldNo, X);
        CellTxt('c', FieldNo, X);
        CellTxt('e', FieldNo, X);
    end;

    procedure PairTxt(FieldNo: Integer; X: Text)
    begin
        CellTxt('d1', FieldNo, X);
        CellTxt('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryCode(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Code[20])
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellCode(Route: Text; FieldNo: Integer; X: Code[20])
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryCode(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedCode(FieldNo: Integer; X: Code[20])
    begin
        CellCode('a', FieldNo, X);
        CellCode('c', FieldNo, X);
        CellCode('e', FieldNo, X);
    end;

    procedure PairCode(FieldNo: Integer; X: Code[20])
    begin
        CellCode('d1', FieldNo, X);
        CellCode('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryGuid(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Guid)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellGuid(Route: Text; FieldNo: Integer; X: Guid)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryGuid(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedGuid(FieldNo: Integer; X: Guid)
    begin
        CellGuid('a', FieldNo, X);
        CellGuid('c', FieldNo, X);
        CellGuid('e', FieldNo, X);
    end;

    procedure PairGuid(FieldNo: Integer; X: Guid)
    begin
        CellGuid('d1', FieldNo, X);
        CellGuid('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryDur(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Duration)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellDur(Route: Text; FieldNo: Integer; X: Duration)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryDur(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedDur(FieldNo: Integer; X: Duration)
    begin
        CellDur('a', FieldNo, X);
        CellDur('c', FieldNo, X);
        CellDur('e', FieldNo, X);
    end;

    procedure PairDur(FieldNo: Integer; X: Duration)
    begin
        CellDur('d1', FieldNo, X);
        CellDur('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryOpt(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Option Zero,One,Two)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellOpt(Route: Text; FieldNo: Integer; X: Option Zero,One,Two)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryOpt(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedOpt(FieldNo: Integer; X: Option Zero,One,Two)
    begin
        CellOpt('a', FieldNo, X);
        CellOpt('c', FieldNo, X);
        CellOpt('e', FieldNo, X);
    end;

    procedure PairOpt(FieldNo: Integer; X: Option Zero,One,Two)
    begin
        CellOpt('d1', FieldNo, X);
        CellOpt('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryEnum(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Enum "SRC Enum")
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
        end;
    end;

    local procedure CellEnum(Route: Text; FieldNo: Integer; X: Enum "SRC Enum")
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryEnum(R, F, Route, X);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedEnum(FieldNo: Integer; X: Enum "SRC Enum")
    begin
        CellEnum('a', FieldNo, X);
        CellEnum('c', FieldNo, X);
        CellEnum('e', FieldNo, X);
    end;

    procedure PairEnum(FieldNo: Integer; X: Enum "SRC Enum")
    begin
        CellEnum('d1', FieldNo, X);
        CellEnum('d2', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryDate(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Date)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
            'gf':
                R.SetFilter(D, '%1', X);
        end;
    end;

    local procedure CellDate(Route: Text; FieldNo: Integer; X: Date)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryDate(R, F, Route, X);
        if Route = 'gf' then
            Outcome(Route, Ok, RecFilter(R, FieldNo))
        else
            Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedDate(FieldNo: Integer; X: Date)
    begin
        CellDate('a', FieldNo, X);
        CellDate('c', FieldNo, X);
        CellDate('e', FieldNo, X);
    end;

    procedure PairDate(FieldNo: Integer; X: Date)
    begin
        CellDate('d1', FieldNo, X);
        CellDate('d2', FieldNo, X);
        // Record.SetFilter(Rec.<field>, '%1', X) compiles only when X has the field's type.
        if FieldNo = 2 then
            CellDate('gf', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryTime(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: Time)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
            'gf':
                R.SetFilter(T, '%1', X);
        end;
    end;

    local procedure CellTime(Route: Text; FieldNo: Integer; X: Time)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryTime(R, F, Route, X);
        if Route = 'gf' then
            Outcome(Route, Ok, RecFilter(R, FieldNo))
        else
            Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedTime(FieldNo: Integer; X: Time)
    begin
        CellTime('a', FieldNo, X);
        CellTime('c', FieldNo, X);
        CellTime('e', FieldNo, X);
    end;

    procedure PairTime(FieldNo: Integer; X: Time)
    begin
        CellTime('d1', FieldNo, X);
        CellTime('d2', FieldNo, X);
        // Record.SetFilter(Rec.<field>, '%1', X) compiles only when X has the field's type.
        if FieldNo = 3 then
            CellTime('gf', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryDT(var R: Record "SRC Row"; var F: FieldRef; Route: Text; X: DateTime)
    var
        V0: Variant;
    begin
        case F.Number() of
            2:
                V0 := 20240101D;
            3:
                V0 := 100000T;
            4:
                V0 := CreateDateTime(20240101D, 100000T);
        end;
        case Route of
            'a':
                F.SetRange(X);
            'c':
                F.SetRange(X, X);
            'd1':
                F.SetRange(V0, X);
            'd2':
                F.SetRange(X, V0);
            'e':
                F.SetFilter('%1', X);
            'gf':
                R.SetFilter(DT, '%1', X);
        end;
    end;

    local procedure CellDT(Route: Text; FieldNo: Integer; X: DateTime)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryDT(R, F, Route, X);
        if Route = 'gf' then
            Outcome(Route, Ok, RecFilter(R, FieldNo))
        else
            Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    procedure TypedDT(FieldNo: Integer; X: DateTime)
    begin
        CellDT('a', FieldNo, X);
        CellDT('c', FieldNo, X);
        CellDT('e', FieldNo, X);
    end;

    procedure PairDT(FieldNo: Integer; X: DateTime)
    begin
        CellDT('d1', FieldNo, X);
        CellDT('d2', FieldNo, X);
        // Record.SetFilter(Rec.<field>, '%1', X) compiles only when X has the field's type.
        if FieldNo = 4 then
            CellDT('gf', FieldNo, X);
    end;

    [TryFunction]
    local procedure TryVar(var R: Record "SRC Row"; var F: FieldRef; Route: Text; V: Variant)
    begin
        case Route of
            'b':
                F.SetRange(V);
            'f':
                F.SetFilter('%1', V);
            'gr':
                case F.Number() of
                    2:
                        R.SetRange(D, V);
                    3:
                        R.SetRange(T, V);
                    4:
                        R.SetRange(DT, V);
                end;
        end;
    end;

    local procedure CellVar(Route: Text; FieldNo: Integer; V: Variant)
    var
        R: Record "SRC Row";
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        Ok := TryVar(R, F, Route, V);
        if Route = 'gr' then
            Outcome(Route, Ok, RecFilter(R, FieldNo))
        else
            Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    // V is a Variant holding the typed source value.
    procedure Variants(FieldNo: Integer; V: Variant)
    begin
        CellVar('b', FieldNo, V);
        CellVar('f', FieldNo, V);
        CellVar('gr', FieldNo, V);
    end;

    [TryFunction]
    local procedure TryIntLit(var F: FieldRef; Route: Text)
    begin
        case Route of
            'aLit':
                F.SetRange(20240101);
            'cLit':
                F.SetRange(20240101, 20240101);
            'eLit':
                F.SetFilter('%1', 20240101);
        end;
    end;

    [TryFunction]
    local procedure TryTxtLit(var F: FieldRef; Route: Text)
    begin
        case Route of
            'aLit':
                F.SetRange('20240101');
            'cLit':
                F.SetRange('20240101', '20240101');
            'eLit':
                F.SetFilter('%1', '20240101');
        end;
    end;

    local procedure CellLit(Kind: Text; Route: Text; FieldNo: Integer)
    var
        RecRef: RecordRef;
        F: FieldRef;
        Ok: Boolean;
    begin
        RecRef.Open(Database::"SRC Row");
        F := RecRef.Field(FieldNo);
        if Kind = 'Int' then
            Ok := TryIntLit(F, Route)
        else
            Ok := TryTxtLit(F, Route);
        Outcome(Route, Ok, F.GetFilter());
        RecRef.Close();
    end;

    // Kind 'Int': the Integer literal 20240101; 'Txt': the Text literal '20240101'.
    procedure Lit(Kind: Text; FieldNo: Integer)
    begin
        CellLit(Kind, 'aLit', FieldNo);
        CellLit(Kind, 'cLit', FieldNo);
        CellLit(Kind, 'eLit', FieldNo);
    end;
}

codeunit 57402 "SRC Numeric Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure IntLit_D_Lit()
    var
        Probe: Codeunit "SRC Probe";
    begin
        Probe.Lit('Int', 2);
#if BC28PLUS
        Probe.Expect('aLit', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('aLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('cLit', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('cLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('eLit', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('eLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntLit_T_Lit()
    var
        Probe: Codeunit "SRC Probe";
    begin
        Probe.Lit('Int', 3);
#if BC28PLUS
        Probe.Expect('aLit', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('aLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('cLit', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('cLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('eLit', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('eLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntLit_DT_Lit()
    var
        Probe: Codeunit "SRC Probe";
    begin
        Probe.Lit('Int', 4);
#if BC28PLUS
        Probe.Expect('aLit', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('aLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('cLit', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('cLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('eLit', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('eLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 20240101;
        Probe.TypedInt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 20240101;
        Probe.PairInt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 20240101;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 20240101;
        Probe.TypedInt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 20240101;
        Probe.PairInt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 20240101;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 20240101;
        Probe.TypedInt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 20240101;
        Probe.PairInt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntVar_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 20240101;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 0;
        Probe.TypedInt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 0;
        Probe.PairInt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 0;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''0'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 0;
        Probe.TypedInt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 0;
        Probe.PairInt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 0;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "0" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 0;
        Probe.TypedInt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 0;
        Probe.PairInt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure IntZero_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 0;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "0" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 120000;
        Probe.TypedInt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 120000;
        Probe.PairInt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 120000;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''120000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 120000;
        Probe.TypedInt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 120000;
        Probe.PairInt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 120000;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "120000" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 120000;
        Probe.TypedInt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
    begin
        X := 120000;
        Probe.PairInt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Int120000_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Integer;
        V: Variant;
    begin
        X := 120000;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "120000" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
    begin
        X := 20240101L;
        Probe.TypedBig(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
    begin
        X := 20240101L;
        Probe.PairBig(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
        V: Variant;
    begin
        X := 20240101L;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
    begin
        X := 20240101L;
        Probe.TypedBig(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
    begin
        X := 20240101L;
        Probe.PairBig(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
        V: Variant;
    begin
        X := 20240101L;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
    begin
        X := 20240101L;
        Probe.TypedBig(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
    begin
        X := 20240101L;
        Probe.PairBig(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Big_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: BigInteger;
        V: Variant;
    begin
        X := 20240101L;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
    begin
        X := 2.5;
        Probe.TypedDec(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
    begin
        X := 2.5;
        Probe.PairDec(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
        V: Variant;
    begin
        X := 2.5;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''2.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
    begin
        X := 2.5;
        Probe.TypedDec(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
    begin
        X := 2.5;
        Probe.PairDec(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
        V: Variant;
    begin
        X := 2.5;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "2.5" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
    begin
        X := 2.5;
        Probe.TypedDec(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
    begin
        X := 2.5;
        Probe.PairDec(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dec_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Decimal;
        V: Variant;
    begin
        X := 2.5;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "2.5" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
    begin
        X := true;
        Probe.TypedBool(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
    begin
        X := true;
        Probe.PairBool(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
        V: Variant;
    begin
        X := true;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''Yes'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
    begin
        X := true;
        Probe.TypedBool(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
    begin
        X := true;
        Probe.PairBool(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
        V: Variant;
    begin
        X := true;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "Yes" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
    begin
        X := true;
        Probe.TypedBool(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
    begin
        X := true;
        Probe.PairBool(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Bool_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Boolean;
        V: Variant;
    begin
        X := true;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "Yes" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;
}

codeunit 57403 "SRC Text Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TxtLit_D_Lit()
    var
        Probe: Codeunit "SRC Probe";
    begin
        Probe.Lit('Txt', 2);
#if BC28PLUS
        Probe.Expect('aLit', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('aLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('cLit', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('cLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('eLit', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('eLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtLit_T_Lit()
    var
        Probe: Codeunit "SRC Probe";
    begin
        Probe.Lit('Txt', 3);
#if BC28PLUS
        Probe.Expect('aLit', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('aLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('cLit', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('cLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('eLit', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('eLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtLit_DT_Lit()
    var
        Probe: Codeunit "SRC Probe";
    begin
        Probe.Lit('Txt', 4);
#if BC28PLUS
        Probe.Expect('aLit', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('aLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('cLit', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('cLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('eLit', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('eLit', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '20240101';
        Probe.TypedTxt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '20240101';
        Probe.PairTxt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '20240101';
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''20240101'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '20240101';
        Probe.TypedTxt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '20240101';
        Probe.PairTxt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '20240101';
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "20240101" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '20240101';
        Probe.TypedTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '20240101';
        Probe.PairTxt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtDigits_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '20240101';
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "20240101" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '2024-01-01';
        Probe.TypedTxt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '2024-01-01';
        Probe.PairTxt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '2024-01-01';
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''2024-01-01'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '2024-01-01';
        Probe.TypedTxt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '2024-01-01';
        Probe.PairTxt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '2024-01-01';
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "2024-01-01" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '2024-01-01';
        Probe.TypedTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '2024-01-01';
        Probe.PairTxt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtIso_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '2024-01-01';
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "2024-01-01" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '01/01/24';
        Probe.TypedTxt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '01/01/24';
        Probe.PairTxt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '01/01/24';
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''01/01/24'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '01/01/24';
        Probe.TypedTxt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '01/01/24';
        Probe.PairTxt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '01/01/24';
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "01/01/24" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '01/01/24';
        Probe.TypedTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '01/01/24';
        Probe.PairTxt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtUs_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '01/01/24';
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "01/01/24" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '10:00:00 AM';
        Probe.TypedTxt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '10:00:00 AM';
        Probe.PairTxt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '10:00:00 AM';
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''10:00:00 AM'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '10:00:00 AM';
        Probe.TypedTxt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '10:00:00 AM';
        Probe.PairTxt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '10:00:00 AM';
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "10:00:00 AM" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '10:00:00 AM';
        Probe.TypedTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '10:00:00 AM';
        Probe.PairTxt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtTime_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '10:00:00 AM';
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "10:00:00 AM" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := 'abc';
        Probe.TypedTxt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := 'abc';
        Probe.PairTxt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := 'abc';
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''abc'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := 'abc';
        Probe.TypedTxt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := 'abc';
        Probe.PairTxt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := 'abc';
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "abc" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := 'abc';
        Probe.TypedTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := 'abc';
        Probe.PairTxt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtAbc_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := 'abc';
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "abc" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '';
        Probe.TypedTxt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '';
        Probe.PairTxt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '';
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date '''' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '';
        Probe.TypedTxt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '';
        Probe.PairTxt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '';
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '';
        Probe.TypedTxt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
    begin
        X := '';
        Probe.PairTxt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure TxtEmpty_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Text;
        V: Variant;
    begin
        X := '';
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
    begin
        X := 'C1';
        Probe.TypedCode(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
    begin
        X := 'C1';
        Probe.PairCode(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
        V: Variant;
    begin
        X := 'C1';
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''C1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
    begin
        X := 'C1';
        Probe.TypedCode(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
    begin
        X := 'C1';
        Probe.PairCode(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
        V: Variant;
    begin
        X := 'C1';
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "C1" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
    begin
        X := 'C1';
        Probe.TypedCode(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
    begin
        X := 'C1';
        Probe.PairCode(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Code_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Code[20];
        V: Variant;
    begin
        X := 'C1';
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "C1" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;
}

codeunit 57404 "SRC Other Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Guid_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        Probe.TypedGuid(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        Probe.PairGuid(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
        V: Variant;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''{11111111-2222-3333-4444-555555555555}'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        Probe.TypedGuid(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        Probe.PairGuid(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
        V: Variant;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        Probe.TypedGuid(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        Probe.PairGuid(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Guid_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Guid;
        V: Variant;
    begin
        Evaluate(X, '{11111111-2222-3333-4444-555555555555}');
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "{11111111-2222-3333-4444-555555555555}" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavGuid to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
    begin
        X := 1500;
        Probe.TypedDur(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
    begin
        X := 1500;
        Probe.PairDur(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
        V: Variant;
    begin
        X := 1500;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''1 second 500 milliseconds'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
    begin
        X := 1500;
        Probe.TypedDur(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
    begin
        X := 1500;
        Probe.PairDur(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
        V: Variant;
    begin
        X := 1500;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
    begin
        X := 1500;
        Probe.TypedDur(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
    begin
        X := 1500;
        Probe.PairDur(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Dur_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Duration;
        V: Variant;
    begin
        X := 1500;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "1 second 500 milliseconds" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
    begin
        X := X::One;
        Probe.TypedOpt(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
    begin
        X := X::One;
        Probe.PairOpt(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
        V: Variant;
    begin
        X := X::One;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
    begin
        X := X::One;
        Probe.TypedOpt(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
    begin
        X := X::One;
        Probe.PairOpt(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
        V: Variant;
    begin
        X := X::One;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
    begin
        X := X::One;
        Probe.TypedOpt(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
    begin
        X := X::One;
        Probe.PairOpt(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Opt_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Option Zero,One,Two;
        V: Variant;
    begin
        X := X::One;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
    begin
        X := "SRC Enum"::One;
        Probe.TypedEnum(2, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
    begin
        X := "SRC Enum"::One;
        Probe.PairEnum(2, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
        V: Variant;
    begin
        X := "SRC Enum"::One;
        V := X;
        Probe.Variants(2, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The format of the Date ''One'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
    begin
        X := "SRC Enum"::One;
        Probe.TypedEnum(3, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
    begin
        X := "SRC Enum"::One;
        Probe.PairEnum(3, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
        V: Variant;
    begin
        X := "SRC Enum"::One;
        V := X;
        Probe.Variants(3, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "One" can''t be evaluated into type Time. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
    begin
        X := "SRC Enum"::One;
        Probe.TypedEnum(4, X);
#if BC28PLUS
        Probe.Expect('a', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('a', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('c', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('c', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('e', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('e', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
    begin
        X := "SRC Enum"::One;
        Probe.PairEnum(4, X);
#if BC28PLUS
        Probe.Expect('d1', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d1', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('d2', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('d2', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;

    [Test]
    procedure Enum_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Enum "SRC Enum";
        V: Variant;
    begin
        X := "SRC Enum"::One;
        V := X;
        Probe.Variants(4, V);
#if BC28PLUS
        Probe.Expect('b', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('b', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('f', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('f', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
#if BC28PLUS
        Probe.Expect('gr', 'E: The value "One" can''t be evaluated into type DateTime. | Evaluate:Conversion');
#else // BC 27.5 gives another result
        Probe.Expect('gr', 'E: Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavOption to System.DateTime. | Conversion');
#endif
        Probe.Verify();
    end;
}

codeunit 57405 "SRC DateTime Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Date_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 20240101D;
        Probe.TypedDate(2, X);
        Probe.Expect('a', 'ok: 01/01/24');
        Probe.Expect('c', 'ok: 01/01/24');
        Probe.Expect('e', 'ok: 01/01/24');
        Probe.Verify();
    end;

    [Test]
    procedure Date_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 20240101D;
        Probe.PairDate(2, X);
        Probe.Expect('d1', 'ok: 01/01/24');
        Probe.Expect('d2', 'ok: 01/01/24');
        Probe.Expect('gf', 'ok: 01/01/24');
        Probe.Verify();
    end;

    [Test]
    procedure Date_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
        V: Variant;
    begin
        X := 20240101D;
        V := X;
        Probe.Variants(2, V);
        Probe.Expect('b', 'ok: 01/01/24');
        Probe.Expect('f', 'ok: 01/01/24');
        Probe.Expect('gr', 'ok: 01/01/24');
        Probe.Verify();
    end;

    [Test]
    procedure Date_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 20240101D;
        Probe.TypedDate(3, X);
        Probe.Expect('a', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('c', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('e', 'E: The time is not valid. | InvalidTime');
        Probe.Verify();
    end;

    [Test]
    procedure Date_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 20240101D;
        Probe.PairDate(3, X);
        Probe.Expect('d1', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('d2', 'E: The time is not valid. | InvalidTime');
        Probe.Verify();
    end;

    [Test]
    procedure Date_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
        V: Variant;
    begin
        X := 20240101D;
        V := X;
        Probe.Variants(3, V);
        Probe.Expect('b', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('f', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('gr', 'E: The time is not valid. | InvalidTime');
        Probe.Verify();
    end;

    [Test]
    procedure Date_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 20240101D;
        Probe.TypedDate(4, X);
        Probe.Expect('a', 'ok: 01/01/24 12:00 AM');
        Probe.Expect('c', 'ok: 01/01/24 12:00 AM');
        Probe.Expect('e', 'ok: 01/01/24 12:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure Date_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 20240101D;
        Probe.PairDate(4, X);
        Probe.Expect('d1', 'ok: 01/01/24 10:00 AM..01/01/24 12:00 AM');
        Probe.Expect('d2', 'ok: 01/01/24 12:00 AM..01/01/24 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure Date_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
        V: Variant;
    begin
        X := 20240101D;
        V := X;
        Probe.Variants(4, V);
        Probe.Expect('b', 'ok: 01/01/24 12:00 AM');
        Probe.Expect('f', 'ok: 01/01/24 12:00 AM');
        Probe.Expect('gr', 'ok: 01/01/24 12:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 0D;
        Probe.TypedDate(2, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 0D;
        Probe.PairDate(2, X);
        Probe.Expect('d1', 'ok: 01/01/24..''''');
        Probe.Expect('d2', 'ok: ''''..01/01/24');
        Probe.Expect('gf', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
        V: Variant;
    begin
        X := 0D;
        V := X;
        Probe.Variants(2, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 0D;
        Probe.TypedDate(3, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 0D;
        Probe.PairDate(3, X);
        Probe.Expect('d1', 'ok: 10:00:00 AM..''''');
        Probe.Expect('d2', 'ok: ''''..10:00:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
        V: Variant;
    begin
        X := 0D;
        V := X;
        Probe.Variants(3, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 0D;
        Probe.TypedDate(4, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
    begin
        X := 0D;
        Probe.PairDate(4, X);
        Probe.Expect('d1', 'ok: 01/01/24 10:00 AM..''''');
        Probe.Expect('d2', 'ok: ''''..01/01/24 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure DateZero_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Date;
        V: Variant;
    begin
        X := 0D;
        V := X;
        Probe.Variants(4, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure Time_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 100000T;
        Probe.TypedTime(2, X);
        Probe.Expect('a', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('c', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('e', 'E: The date is not valid. | DateInvalid');
        Probe.Verify();
    end;

    [Test]
    procedure Time_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 100000T;
        Probe.PairTime(2, X);
        Probe.Expect('d1', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('d2', 'E: The date is not valid. | DateInvalid');
        Probe.Verify();
    end;

    [Test]
    procedure Time_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
        V: Variant;
    begin
        X := 100000T;
        V := X;
        Probe.Variants(2, V);
        Probe.Expect('b', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('f', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('gr', 'E: The date is not valid. | DateInvalid');
        Probe.Verify();
    end;

    [Test]
    procedure Time_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 100000T;
        Probe.TypedTime(3, X);
        Probe.Expect('a', 'ok: 10:00:00 AM');
        Probe.Expect('c', 'ok: 10:00:00 AM');
        Probe.Expect('e', 'ok: 10:00:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure Time_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 100000T;
        Probe.PairTime(3, X);
        Probe.Expect('d1', 'ok: 10:00:00 AM');
        Probe.Expect('d2', 'ok: 10:00:00 AM');
        Probe.Expect('gf', 'ok: 10:00:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure Time_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
        V: Variant;
    begin
        X := 100000T;
        V := X;
        Probe.Variants(3, V);
        Probe.Expect('b', 'ok: 10:00:00 AM');
        Probe.Expect('f', 'ok: 10:00:00 AM');
        Probe.Expect('gr', 'ok: 10:00:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure Time_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 100000T;
        Probe.TypedTime(4, X);
        Probe.Expect('a', 'ok: 01/02/0001 10:00 AM');
        Probe.Expect('c', 'ok: 01/02/0001 10:00 AM');
        Probe.Expect('e', 'ok: 01/02/0001 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure Time_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 100000T;
        Probe.PairTime(4, X);
        Probe.Expect('d1', 'ok: 01/01/24 10:00 AM..01/02/0001 10:00 AM');
        Probe.Expect('d2', 'ok: 01/02/0001 10:00 AM..01/01/24 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure Time_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
        V: Variant;
    begin
        X := 100000T;
        V := X;
        Probe.Variants(4, V);
        Probe.Expect('b', 'ok: 01/02/0001 10:00 AM');
        Probe.Expect('f', 'ok: 01/02/0001 10:00 AM');
        Probe.Expect('gr', 'ok: 01/02/0001 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 0T;
        Probe.TypedTime(2, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 0T;
        Probe.PairTime(2, X);
        Probe.Expect('d1', 'ok: 01/01/24..''''');
        Probe.Expect('d2', 'ok: ''''..01/01/24');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
        V: Variant;
    begin
        X := 0T;
        V := X;
        Probe.Variants(2, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 0T;
        Probe.TypedTime(3, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 0T;
        Probe.PairTime(3, X);
        Probe.Expect('d1', 'ok: 10:00:00 AM..''''');
        Probe.Expect('d2', 'ok: ''''..10:00:00 AM');
        Probe.Expect('gf', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
        V: Variant;
    begin
        X := 0T;
        V := X;
        Probe.Variants(3, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 0T;
        Probe.TypedTime(4, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
    begin
        X := 0T;
        Probe.PairTime(4, X);
        Probe.Expect('d1', 'ok: 01/01/24 10:00 AM..''''');
        Probe.Expect('d2', 'ok: ''''..01/01/24 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure TimeZero_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: Time;
        V: Variant;
    begin
        X := 0T;
        V := X;
        Probe.Variants(4, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DT_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := CreateDateTime(20240101D, 100000T);
        Probe.TypedDT(2, X);
        Probe.Expect('a', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('c', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('e', 'E: The date is not valid. | DateInvalid');
        Probe.Verify();
    end;

    [Test]
    procedure DT_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := CreateDateTime(20240101D, 100000T);
        Probe.PairDT(2, X);
        Probe.Expect('d1', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('d2', 'E: The date is not valid. | DateInvalid');
        Probe.Verify();
    end;

    [Test]
    procedure DT_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
        V: Variant;
    begin
        X := CreateDateTime(20240101D, 100000T);
        V := X;
        Probe.Variants(2, V);
        Probe.Expect('b', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('f', 'E: The date is not valid. | DateInvalid');
        Probe.Expect('gr', 'E: The date is not valid. | DateInvalid');
        Probe.Verify();
    end;

    [Test]
    procedure DT_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := CreateDateTime(20240101D, 100000T);
        Probe.TypedDT(3, X);
        Probe.Expect('a', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('c', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('e', 'E: The time is not valid. | InvalidTime');
        Probe.Verify();
    end;

    [Test]
    procedure DT_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := CreateDateTime(20240101D, 100000T);
        Probe.PairDT(3, X);
        Probe.Expect('d1', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('d2', 'E: The time is not valid. | InvalidTime');
        Probe.Verify();
    end;

    [Test]
    procedure DT_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
        V: Variant;
    begin
        X := CreateDateTime(20240101D, 100000T);
        V := X;
        Probe.Variants(3, V);
        Probe.Expect('b', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('f', 'E: The time is not valid. | InvalidTime');
        Probe.Expect('gr', 'E: The time is not valid. | InvalidTime');
        Probe.Verify();
    end;

    [Test]
    procedure DT_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := CreateDateTime(20240101D, 100000T);
        Probe.TypedDT(4, X);
        Probe.Expect('a', 'ok: 01/01/24 10:00 AM');
        Probe.Expect('c', 'ok: 01/01/24 10:00 AM');
        Probe.Expect('e', 'ok: 01/01/24 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure DT_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := CreateDateTime(20240101D, 100000T);
        Probe.PairDT(4, X);
        Probe.Expect('d1', 'ok: 01/01/24 10:00 AM');
        Probe.Expect('d2', 'ok: 01/01/24 10:00 AM');
        Probe.Expect('gf', 'ok: 01/01/24 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure DT_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
        V: Variant;
    begin
        X := CreateDateTime(20240101D, 100000T);
        V := X;
        Probe.Variants(4, V);
        Probe.Expect('b', 'ok: 01/01/24 10:00 AM');
        Probe.Expect('f', 'ok: 01/01/24 10:00 AM');
        Probe.Expect('gr', 'ok: 01/01/24 10:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_D_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := 0DT;
        Probe.TypedDT(2, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_D_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := 0DT;
        Probe.PairDT(2, X);
        Probe.Expect('d1', 'ok: 01/01/24..''''');
        Probe.Expect('d2', 'ok: ''''..01/01/24');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_D_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
        V: Variant;
    begin
        X := 0DT;
        V := X;
        Probe.Variants(2, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_T_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := 0DT;
        Probe.TypedDT(3, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_T_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := 0DT;
        Probe.PairDT(3, X);
        Probe.Expect('d1', 'ok: 10:00:00 AM..''''');
        Probe.Expect('d2', 'ok: ''''..10:00:00 AM');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_T_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
        V: Variant;
    begin
        X := 0DT;
        V := X;
        Probe.Variants(3, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_DT_Typed()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := 0DT;
        Probe.TypedDT(4, X);
        Probe.Expect('a', 'ok: ''''');
        Probe.Expect('c', 'ok: ''''');
        Probe.Expect('e', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_DT_Pair()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
    begin
        X := 0DT;
        Probe.PairDT(4, X);
        Probe.Expect('d1', 'ok: 01/01/24 10:00 AM..''''');
        Probe.Expect('d2', 'ok: ''''..01/01/24 10:00 AM');
        Probe.Expect('gf', 'ok: ''''');
        Probe.Verify();
    end;

    [Test]
    procedure DTZero_DT_Var()
    var
        Probe: Codeunit "SRC Probe";
        X: DateTime;
        V: Variant;
    begin
        X := 0DT;
        V := X;
        Probe.Variants(4, V);
        Probe.Expect('b', 'ok: ''''');
        Probe.Expect('f', 'ok: ''''');
        Probe.Expect('gr', 'ok: ''''');
        Probe.Verify();
    end;
}
