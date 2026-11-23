codeunit 55631 "RQ ti I SubOne"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ ti Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 55632 "RQ ti I SubTwo"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ ti Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
