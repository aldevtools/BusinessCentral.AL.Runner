codeunit 52030 "RP Chain B Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RP Chain Events", 'OnRegisterPrefix', '', false, false)]
    local procedure HandleRegister(var Map: Dictionary of [Text, Text])
    begin
        if not Map.ContainsKey('B') then
            Map.Add('B', 'PFX');
    end;
}
