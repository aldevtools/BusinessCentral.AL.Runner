dotnet
{
    assembly("mscorlib")
    {
        type("System.TimeZoneInfo"; NbTimeZoneInfo) { }
        type("System.DateTime"; NbDateTime) { }
    }
}

// The dates are of the session clock's year: off Windows AL Runner serves TimeZoneInfo.GetUtcOffset only for those.
codeunit 50310 "TZ Utc Instant Tests"
{
    Subtype = Test;

    local procedure Year(): Integer
    begin
        exit(Date2DMY(Today, 3));
    end;

    local procedure LastSunday(Month: Integer): Date
    var
        D: Date;
    begin
        D := CalcDate('<CM>', DMY2Date(1, Month, Year()));
        while Date2DWY(D, 1) <> 7 do
            D := D - 1;
        exit(D);
    end;

    local procedure NthSunday(N: Integer; Month: Integer): Date
    var
        D: Date;
    begin
        D := DMY2Date(1, Month, Year());
        while Date2DWY(D, 1) <> 7 do
            D := D + 1;
        exit(D + 7 * (N - 1));
    end;

    local procedure Offset(ZoneId: Text; At: DateTime): Integer
    var
        Zone: DotNet NbTimeZoneInfo;
        D: Duration;
    begin
        Zone := Zone.FindSystemTimeZoneById(ZoneId);
        D := Zone.GetUtcOffset(At);
        exit(D div 60000);
    end;

    local procedure Dst(ZoneId: Text; At: DateTime): Boolean
    var
        Zone: DotNet NbTimeZoneInfo;
    begin
        Zone := Zone.FindSystemTimeZoneById(ZoneId);
        exit(Zone.IsDaylightSavingTime(At));
    end;

    local procedure Check(Got: Integer; Expected: Integer; What: Text)
    begin
        if Got <> Expected then
            Error('%1: offset %2 min, expected %3', What, Got, Expected);
    end;

    // The reporter's eight moments (al-runner issue 8587), W. Europe, in the clock's year.
    [Test]
    procedure WestEurope_Transitions()
    var
        Spring: Date;
        Autumn: Date;
        Z: Text;
    begin
        Z := 'W. Europe Standard Time';
        Spring := LastSunday(3);
        Autumn := LastSunday(10);
        Check(Offset(Z, CreateDateTime(DMY2Date(15, 1, Year()), 120000T)), 60, 'winter');
        Check(Offset(Z, CreateDateTime(DMY2Date(15, 7, Year()), 120000T)), 120, 'summer');
        Check(Offset(Z, CreateDateTime(Spring, 005959T)), 60, 'spring before');
        Check(Offset(Z, CreateDateTime(Spring, 010000T)), 120, 'spring at');
        Check(Offset(Z, CreateDateTime(Spring, 010001T)), 120, 'spring after');
        Check(Offset(Z, CreateDateTime(Autumn, 005959T)), 120, 'autumn before');
        Check(Offset(Z, CreateDateTime(Autumn, 010000T)), 60, 'autumn at');
        Check(Offset(Z, CreateDateTime(Autumn, 010001T)), 60, 'autumn after');
    end;

    // A plain winter moment: main answered it right too; head flags it.
    [Test]
    procedure WestEurope_Winter()
    begin
        Check(Offset('W. Europe Standard Time', CreateDateTime(DMY2Date(15, 1, Year()), 120000T)), 60, 'winter');
    end;

    // Southern hemisphere: AUS Eastern leaves summer time on the first Sunday of April at 16:00 UTC of the Saturday,
    // and enters it on the first Sunday of October at 16:00 UTC of the Saturday.
    [Test]
    procedure AusEastern_Transitions()
    var
        Z: Text;
    begin
        Z := 'AUS Eastern Standard Time';
        Check(Offset(Z, CreateDateTime(NthSunday(1, 4) - 1, 155959T)), 660, 'april before');
        Check(Offset(Z, CreateDateTime(NthSunday(1, 4) - 1, 160000T)), 600, 'april at');
        Check(Offset(Z, CreateDateTime(NthSunday(1, 10) - 1, 155959T)), 600, 'october before');
        Check(Offset(Z, CreateDateTime(NthSunday(1, 10) - 1, 160000T)), 660, 'october at');
    end;

    // Pacific: second Sunday of March 10:00 UTC, first Sunday of November 09:00 UTC.
    [Test]
    procedure Pacific_Transitions()
    var
        Z: Text;
    begin
        Z := 'Pacific Standard Time';
        Check(Offset(Z, CreateDateTime(NthSunday(2, 3), 095959T)), -480, 'march before');
        Check(Offset(Z, CreateDateTime(NthSunday(2, 3), 100000T)), -420, 'march at');
        Check(Offset(Z, CreateDateTime(NthSunday(1, 11), 085959T)), -420, 'november before');
        Check(Offset(Z, CreateDateTime(NthSunday(1, 11), 090000T)), -480, 'november at');
    end;

    local procedure SpringMoment(): DateTime
    begin
        exit(CreateDateTime(LastSunday(3), 010000T));
    end;

    // A DotNet DateTime variable that got an AL DateTime: BC holds a .NET DateTime whose kind is not measured.
    [Test]
    procedure Argument_DotNetVariableFromAl()
    var
        Zone: DotNet NbTimeZoneInfo;
        Dn: DotNet NbDateTime;
        D: Duration;
    begin
        Dn := SpringMoment();
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(Dn);
        Check(D div 60000, 120, 'dotnet variable');
    end;

    // A DateTime made in .NET (kind Unspecified there): BC reads it as the zone's clock time.
    [Test]
    procedure Argument_DotNetMade()
    var
        Zone: DotNet NbTimeZoneInfo;
        Dn: DotNet NbDateTime;
        D: Duration;
        S: Date;
    begin
        S := LastSunday(3);
        Dn := Dn.DateTime(Year(), Date2DMY(S, 2), Date2DMY(S, 1), 1, 0, 0);
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(Dn);
        Check(D div 60000, 60, 'dotnet made');
    end;

    // The UTC zone: 0 at any moment (in process, main's answer).
    [Test]
    procedure UtcZone()
    begin
        Check(Offset('UTC', SpringMoment()), 0, 'utc');
    end;

}

