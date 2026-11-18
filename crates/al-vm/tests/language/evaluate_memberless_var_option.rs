//! `Evaluate` into a `var` Option parameter declared without members, or into an element of a `var` array of such Options
//! (al-runner issue 7994): the text is read against the caller's members (assumption var-option-caller-members, corpus PR
//! 1432, OptionVarMemberlessEvaluate.al; Microsoft's System Application test `Json Test`.
//! TestGetEnumPropertyValueFromJObjectByName passes on BC 28.4). A caller without members has none to read; a blank text from
//! a caller with an empty member or without members refuses (not measured). The neighbors keep their results. More shapes: `var_option_member_lists.rs`.

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

    procedure EmptyMemberCallerBlankText(): Text
    var Value: Option ,PrimaryKey,NonKeyFields;
    begin
        Value := Value::PrimaryKey;
        EvaluateInto(Value, '');
        exit(Format(Value));
    end;

    procedure MemberlessBlankTextResultUsed(): Text
    var Value: Option;
    begin
        exit(Format(TryEvaluateInto(Value, '')));
    end;

    procedure BlankMemberCallerBlankText(): Text
    var Value: Option " ",A,B;
    begin
        Value := Value::A;
        EvaluateInto(Value, '');
        exit(Format(Value, 0, 2));
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

fn assumed(routine: &str) -> String {
    al_builtins::assumptions::clear_hits();
    let out = text(al_testkit::run_in(SRC, "C", routine));
    assert!(
        al_builtins::assumptions::take_hits().contains(&al_builtins::assumptions::Assumption::VarOptionCallerMembers),
        "{routine}: not flagged"
    );
    out
}

#[test]
fn a_member_name_of_a_caller_with_members_reads_its_members() {
    assert_eq!(assumed("LocalByMemberName"), "Second");
}

#[test]
fn a_member_name_in_an_event_subscriber_of_a_caller_without_members_raises_the_empty_member_list() {
    // OptionVarMemberlessEvaluate.al: a caller declared without members has no member names to read.
    match al_testkit::run_in(SRC, "C", "SubscriberByMemberName") {
        Err(Signal::Error(e)) => {
            assert_eq!(e.message, "'Second' is not an option. The existing options are: ");
            assert_eq!(e.code, "InvalidOptionString");
        }
        other => panic!("expected the error, got {other:?}"),
    }
}

#[test]
fn a_member_name_of_a_record_field_caller_reads_its_members() {
    assert_eq!(assumed("FieldByMemberName"), "Second");
}

#[test]
fn a_member_name_with_the_result_used_reads_the_caller_s_members() {
    assert_eq!(assumed("FieldByMemberNameResultUsed"), "Yes");
}

#[test]
fn a_blank_text_as_a_statement_is_refused() {
    match al_testkit::run_in(SRC, "C", "MemberlessBlankText") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("declared without members"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The probe measures a blank text from callers whose members all have a name (OptionVarMemberlessEvaluate.al:170-185,
/// :286-325). A caller with an empty member is not measured.
#[test]
fn a_blank_text_from_a_caller_with_an_empty_member_is_refused() {
    for routine in ["EmptyMemberCallerBlankText", "MemberlessBlankTextResultUsed"] {
        match al_testkit::run_in(SRC, "C", routine) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("an empty member"), "{routine}: {why}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn a_blank_text_from_a_caller_with_a_blank_member_reads_that_member() {
    assert_eq!(assumed("BlankMemberCallerBlankText"), "0");
}

#[test]
fn a_member_name_into_an_element_of_a_var_array_keeps_the_caller_s_members() {
    assert_eq!(assumed("ArrayElementByMemberName"), "Second");
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
