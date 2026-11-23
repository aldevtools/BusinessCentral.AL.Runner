codeunit 55940 "RQ ct B2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ ct Events";
        R: Record "RQ ct Row";
    begin
        if Events.AskB2() > 0 then;
        R.Code := 'B2';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55941 "RQ ct B2 SubB1"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ ct Events", 'OnB1', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