// Members that off Windows the policy refuses; on Windows they reach .NET. Kept apart so their refusal does not stop
// the tests above.
codeunit 50311 "TZ Kind Members Tests"
{
    Subtype = Test;

    local procedure Year(): Integer
    begin
        exit(Date2DMY(Today, 3));
    end;

    local procedure LastSunday(Month: Integer): Date
    var
        D: Date;
    begin
        D := CalcDate('<CM>', DMY2Date(1, Month, Year()));
        while Date2DWY(D, 1) <> 7 do
            D := D - 1;
        exit(D);
    end;

    local procedure SpringMoment(): DateTime
    begin
        exit(CreateDateTime(LastSunday(3), 010000T));
    end;

    local procedure Dst(ZoneId: Text; At: DateTime): Boolean
    var
        Zone: DotNet NbTimeZoneInfo;
    begin
        Zone := Zone.FindSystemTimeZoneById(ZoneId);
        exit(Zone.IsDaylightSavingTime(At));
    end;

    // IsDaylightSavingTime at the W. Europe transitions (System App "Time Zone".IsDaylightSavingTime calls it).
    [Test]
    procedure WestEurope_IsDaylightSavingTime()
    var
        Z: Text;
    begin
        Z := 'W. Europe Standard Time';
        if Dst(Z, CreateDateTime(LastSunday(3), 005959T)) then
            Error('spring before');
        if not Dst(Z, CreateDateTime(LastSunday(3), 010000T)) then
            Error('spring at');
        if not Dst(Z, CreateDateTime(LastSunday(10), 005959T)) then
            Error('autumn before');
        if Dst(Z, CreateDateTime(LastSunday(10), 010000T)) then
            Error('autumn at');
    end;

    // Another member that reads a DateTime's kind: refused.
    [Test]
    procedure ConvertTimeFromUtc_AlDateTime()
    var
        Zone: DotNet NbTimeZoneInfo;
        R: DateTime;
    begin
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        R := Zone.ConvertTimeFromUtc(SpringMoment(), Zone);
        if R <> SpringMoment() + 2 * 3600000 then
            Error('converted %1', R);
    end;

    [Test]
    procedure IsAmbiguousTime_AlDateTime()
    var
        Zone: DotNet NbTimeZoneInfo;
    begin
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        if Zone.IsAmbiguousTime(SpringMoment()) then
            Error('ambiguous');
    end;
}

