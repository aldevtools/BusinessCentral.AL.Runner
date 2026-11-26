codeunit 54130 "RQ dc2 A Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ dc2 Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
