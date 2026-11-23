// The partner app's OnAfterLogin subscriber raises the hub's event; it does not depend on the apps that subscribe to it.
codeunit 60950 "XS Partner Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "XS Touch Hub";
    begin
        Hub.Touch();
    end;
}
