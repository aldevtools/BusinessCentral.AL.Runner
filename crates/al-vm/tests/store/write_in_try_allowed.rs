//! al-runner issue 7143: `--allow-write-in-try-functions` (the service-tier setting DisableWriteInsideTryFunctions off, as on
//! Business Central online). What BC does with the setting off is not measured yet, so a write of a TryFunction whose
//! result is used is refused in that mode instead of raising TryFunctionWrite; the default stays the denied mode the
//! corpus pins (errors/CuTryTests.al:83-150).

use al_builtins::native::Signal;
use al_testkit::{Built, blocked, compile};
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 51700 "WT Row"
{
    fields { field(1; "No."; Integer) { } field(2; Description; Text[50]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 51701 "WT Writer"
{
    procedure ModifyRow(): Boolean
    begin
        exit(TryModifyRow());
    end;

    procedure DeleteAllRows(): Boolean
    begin
        exit(TryDeleteAllRows());
    end;

    [TryFunction]
    local procedure TryModifyRow()
    var Row: Record "WT Row";
    begin
        Row.Get(1);
        Row.Description := 'changed';
        Row.Modify();
    end;

    [TryFunction]
    local procedure TryDeleteAllRows()
    var Row: Record "WT Row";
    begin
        Row.DeleteAll();
    end;

    procedure Setup()
    var Row: Record "WT Row";
    begin
        Row."No." := 1;
        Row.Insert();
    end;
}
"#;

fn call(allowed: bool, routine: &str) -> Result<al_builtins::value::Value, Signal> {
    let lowered = compile(SRC);
    let mut engine = Engine::new(lowered.program.clone(), blocked(&lowered));
    std::sync::Arc::get_mut(&mut engine)
        .expect("engine not shared")
        .write_in_try_allowed = allowed;
    let mut built = Built {
        session: Session::new(engine),
        lowered,
    };
    built.run_in("WT Writer", "Setup").expect("setup");
    built.run_in("WT Writer", routine)
}

fn unsupported(r: Result<al_builtins::value::Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn modify_in_try_function_is_refused_with_writes_allowed() {
    let why = unsupported(call(true, "ModifyRow"));
    assert!(
        why.contains("MODIFY inside a TryFunction whose result is used, with --allow-write-in-try-functions"),
        "{why}"
    );
}

#[test]
fn delete_all_in_try_function_is_refused_with_writes_allowed() {
    let why = unsupported(call(true, "DeleteAllRows"));
    assert!(
        why.contains("DELETEALL inside a TryFunction whose result is used, with --allow-write-in-try-functions"),
        "{why}"
    );
}

#[test]
fn modify_in_try_function_raises_try_function_write_by_default() {
    match call(false, "ModifyRow") {
        Err(Signal::Error(e)) => assert_eq!(e.code, "TryFunctionWrite", "{e:?}"),
        other => panic!("expected TryFunctionWrite, got {other:?}"),
    }
}
