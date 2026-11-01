//! TaskScheduler tasks (al-vm tasks.rs). Pins: platform/W6TaskScheduler.al:88-176, platform/lt-platform-members/
//! PlTTaskScheduler.al:40-215.

use al_builtins::native::Signal;

const SRC: &str = r#"
table 2000000175 "Scheduled Task"
{
    fields
    {
        field(1; ID; Guid) { }
        field(2; "User ID"; Guid) { }
        field(3; "User Name"; Code[50]) { }
        field(5; "Run Codeunit"; Integer) { }
        field(6; "Failure Codeunit"; Integer) { }
        field(8; "Not Before"; DateTime) { }
        field(9; "Is Ready"; Boolean) { }
        field(10; Company; Text[30]) { }
    }
    keys { key(PK; ID) { } }
}
codeunit 50100 "TS Target" { trigger OnRun() begin end; }
codeunit 50101 "TS Probe"
{
    procedure Flow(): Text
    var
        Id: Guid;
        Id2: Guid;
        Empty: Guid;
        R: Text;
    begin
        Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        Id2 := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, true, CompanyName(), CurrentDateTime() + 1000);
        R := Format(IsNullGuid(Id)) + Format(Id <> Id2) + Format(TaskScheduler.TaskExists(Id)) + Format(TaskScheduler.TaskExists(Id2));
        TaskScheduler.SetTaskReady(Id);
        TaskScheduler.SetTaskReady(CreateGuid());
        R += Format(TaskScheduler.TaskExists(Id));
        R += Format(TaskScheduler.CancelTask(Id)) + Format(TaskScheduler.TaskExists(Id)) + Format(TaskScheduler.CancelTask(Id));
        R += Format(TaskScheduler.TaskExists(CreateGuid())) + Format(TaskScheduler.TaskExists(Empty)) + Format(TaskScheduler.CancelTask(Empty));
        TaskScheduler.CreateTask(0, 0, false);
        exit(R);
    end;

    procedure AfterCommit(): Boolean
    var
        Id: Guid;
    begin
        Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        Commit();
        exit(TaskScheduler.TaskExists(Id));
    end;

    procedure ReadTaskRow(): Boolean
    var
        Task: Record "Scheduled Task";
        Id: Guid;
    begin
        Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        exit(Task.Get(Id));
    end;

    // platform/lt-wd/WdScheduledTaskGet.al (90751) and session/lt-cn-session-leads/CnSessionLeads.al:23-46.
    procedure RowOfOpenTask(): Text
    var
        Task: Record "Scheduled Task";
        Id: Guid;
        R: Text;
    begin
        Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        R := Format(Task.Get(Id));
        R += Format(Task."User ID" = UserSecurityId()) + Format(Task."User Name" = UserId()) + Format(Task.Company = CompanyName());
        R += Format(Task."Run Codeunit" = Codeunit::"TS Target") + Format(Task."Is Ready");
        Task.Reset();
        Task.SetRange(ID, Id);
        R += Format(Task.Count()) + Format(Task.IsEmpty()) + Format(Task.FindFirst());
        R += Format(TaskScheduler.CancelTask(Id));
        R += Format(Task.Count()) + Format(Task.IsEmpty());
        Clear(Task);
        R += Format(Task.Get(Id)) + Format(Task.Get(CreateGuid()));
        exit(R);
    end;

    procedure UnpinnedColumnOfTask(): Boolean
    var
        Task: Record "Scheduled Task";
    begin
        Task.Get(TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false));
        exit(Task."Failure Codeunit" = 0);
    end;

    // platform/lt-y43-task-after-commit/Y43TaskAfterCommit.al (corpus master): after the Commit the task is still there, not
    // run, with its columns.
    local procedure ReadTask(Id: Guid): Text
    var
        Task: Record "Scheduled Task";
    begin
        if not Task.Get(Id) then
            exit('No');
        exit('Yes ready=' + Format(Task."Is Ready") + ' cu=' + Format(Task."Run Codeunit") + ' notBeforeBlank=' + Format(Task."Not Before" = 0DT));
    end;

    procedure RowAfterCommit(): Text
    var
        Task: Record "Scheduled Task";
        Ready: Guid;
        NotReady: Guid;
        Later: Guid;
        NotBefore: DateTime;
        R: Text;
    begin
        NotBefore := CreateDateTime(Today() + 30, 0T);
        Ready := TaskScheduler.CreateTask(Codeunit::"TS Target", 0);
        NotReady := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        Later := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, true, CompanyName(), NotBefore);
        R := ReadTask(Ready) + '|';
        Commit();
        R += ReadTask(Ready) + '|' + ReadTask(NotReady) + '|' + ReadTask(Later) + '|';
        Task.Get(Later);
        R += Format(Task."Not Before" = NotBefore) + '|';
        Task.Reset();
        Task.SetRange("Is Ready", true);
        Task.SetRange("User ID", UserSecurityId());
        R += Format(Task.Count()) + '|';
        R += Format(TaskScheduler.TaskExists(NotReady)) + Format(TaskScheduler.CancelTask(NotReady)) + ReadTask(NotReady)
            + Format(TaskScheduler.TaskExists(NotReady)) + Format(TaskScheduler.CancelTask(NotReady));
        exit(R);
    end;

    procedure RowRolledBack(): Boolean
    var
        Task: Record "Scheduled Task";
        Id: Guid;
    begin
        asserterror begin
            Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
            Error('x');
        end;
        exit(Task.Get(Id));
    end;

    procedure UnknownCodeunit()
    begin
        TaskScheduler.CreateTask(99999999, 0, false);
    end;
}
"#;

