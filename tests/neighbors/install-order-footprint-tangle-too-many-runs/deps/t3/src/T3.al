codeunit 56030 "RQ tr T3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ tr Events";
        R: Record "RQ tr Row";
    begin
        if Events.AskThree() > 0 then;
        R.Code := 'T3';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 56031 "RQ tr T3 SubTwo"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tr Events", 'OnTwo', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
