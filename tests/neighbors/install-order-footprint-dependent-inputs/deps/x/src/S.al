codeunit 54430 "RQ di X Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ di Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
