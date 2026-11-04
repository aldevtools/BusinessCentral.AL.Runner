//! A guarded `Codeunit.Run` whose OnRun uses the result of a TryFunction that writes: the TryFunctionWrite error ends the run,
//! the guard traps it (returns false, last error text and code set, the run's rows undone).
//!
//! Pinned by errors/Y68Small.al:211-274 (Q6_CodeunitRun_GuardedOnRunWrites_*), values measured on the
//! 27.5, 28.5 and 29.0 CI containers (the SaaS server does not apply the rule).

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const FIXTURES: &str = r#"
table 51400 "GR Key"
{
    fields { field(1; A; Code[20]) { } field(2; B; Text[50]) { } }
    keys { key(PK; A, B) { Clustered = true; } }
}

codeunit 51401 "GR Runner"
{
    SingleInstance = false;
    var Mode: Integer; LogText: Text;
    procedure SetMode(NewMode: Integer) begin Mode := NewMode; LogText := ''; end;
    procedure Log(): Text begin exit(LogText); end;
    trigger OnRun()
    var Ok: Boolean;
    begin
        LogText += 'start;';
        case Mode of
            1: Ok := TryInsert('1');
            2: TryInsert('2');
            3: if TryInsert('3') then LogText += 'then;' else LogText += 'else;';
            4: Ok := TryModifyOnly('4');
            5: Ok := TryTemp('5');
        end;
        LogText += 'end ok=' + Format(Ok) + ';';
    end;
    [TryFunction]
    local procedure TryInsert(KeyA: Code[20])
    var K: Record "GR Key";
    begin K.A := KeyA; K.Insert(); end;
    [TryFunction]
    local procedure TryModifyOnly(KeyA: Code[20])
    var K: Record "GR Key";
    begin K.A := KeyA; K.Modify(); end;
    [TryFunction]
    local procedure TryTemp(KeyA: Code[20])
    var K: Record "GR Key" temporary;
    begin K.A := KeyA; K.Insert(); end;
}
"#;

const BODY: &str = r#"
    Subtype = Test;
    local procedure RunMode(Mode: Integer): Text
    var K: Record "GR Key"; Runner: Codeunit "GR Runner"; Ok: Boolean; Rows: Text;
    begin
        K.DeleteAll();
        Commit();
        ClearLastError();
        Runner.SetMode(Mode);
        Ok := Runner.Run();
        if K.FindSet() then repeat Rows += '[' + K.A + '|' + K.B + ']'; until K.Next() = 0;
        exit(Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Rows + '|' + Runner.Log());
    end;
    [Test] procedure M1(): Text begin exit(RunMode(1)); end;
    [Test] procedure M2(): Text begin exit(RunMode(2)); end;
    [Test] procedure M3(): Text begin exit(RunMode(3)); end;
    [Test] procedure M4(): Text begin exit(RunMode(4)); end;
    [Test] procedure M5(): Text begin exit(RunMode(5)); end;
    [Test] procedure Direct(): Text
    var Runner: Codeunit "GR Runner"; Ok: Boolean;
    begin Runner.SetMode(1); Ok := Runner.Run(); exit(Format(Ok)); end;
"#;

fn go(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    run_in(&format!("{FIXTURES}\ncodeunit 51402 C {{ {BODY} }}"), "C", routine)
}

fn trapped(function: &str) -> String {
    format!(
        "No|Call to the function '{function}' is not allowed inside the call to 'RunMode' when it is used as a TryFunction.|TryFunctionWrite||start;"
    )
}

/// Y68Small.al:221-225 (expression form), :231-240 (if and if-not forms).
#[test]
fn a_used_try_that_inserts_inside_a_guarded_run_is_trapped_by_the_guard() {
    assert_eq!(text(go("M1")), trapped("INSERT"));
    assert_eq!(text(go("M3")), trapped("INSERT"));
}

/// Y68Small.al:226-230: a TryFunction whose result is unused writes, and the guarded run commits it.
#[test]
fn an_unused_try_inside_a_guarded_run_writes() {
    assert_eq!(text(go("M2")), "Yes|||[2|]|start;end ok=No;");
}

/// Y68Small.al:263-267: Modify names MODIFY, also on a row that does not exist.
#[test]
fn a_used_try_that_modifies_inside_a_guarded_run_names_modify() {
    assert_eq!(text(go("M4")), trapped("MODIFY"));
}

/// Y68Small.al:268-272: a temporary record is written.
#[test]
fn a_used_try_on_a_temporary_record_inside_a_guarded_run_succeeds() {
    assert_eq!(text(go("M5")), "Yes||||start;end ok=Yes;");
}

/// The scope name is pinned only for a run called by a helper of the test method (Y68Small.al:191-201); a run called by the
/// test method itself keeps the unpinned text.
#[test]
fn a_guarded_run_called_by_the_test_method_itself_stays_unpinned() {
    match go("Direct") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("not pinned"), "{m}"),
        other => panic!("{other:?}"),
    }
}
