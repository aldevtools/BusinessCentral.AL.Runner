codeunit 55830 "RQ tm T2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ tm Events";
        R: Record "RQ tm Row";
    begin
        if Events.AskTwo() > 0 then;
        R.Code := 'T2';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55831 "RQ tm T2 SubOne"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tm Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
