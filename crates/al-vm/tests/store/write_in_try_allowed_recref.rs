//! al-runner issue 7347: with `--allow-write-in-try-functions` a write through a RecordRef inside a TryFunction whose result
//! is used runs as a write through a Record does: Insert, Modify, Delete, DeleteAll and Rename change the rows, the
//! TryFunction returns true, and an error after the write makes it return false without undoing the write (corpus
//! tests/al-language-trywrite-allowed/TryWriteAllowed.al, the RecRef* tests, run by the corpus CI job "BC TryWrites").

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{Built, blocked, compile, text};
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 51730 "WR Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 51731 "WR Writer"
{
    procedure Initialize()
    var Row: Record "WR Row";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Name := 'a'; Row.Insert();
        Row.Id := 2; Row.Name := 'b'; Row.Insert();
        Row.Id := 3; Row.Name := 'c'; Row.Insert();
        Commit();
        ClearLastError();
    end;

    local procedure State(): Text
    var Row: Record "WR Row"; Result: Text;
    begin
        if Row.FindSet() then
            repeat
                Result += Format(Row.Id) + '=' + Row.Name + ';';
            until Row.Next() = 0;
        exit(Result);
    end;

    [TryFunction]
    local procedure TryWriteRef(Op: Text; FailAfter: Boolean)
    var RecRef: RecordRef;
    begin
        RecRef.Open(Database::"WR Row");
        case Op of
            'insert': begin RecRef.Init(); RecRef.Field(1).Value := 10; RecRef.Field(2).Value := 'new'; RecRef.Insert(); end;
            'modify': begin RecRef.FindFirst(); RecRef.Field(2).Value := 'mod'; RecRef.Modify(); end;
            'delete': begin RecRef.FindFirst(); RecRef.Delete(); end;
            'deleteall': RecRef.DeleteAll();
            'rename': begin RecRef.FindFirst(); RecRef.Rename(11); end;
        end;
        if FailAfter then
            Error('error after write');
    end;

    procedure Probe(Op: Text; FailAfter: Boolean): Text
    var Ok: Boolean; Result: Text;
    begin
        Initialize();
        Ok := TryWriteRef(Op, FailAfter);
        Result := Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|';
        ClearLastError();
        Result += State() + '|';
        Commit();
        exit(Result + State());
    end;
}
"#;

fn probe(allowed: bool, op: &str, fail_after: bool) -> Result<Value, Signal> {
    let lowered = compile(SRC);
    let mut engine = Engine::new(lowered.program.clone(), blocked(&lowered));
    std::sync::Arc::get_mut(&mut engine)
        .expect("engine not shared")
        .write_in_try_allowed = allowed;
    let mut b = Built {
        session: Session::new(engine),
        lowered,
    };
    let (oi, ri) = b.routine_in("WR Writer", "Probe");
    b.call(oi, ri, vec![Value::Text(op.into()), Value::Boolean(fail_after)])
        .map(|(v, _)| v)
}

/// TryWriteAllowed.al RecRefInsert_* to RecRefRename_*: the same results as the writes through a Record.
#[test]
fn recordref_writes_in_try_function_run_with_writes_allowed() {
    let cases = [
        ("insert", "1=a;2=b;3=c;10=new;"),
        ("modify", "1=mod;2=b;3=c;"),
        ("delete", "2=b;3=c;"),
        ("deleteall", ""),
        ("rename", "2=b;3=c;11=a;"),
    ];
    for (op, state) in cases {
        assert_eq!(text(probe(true, op, false)), format!("Yes|||{state}|{state}"), "{op}");
        assert_eq!(
            text(probe(true, op, true)),
            format!("No|error after write|Dialog|{state}|{state}"),
            "{op} with an error after it"
        );
    }
}

/// In the default mode a RecordRef write in a used TryFunction stays refused: no probe pins it there.
#[test]
fn recordref_write_in_try_function_stays_refused_by_default() {
    match probe(false, "modify", false) {
        Err(Signal::Unsupported(m)) => assert!(m.contains("RecordRef.Modify"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
