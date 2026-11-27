// The test session's own Company Triggers subscribers: BC answers Yes there and No in the test procedure (corpus PR
// 1592, install-upgrade/IcInstallCanCreateTaskTests.al:197-199). The values stay in a SingleInstance codeunit, not a table.
codeunit 52643 "CT Open Seen"
{
    SingleInstance = true;

    var
        OnOpen: Text;
        OnOpenCompleted: Text;

    procedure SetOpen(Value: Text)
    begin
        OnOpen := Value;
    end;

    procedure SetOpenCompleted(Value: Text)
    begin
        OnOpenCompleted := Value;
    end;

    procedure Open(): Text
    begin
        exit(OnOpen);
    end;

    procedure OpenCompleted(): Text
    begin
        exit(OnOpenCompleted);
    end;
}

codeunit 52644 "CT Open Test Session"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpen', '', false, false)]
    local procedure Open()
    var
        Seen: Codeunit "CT Open Seen";
    begin
        if Session.GetExecutionContext() <> ExecutionContext::Normal then
            exit;
        Seen.SetOpen(Format(TaskScheduler.CanCreateTask()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure OpenCompleted()
    var
        Seen: Codeunit "CT Open Seen";
    begin
        if Session.GetExecutionContext() <> ExecutionContext::Normal then
            exit;
        Seen.SetOpenCompleted(Format(TaskScheduler.CanCreateTask()));
    end;
}
