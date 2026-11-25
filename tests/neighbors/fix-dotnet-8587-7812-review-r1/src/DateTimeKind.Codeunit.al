dotnet
{
    assembly("mscorlib")
    {
        type("System.TimeZoneInfo"; RrTimeZoneInfo) { }
        type("System.DateTime"; RrDateTime) { }
    }
}

// GetUtcOffset of an AL DateTime from a record field and from CurrentDateTime: the entry measured a parameter only, so both are
// refused. One codeunit each, so a refusal does not stop the other.
codeunit 50327 "Review R1 Zone Origin Tests"
{
    Subtype = Test;

    local procedure SpringMoment(): DateTime
    var
        D: Date;
    begin
        // The last Sunday of March of the session clock's year, 01:00 UTC: the W. Europe spring transition.
        D := CalcDate('<CM>', DMY2Date(1, 3, Date2DMY(Today, 3)));
        while Date2DWY(D, 1) <> 7 do
            D := D - 1;
        exit(CreateDateTime(D, 010000T));
    end;

    [Test]
    procedure Argument_RecordField()
    var
        Buf: Record "Review R1 Buffer" temporary;
        Zone: DotNet RrTimeZoneInfo;
        D: Duration;
    begin
        Buf."Entry No." := 1;
        Buf.At := SpringMoment();
        Buf.Insert();
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        D := Zone.GetUtcOffset(Buf.At);
        if D div 60000 <> 120 then
            Error('field: offset %1 min', D div 60000);
    end;

}

codeunit 50333 "Review R1 Current Tests"
{
    Subtype = Test;

    [Test]
    procedure Argument_CurrentDateTime()
    var
        Zone: DotNet RrTimeZoneInfo;
        D: Duration;
        Same: Duration;
        At: DateTime;
    begin
        Zone := Zone.FindSystemTimeZoneById('W. Europe Standard Time');
        At := CurrentDateTime;
        D := Zone.GetUtcOffset(CurrentDateTime);
        Same := Zone.GetUtcOffset(At);
        if D <> Same then
            Error('current: offset %1, variable %2', D, Same);
    end;
}

// The members of a DotNet DateTime that read its kind: BC gives one assigned from an AL DateTime the kind Utc
// (corpus pull request 1582, kind_2023-01-15T12:00:00Z); the runner does not track it, so they are refused.
codeunit 50328 "Review R1 Kind Tests"
{
    Subtype = Test;

    [Test]
    procedure DotNetKindFromAl()
    var
        Dn: DotNet RrDateTime;
    begin
        Dn := CreateDateTime(DMY2Date(15, 1, 2023), 120000T);
        if Format(Dn.Kind) <> 'Utc' then
            Error('kind %1', Format(Dn.Kind));
    end;
}

codeunit 50329 "Review R1 RoundTrip Tests"
{
    Subtype = Test;

    [Test]
    procedure DotNetRoundTripFromAl()
    var
        Dn: DotNet RrDateTime;
    begin
        Dn := CreateDateTime(DMY2Date(15, 1, 2023), 120000T);
        if Dn.ToString('o') <> '2023-01-15T12:00:00.0000000Z' then
            Error('round trip %1', Dn.ToString('o'));
    end;
}
