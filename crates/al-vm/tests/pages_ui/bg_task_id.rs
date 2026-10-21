//! `CurrPage.EnqueueBackgroundTask` under a TestPage: the task runs before the next statement (corpus
//! handlers/TestPageCancelBackgroundTaskContracts.al, bgtasks/W2BackgroundTasks.al:CancelPreviousTask_OnRowMove), after the
//! call stored the id in its `var TaskId`: assumed (bg-task-id-set-before-completion) for Tests-ERM 134385
//! CustomerStatisticsYearToDate, where page 9082 compares the trigger's TaskId with that variable.

const SRC: &str = r#"
table 69990 "BgId Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69991 "BgId Worker"
{
    trigger OnRun()
    var
        R: Dictionary of [Text, Text];
    begin
        R.Add('R', 'done');
        Page.SetBackgroundTaskResult(R);
    end;
}
codeunit 69992 "BgId Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Get(): Text begin exit(L); end;
}
page 69993 "BgId Card"
{
    PageType = Card; SourceTable = "BgId Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    var TaskId: Integer; G: Integer;
    trigger OnOpenPage()
    var
        Args: Dictionary of [Text, Text];
        Log: Codeunit "BgId Log";
    begin
        G := 5;
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"BgId Worker", Args, 5000);
        Log.Add('after enqueue ' + Format(TaskId));
    end;
    trigger OnPageBackgroundTaskCompleted(Completed: Integer; Results: Dictionary of [Text, Text])
    var
        Log: Codeunit "BgId Log";
    begin
        Log.Add(StrSubstNo('completed %1 var=%2 g=%3', Completed, TaskId, G));
    end;
}
codeunit 69994 C
{
    procedure Run(): Text
    var
        R: Record "BgId Row";
        P: TestPage "BgId Card";
        Log: Codeunit "BgId Log";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
        P.Close();
        exit(Log.Get());
    end;
}
"#;

#[test]
fn completion_trigger_sees_the_task_id_variable_set() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Run")),
        "completed 1 var=1 g=5;after enqueue 1;"
    );
}
