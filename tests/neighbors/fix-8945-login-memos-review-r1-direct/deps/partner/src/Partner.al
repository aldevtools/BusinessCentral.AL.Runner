// Partner app (publisher LD Partner Co): its login subscriber changes one global of Change Log Management by a direct call
// before its write's insert event loads the other, and asks API Webhook Notification Mgt. by a direct call.
table 61060 "LDP Tab"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 61061 "LDP Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        T: Record "LDP Tab";
        C: Codeunit "Change Log Management";
        W: Codeunit "API Webhook Notification Mgt.";
    begin
        C.MarkMonitor();
        T.K := 1;
        T.Insert();
        W.Ask();
    end;
}
