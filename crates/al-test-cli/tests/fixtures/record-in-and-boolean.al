table 50910 "Probe Predicate Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Approved; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50913 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RecordFieldFollowedByBoolean()
    var
        Row: Record "Probe Predicate Row";
        Observed: Text;
        Hits: Integer;
        N: Integer;
    begin
        for N := 0 to 3 do begin
            Row.Id := N;
            Row.Approved := false;
            Observed += Format(Row.Id in [1, 2] and Row.Approved) + '/';
            Observed += Format(Row.Id in [1, 2] and Right(Hits, false)) + '/';
            Row.Approved := true;
            Observed += Format(Row.Id in [1, 2] and Row.Approved) + '/';
            Observed += Format(Row.Id in [1, 2] and Right(Hits, true)) + '|';
        end;
        if (Observed <> 'No/No/No/No|No/No/Yes/Yes|No/No/Yes/Yes|No/No/No/No|') or (Hits <> 8) then
            Error('Membership observation %1, RHS calls %2', Observed, Hits);
    end;

    local procedure Right(var Hits: Integer; Value: Boolean): Boolean
    begin
        Hits += 1;
        exit(Value);
    end;
}
