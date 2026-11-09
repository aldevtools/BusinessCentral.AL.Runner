//! The worker of a page background task runs in a child session (al-runner issue 7342; corpus
//! pages/background-tasks/PbtChildSession.al on corpus master, PR 1230): another `SessionId()`, a fresh SingleInstance
//! state per task that the test does not see afterwards, no manual binding of the test, the automatic subscribers; the
//! Completed and Error triggers run in the test session. Mirrors the corpus cases until the submodule pin has them.

const SRC: &str = r#"
table 69960 "Pbc Row" { fields { field(1; No; Code[20]) { } field(2; Mode; Text[50]) { } } keys { key(PK; No) { Clustered = true; } } }
codeunit 69961 "Pbc State"
{
    SingleInstance = true;
    var Value: Text; Calls: Integer;
    procedure Set(V: Text) begin Value := V; end;
    procedure Get(): Text begin exit(Value); end;
    procedure Bump(): Integer begin Calls += 1; exit(Calls); end;
}
codeunit 69962 "Pbc Worker"
{
    trigger OnRun()
    var
        State: Codeunit "Pbc State";
        Params: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Mode: Text; Manual: Text; Auto: Text; Seen: Text;
    begin
        Params := Page.GetBackgroundParameters();
        Params.Get('Mode', Mode);
        Seen := State.Get();
        OnAskManual(Manual);
        OnAskAutomatic(Auto);
        if (Mode = 'NeedManual') and (Manual = '') then
            Error('the manual subscriber did not answer');
        Results.Add('Session', Format(SessionId()));
        Results.Add('State', Seen);
        Results.Add('Manual', Manual);
        Results.Add('Auto', Auto);
        Results.Add('Calls', Format(State.Bump()));
        State.Set('set by the task');
        Page.SetBackgroundTaskResult(Results);
    end;
    [IntegrationEvent(false, false)] local procedure OnAskManual(var Answer: Text) begin end;
    [IntegrationEvent(false, false)] local procedure OnAskAutomatic(var Answer: Text) begin end;
}
codeunit 69963 "Pbc Manual"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Pbc Worker", 'OnAskManual', '', false, false)]
    local procedure AnswerManual(var Answer: Text) begin Answer := 'manual'; end;
}
codeunit 69964 "Pbc Auto"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Pbc Worker", 'OnAskAutomatic', '', false, false)]
    local procedure AnswerAutomatic(var Answer: Text) begin Answer := 'automatic'; end;
}
page 69965 "Pbc Card"
{
    PageType = Card; SourceTable = "Pbc Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec.No) { }
            field(TaskSessionCtl; TaskSession) { }
            field(CompletedSessionCtl; CompletedSession) { }
            field(TaskStateCtl; TaskState) { }
            field(ManualCtl; ManualAnswer) { }
            field(AutoCtl; AutoAnswer) { }
            field(CallsCtl; CallsHistory) { }
            field(CompletedCountCtl; CompletedCount) { }
            field(ErrorCountCtl; ErrorCount) { }
            field(ErrorCodeCtl; LastErrorCode) { }
            field(ErrorTextCtl; LastErrorText) { }
            field(ErrorSessionCtl; ErrorSession) { }
        }
    }
    trigger OnAfterGetCurrRecord()
    var Args: Dictionary of [Text, Text];
    begin
        Args.Add('Mode', Rec.Mode);
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"Pbc Worker", Args, 5000);
    end;
    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    var Calls: Text;
    begin
        CompletedCount += 1;
        CompletedSession := Format(SessionId());
        Results.Get('Session', TaskSession);
        Results.Get('State', TaskState);
        Results.Get('Manual', ManualAnswer);
        Results.Get('Auto', AutoAnswer);
        Results.Get('Calls', Calls);
        CallsHistory += Calls + ';';
    end;
    trigger OnPageBackgroundTaskError(ErrorTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        ErrorCount += 1;
        ErrorSession := Format(SessionId());
        LastErrorCode := ErrorCode;
        LastErrorText := ErrorText;
        IsHandled := true;
    end;
    var
        TaskId: Integer; TaskSession: Text; CompletedSession: Text; TaskState: Text; ManualAnswer: Text; AutoAnswer: Text;
        CallsHistory: Text; CompletedCount: Integer; ErrorCount: Integer; LastErrorCode: Text; LastErrorText: Text; ErrorSession: Text;
}
codeunit 69966 C
{
    var State: Codeunit "Pbc State";

    local procedure Seed(No: Code[20]; Mode: Text[50])
    var R: Record "Pbc Row";
    begin
        R.No := No; R.Mode := Mode; R.Insert();
    end;

    // PbtChildSession.al:209-229
    procedure OtherSession(): Text
    var Card: TestPage "Pbc Card";
    begin
        Seed('PBT-1', '');
        Card.OpenView();
        exit(StrSubstNo('%1|%2|%3|%4', Card.CompletedCountCtl.AsInteger(), Card.TaskSessionCtl.Value() <> Format(SessionId()),
            Card.TaskSessionCtl.Value() <> '', Card.CompletedSessionCtl.Value() = Format(SessionId())));
    end;

    // PbtChildSession.al:231-248
    procedure StateOfTest(): Text
    var Card: TestPage "Pbc Card";
    begin
        Seed('PBT-1', '');
        State.Set('set by the test');
        Card.OpenView();
        exit(StrSubstNo('[%1]|%2', Card.TaskStateCtl.Value(), State.Get()));
    end;

    // PbtChildSession.al:250-270
    procedure SecondTask(): Text
    var Card: TestPage "Pbc Card";
    begin
        Seed('PBT-1', '');
        Seed('PBT-2', '');
        Card.OpenView();
        Card.GoToKey('PBT-2');
        exit(StrSubstNo('%1|[%2]|%3', Card.CallsCtl.Value(), Card.TaskStateCtl.Value(), State.Bump()));
    end;

    // PbtChildSession.al:272-305
    procedure Bindings(): Text
    var
        Card: TestPage "Pbc Card";
        Sub: Codeunit "Pbc Manual";
    begin
        Seed('PBT-1', '');
        BindSubscription(Sub);
        Card.OpenView();
        exit(StrSubstNo('[%1]|%2', Card.ManualCtl.Value(), Card.AutoCtl.Value()));
    end;

    // PbtChildSession.al:307-332
    procedure NeedManual(): Text
    var
        Card: TestPage "Pbc Card";
        Sub: Codeunit "Pbc Manual";
    begin
        Seed('PBT-1', 'NeedManual');
        BindSubscription(Sub);
        Card.OpenView();
        exit(StrSubstNo('%1|%2|%3|%4|%5', Card.CompletedCountCtl.AsInteger(), Card.ErrorCountCtl.AsInteger(),
            Card.ErrorTextCtl.Value(), Card.ErrorCodeCtl.Value(), Card.ErrorSessionCtl.Value() = Format(SessionId())));
    end;

    // PbtChildSession.al:334-367
    procedure RunTask(): Text
    var
        Card: TestPage "Pbc Card";
        Sub: Codeunit "Pbc Manual";
        Params: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        S: Text; St: Text; M: Text; A: Text;
    begin
        Seed('PBT-1', '');
        State.Set('set by the test');
        BindSubscription(Sub);
        Card.OpenView();
        Params.Add('Mode', '');
        Results := Card.RunPageBackgroundTask(Codeunit::"Pbc Worker", Params, false);
        Results.Get('Session', S);
        Results.Get('State', St);
        Results.Get('Manual', M);
        Results.Get('Auto', A);
        exit(StrSubstNo('%1|[%2]|[%3]|%4|%5', S <> Format(SessionId()), St, M, A, State.Get()));
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn worker_runs_in_another_session_and_triggers_in_the_test_session() {
    assert_eq!(run("OtherSession"), "1|Yes|Yes|Yes");
}

#[test]
fn worker_does_not_see_or_change_the_single_instance_state_of_the_test() {
    assert_eq!(run("StateOfTest"), "[]|set by the test");
}

#[test]
fn every_task_starts_with_a_fresh_single_instance_codeunit() {
    // Three completions on BC (PbtChildSession.al:266): whatever the runtime's count of tasks, each saw a counter of 1.
    let got = run("SecondTask");
    let (calls, rest) = got.split_once('|').unwrap();
    assert!(!calls.is_empty() && calls.split(';').filter(|c| !c.is_empty()).all(|c| c == "1"), "{got}");
    assert_eq!(rest, "[]|1");
}

#[test]
fn manual_binding_of_the_test_does_not_apply_and_automatic_subscriber_answers() {
    assert_eq!(run("Bindings"), "[]|automatic");
}

#[test]
fn worker_needing_the_manual_binding_raises_the_error_trigger() {
    assert_eq!(run("NeedManual"), "0|1|the manual subscriber did not answer|Dialog|Yes");
}

#[test]
fn run_page_background_task_runs_the_worker_in_a_child_session() {
    assert_eq!(run("RunTask"), "Yes|[]|[]|automatic|set by the test");
}
