//! `[ErrorBehavior(ErrorBehavior::Collect)]` and the error-collection builtins, end to end.
//! Each test names the corpus test that pins what it asserts (errors/errorinfo/TestErrorInfoType.al).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const HELPERS: &str = r#"
codeunit 50100 C
{
    // errors/errorinfo/TestErrorInfoType.al:642-656: the raisers carry no attribute.
    local procedure RaiseCollectible(Message: Text)
    var EI: ErrorInfo;
    begin EI := ErrorInfo.Create(Message, true); Error(EI); end;

    local procedure RaiseNonCollectible(Message: Text)
    var EI: ErrorInfo;
    begin EI := ErrorInfo.Create(Message, false); Error(EI); end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure ContinuesAfterCollect(Message: Text): Text
    var Reached: Text;
    begin
        RaiseCollectible(Message);
        Reached := 'reached';
        if not HasCollectedErrors() then Error('NOT HELD');
        if not IsCollectingErrors() then Error('NOT COLLECTING');
        if Reached <> 'reached' then Error('UNREACHABLE');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectOne(Message: Text)
    var Errors: List of [ErrorInfo];
    begin
        RaiseCollectible(Message);
        Errors := GetCollectedErrors();
        if Errors.Count() <> 1 then Error('COUNT ' + Format(Errors.Count()));
        if Errors.Get(1).Message() <> Message then Error('MESSAGE');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectTwo(First: Text; Second: Text)
    var Errors: List of [ErrorInfo];
    begin
        RaiseCollectible(First);
        RaiseCollectible(Second);
        Errors := GetCollectedErrors();
        if Errors.Count() <> 2 then Error('COUNT ' + Format(Errors.Count()));
        if Errors.Get(1).Message() <> First then Error('FIRST');
        if Errors.Get(2).Message() <> Second then Error('SECOND');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectNonCollectible(Message: Text)
    begin
        RaiseNonCollectible(Message);
        Error('UNREACHABLE');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure ClearInside(Message: Text)
    var Errors: List of [ErrorInfo];
    begin
        RaiseCollectible(Message);
        if not HasCollectedErrors() then Error('NOT HELD');
        ClearCollectedErrors();
        if HasCollectedErrors() then Error('STILL HELD');
        Errors := GetCollectedErrors();
        if Errors.Count() <> 0 then Error('NOT EMPTY');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure Outer(): Text
    var a: Text;
    begin
        Inner();
        exit(Format(IsCollectingErrors()) + Format(GetCollectedErrors().Count()));
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure Inner()
    begin
        RaiseCollectible('from inner');
    end;

    BODIES
}
"#;

fn run_in(body: &str) -> Result<Value, Signal> {
    let src = HELPERS.replace("BODIES", body);
    al_testkit::run(&src, "F")
}

fn text(body: &str) -> String {
    match run_in(&format!("procedure F(): Text var a: Text; begin {body} end;")) {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_collectible_error_does_not_abort_the_scope_and_is_rethrown_on_exit() {
    // errors/errorinfo/TestErrorInfoType.al:448-459 (CollectibleError_UnderCollectScope_DoesNotAbortCaller):
    // the helper reaches its later statements, and the scope rethrows the collected message.
    assert_eq!(
        text(
            "ClearCollectedErrors(); asserterror ContinuesAfterCollect('collected message'); exit(GetLastErrorText());"
        ),
        "collected message"
    );
}

#[test]
fn the_collected_error_carries_its_message_and_the_count_is_one() {
    // errors/errorinfo/TestErrorInfoType.al:464-470 (CollectibleError_IsCollectedWithItsMessage).
    assert_eq!(
        text("asserterror CollectOne('the collected message'); exit(GetLastErrorText());"),
        "the collected message"
    );
}

#[test]
fn a_non_collectible_error_unwinds_with_the_raisers_message() {
    // errors/errorinfo/TestErrorInfoType.al:474-490 (NonCollectibleError_InCollectScope_IsNotCollected).
    assert_eq!(
        text("asserterror CollectNonCollectible('not collectible'); exit(GetLastErrorText());"),
        "not collectible"
    );
}

#[test]
fn collected_errors_accumulate_in_raise_order() {
    // errors/errorinfo/TestErrorInfoType.al:494-503: the scope-exit text for two errors contains the
    // first message; the helper's own order assertions fail with their own text if wrong.
    let t = text("asserterror CollectTwo('first raised', 'second raised'); exit(GetLastErrorText());");
    assert!(t.contains("first raised"), "{t}");
    assert!(
        !t.contains("COUNT") && !t.contains("FIRST") && !t.contains("SECOND"),
        "{t}"
    );
}

#[test]
fn clearing_inside_the_scope_empties_the_set_and_disarms_the_rethrow() {
    // errors/errorinfo/TestErrorInfoType.al:507-520 (ClearCollectedErrors_EmptiesTheSetInsideTheScope):
    // the helper returns normally.
    assert_eq!(text("ClearInside('to be cleared'); exit('returned');"), "returned");
}

#[test]
fn outside_a_scope_nothing_is_collecting_and_clearing_does_not_throw() {
    // session-system/TestSystemGlobals.al:16-21, session-system/TestFinalCoverage.al:127-143.
    assert_eq!(
        text("ClearCollectedErrors(); exit(Format(IsCollectingErrors()) + Format(GetCollectedErrors().Count()));"),
        "No0"
    );
}

#[test]
fn has_collected_errors_outside_a_scope_answers_true() {
    // errors/errorinfo/TestErrorInfoType.al finding D (lines 133-141): outside a collecting scope the
    // global answers TRUE (0 < 0 - (-1)); the tier failed an assertion of false on every leg.
    assert_eq!(text("exit(Format(HasCollectedErrors()));"), "Yes");
}

#[test]
fn a_nested_scope_hands_its_errors_to_the_outermost_scope() {
    // errors/errorinfo/TestErrorInfoType.al:128-131: only closing the OUTERMOST scope rethrows.
    assert_eq!(
        text("asserterror a := Outer(); exit(GetLastErrorText());"),
        "from inner"
    );
}

#[test]
fn an_error_with_error_info_outside_a_scope_unwinds_even_if_collectible() {
    // errors/errorinfo/TestErrorInfoType.al:474-490: Collect is what makes a collectible error
    // collectable; with no scope it is raised like any other.
    assert_eq!(
        text("asserterror RaiseCollectible('plain'); exit(GetLastErrorText());"),
        "plain"
    );
}
