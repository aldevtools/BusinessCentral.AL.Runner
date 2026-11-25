codeunit 50920 "Order Probe B Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Order Probe Events", 'OnRegisterPrefix', '', false, false)]
    local procedure HandleRegister(var Map: Dictionary of [Text, Text])
    begin
        if not Map.ContainsKey('B') then
            Map.Add('B', 'PFX-B');
    end;
}
