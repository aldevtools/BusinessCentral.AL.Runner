codeunit 54420 "RQ fo C Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ fo Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
