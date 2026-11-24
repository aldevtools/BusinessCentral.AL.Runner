codeunit 55910 "RQ ct A2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ ct Events";
        R: Record "RQ ct Row";
    begin
        if Events.AskA2() > 0 then;
        R.Code := 'A2';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55911 "RQ ct A2 SubA1"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ ct Events", 'OnA1', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