#[test]
fn created_tasks_exist_until_cancelled() {
    let r = al_testkit::text(al_testkit::run_in(SRC, "TS Probe", "Flow"));
    assert_eq!(r, "NoYesYesYesYesYesNoNoNoNoNo");
}

// session/lt-cn-session-leads/CnSessionLeads.al:23-46 and platform/lt-wd/WdScheduledTaskGet.al: the task created in the open
// transaction has its row, with the pinned columns, until CancelTask removes it.
#[test]
fn a_task_of_the_open_transaction_has_its_row() {
    let r = al_testkit::run_in(SRC, "TS Probe", "ReadTaskRow");
    assert!(matches!(r, Ok(al_builtins::value::Value::Boolean(true))), "{r:?}");
    let r = al_testkit::text(al_testkit::run_in(SRC, "TS Probe", "RowOfOpenTask"));
    assert_eq!(r, "YesYesYesYesYesNo1NoYesYes0YesNoNo");
}

// The columns no corpus test reads (Failure Codeunit, ...) are refused when read.
#[test]
fn an_unpinned_column_of_the_task_row_is_refused() {
    let r = al_testkit::run_in(SRC, "TS Probe", "UnpinnedColumnOfTask");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("Scheduled Task")),
        "{r:?}"
    );
}

// A rollback takes the row back with the task (PlTTaskScheduler.al:198-215).
#[test]
fn a_rolled_back_task_has_no_row() {
    let r = al_testkit::run_in(SRC, "TS Probe", "RowRolledBack");
    assert!(matches!(r, Ok(al_builtins::value::Value::Boolean(false))), "{r:?}");
}

// platform/lt-y43-task-after-commit/Y43TaskAfterCommit.al:258-268 (TaskExists after the Commit) and :170-182, :184-207,
// :282-298 (al-runner issue 6592): a committed task keeps its row and is not run.
#[test]
fn a_task_whose_creation_was_committed_still_exists() {
    let r = al_testkit::run_in(SRC, "TS Probe", "AfterCommit");
    assert!(matches!(r, Ok(al_builtins::value::Value::Boolean(true))), "{r:?}");
    let r = al_testkit::text(al_testkit::run_in(SRC, "TS Probe", "RowAfterCommit"));
    assert_eq!(
        r,
        "Yes ready=Yes cu=50100 notBeforeBlank=Yes|Yes ready=Yes cu=50100 notBeforeBlank=Yes|\
         Yes ready=No cu=50100 notBeforeBlank=Yes|Yes ready=Yes cu=50100 notBeforeBlank=No|Yes|2|YesYesNoNoNo"
    );
}

#[test]
fn an_unknown_codeunit_raises_the_missing_object_error() {
    let m = al_testkit::error_message(al_testkit::run_in(SRC, "TS Probe", "UnknownCodeunit"));
    assert!(
        m.starts_with("You tried to invoke the CodeUnit object with the ID 99999999 from the object"),
        "{m}"
    );
}
