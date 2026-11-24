codeunit 56240 "RQ itm B1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ itm Events";
    begin
        if Events.AskB1() > 0 then;
    end;
}

codeunit 56241 "RQ itm Sub56241"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnX', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56242 "RQ itm Sub56242"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnB3', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
