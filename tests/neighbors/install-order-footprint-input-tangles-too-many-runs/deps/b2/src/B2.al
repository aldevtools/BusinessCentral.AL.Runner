codeunit 56250 "RQ itm B2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ itm Events";
    begin
        if Events.AskB2() > 0 then;
    end;
}

codeunit 56251 "RQ itm Sub56251"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnX', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}

codeunit 56252 "RQ itm Sub56252"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ itm Events", 'OnB1', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
