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
    let probe = (s.engine.program.objects.iter())
        .position(|o| o.name == "IT Probe")
        .expect("probe");
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

/// Without the table a later session cannot see the task through its row, which exists in BC
/// (IcInstallCanCreateTaskTests.al:99): the CreateTask runs (the session that ran it holds the task, as main does), and a
/// later session's TaskExists of it is refused.
#[test]
fn a_task_install_code_created_without_the_scheduled_task_table_is_refused_when_read_later() {
    let mut s = session(PROBE);
    s.set_install_without_client(true);
    let Ok(Value::Guid(id)) = call(&mut s, "Create", vec![]) else {
        panic!("created")
    };
    assert!(matches!(
        call(&mut s, "Exists", vec![Value::Guid(id)]),
        Ok(Value::Boolean(true))
    ));
    s.set_install_without_client(false);
    let b = s.capture_baseline();
    let mut later = Session::new(s.engine.clone());
    later.restore_baseline(&b);
    let r = call(&mut later, "Exists", vec![Value::Guid(id)]);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("Scheduled Task")),
        "{r:?}"
    );
}

/// A task install code created exists in a later session that starts from the install baseline, through its row, with
/// the mark (IcInstallCanCreateTaskTests.al:99): the captured baseline, its bytes and its frozen copy carry the task's id.
/// A task id install code did not create keeps main's false and no mark.
#[test]
fn a_task_install_code_created_exists_later_through_its_row_with_the_mark() {
    let mut s = session(&format!("{TABLE}{PROBE}"));
    s.set_install_without_client(true);
    s.set_install_scope(true);
    let Ok(Value::Guid(id)) = call(&mut s, "Create", vec![]) else {
        panic!("created")
    };
    s.set_install_without_client(false);
    s.set_install_scope(false);
    let captured = s.capture_baseline();
    let read_back = crate::Baseline::from_bytes(&captured.to_bytes().expect("bytes")).expect("read back");
    let thawed = captured.freeze().expect("frozen").thaw(None);
    for b in [&captured, &read_back, &thawed] {
        let mut later = Session::new(s.engine.clone());
        later.restore_baseline(b);
        let _ = take_hits();
        assert!(matches!(
            call(&mut later, "Exists", vec![Value::Guid([7; 16])]),
            Ok(Value::Boolean(false))
        ));
        assert!(!take_hits().contains(&Assumption::InstallSessionCanCreateTask));
        assert!(matches!(
            call(&mut later, "Exists", vec![Value::Guid(id)]),
            Ok(Value::Boolean(true))
        ));
        assert!(take_hits().contains(&Assumption::InstallSessionCanCreateTask));
    }
}

/// A task a test codeunit's own session created and then forgot (`tasks` cleared) is not one install code created: no
/// row read stands for it and no mark.
#[test]
fn a_task_created_outside_install_code_is_not_carried_as_an_install_task() {
    let mut s = session(&format!("{TABLE}{PROBE}"));
    let Ok(Value::Guid(id)) = call(&mut s, "Create", vec![]) else {
        panic!("created")
    };
    let b = s.capture_baseline();
    let mut later = Session::new(s.engine.clone());
    later.restore_baseline(&b);
    let _ = take_hits();
    assert!(matches!(
        call(&mut later, "Exists", vec![Value::Guid(id)]),
        Ok(Value::Boolean(false))
    ));
    assert!(!take_hits().contains(&Assumption::InstallSessionCanCreateTask));
}
