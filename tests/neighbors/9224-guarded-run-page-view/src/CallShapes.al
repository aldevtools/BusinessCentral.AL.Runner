// Call paths of the guarded run that corpus PR 1622 did not measure (review round 1 of runner PR 9252). The probe's
// OnAction calls "GPC Run".RunCopy(Rec), a codeunit method, and that method runs the guarded run. Each extra of GPC Log
// runs the same steps on another path: 'direct' runs it in the OnAction body itself, 'local' calls RunCopy from a local
// procedure of the page, 'method' calls RunCopy from a method of another codeunit, 'event' runs it in a subscriber of an
// event the action raises. The runtime keeps main's result for each.
page 69181 "GPN Call Shapes"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    LogCu: Codeunit "GPC Log";
                    Wrapper: Codeunit "GPN Call Wrapper";
                    R: Record "GPC Z";
                    Ok: Boolean;
                begin
                    case LogCu.GetExtra() of
                        'direct':
                            begin
                                R.Copy(Rec);
                                ClearLastError();
                                Ok := Codeunit.Run(Codeunit::"GPC Inner", R);
                                LogCu.Log('run=' + Format(Ok));
                                LogCu.Log('code=' + GetLastErrorCode());
                            end;
                        'local':
                            LocalRun(Rec);
                        'method':
                            Wrapper.Run(Rec);
                        'event':
                            OnRunGuarded(Rec);
                    end;
                end;
            }
        }
    }

    local procedure LocalRun(var PageRec: Record "GPC Z")
    var
        Runner: Codeunit "GPC Run";
    begin
        Runner.RunCopy(PageRec);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnRunGuarded(var PageRec: Record "GPC Z")
    begin
    end;
}

codeunit 69182 "GPN Call Wrapper"
{
    procedure Run(var PageRec: Record "GPC Z")
    var
        Runner: Codeunit "GPC Run";
    begin
        Runner.RunCopy(PageRec);
    end;

    [EventSubscriber(ObjectType::Page, Page::"GPN Call Shapes", 'OnRunGuarded', '', false, false)]
    local procedure RunOnEvent(var PageRec: Record "GPC Z")
    var
        LogCu: Codeunit "GPC Log";
        R: Record "GPC Z";
        Ok: Boolean;
    begin
        R.Copy(PageRec);
        ClearLastError();
        Ok := Codeunit.Run(Codeunit::"GPC Inner", R);
        LogCu.Log('run=' + Format(Ok));
        LogCu.Log('code=' + GetLastErrorCode());
    end;
}
