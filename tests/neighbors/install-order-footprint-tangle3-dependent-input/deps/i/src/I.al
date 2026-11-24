codeunit 55741 "RQ t3 I SubThree"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ t3 Events", 'OnThree', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
