codeunit 56110 "RQ iti I1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ iti Events";
    begin
        if Events.AskI1() > 0 then;
    end;
}

codeunit 56111 "RQ iti Sub56111"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ iti Events", 'OnX', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56112 "RQ iti Sub56112"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ iti Events", 'OnI2', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
