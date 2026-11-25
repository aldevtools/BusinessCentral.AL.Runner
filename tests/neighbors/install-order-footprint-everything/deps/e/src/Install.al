codeunit 51520 "FP Everything E Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Own: Record "FP Everything TE";
        TaskId: Guid;
    begin
        if TaskScheduler.TaskExists(TaskId) then;
        Own.Code := 'E';
        Own.Value := 'SET';
        if not Own.Insert() then Own.Modify();
    end;
}
