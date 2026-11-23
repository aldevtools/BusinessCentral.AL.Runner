codeunit 55930 "RQ ct A1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ ct Events";
        R: Record "RQ ct Row";
    begin
        if Events.AskA1() > 0 then;
        R.Code := 'A1';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55931 "RQ ct A1 SubA2"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ ct Events", 'OnA2', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
