//! `Company` of the Scheduled Task row of a task created by `TaskScheduler.CreateTask` with a company argument: the argument
//! as given when it names the session's own company, the current company when it is blank (al-runner issue 7589, the
//! reporter's measurement on BC 29.0; assumption `scheduled-task-company-argument`). Without the argument it is pinned
//! (session-system/CnSessionLeads.al:34, :42). Another company leaves the column unknown, and a read of it is refused.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

// The reporter's repro (al-runner issue 7589), unchanged but for the test codeunit, which runs here as a plain one; and the
// other rows of the reporter's table.
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

codeunit 50200 "PX Worker"
{
    trigger OnRun()
    begin
    end;
}

codeunit 50202 "PX Task Company Arg"
{
    procedure CompanyArg_FilterOnCompany()
    var
        T: Record "Scheduled Task";
    begin
        TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, CompanyName());
        T.SetRange(Company, CompanyName());
        T.SetRange("Run Codeunit", Codeunit::"PX Worker");
        if T.IsEmpty() then
            Error('no row');
    end;
}

codeunit 50203 "PX Shapes"
{
    local procedure Read(Id: Guid): Text
    var
        T: Record "Scheduled Task";
        Found: Boolean;
    begin
        T.Get(Id);
        T.SetRange(Company, CompanyName());
        T.SetRange(ID, Id);
        Found := not T.IsEmpty();
        exit(Format(T.Company = CompanyName()) + '|' + Format(T.Company = UpperCase(CompanyName())) + '|' + Format(Found));
    end;

    // The reporter's guard: a second call reads the row of the first and schedules nothing.
    procedure Guard(): Integer
    var
        T: Record "Scheduled Task";
        I: Integer;
    begin
        for I := 1 to 2 do begin
            T.Reset();
            T.SetRange(Company, CompanyName());
            T.SetRange("Run Codeunit", Codeunit::"PX Worker");
            if T.IsEmpty() then
                TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, CompanyName(), CurrentDateTime() + 60000);
        end;
        T.Reset();
        T.SetRange("Run Codeunit", Codeunit::"PX Worker");
        exit(T.Count());
    end;

    procedure Current(): Text
    begin
        exit(Read(TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, CompanyName())));
    end;

    procedure Blank(): Text
    begin
        exit(Read(TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, '')));
    end;

    procedure Upper(): Text
    begin
        exit(Read(TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, UpperCase(CompanyName()))));
    end;

    procedure Lower(): Text
    begin
        exit(Read(TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, LowerCase(CompanyName()))));
    end;

    // Creates the task and reads nothing of the table.
    procedure CreateOnly()
    begin
        TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, CompanyName());
    end;

    // Reads the row but not its Company (Y43TaskAfterCommit.al reads Not Before of such a task).
    procedure ReadOtherColumn(): Boolean
    var
        T: Record "Scheduled Task";
    begin
        T.Get(TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, CompanyName()));
        T.SetRange("Run Codeunit", Codeunit::"PX Worker");
        exit(T."Run Codeunit" = Codeunit::"PX Worker");
    end;

    procedure CreateAndCommit()
    begin
        TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, CompanyName());
        Commit();
    end;

    // The reporter's guard in a later test: reads the row another routine created.
    procedure ReadOnly(): Boolean
    var
        T: Record "Scheduled Task";
    begin
        T.SetRange(Company, CompanyName());
        T.SetRange("Run Codeunit", Codeunit::"PX Worker");
        exit(not T.IsEmpty());
    end;

    procedure CreateAndRollBack()
    begin
        asserterror begin
            TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, CompanyName());
            Error('roll back');
        end;
    end;

    procedure Other(): Text
    begin
        exit(Read(TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true, 'No Such Company For Probe')));
    end;

    // A task without the company argument takes no assumption (CnSessionLeads.al:34, :42).
    procedure NoArgument(): Text
    begin
        exit(Read(TaskScheduler.CreateTask(Codeunit::"PX Worker", 0, true)));
    end;
}
"#;

