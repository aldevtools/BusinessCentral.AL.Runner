codeunit 55710 "RQ t3 T1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ t3 Events";
        R: Record "RQ t3 Row";
    begin
        if Events.AskOne() > 0 then;
        R.Code := 'T1';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}

codeunit 55711 "RQ t3 T1 SubThree"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RQ t3 Events", 'OnThree', '', false, false)]
    local procedure Answer(var Count: Integer)
    begin
        Count += 1;
    end;
}
