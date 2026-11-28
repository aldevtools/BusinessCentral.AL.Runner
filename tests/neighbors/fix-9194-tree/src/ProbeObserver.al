// The observe-only helper of the probe workflow: a probe test calls Observe / ObserveError for every question and Finish() at the end;
// Finish raises Error('OBSERVE:' + name, value pairs) so the failure message carries the values BC produced. A script then turns each test's
// Finish() into Assert.AreEqual on the observed values.
//   pair separator: U+241E, name/value separator: U+241F (printable, so the failure message stays valid XML)
//   a value is the text as given with CR, LF and TAB written as \r, \n and \t and the two separators as ?
//   ObserveError(Name) records GetLastErrorText() as Name|text and GetLastErrorCode() as Name|code, then clears the last error.
// A failure message keeps about 2000 characters: Finish() raises at most 1800 characters of pairs, ends a cut message with the marker
// <MORE> and keeps the rest in the Observer variable of the test codeunit; a test named <Test>_More1, _More2 ... that calls More() raises the next
// piece (the tests of a codeunit run in one session, in declaration order). Not a test codeunit.
codeunit 69160 "Probe Observer"
{
    var
        Pairs: Text;
        Expected: Text;
        Ignored: List of [Text];
        Remaining: Text;

    procedure Observe(ObservedName: Text; ObservedValue: Text)
    var
        Rs: Text[1];
        Us: Text[1];
        Cr: Text[1];
        Lf: Text[1];
        Tab: Text[1];
        Escaped: Text;
    begin
        Rs[1] := 9246;
        Us[1] := 9247;
        Cr[1] := 13;
        Lf[1] := 10;
        Tab[1] := 9;
        Escaped := Scrub(ObservedValue).Replace(Rs, '?').Replace(Us, '?').Replace(Cr, '\r').Replace(Lf, '\n').Replace(Tab, '\t');
        Escaped := EscapeControl(Escaped);
        Pairs += ObservedName + Us + Escaped + Rs;
    end;

    // Control characters (and the non characters U+FFFE / U+FFFF) are not valid in the junit XML the runner writes: show them as \uXXXX.
    local procedure EscapeControl(Value: Text): Text
    var
        i: Integer;
        Code: Integer;
        Result: Text;
        Hex: Text;
    begin
        Hex := '0123456789ABCDEF';
        for i := 1 to StrLen(Value) do begin
            Code := Value[i];
            if (Code < 32) or (Code = 65534) or (Code = 65535) or ((Code >= 55296) and (Code <= 57343)) then
                Result += '\u' + CopyStr(Hex, (Code div 4096) + 1, 1) + CopyStr(Hex, ((Code div 256) mod 16) + 1, 1) + CopyStr(Hex, ((Code div 16) mod 16) + 1, 1) + CopyStr(Hex, (Code mod 16) + 1, 1)
            else
                Result += CopyStr(Value, i, 1);
        end;
        exit(Result);
    end;

    // Replaces what changes from run to run (the app version, the run time, the company id, the runner) by a fixed placeholder, so an
    // observed text can be asserted: <ExtensionVersion>, <DateAndTime>, <CompanyId>, <Year>, <MonthNumber>, <DayNumber>, <Hour>, <Minute>,
    // <UserName>, <TenantEntraId>, <EnvironmentName>, <EnvironmentType>, <CompanyName> and <CompanyDisplayName> elements of a report dataset header, and the app version at the end of a call stack frame.
    local procedure Scrub(Value: Text): Text
    begin
        Value := ReplaceBetween(Value, '<ExtensionVersion>', '</ExtensionVersion>', 'x');
        Value := ReplaceBetween(Value, '<DateAndTime>', '</DateAndTime>', 'x');
        Value := ReplaceBetween(Value, '<CompanyId>', '</CompanyId>', 'x');
        Value := ReplaceBetween(Value, '<Year>', '</Year>', 'x');
        Value := ReplaceBetween(Value, '<MonthNumber>', '</MonthNumber>', 'x');
        Value := ReplaceBetween(Value, '<DayNumber>', '</DayNumber>', 'x');
        Value := ReplaceBetween(Value, '<Hour>', '</Hour>', 'x');
        Value := ReplaceBetween(Value, '<Minute>', '</Minute>', 'x');
        Value := ReplaceBetween(Value, '<UserName>', '</UserName>', 'x');
        Value := ReplaceBetween(Value, '<TenantEntraId>', '</TenantEntraId>', 'x');
        Value := ReplaceBetween(Value, '<EnvironmentName>', '</EnvironmentName>', 'x');
        Value := ReplaceBetween(Value, '<EnvironmentType>', '</EnvironmentType>', 'x');
        Value := ReplaceBetween(Value, '<CompanyName>', '</CompanyName>', 'x');
        Value := ReplaceBetween(Value, '<CompanyDisplayName>', '</CompanyDisplayName>', 'x');
        Value := ReplaceBetween(Value, 'AL Language Coverage Tests by AL Language version ', '\', 'x');
        Value := ReplaceBetween(Value, '<BCReportInformation>', '</BCReportInformation>', 'x');
        Value := ReplaceBetween(Value, 'Correlation ID: ', '\', 'x');
        Value := ReplaceBetween(Value, 'with ID = ', ' ', 'n');
        Value := ReplaceBetween(Value, 'compiled with emit version ', '.', 'n');
        Value := ReplaceBetween(Value, 'could not be found in app ''AL Language Coverage Tests by AL Language ', '''', 'x');
        if CompanyName() <> '' then begin
            Value := Value.Replace(CompanyName(), '<Company>');
            Value := Value.Replace(CompanyName().Replace(' ', '%20'), '<Company>');
        end;
        Value := ReplaceBetween(Value, 'Identification fields and values: {', '}', 'x');
        Value := ReplaceBetween(Value, 'App Package ID={', '}', 'x');
        Value := ReplaceBetween(Value, 'App Runtime Package ID={', '}', 'x');
        Value := ReplaceBetween(Value, 'Originating Package ID={', '}', 'x');
        Value := ReplaceBetween(Value, 'Link ID=', ' |', 'x');
        Value := ReplaceBetween(Value, 'Created=', ' |', 'x');
        exit(Value);
    end;

    // Every occurrence of Start ... End becomes Start + With + End (End = '\' also matches the end of the text).
    local procedure ReplaceBetween(Value: Text; StartTag: Text; EndTag: Text; Replacement: Text): Text
    var
        Result: Text;
        Rest: Text;
        StartAt: Integer;
        EndAt: Integer;
    begin
        Rest := Value;
        while true do begin
            StartAt := Rest.IndexOf(StartTag);
            if StartAt = 0 then
                break;
            Result += Rest.Substring(1, StartAt + StrLen(StartTag) - 1);
            Rest := Rest.Substring(StartAt + StrLen(StartTag));
            EndAt := Rest.IndexOf(EndTag);
            if EndAt = 0 then begin
                Result += Replacement;
                Rest := '';
                break;
            end;
            Result += Replacement;
            Rest := Rest.Substring(EndAt);
        end;
        exit(Result + Rest);
    end;

    procedure ObserveError(ObservedName: Text)
    begin
        Observe(ObservedName + '|text', GetLastErrorText());
        Observe(ObservedName + '|code', GetLastErrorCode());
        ClearLastError();
    end;

    // The final form of an observed test: the script that turns Finish() into Expect(...) calls and Verify() writes the values BC produced.
    procedure Expect(ExpectedName: Text; ExpectedValue: Text)
    var
        Rs: Text[1];
        Us: Text[1];
    begin
        Rs[1] := 9246;
        Us[1] := 9247;
        Expected += ExpectedName + Us + ExpectedValue + Rs;
    end;

    local procedure KeepAsserted(All: List of [Text]) Kept: List of [Text]
    var
        Pair: Text;
        Us: Text[1];
    begin
        Us[1] := 9247;
        foreach Pair in All do
            if not Ignored.Contains(CopyStr(Pair, 1, StrPos(Pair + Us, Us) - 1)) then
                Kept.Add(Pair);
    end;

    // Names of observations that are computed but not asserted (a dump, environment data).
    procedure Ignore(IgnoredName: Text)
    begin
        Ignored.Add(IgnoredName);
    end;

    procedure Verify()
    var
        Rs: Text[1];
        Us: Text[1];
        Got: Text;
        Want: Text;
        GotPair: Text;
        WantPair: Text;
        GotList: List of [Text];
        WantList: List of [Text];
        Index: Integer;
    begin
        Rs[1] := 9246;
        Us[1] := 9247;
        Got := Pairs;
        Want := Expected;
        Pairs := '';
        Expected := '';
        if (Ignored.Count() = 0) and (Got = Want) then
            exit;
        GotList := KeepAsserted(Got.Split(Rs));
        Clear(Ignored);
        WantList := Want.Split(Rs);
        for Index := 1 to WantList.Count() do begin
            WantPair := WantList.Get(Index);
            if Index > GotList.Count() then
                Error('observation %1 missing: expected [%2]', Index, WantPair.Replace(Us, ' = '));
            GotPair := GotList.Get(Index);
            if GotPair <> WantPair then
                Error('observation %1: expected [%2] but was [%3]', Index, Around(WantPair.Replace(Us, ' = '), GotPair.Replace(Us, ' = ')), Around(GotPair.Replace(Us, ' = '), WantPair.Replace(Us, ' = ')));
        end;
        if GotList.Count() > WantList.Count() then
            Error('%1 more observation(s) than expected, the first [%2]', GotList.Count() - WantList.Count(), GotList.Get(WantList.Count() + 1).Replace(Us, ' = '));
    end;

    // The part of Value around the first character that differs from Other (a long observation would be cut by the message limit).
    local procedure Around(Value: Text; Other: Text): Text
    var
        At: Integer;
    begin
        if (StrLen(Value) <= 300) and (StrLen(Other) <= 300) then
            exit(Value);
        At := 1;
        while (At <= StrLen(Value)) and (At <= StrLen(Other)) and (Value[At] = Other[At]) do
            At += 1;
        exit('(at ' + Format(At) + ') ' + CopyStr(Value, Maximum(1, At - 100), 300));
    end;

    local procedure Maximum(A: Integer; B: Integer): Integer
    begin
        if A > B then
            exit(A);
        exit(B);
    end;

    // For a test whose values depend on the tier (they are recorded, not asserted).
    procedure Discard()
    begin
        Clear(Ignored);
        Pairs := '';
        Expected := '';
    end;

    procedure Finish()
    var
        State: Codeunit "Probe Observer State";
    begin
        Remaining := Pairs;
        Pairs := '';
        State.Raise('OBSERVE', Remaining);
    end;

    procedure More()
    var
        State: Codeunit "Probe Observer State";
    begin
        State.Raise('OBSERVEMORE', Remaining);
    end;
}

// Plain helper (not SingleInstance: the rest of a long observation lives in the Observer variable of the test codeunit, so no state is shared
// between test codeunits).
codeunit 69163 "Probe Observer State"
{
    procedure Raise(Prefix: Text; var Rest: Text)
    var
        Piece: Text;
    begin
        Piece := CopyStr(Rest, 1, 1800);
        Rest := CopyStr(Rest, 1801);
        if Rest <> '' then
            Error('%1:%2<MORE>', Prefix, Piece);
        Error('%1:%2', Prefix, Piece);
    end;
}

// Kept from an earlier version of Finish() that stored the text here (a test run rolls the rows back, so it never worked); removing it
// would be a destructive schema change for databases that have the earlier app version.
table 69162 "Probe Observer Entry"
{
    DataClassification = SystemMetadata;
    ObsoleteState = Pending;
    ObsoleteReason = 'Not used.';
    ObsoleteTag = '1.0';

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Content; Blob) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}
