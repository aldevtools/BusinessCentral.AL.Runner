//! TestPage calls in the worker of a page background task, without `--no-client`: they run as before the `--no-client`
//! refusal of TestPage calls (`clientless_page_guard`), which reads only the test session's flag. A call on a page that is
//! not open answers "The TestPage is not open." and an open goes on, as on main before that refusal.

const SRC: &str = r#"
table 69970 "Btp Row" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 69971 "Btp Other" { PageType = List; SourceTable = "Btp Row"; layout { area(Content) { repeater(G) { field("No."; Rec."No.") { } } } } }
codeunit 69972 "Btp Worker"
{
    trigger OnRun()
    var
        Other: TestPage "Btp Other";
        Results: Dictionary of [Text, Text];
        Steps: Text;
    begin
        if not TryGoToKey(Other) then Steps += 'gotokey-err:' + GetLastErrorText() + ';' else Steps += 'gotokey-ok;';
        if not TryOpen(Other) then Steps += 'open-err:' + GetLastErrorText() + ';' else Steps += 'open-ok;';
        Results.Add('Steps', Steps);
        Page.SetBackgroundTaskResult(Results);
    end;
    [TryFunction] local procedure TryGoToKey(var P: TestPage "Btp Other") begin P.GoToKey('A'); end;
    [TryFunction] local procedure TryOpen(var P: TestPage "Btp Other") begin P.OpenView(); P.Close(); end;
}
page 69973 "Btp Card"
{
    PageType = Card; SourceTable = "Btp Row";
    layout { area(Content) { field("No."; Rec."No.") { } field(StepsCtl; Steps) { } field(ErrCtl; Err) { } } }
    trigger OnAfterGetCurrRecord()
    var Args: Dictionary of [Text, Text];
    begin
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"Btp Worker", Args, 5000);
    end;
    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        Results.Get('Steps', Steps);
    end;
    trigger OnPageBackgroundTaskError(ErrorTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        Err := ErrorText; IsHandled := true;
    end;
    var TaskId: Integer; Steps: Text; Err: Text;
}
codeunit 69974 C
{
    procedure Run(): Text
    var
        R: Record "Btp Row";
        Card: TestPage "Btp Card";
    begin
        R."No." := 'X'; R.Insert();
        Card.OpenView();
        exit(StrSubstNo('steps=[%1] err=[%2]', Card.StepsCtl.Value(), Card.ErrCtl.Value()));
    end;
}
"#;

#[test]
fn test_page_calls_in_a_worker_without_no_client_run_as_before() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Run")),
        "steps=[gotokey-err:The TestPage is not open.;open-ok;] err=[]"
    );
}
