codeunit 54520 "RQ tg T2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ tg Events";
        R: Record "RQ tg Row";
    begin
        if Events.AskTwo() > 0 then;
        R.Code := 'T2';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 54521 "RQ tg T2 Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tg Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
