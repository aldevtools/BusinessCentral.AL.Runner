codeunit 56210 "RQ itm A1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ itm Events";
    begin
        if Events.AskA1() > 0 then;
    end;
}

codeunit 56211 "RQ itm Sub56211"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnX', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56212 "RQ itm Sub56212"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnA3', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
