codeunit 56051 "RQ tr I2 SubOne"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tr Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56052 "RQ tr I2 SubTwo"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tr Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
