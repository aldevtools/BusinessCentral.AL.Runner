codeunit 55720 "RQ t3 T2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ t3 Events";
        R: Record "RQ t3 Row";
    begin
        if Events.AskTwo() > 0 then;
        R.Code := 'T2';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55721 "RQ t3 T2 SubOne"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ t3 Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
