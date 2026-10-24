//! `CurrPage.EnqueueBackgroundTask` with its fifth argument, a `PageBackgroundTaskErrorLevel` (al-runner issue 5806). The type's
//! members and ordinals are pinned by lt-ac/ACPageBackgroundTaskErrorLevel.al; a task that succeeds under any level runs as
//! one without the argument (bgtasks/lt-bj-no-gui/BjNoGuiDialogs.al:86, EnqueueBackgroundTask_WithHandlers, level Error);
//! a failing worker under Ignore runs the error trigger and leaves the page open (bgtasks/W2BackgroundTasks.al:455
//! ErrorLevel_Ignore). A failing worker under Error or Warning is not pinned and is refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 69980 "BgLv Row" { fields { field(1; "No."; Code[20]) { } field(2; Op; Text[10]) { } field(3; Level; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 69981 "BgLv Worker"
{
    trigger OnRun()
    var
        Params: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        No: Text;
        Op: Text;
    begin
        Params := Page.GetBackgroundParameters();
        Params.Get('No', No);
        Params.Get('Op', Op);
        if Op = 'fail' then
            Error('worker failed');
        Results.Add('Result', 'done:' + No);
        Page.SetBackgroundTaskResult(Results);
    end;
}
page 69982 "BgLv Card"
{
    PageType = Card;
    SourceTable = "BgLv Row";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            field(TaskResult; TaskResult) { ApplicationArea = All; }
        }
    }
    trigger OnAfterGetCurrRecord()
    var
        Params: Dictionary of [Text, Text];
    begin
        if TaskId <> 0 then
            CurrPage.CancelBackgroundTask(TaskId);
        TaskResult := '';
        Params.Add('No', Rec."No.");
        Params.Add('Op', Rec.Op);
        case Rec.Level of
            0:
                CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"BgLv Worker", Params, 120000, PageBackgroundTaskErrorLevel::Error);
            1:
                CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"BgLv Worker", Params, 120000, PageBackgroundTaskErrorLevel::Warning);
            2:
                CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"BgLv Worker", Params, 120000, PageBackgroundTaskErrorLevel::Ignore);
        end;
    end;
    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId <> TaskId then
            exit;
        Results.Get('Result', TaskResult);
    end;
    trigger OnPageBackgroundTaskError(FailedTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        if FailedTaskId = TaskId then
            TaskResult := 'error';
    end;
    var
        TaskResult: Text;
        TaskId: Integer;
}
codeunit 69983 C
{
    procedure Run(L: Integer; Op: Text): Text
    var
        Row: Record "BgLv Row";
        Card: TestPage "BgLv Card";
        Result: Text;
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Op := Op;
        Row.Level := L;
        Row.Insert();
        Card.OpenView();
        Card.GoToRecord(Row);
        Result := Card.TaskResult.Value();
        Card.Close();
        exit(Result);
    end;

    procedure Warning(): Text begin exit(Run(1, '')); end;
    procedure ErrorLvl(): Text begin exit(Run(0, '')); end;
    procedure IgnoreOk(): Text begin exit(Run(2, '')); end;
    procedure IgnoreFails(): Text begin exit(Run(2, 'fail')); end;
    procedure WarningFails(): Text begin exit(Run(1, 'fail')); end;
    procedure ErrorFails(): Text begin exit(Run(0, 'fail')); end;

    procedure FormatAll(): Text
    var
        V: PageBackgroundTaskErrorLevel;
    begin
        exit(Format(PageBackgroundTaskErrorLevel::Warning) + ',' + Format(PageBackgroundTaskErrorLevel::Ignore, 0, 2) + ','
            + Format(PageBackgroundTaskErrorLevel::Warning, 0, 1) + ',' + Format(V) + ',' + Format(V, 0, 2));
    end;

    procedure EvaluateNumber(): Text
    var
        V: PageBackgroundTaskErrorLevel;
    begin
        Evaluate(V, '10');
        exit(Format(V));
    end;
}
"#;

fn run(routine: &str) -> Result<Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

fn refused(routine: &str) -> String {
    match run(routine) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine}: {other:?}"),
    }
}

/// The reporter's shape (al-runner 5806): the task under Warning completes before GoToRecord returns.
#[test]
fn warning_level_task_completes() {
    assert_eq!(al_testkit::text(run("Warning")), "done:R1");
}

#[test]
fn error_and_ignore_level_tasks_complete() {
    assert_eq!(al_testkit::text(run("ErrorLvl")), "done:R1");
    assert_eq!(al_testkit::text(run("IgnoreOk")), "done:R1");
}

/// bgtasks/W2BackgroundTasks.al:455 ErrorLevel_Ignore: the error trigger runs and the page stays open.
#[test]
fn ignore_level_failing_task_runs_the_error_trigger_and_keeps_the_page() {
    assert_eq!(al_testkit::text(run("IgnoreFails")), "error");
}

#[test]
fn failing_task_under_warning_or_error_level_is_refused() {
    for routine in ["WarningFails", "ErrorFails"] {
        let why = refused(routine);
        assert!(why.contains("failing worker"), "{why}");
    }
}

/// lt-ac/ACPageBackgroundTaskErrorLevel.al: names, ordinals 0, 10, 20, format 1 prints the name, the default is Error.
#[test]
fn format_of_the_type() {
    assert_eq!(al_testkit::text(run("FormatAll")), "Warning,20,Warning,Error,0");
}

/// Evaluate into the type is not pinned (lt-ac/QbEvaluateBgTask.al is not a case of the oracle): refused.
#[test]
fn evaluate_into_the_type_is_refused() {
    let why = refused("EvaluateNumber");
    assert!(why.contains("PageBackgroundTaskErrorLevel"), "{why}");
}
