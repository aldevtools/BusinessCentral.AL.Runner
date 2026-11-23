codeunit 53240 "RQ hz C Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ hz Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
