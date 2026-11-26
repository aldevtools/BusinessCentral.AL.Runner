//! Tasks created in install code (assumption install-session-can-create-task): the refusals of `create_task` and the
//! Scheduled Task rows that stand for them in a later session (`install_task_row`, `install_task_rows_assumed`).

use crate::{Engine, Session};
use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const TABLE: &str = r#"
table 2000000175 "Scheduled Task"
{
    fields { field(1; ID; Guid) { } field(5; "Run Codeunit"; Integer) { } field(9; "Is Ready"; Boolean) { } }
    keys { key(PK; ID) { } }
}
"#;

const PROBE: &str = r#"
codeunit 50100 "IT Target" { trigger OnRun() begin end; }
codeunit 50101 "IT Probe"
{
    procedure Create(): Guid
    begin
        exit(TaskScheduler.CreateTask(Codeunit::"IT Target", 0));
    end;
    procedure CreateInTry(): Boolean
    begin
        exit(TryCreate());
    end;
    [TryFunction]
    local procedure TryCreate()
    begin
        TaskScheduler.CreateTask(Codeunit::"IT Target", 0);
    end;
    procedure Exists(Id: Guid): Boolean
    begin
        exit(TaskScheduler.TaskExists(Id));
    end;
}
"#;

fn session(src: &str) -> Session {
    let l = al_testkit::compile(src);
    Session::new(Engine::new(l.program.clone(), al_testkit::blocked(&l)))
}

fn call(s: &mut Session, routine: &str, args: Vec<Value>) -> Result<Value, Signal> {
    let probe = (s.engine.program.objects.iter()).position(|o| o.name == "IT Probe").expect("probe");
    let ri = (s.engine.program.objects[probe].routines.iter())
        .position(|r| r.name == routine)
        .expect("routine");
    let inst = s.codeunit_instance(probe);
    s.call(probe, ri, inst, args).map(|(v, _)| v)
}

/// Al-runner issue 9134: BC fails the install there with an error the corpus does not assert yet.
#[test]
fn create_task_in_a_try_function_of_install_code_is_refused() {
    let mut s = session(&format!("{TABLE}{PROBE}"));
    s.set_install_without_client(true);
    let r = call(&mut s, "CreateInTry", vec![]);
    assert!(matches!(&r, Err(Signal::Unsupported(m)) if m.contains("9134")), "{r:?}");
}

/// Without the table the task's id is lost to the test, which sees it in BC (IcInstallCanCreateTaskTests.al:99).
#[test]
fn create_task_in_install_code_without_the_scheduled_task_table_is_refused() {
    let mut s = session(PROBE);
    s.set_install_without_client(true);
    let r = call(&mut s, "Create", vec![]);
    assert!(matches!(&r, Err(Signal::Unsupported(m)) if m.contains("Scheduled Task")), "{r:?}");
    // Outside install code main's rules hold: the task is created.
    let mut s = session(PROBE);
    assert!(matches!(call(&mut s, "Create", vec![]), Ok(Value::Guid(_))));
}

/// A task install code created exists in a session that does not hold its id, through its row, with the mark; without
/// install rows in the table, an unknown id keeps main's false and no mark.
#[test]
fn a_task_install_code_created_exists_later_through_its_row_with_the_mark() {
    let mut s = session(&format!("{TABLE}{PROBE}"));
    s.set_install_without_client(true);
    s.set_install_scope(true);
    let Ok(Value::Guid(id)) = call(&mut s, "Create", vec![]) else { panic!("created") };
    s.set_install_without_client(false);
    s.set_install_scope(false);
    s.tasks = Default::default();
    let _ = take_hits();
    assert!(matches!(call(&mut s, "Exists", vec![Value::Guid(id)]), Ok(Value::Boolean(false))));
    assert!(!take_hits().contains(&Assumption::InstallSessionCanCreateTask));
    s.baseline_rows.insert(super::SCHEDULED_TASK);
    assert!(matches!(call(&mut s, "Exists", vec![Value::Guid(id)]), Ok(Value::Boolean(true))));
    assert!(take_hits().contains(&Assumption::InstallSessionCanCreateTask));
}
