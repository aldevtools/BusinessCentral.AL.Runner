table 52640 "CT Open TE"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52641 "CT Open Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "CT Open TE";
    begin
        Row.Code := 'INSTALL';
        Row.Value := 'ran';
        if not Row.Insert() then
            Row.Modify();
    end;
}

codeunit 52642 "CT Open Subscriber"
{
    // The app's own Company Triggers subscriber runs in its install session before its install code
    // (record/BfInstallTests.al:125-140).
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure OpenCompleted()
    var
        Row: Record "CT Open TE";
    begin
        if Session.GetExecutionContext() <> ExecutionContext::Install then
            exit;
        Row.Code := 'OPEN';
        Row.Value := Format(TaskScheduler.CanCreateTask());
        if not Row.Insert() then
            Row.Modify();
    end;
}
