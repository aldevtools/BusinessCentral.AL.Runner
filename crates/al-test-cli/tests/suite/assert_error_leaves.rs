//! Leaving an `asserterror` body by exit, break or continue (al-runner issue 8289). No corpus test measures it, so every such
//! leave is refused when the run reaches it: the lowering turns it into a refusal, and the VM refuses an `exit` with a value
//! whose value raised nothing. A leave the run does not reach, a break or continue of a loop inside the body, and an exit of a
//! procedure the body calls keep main's result.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const LEAVES: &str = r#"
codeunit 51960 "AE Leaves"
{
    Subtype = Test;

    [Test]
    procedure ContinueOutOfTheBody()
    var
        I: Integer;
    begin
        for I := 1 to 2 do
            asserterror begin
                I := I;
                continue;
            end;
    end;

    [Test]
    procedure BreakOutOfTheBody()
    var
        I: Integer;
    begin
        for I := 1 to 2 do
            asserterror begin
                I := I;
                break;
            end;
    end;

    [Test]
    procedure ExitOutOfTheBody()
    var
        I: Integer;
    begin
        asserterror begin
            I := 1;
            exit;
        end;
    end;

    [Test]
    procedure ExitInsideANestedAssertError()
    var
        I: Integer;
    begin
        asserterror begin
            asserterror exit;
            Error('x');
        end;
    end;

    [Test]
    procedure ExitAsTheWholeBody()
    begin
        asserterror exit;
    end;

    [Test]
    procedure ExitWithAValueOfAProcedureThatLeaves()
    begin
        if Answer() <> 1 then
            Error('answer');
    end;

    local procedure Answer(): Integer
    begin
        asserterror exit(1);
    end;

    [Test]
    procedure ExitWithAValueThatRaises()
    begin
        if Raises() <> '' then
            Error('returned');
        if GetLastErrorText() <> 'The SELECTSTR comma-string A,B,C does not contain a value for index 5.' then
            Error('%1', GetLastErrorText());
    end;

    local procedure Raises(): Text
    begin
        asserterror exit(SelectStr(5, 'A,B,C'));
    end;

    [Test]
    procedure BreakOfALoopInsideTheBody()
    begin
        asserterror begin
            repeat
                break;
            until false;
            Error('x');
        end;
        if GetLastErrorText() <> 'x' then
            Error('%1', GetLastErrorText());
    end;

    [Test]
    procedure ContinueOfALoopInsideTheBody()
    var
        I: Integer;
    begin
        asserterror begin
            for I := 1 to 2 do
                continue;
            Error('x');
        end;
    end;

    [Test]
    procedure ExitOfACalledProcedure()
    begin
        asserterror begin
            Leave();
            Error('x');
        end;
    end;

    local procedure Leave()
    begin
        exit;
    end;

    [Test]
    procedure UntakenExit()
    var
        Skip: Boolean;
    begin
        asserterror begin
            if Skip then
                exit;
            Error('x');
        end;
        if GetLastErrorText() <> 'x' then
            Error('%1', GetLastErrorText());
    end;

    [Test]
    procedure UntakenBreak()
    var
        I: Integer;
        Skip: Boolean;
    begin
        for I := 1 to 2 do
            asserterror begin
                if Skip then
                    break;
                Error('x');
            end;
        if GetLastErrorText() <> 'x' then
            Error('%1', GetLastErrorText());
    end;

    [Test]
    procedure UntakenContinue()
    var
        I: Integer;
        Skip: Boolean;
    begin
        for I := 1 to 2 do
            asserterror begin
                case Skip of
                    true:
                        continue;
                end;
                Error('x');
            end;
        if GetLastErrorText() <> 'x' then
            Error('%1', GetLastErrorText());
    end;

    [Test]
    procedure TakenExitAfterAStatement()
    var
        Skip: Boolean;
    begin
        Skip := true;
        asserterror begin
            if Skip then
                exit;
            Error('x');
        end;
    end;
}
"#;

fn of(out: &[(String, Outcome)], n: &str) -> Outcome {
    out.iter()
        .find(|(t, _)| t == n)
        .map(|(_, o)| o.clone())
        .unwrap_or_else(|| panic!("no outcome for {n}: {out:?}"))
}

#[test]
fn issue_8289_every_leave_of_an_assert_error_body_is_refused() {
    let out = outcomes_in(LEAVES, "AE Leaves");
    for n in [
        "ContinueOutOfTheBody",
        "BreakOutOfTheBody",
        "ExitOutOfTheBody",
        "ExitInsideANestedAssertError",
        "ExitAsTheWholeBody",
        "ExitWithAValueOfAProcedureThatLeaves",
    ] {
        match of(&out, n) {
            Outcome::NotRun(r) => assert!(
                r.contains("an asserterror body left by exit, break or continue"),
                "{n}: {r}"
            ),
            other => panic!("{n}: expected a refusal, got {other:?}"),
        }
    }
}

/// A leave the run does not reach keeps main's result: the body raises its error and the asserterror catches it. One the
/// run reaches under a condition is refused when it is reached.
#[test]
fn an_untaken_leave_of_an_assert_error_body_keeps_main_s_result() {
    let out = outcomes_in(LEAVES, "AE Leaves");
    for n in ["UntakenExit", "UntakenBreak", "UntakenContinue"] {
        assert_eq!(of(&out, n), Outcome::Pass, "{n}");
    }
    match of(&out, "TakenExitAfterAStatement") {
        Outcome::NotRun(r) => assert!(r.contains("an asserterror body left by exit, break or continue"), "{r}"),
        other => panic!("TakenExitAfterAStatement: expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_leave_that_stays_inside_the_assert_error_body_keeps_main_s_result() {
    let out = outcomes_in(LEAVES, "AE Leaves");
    for n in [
        "BreakOfALoopInsideTheBody",
        "ContinueOfALoopInsideTheBody",
        "ExitOfACalledProcedure",
        "ExitWithAValueThatRaises",
    ] {
        assert_eq!(of(&out, n), Outcome::Pass, "{n}");
    }
}