// The argument is the result of a builtin other than CreateDateTime and CurrentDateTime: its origin is not read, so
// refused. Its own codeunit, so the refusal does not stop other tests.
codeunit 50312 "TZ Other Builtin Tests"
{
    Subtype = Test;

    [Test]
    procedure Argument_RoundDateTimeResult()
    var
        Zone: DotNet NbTimeZoneInfo;
        D: Duration;
        S: Date;
    begin
        S := CalcDate('<CM>', DMY2Date(1, 3, Date2DMY(Today, 3)));
        while Date2DWY(S, 1) <> 7 do
            S := S - 1;
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(RoundDateTime(CreateDateTime(S, 010000T), 1000));
        if D div 60000 <> 120 then
            Error('round: offset %1 min', D div 60000);
    end;
}

// The argument is a Date: refused (the kind BC gives it is not measured). Its own codeunit.
codeunit 50313 "TZ Date Argument Tests"
{
    Subtype = Test;

    [Test]
    procedure Argument_Date()
    var
        Zone: DotNet NbTimeZoneInfo;
        D: Duration;
    begin
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(DMY2Date(15, 1, Date2DMY(Today, 3)));
        if D div 60000 <> 60 then
            Error('date: offset %1 min', D div 60000);
    end;
}

// Helpers of the refused-origin codeunits below.
codeunit 50314 "TZ Origin Help"
{
    var
        GlobalMoment: DateTime;

    procedure LastSunday(Month: Integer): Date
    var
        D: Date;
    begin
        D := CalcDate('<CM>', DMY2Date(1, Month, Date2DMY(Today, 3)));
        while Date2DWY(D, 1) <> 7 do
            D := D - 1;
        exit(D);
    end;

    procedure SpringMoment(): DateTime
    begin
        exit(CreateDateTime(LastSunday(3), 010000T));
    end;

    procedure SetGlobal()
    begin
        GlobalMoment := SpringMoment();
    end;

    procedure OffsetOfGlobal(): Integer
    var
        Zone: DotNet NbTimeZoneInfo;
        D: Duration;
    begin
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(GlobalMoment);
        exit(D div 60000);
    end;

    procedure OffsetOfProcedureResult(): Integer
    var
        Zone: DotNet NbTimeZoneInfo;
        D: Duration;
    begin
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(SpringMoment());
        exit(D div 60000);
    end;

    procedure OffsetOfBuiltinResult(): Integer
    var
        Zone: DotNet NbTimeZoneInfo;
        D: Duration;
    begin
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(CreateDateTime(LastSunday(3), 010000T));
        exit(D div 60000);
    end;
}

// A DateTime that is no parameter (a procedure result, a global variable, CreateDateTime passed directly): the entry
// measured the parameter only, so each is refused. One codeunit per origin, so a refusal does not stop another test.
codeunit 50315 "TZ Procedure Result Tests"
{
    Subtype = Test;

    [Test]
    procedure Argument_ProcedureResult()
    var
        Help: Codeunit "TZ Origin Help";
    begin
        if Help.OffsetOfProcedureResult() <> 120 then
            Error('procedure result');
    end;
}

codeunit 50316 "TZ Global Variable Tests"
{
    Subtype = Test;

    [Test]
    procedure Argument_GlobalVariable()
    var
        Help: Codeunit "TZ Origin Help";
    begin
        Help.SetGlobal();
        if Help.OffsetOfGlobal() <> 120 then
            Error('global');
    end;
}

codeunit 50317 "TZ Builtin Result Tests"
{
    Subtype = Test;

    [Test]
    procedure Argument_BuiltinResult()
    var
        Help: Codeunit "TZ Origin Help";
    begin
        if Help.OffsetOfBuiltinResult() <> 120 then
            Error('builtin');
    end;
}
