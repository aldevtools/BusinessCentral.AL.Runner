codeunit 56220 "RQ itm A2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ itm Events";
    begin
        if Events.AskA2() > 0 then;
    end;
}

codeunit 56221 "RQ itm Sub56221"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnX', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56222 "RQ itm Sub56222"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnA1', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
