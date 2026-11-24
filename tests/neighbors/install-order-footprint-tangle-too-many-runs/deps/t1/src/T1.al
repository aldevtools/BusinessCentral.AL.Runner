codeunit 56010 "RQ tr T1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ tr Events";
        R: Record "RQ tr Row";
    begin
        if Events.AskOne() > 0 then;
        R.Code := 'T1';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 56011 "RQ tr T1 SubThree"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ tr Events", 'OnThree', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
