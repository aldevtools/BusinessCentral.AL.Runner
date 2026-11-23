codeunit 54530 "RQ tg I Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tg Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
