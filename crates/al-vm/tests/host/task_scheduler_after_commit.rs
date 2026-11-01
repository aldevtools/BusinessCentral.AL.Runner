//! A TaskScheduler task after the transaction that created it was committed: it stays in the Scheduled Task table as it
//! was created and never runs in a test (platform/lt-y43-task-after-commit/Y43TaskAfterCommit.al, corpus PR 966).

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
table 50110 "TS Holder"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; TaskId; Guid) { }
        field(3; Scheduled; Boolean) { FieldClass = FlowField; CalcFormula = exist("Scheduled Task" where(ID = field(TaskId))); }
    }
    keys { key(PK; Id) { } }
}
codeunit 50100 "TS Target" { trigger OnRun() begin end; }
codeunit 50101 "TS Probe"
{
    local procedure Read(Id: Guid): Text
    var
        Task: Record "Scheduled Task";
    begin
        if not Task.Get(Id) then
            exit('No;');
        exit('Yes ' + Format(Task."Is Ready") + ' ' + Format(Task."Run Codeunit") + ' ' + Format(Task."Not Before" = 0DT) + ';');
    end;

    local procedure ReadyCount(): Integer
    var
        Task: Record "Scheduled Task";
    begin
        Task.SetRange("Is Ready", true);
        Task.SetRange("User ID", UserSecurityId());
        exit(Task.Count());
    end;

    procedure Rows(): Text
    var
        Task: Record "Scheduled Task";
        Ready: Guid;
        NotReady: Guid;
        Later: Guid;
        NotBefore: DateTime;
        R: Text;
    begin
        Ready := TaskScheduler.CreateTask(Codeunit::"TS Target", 0);
        R := Read(Ready) + Format(ReadyCount()) + ';';
        Commit();
        NotReady := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        Commit();
        R += Read(Ready) + Read(NotReady) + Format(ReadyCount()) + ';';
        NotBefore := CreateDateTime(Today() + 30, 0T);
        Later := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, true, CompanyName(), NotBefore);
        Commit();
        Task.Get(Later);
        R += Read(Later) + Format(Task."Not Before" = NotBefore) + ';';
        R += Format(TaskScheduler.CancelTask(Ready)) + Format(ReadyCount()) + ';';
        exit(R);
    end;

    procedure Calls(): Text
    var
        Task: Record "Scheduled Task";
        Id: Guid;
        R: Text;
    begin
        Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        Commit();
        R := Format(TaskScheduler.TaskExists(Id)) + Format(TaskScheduler.SetTaskReady(Id));
        Task.Get(Id);
        R += Format(Task."Is Ready");
        R += Format(TaskScheduler.CancelTask(Id)) + Format(Task.Get(Id)) + Format(TaskScheduler.TaskExists(Id)) + Format(TaskScheduler.CancelTask(Id));
        exit(R);
    end;

    procedure FlowField(): Text
    var
        H: Record "TS Holder";
        Id: Guid;
        R: Text;
    begin
        Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        H.Id := 1;
        H.TaskId := Id;
        H.Insert();
        H.Id := 2;
        H.TaskId := CreateGuid();
        H.Insert();
        Commit();
        H.Get(1);
        H.CalcFields(Scheduled);
        R := Format(H.Scheduled);
        H.Get(2);
        H.CalcFields(Scheduled);
        R += Format(H.Scheduled);
        TaskScheduler.CancelTask(Id);
        H.Get(1);
        H.CalcFields(Scheduled);
        exit(R + Format(H.Scheduled));
    end;

    // Only that Not Before is no longer blank after SetTaskReady is pinned, not its value.
    procedure NotBeforeAfterSetTaskReady(): Text
    var
        Task: Record "Scheduled Task";
        Id: Guid;
    begin
        Id := TaskScheduler.CreateTask(Codeunit::"TS Target", 0, false);
        Commit();
        TaskScheduler.SetTaskReady(Id);
        Task.Get(Id);
        exit(Format(Task."Not Before"));
    end;
}
"#;

#[test]
fn a_committed_task_stays_in_the_table_as_created() {
    let r = al_testkit::text(al_testkit::run_in(SRC, "TS Probe", "Rows"));
    assert_eq!(
        r,
        "Yes Yes 50100 Yes;1;Yes Yes 50100 Yes;Yes No 50100 Yes;1;Yes Yes 50100 No;Yes;Yes1;"
    );
}

#[test]
fn taskscheduler_calls_on_a_committed_task() {
    let r = al_testkit::text(al_testkit::run_in(SRC, "TS Probe", "Calls"));
    assert_eq!(r, "YesYesYesYesNoNoNo");
}

#[test]
fn an_exist_flowfield_over_a_committed_task() {
    let r = al_testkit::text(al_testkit::run_in(SRC, "TS Probe", "FlowField"));
    assert_eq!(r, "YesNoNo");
}

#[test]
fn not_before_after_set_task_ready_is_refused() {
    let r = al_testkit::run_in(SRC, "TS Probe", "NotBeforeAfterSetTaskReady");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("Scheduled Task")),
        "{r:?}"
    );
}
