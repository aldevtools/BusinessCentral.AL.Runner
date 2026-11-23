codeunit 54420 "RQ di L Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ di Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
