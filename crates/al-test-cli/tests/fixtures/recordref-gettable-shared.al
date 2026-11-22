table 50950 "Probe Ref Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Category; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50953 T
{
    Subtype = Test;
    EventSubscriberInstance = Manual;
    TestPermissions = Disabled;

    [Test]
    procedure GetTableInByValueSubscriberChangesPublisherRow()
    var
        Input: Record "Probe Ref Row";
        Ref: RecordRef;
        Observed: Variant;
        Handler: Codeunit T;
    begin
        Input.Id := 1;
        Ref.GetTable(Input);
        BindSubscription(Handler);
        OnReplace(Ref);
        UnbindSubscription(Handler);
        Observed := Ref.Field(2).Value;
        if Format(Observed) <> '7' then
            Error('Expected the caller field to be 7 after GetTable. Got %1.', Observed);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnReplace(Ref: RecordRef)
    begin
    end;

    [Test]
    procedure GetTableUpdatesAnAssignedAliasOfTheSameTable()
    var
        Row: Record "Probe Ref Row";
        Original: RecordRef;
        Alias: RecordRef;
        Observed: Variant;
    begin
        Row.Id := 1;
        Original.GetTable(Row);
        Alias := Original;
        Row.Category := 7;
        Alias.GetTable(Row);
        Observed := Original.Field(2).Value;
        if Format(Observed) <> '7' then
            Error('The assigned alias changed the value to %1.', Observed);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::T, 'OnReplace', '', false, false)]
    local procedure ReplaceByValue(Ref: RecordRef)
    var
        Input: Record "Probe Ref Row";
    begin
        Input.Id := 1;
        Input.Category := 7;
        Ref.GetTable(Input);
    end;
}
