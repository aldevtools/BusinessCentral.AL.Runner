codeunit 55820 "RQ tm T1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ tm Events";
        R: Record "RQ tm Row";
    begin
        if Events.AskOne() > 0 then;
        R.Code := 'T1';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55821 "RQ tm T1 SubTwo"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tm Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
