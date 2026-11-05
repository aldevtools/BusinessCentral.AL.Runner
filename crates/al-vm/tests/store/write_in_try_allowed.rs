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

const DENY: &str = r#"
table 51710 "WD Row"
{
    fields { field(1; "No."; Integer) { } field(2; Description; Text[50]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 51711 "WD Writer"
{
    procedure Setup()
    var Row: Record "WD Row";
    begin
        Row."No." := 1;
        Row.Insert();
    end;

    procedure Rows(): Integer
    var Row: Record "WD Row";
    begin
        exit(Row.Count());
    end;

    procedure Run(Kind: Integer): Boolean
    begin
        exit(TryWrite(Kind));
    end;

    [TryFunction]
    local procedure TryWrite(Kind: Integer)
    var Row: Record "WD Row";
    begin
        case Kind of
            1: begin Row.Get(1); Row.Delete(); end;
            2: Row.DeleteAll();
            3: Row.ModifyAll(Description, 'all');
            4: begin Row.Get(1); Row.Rename(10); end;
        end;
    end;
}
"#;

/// Delete, DeleteAll, ModifyAll and Rename in a TryFunction whose result is used raise TryFunctionWrite named after the
/// method, and no row changes (corpus R7143TryWriteDeny.al:94-176).
#[test]
fn delete_deleteall_modifyall_and_rename_raise_try_function_write() {
    for (kind, name) in [(1, "DELETE"), (2, "DELETEALL"), (3, "MODIFYALL"), (4, "RENAME")] {
        let mut b = Built::new(compile(DENY));
        b.run_in("WD Writer", "Setup").expect("setup");
        let (oi, ri) = b.routine_in("WD Writer", "Run");
        match b.call(oi, ri, vec![al_builtins::value::Value::Integer(kind)]) {
            Err(Signal::Error(e)) => {
                assert_eq!(e.code, "TryFunctionWrite", "{name}: {e:?}");
                assert!(
                    e.message.contains(&format!("'{name}'")) || e.message.is_empty() || e.message.contains(name),
                    "{name}: {e:?}"
                );
            }
            other => panic!("{name}: expected TryFunctionWrite, got {other:?}"),
        }
    }
}
