codeunit 56120 "RQ iti I2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ iti Events";
    begin
        if Events.AskI2() > 0 then;
    end;
}

codeunit 56121 "RQ iti Sub56121"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ iti Events", 'OnX', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56122 "RQ iti Sub56122"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ iti Events", 'OnI1', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
