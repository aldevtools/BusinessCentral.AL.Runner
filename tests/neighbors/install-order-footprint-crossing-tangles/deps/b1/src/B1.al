codeunit 55920 "RQ ct B1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ ct Events";
        R: Record "RQ ct Row";
    begin
        if Events.AskB1() > 0 then;
        R.Code := 'B1';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55921 "RQ ct B1 SubB2"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ ct Events", 'OnB2', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
