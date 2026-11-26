codeunit 54021 "RQ dc1 G Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ dc1 Events", 'OnOne', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}
