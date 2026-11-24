codeunit 52530 "RP Taint B Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RP Taint Events", 'OnAsk', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
