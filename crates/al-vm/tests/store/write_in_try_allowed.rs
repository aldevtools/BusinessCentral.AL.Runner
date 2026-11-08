//! al-runner issue 7143: `--allow-write-in-try-functions` (the service-tier setting DisableWriteInsideTryFunctions off, as on
//! Business Central online). With the setting off a write of a TryFunction whose result is used runs: the TryFunction returns
//! true, or false with the error's last error text and code when an error follows the write, and the write stays
//! (corpus tests/al-language-trywrite-allowed/TryWriteAllowed.al:44-144, run by the corpus CI job "BC TryWrites"). The
//! default stays the denied mode the corpus pins (errors/CuTryTests.al:83-150).

use al_builtins::native::Signal;
use al_testkit::{Built, blocked, compile, text};
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

const ALLOWED: &str = r#"
table 51720 "WA Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
    // The triggers of the corpus table LT CU Row (errors/CuFixtures.al:20-30), which TryWriteAllowed.al writes.
    trigger OnInsert() begin if Name = 'commit-in-oninsert' then Commit(); end;
    trigger OnModify() begin if Name = 'commit-in-onmodify' then Commit(); end;
}

codeunit 51721 "WA Writer"
{
    procedure Initialize()
    var Row: Record "WA Row";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Name := 'a'; Row.Insert();
        Row.Id := 2; Row.Name := 'b'; Row.Insert();
        Row.Id := 3; Row.Name := 'c'; Row.Insert();
        Commit();
        ClearLastError();
    end;

    local procedure State(): Text
    var Row: Record "WA Row"; Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Format(Row.Id) + '=' + Row.Name + ';';
            until Row.Next() = 0;
        exit(Result);
    end;

    [TryFunction]
    local procedure TryWrite(Op: Text; FailAfter: Boolean)
    var Row: Record "WA Row";
    begin
        case Op of
            'insert': begin Row.Id := 10; Row.Name := 'new'; Row.Insert(); end;
            'modify': begin Row.Get(1); Row.Name := 'mod'; Row.Modify(); end;
            'delete': begin Row.Get(1); Row.Delete(); end;
            'deleteall': Row.DeleteAll();
            'modifyall': Row.ModifyAll(Name, 'all');
            'rename': begin Row.Get(1); Row.Rename(11); end;
            'commit': Commit();
        end;
        if FailAfter then
            Error('error after write');
    end;

    procedure Probe(Op: Text; FailAfter: Boolean): Text
    var Ok: Boolean; Result: Text;
    begin
        Initialize();
        Ok := TryWrite(Op, FailAfter);
        Result := Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|';
        ClearLastError();
        Result += State() + '|';
        Commit();
        exit(Result + State());
    end;
}
"#;

fn probe(op: &str, fail_after: bool) -> Result<al_builtins::value::Value, Signal> {
    use al_builtins::value::Value;
    let lowered = compile(ALLOWED);
    let mut engine = Engine::new(lowered.program.clone(), blocked(&lowered));
    std::sync::Arc::get_mut(&mut engine)
        .expect("engine not shared")
        .write_in_try_allowed = true;
    let mut b = Built {
        session: Session::new(engine),
        lowered,
    };
    let (oi, ri) = b.routine_in("WA Writer", "Probe");
    b.call(oi, ri, vec![Value::Text(op.into()), Value::Boolean(fail_after)])
        .map(|(v, _)| v)
}

/// Each write kind runs inside a TryFunction whose result is used: the TryFunction returns true and the write stays; with an
/// error after the write it returns false, the last error is the error's text and code 'Dialog', and the write stays, also
/// after a Commit (TryWriteAllowed.al:85-144: Insert, Modify, Delete, DeleteAll, ModifyAll, Rename, and Commit).
#[test]
fn writes_in_try_function_run_with_writes_allowed() {
    let cases = [
        ("insert", "1=a;2=b;3=c;10=new;"),
        ("modify", "1=mod;2=b;3=c;"),
        ("delete", "2=b;3=c;"),
        ("deleteall", ""),
        ("modifyall", "1=all;2=all;3=all;"),
        ("rename", "2=b;3=c;11=a;"),
    ];
    for (op, state) in cases {
        assert_eq!(text(probe(op, false)), format!("Yes|||{state}|{state}"), "{op}");
        assert_eq!(
            text(probe(op, true)),
            format!("No|error after write|Dialog|{state}|{state}"),
            "{op} with an error after it"
        );
    }
    assert_eq!(
        text(probe("commit", false)),
        "Yes|||1=a;2=b;3=c;|1=a;2=b;3=c;",
        "commit"
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
