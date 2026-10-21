//! A global passed by `var` to a procedure while code reaches the same global by its own name. BC passes a `var` argument
//! by reference, so the parameter and the global would be one variable; the runtime copies the argument in and back. Only
//! the event form is pinned (events/lt-event-var-through-sender/EventVarThroughSender.al), so a plain procedure call where
//! the difference shows is refused (docs/probes-needed.md, "var parameter that is a global of the caller").
//! Found by Microsoft's Tests-ERM (Reminder Header.InsertTextLines passes its global NextLineNo by var and
//! InsertBlankLine adds to the global: "Reminder Line already exists" in 134997 and 134012).

use al_builtins::native::Signal;
use al_testkit::{int, run};

const SRC: &str = r#"
codeunit 50100 Alias
{
    var
        Next: Integer;

    procedure ViaNestedWrite(): Integer
    begin
        Next := 10;
        Bump(Next);
        exit(Next);
    end;

    local procedure Bump(var N: Integer)
    begin
        AddToGlobal();
        N := N + 1;
    end;

    local procedure AddToGlobal()
    begin
        Next := Next + 100;
    end;

    procedure ViaDirectRead(): Integer
    var
        Seen: Integer;
    begin
        Next := 10;
        Seen := ReadGlobal(Next);
        exit(Seen);
    end;

    local procedure ReadGlobal(var N: Integer): Integer
    begin
        N := 20;
        exit(Next);
    end;

    procedure LocalByVar(): Integer
    var
        L: Integer;
    begin
        Next := 10;
        L := 1;
        BumpLocal(L);
        exit(L + Next);
    end;

    local procedure BumpLocal(var N: Integer)
    begin
        AddToGlobal();
        N := N + 1;
    end;

    procedure GlobalByVarUntouched(): Integer
    begin
        Next := 10;
        Twice(Next);
        exit(Next);
    end;

    local procedure Twice(var N: Integer)
    begin
        N := N * 2;
        Helper();
    end;

    local procedure Helper()
    begin
    end;
}
"#;

#[test]
fn global_by_var_changed_by_a_nested_call_is_refused() {
    let r = run(SRC, "ViaNestedWrite");
    assert!(
        matches!(r, Err(Signal::Unsupported(ref why)) if why.contains("by var")),
        "{r:?}"
    );
}

#[test]
fn global_by_var_read_by_its_name_in_the_callee_is_refused() {
    let r = run(SRC, "ViaDirectRead");
    assert!(
        matches!(r, Err(Signal::Unsupported(ref why)) if why.contains("by var")),
        "{r:?}"
    );
}

/// A local passed by var cannot be reached by another name: nothing to refuse
/// (codeunit/TestScopeIsolationContracts.al:176 pins the write-back).
#[test]
fn local_by_var_runs() {
    assert_eq!(int(run(SRC, "LocalByVar")), 2 + 110);
}

/// A global passed by var that nothing else touches while the callee runs: the copy and the reference agree
/// (codeunit/TestCodeunitRunContracts.al:149 pins that the change reaches the caller).
#[test]
fn global_by_var_not_touched_otherwise_runs() {
    assert_eq!(int(run(SRC, "GlobalByVarUntouched")), 20);
}
