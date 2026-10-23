//! SetValue shapes writer QZ pinned in corpus bump 54: a list control over an Enum field with NotBlank
//! (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:4021), CharAllowed on a list (record/lt-x-table-props/XTablePropsTests.al:928)
//! and expression controls of a card without a source table (misc/lt-protected-var-abs-query-views/P146PageExprSetValue.al:506).

fn text(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) -> String {
    match r {
        Ok(al_builtins::value::Value::Text(t)) => t.to_string(),
        other => panic!("{other:?}"),
    }
}

const SRC: &str = r#"
enum 69970 "NB Kind" { Extensible = false; value(0; " ") { } value(1; Person) { } }
table 69971 "NB Row"
{
    fields { field(1; Code; Code[20]) { } field(2; KindNB; Enum "NB Kind") { NotBlank = true; } }
    keys { key(PK; Code) { } }
}
page 69972 "NB List"
{
    PageType = List; SourceTable = "NB Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(KindNB; Rec.KindNB) { } } } }
}
codeunit 69973 C
{
    procedure Steps(): Text
    var R: Record "NB Row"; P: TestPage "NB List"; Log: Text;
    begin
        R.Code := 'N1'; R.KindNB := R.KindNB::Person; R.Insert(); Commit();
        P.OpenEdit();
        P.GoToRecord(R);
        asserterror P.KindNB.SetValue('');
        Log := GetLastErrorText() + '|' + P.KindNB.Value();
        asserterror begin P.KindNB.SetValue(' '); Error('no error'); end;
        Log += '|' + GetLastErrorText() + '|' + P.KindNB.Value();
        asserterror begin P.KindNB.SetValue(0); Error('no error'); end;
        Log += '|' + GetLastErrorText() + '|' + P.KindNB.Value();
        P.Close();
        exit(Log);
    end;
}
"#;

#[test]
fn a_not_blank_enum_field_on_a_list_takes_the_blank_member_and_refuses_the_empty_text() {
    let got = text(al_testkit::run_in(SRC, "C", "Steps"));
    assert_eq!(
        got,
        "Validation error for Field: KindNB,  Message = 'Your entry of '' is not an acceptable value for 'KindNB'.'|1|no error| |no error| "
    );
}

const CHARS: &str = r#"
table 69974 "CA Row"
{
    fields { field(1; Id; Integer) { } field(2; Digits; Code[20]) { CharAllowed = '09'; } }
    keys { key(PK; Id) { } }
}
page 69975 "CA List"
{
    PageType = List; SourceTable = "CA Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(Digits; Rec.Digits) { } } } }
}
codeunit 69976 C
{
    procedure Steps(): Text
    var R: Record "CA Row"; P: TestPage "CA List"; Log: Text;
    begin
        R.Id := 2; R.Insert(); Commit();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Digits.SetValue('456');
        asserterror P.Digits.SetValue('4x6');
        Log := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + P.Digits.Value();
        P.Close();
        exit(Log);
    end;
}
"#;

#[test]
fn char_allowed_of_a_table_field_on_a_list_names_the_character_without_the_refresh_hint() {
    assert_eq!(
        text(al_testkit::run_in(CHARS, "C", "Steps")),
        "Validation error for Field: Digits,  Message = 'The character X is not valid in this field. You can specify the following characters: 09.'|TestValidation|456"
    );
}

const EXPR: &str = r#"
page 69977 "EX Card"
{
    PageType = Card; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Total; GetTotal()) { } field(SumCtl; ValA + ValB) { } field(Plain; PlainText) { } } }
    trigger OnOpenPage() begin ValA := 2; ValB := 3; PlainText := 'abc'; end;
    var ValA: Integer; ValB: Integer; PlainText: Text;
    local procedure GetTotal(): Decimal begin exit(42); end;
}
codeunit 69978 C
{
    procedure Steps(): Text
    var P: TestPage "EX Card"; Log: Text;
    begin
        P.OpenEdit();
        Log := Format(P.Total.Editable()) + Format(P.SumCtl.Editable()) + Format(P.Plain.Editable());
        P.Total.SetValue('5');
        P.SumCtl.SetValue('9');
        Log += '|' + P.Total.Value() + '|' + P.SumCtl.Value();
        P.Close();
        exit(Log);
    end;
}
"#;

#[test]
fn an_expression_control_of_a_card_without_a_source_table_is_read_only_and_set_value_does_nothing() {
    assert_eq!(text(al_testkit::run_in(EXPR, "C", "Steps")), "NoNoYes|42.00|5");
}
