codeunit 53040 "RQ hidden C Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ hidden Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
