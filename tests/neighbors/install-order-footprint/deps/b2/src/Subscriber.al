codeunit 50950 "Order Probe B2 Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Order Probe Events", 'OnRegisterPrefix', '', false, false)]
    local procedure HandleRegister(var Map: Dictionary of [Text, Text])
    begin
        if not Map.ContainsKey('B2') then
            Map.Add('B2', 'PFX-B2');
    end;
}
