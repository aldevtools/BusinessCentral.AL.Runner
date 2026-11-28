// Partner app (publisher LR Partner Co): its login subscriber loads Change Log Management by a direct call and resets it,
// then writes its table, whose insert event loads it again; its insert subscriber writes the inner table, whose setup ask
// (nested in the insert raise) asks API Webhook Notification Mgt.; it fills Application Area Cache.
table 61030 "LRP Tab"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

table 61031 "LRP Inner"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 61032 "LRP Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        T: Record "LRP Tab";
        C: Codeunit "Change Log Management";
        A: Codeunit "Application Area Cache";
    begin
        C.Load();
        C.Reset();
        T.K := 1;
        T.Insert();
        A.Fill();
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    var
        I: Record "LRP Inner";
    begin
        if RecRef.Number <> Database::"LRP Tab" then
            exit;
        I.K := 1;
        if I.Insert() then;
    end;
}
