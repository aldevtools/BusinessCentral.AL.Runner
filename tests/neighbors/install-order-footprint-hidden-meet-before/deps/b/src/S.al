codeunit 53030 "RQ hidden B Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ hidden Events", 'OnOne', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
