codeunit 54430 "RQ fo A Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ fo Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
