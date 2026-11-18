//! `Evaluate` into a `var` Option parameter declared without members, or into an element of a `var` array of such Options
//! (al-runner issue 7994). BC reads the caller's members there (Microsoft's System Application test `Json Test`.
//! TestGetEnumPropertyValueFromJObjectByName passes on BC 28.4); the parameter holds none in the runner, so a text that is
//! not an ordinal refuses, whatever the caller: a variable with members, a record field, a variable without members, and
//! an event subscriber. An array element has the caller's members for the Evaluate, but the store back drops them and the
//! caller would show the number (issue 8291), so it refuses with that reason. The neighbors keep their results.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69811 Row
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = First,Second,Third; }
    }
}

codeunit 69810 C
{
    procedure LocalByMemberName(): Text
    var Value: Option First,Second,Third;
    begin
        EvaluateInto(Value, 'Second');
        exit(Format(Value));
    end;

    procedure SubscriberByMemberName(): Text
    var Value: Option;
    begin
        OnEvaluate(Value);
        exit(Format(Value));
    end;

    procedure FieldByMemberName(): Text
    var R: Record Row;
    begin
        EvaluateInto(R.Opt, 'Second');
        exit(Format(R.Opt));
    end;

    procedure FieldByMemberNameResultUsed(): Text
    var R: Record Row;
    begin
        exit(Format(TryEvaluateInto(R.Opt, 'Second')));
    end;

    procedure MemberlessBlankText(): Text
    var Value: Option;
    begin
        EvaluateInto(Value, '');
        exit(Format(Value));
    end;

    procedure FieldByOrdinal(): Text
    var R: Record Row;
    begin
        EvaluateInto(R.Opt, '1');
        exit(Format(R.Opt) + '|' + Format(R.Opt, 0, 2));
    end;

    procedure MemberlessByOrdinal(): Text
    var Value: Option;
    begin
        EvaluateInto(Value, '1');
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    procedure ArrayElementByMemberName(): Text
    var A: array[2] of Option First,Second,Third;
    begin
        EvaluateIntoElement(A, 'Second');
        exit(Format(A[1]));
    end;

    procedure ArrayElementByOrdinal(): Text
    var A: array[2] of Option;
    begin
        EvaluateIntoElement(A, '1');
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
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

    local procedure EvaluateIntoElement(var A: array[2] of Option; T: Text)
    begin
        Evaluate(A[1], T);
    end;

    local procedure EvaluateByValue(Value: Option; T: Text)
    begin
        Evaluate(Value, T);
    end;

    local procedure EvaluateIntoDeclared(var Value: Option First,Second,Third; T: Text)
    begin
        Evaluate(Value, T);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnEvaluate(var Value: Option)
    begin
    end;
}

codeunit 69812 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::C, 'OnEvaluate', '', false, false)]
    local procedure EvaluateOnEvent(var Value: Option)
    begin
        Evaluate(Value, 'Second');
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
fn a_member_name_of_a_caller_with_members_is_refused() {
    refused("LocalByMemberName");
}

#[test]
fn a_member_name_in_an_event_subscriber_is_refused() {
    refused("SubscriberByMemberName");
}

#[test]
fn a_member_name_of_a_record_field_caller_is_refused() {
    refused("FieldByMemberName");
}

#[test]
fn a_member_name_with_the_result_used_is_refused() {
    refused("FieldByMemberNameResultUsed");
}

#[test]
fn a_blank_text_is_refused() {
    refused("MemberlessBlankText");
}

#[test]
fn a_member_name_into_an_element_of_a_var_array_is_refused_with_its_own_reason() {
    match al_testkit::run_in(SRC, "C", "ArrayElementByMemberName") {
        Err(Signal::Unsupported(why)) => {
            assert!(why.contains("the store drops them"), "{why}");
            assert!(why.contains("issue 8291"), "{why}");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn an_ordinal_keeps_main_s_result_for_a_record_field_caller() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "FieldByOrdinal")), "Second|1");
}

#[test]
fn an_ordinal_keeps_main_s_result_for_a_caller_without_members() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "MemberlessByOrdinal")), "1|1");
}

#[test]
fn an_ordinal_into_an_element_of_a_var_array_keeps_main_s_result() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ArrayElementByOrdinal")), "1|1");
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
