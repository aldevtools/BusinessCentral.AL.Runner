codeunit 53230 "RQ hz B Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ hz Events", 'OnOne', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
