codeunit 54030 "RQ dc1 A Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ dc1 Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
