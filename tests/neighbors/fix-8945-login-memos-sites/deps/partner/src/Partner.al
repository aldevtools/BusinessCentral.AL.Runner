// Partner app (publisher LS Partner Co): one login subscriber writes its table (setup ask and insert event) and fills
// Environment Information Impl. alone; its insert event subscriber fetches the profile, inside the raise.
table 60970 "LSP Tab"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 60971 "LSP Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        T: Record "LSP Tab";
        E: Codeunit "Environment Information Impl.";
    begin
        T.K := 1;
        T.Insert();
        E.IsSandbox();
        E.IsSaaS();
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    var
        P: Codeunit "Conf./Personalization Mgt.";
    begin
        P.Fetch();
    end;
}
