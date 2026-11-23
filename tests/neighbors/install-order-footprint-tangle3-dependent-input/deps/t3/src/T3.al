codeunit 55730 "RQ t3 T3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ t3 Events";
        R: Record "RQ t3 Row";
    begin
        if Events.AskThree() > 0 then;
        R.Code := 'T3';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55731 "RQ t3 T3 SubTwo"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ t3 Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
