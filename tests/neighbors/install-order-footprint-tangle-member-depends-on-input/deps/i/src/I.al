codeunit 55811 "RQ tm I SubOne"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tm Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
