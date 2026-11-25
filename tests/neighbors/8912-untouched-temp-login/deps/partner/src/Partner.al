// The partner's OnAfterLogin subscriber: it raises "UT Ping", which reaches the stand-in test library, an app the partner
// app does not depend on.
codeunit 61850 "UT Partner Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Ping: Codeunit "UT Ping";
    begin
        Ping.Raise();
    end;
}
