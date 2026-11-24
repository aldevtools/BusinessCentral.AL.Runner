codeunit 56041 "RQ tr I1 SubOne"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tr Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56042 "RQ tr I1 SubTwo"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tr Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
