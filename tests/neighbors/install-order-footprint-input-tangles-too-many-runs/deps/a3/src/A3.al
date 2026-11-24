codeunit 56230 "RQ itm A3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ itm Events";
    begin
        if Events.AskA3() > 0 then;
    end;
}

codeunit 56231 "RQ itm Sub56231"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnX', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56232 "RQ itm Sub56232"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnA2', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
