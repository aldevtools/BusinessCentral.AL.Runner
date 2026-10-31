//! An error in an OnBeforeValidateEvent or OnAfterValidateEvent subscriber puts the record back at its values from before the
//! Validate, as one in the field's OnValidate does (events/lt-db-events/DbtEvents.al:623 Validate_ErrorInSubscriber).

const SRC: &str = r#"
table 50960 "Ve Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { trigger OnValidate() begin end; } }
    keys { key(PK; Id) { } }
}
codeunit 50961 "Ve Fail"
{
    SingleInstance = true;
    var Point: Text;
    procedure Set(P: Text) begin Point := P; end;
    procedure Get(): Text begin exit(Point); end;
}
codeunit 50962 "Ve Subs"
{
    [EventSubscriber(ObjectType::Table, Database::"Ve Row", 'OnBeforeValidateEvent', 'Name', false, false)]
    local procedure OnBefore(var Rec: Record "Ve Row")
    var F: Codeunit "Ve Fail";
    begin
        if F.Get() = 'B' then Error('fail before');
    end;

    [EventSubscriber(ObjectType::Table, Database::"Ve Row", 'OnAfterValidateEvent', 'Name', false, false)]
    local procedure OnAfter(var Rec: Record "Ve Row")
    var F: Codeunit "Ve Fail";
    begin
        if F.Get() = 'A' then Error('fail after');
    end;
}
codeunit 50963 C
{
    procedure F(): Text
    var R: Record "Ve Row"; Fail: Codeunit "Ve Fail"; T: Text;
    begin
        R.Id := 1; R.Name := 'keep';
        Fail.Set('B');
        asserterror R.Validate(Name, 'changed');
        T := GetLastErrorText() + ':' + R.Name;
        Fail.Set('A');
        asserterror R.Validate(Name, 'changed3');
        T += '|' + GetLastErrorText() + ':' + R.Name;
        Fail.Set('');
        exit(T);
    end;
}
"#;

#[test]
fn an_error_in_a_validate_event_subscriber_restores_the_record() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "fail before:keep|fail after:keep"
    );
}