fn run(object: &str, routine: &str) -> (Result<Value, Signal>, Vec<Assumption>) {
    let mut b = Built::from_source(SRC);
    clear_hits();
    let r = b.run_in(object, routine);
    (r, take_hits())
}

fn text(routine: &str) -> (String, Vec<Assumption>) {
    let (r, hits) = run("PX Shapes", routine);
    (al_testkit::text(r), hits)
}

const FLAG: Assumption = Assumption::ScheduledTaskCompanyArgument;

#[test]
fn the_reporters_repro_finds_the_task_by_company() {
    let (r, hits) = run("PX Task Company Arg", "CompanyArg_FilterOnCompany");
    assert!(r.is_ok(), "{r:?}");
    assert_eq!(hits, vec![FLAG]);
}

#[test]
fn the_guard_schedules_the_task_once() {
    let (r, hits) = run("PX Shapes", "Guard");
    assert!(matches!(r, Ok(Value::Integer(1))), "{r:?}");
    assert_eq!(hits, vec![FLAG]);
}

#[test]
fn company_holds_the_current_company_for_its_name_and_for_a_blank_argument() {
    assert_eq!(text("Current"), ("Yes|No|Yes".to_string(), vec![FLAG]));
    assert_eq!(text("Blank"), ("Yes|No|Yes".to_string(), vec![FLAG]));
}

#[test]
fn company_holds_an_upper_cased_name_as_given() {
    assert_eq!(text("Upper"), ("No|Yes|No".to_string(), vec![FLAG]));
}

#[test]
fn company_holds_a_lower_cased_name_as_given() {
    assert_eq!(text("Lower"), ("No|No|No".to_string(), vec![FLAG]));
}

// The mark is made where the row is read, not where it is written: the runner clears the marks before each test, so a
// task the login or an earlier test created must flag the test that reads it.
#[test]
fn creating_the_task_alone_takes_no_assumption() {
    let (r, hits) = run("PX Shapes", "CreateOnly");
    assert!(r.is_ok(), "{r:?}");
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn reading_another_column_of_the_row_takes_no_assumption() {
    let (r, hits) = run("PX Shapes", "ReadOtherColumn");
    assert!(matches!(r, Ok(Value::Boolean(true))), "{r:?}");
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn a_later_read_of_the_task_takes_the_assumption() {
    let mut b = Built::from_source(SRC);
    assert!(b.run_in("PX Shapes", "CreateOnly").is_ok());
    clear_hits();
    assert!(matches!(b.run_in("PX Shapes", "ReadOnly"), Ok(Value::Boolean(true))));
    assert_eq!(take_hits(), vec![FLAG]);
}

#[test]
fn a_read_after_the_commit_takes_the_assumption() {
    let mut b = Built::from_source(SRC);
    assert!(b.run_in("PX Shapes", "CreateAndCommit").is_ok());
    clear_hits();
    assert!(matches!(b.run_in("PX Shapes", "ReadOnly"), Ok(Value::Boolean(true))));
    assert_eq!(take_hits(), vec![FLAG]);
}

#[test]
fn a_rolled_back_task_leaves_no_assumption() {
    let mut b = Built::from_source(SRC);
    assert!(b.run_in("PX Shapes", "CreateAndRollBack").is_ok());
    clear_hits();
    assert!(matches!(b.run_in("PX Shapes", "ReadOnly"), Ok(Value::Boolean(false))));
    assert!(take_hits().is_empty());
}

#[test]
fn another_company_leaves_company_unknown() {
    let (r, _) = run("PX Shapes", "Other");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("'Company'")),
        "{r:?}"
    );
}

#[test]
fn a_task_without_the_argument_takes_no_assumption() {
    assert_eq!(text("NoArgument"), ("Yes|No|Yes".to_string(), vec![]));
}
