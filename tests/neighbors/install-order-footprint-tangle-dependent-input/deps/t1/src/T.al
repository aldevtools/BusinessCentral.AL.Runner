codeunit 54510 "RQ tg T1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ tg Events";
        R: Record "RQ tg Row";
    begin
        if Events.AskOne() > 0 then;
        R.Code := 'T1';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 54511 "RQ tg T1 Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tg Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
