//! `Evaluate` into a `var` Option parameter declared without members (al-runner issue 7994). BC reads the caller's members
//! there (Microsoft's System Application test `Json Test`.TestGetEnumPropertyValueFromJObjectByName passes on BC 28.4); the
//! parameter holds none in the runner, so a text that is not an ordinal refuses. The neighbors keep their results.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
codeunit 69810 C
{
    procedure ByMemberName(): Text
    var Value: Option First,Second,Third;
    begin
        EvaluateInto(Value, 'Second');
        exit(Format(Value, 0, 2));
    end;

    procedure ByMemberNameResultUsed(): Text
    var Value: Option First,Second,Third;
    begin
        exit(Format(TryEvaluateInto(Value, 'Second')));
    end;

    procedure BlankText(): Text
    var Value: Option " ",First;
    begin
        EvaluateInto(Value, '');
        exit(Format(Value, 0, 2));
    end;

    procedure ByOrdinal(): Text
    var Value: Option First,Second,Third;
    begin
        EvaluateInto(Value, '1');
        exit(Format(Value, 0, 2));
    end;

    procedure ByValueParameter(): Text
    var Value: Option First,Second,Third;
    begin
        asserterror EvaluateByValue(Value, 'Second');
        exit(GetLastErrorCode());
    end;

    procedure MemberlessLocal(): Text
    var Value: Option;
    begin
        asserterror Evaluate(Value, 'Second');
        exit(GetLastErrorCode());
    end;

    procedure VarParameterWithMembers(): Text
    var Value: Option First,Second,Third;
    begin
        EvaluateIntoDeclared(Value, 'Second');
        exit(Format(Value));
    end;

    local procedure EvaluateInto(var Value: Option; T: Text)
    begin
        Evaluate(Value, T, 0);
    end;

    local procedure TryEvaluateInto(var Value: Option; T: Text): Boolean
    begin
        exit(Evaluate(Value, T));
    end;

    local procedure EvaluateByValue(Value: Option; T: Text)
    begin
        Evaluate(Value, T);
    end;

    local procedure EvaluateIntoDeclared(var Value: Option First,Second,Third; T: Text)
    begin
        Evaluate(Value, T);
    end;
}
"#;

fn refused(routine: &str) {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("declared without members"), "{why}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_member_name_of_the_caller_is_refused() {
    refused("ByMemberName");
}

#[test]
fn a_member_name_with_the_result_used_is_refused() {
    refused("ByMemberNameResultUsed");
}

#[test]
fn a_blank_text_is_refused() {
    refused("BlankText");
}

#[test]
fn an_ordinal_keeps_main_s_result() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ByOrdinal")), "1");
}

#[test]
fn a_by_value_parameter_keeps_main_s_error() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ByValueParameter")),
        "InvalidOptionString"
    );
}

#[test]
fn a_memberless_local_variable_keeps_main_s_error() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "MemberlessLocal")),
        "InvalidOptionString"
    );
}

#[test]
fn a_var_parameter_with_members_reads_its_own_members() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "VarParameterWithMembers")), "Second");
}
