codeunit 56020 "RQ tr T2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ tr Events";
        R: Record "RQ tr Row";
    begin
        if Events.AskTwo() > 0 then;
        R.Code := 'T2';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 56021 "RQ tr T2 SubOne"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tr Events", 'OnOne', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
