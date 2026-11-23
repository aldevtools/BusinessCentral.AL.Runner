codeunit 54121 "RQ dc2 G Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ dc2 Events", 'OnOne', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
