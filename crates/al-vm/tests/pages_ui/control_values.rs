//! What a TestPage control shows and takes: the option list of an Enum control (its OptionCaption, or the enum's captions
//! split at every comma), AsDecimal of a Text control, a table field's ValuesAllowed on SetValue, and the mark
//! SetSelectionFilter puts on the selected row. Each test names the corpus test that pins what BC does.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal naming {needle}, got {other:?}"),
    }
}

const SRC: &str = r#"
enum 70100 "CV Sparse" { Extensible = true; value(0; " ") { } value(1; value1) { Caption = 'cap1'; } value(4; value4) { Caption = 'cap4'; } value(5; value5) { Caption = 'cap5'; } }
enum 70101 "CV Tier" { Extensible = true; value(0; Plain) { Caption = 'Plain'; } value(1; Diamond) { Caption = 'Diamond Level, with bonus'; } value(2; Gold) { Caption = 'with bonus'; } value(3; Silver) { Caption = 'Silver'; } }
table 70102 "CV Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; E; Enum "CV Sparse") { }
        field(3; Tier; Enum "CV Tier") { }
        field(4; Txt; Text[30]) { }
        field(5; C1; Code[1]) { ValuesAllowed = '', 'A', 'N'; }
        field(6; N; Integer) { ValuesAllowed = 1, 2; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
page 70103 "CV Card"
{
    PageType = Card; SourceTable = "CV Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { }
            field(E; Rec.E) { OptionCaption = ' ,c1,,,c4,c5'; }
            field(Tier; Rec.Tier) { }
            field(TxtCtl; Rec.Txt) { }
            field(C1Ctl; Rec.C1) { }
            field(NCtl; Rec.N) { }
            field(TierVar; TierV) { }
            field(EAllowed; Rec.E) { ValuesAllowed = value1, value4; }
        }
    }
    var TierV: Enum "CV Tier";
}
page 70104 "CV List"
{
    PageType = List; SourceTable = "CV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(C1Ctl; Rec.C1) { } } } }
    actions
    {
        area(Processing)
        {
            action(Sel)
            {
                trigger OnAction()
                var Target: Record "CV Row"; Marked: Text;
                begin
                    CurrPage.SetSelectionFilter(Target);
                    if Target.FindSet() then repeat if Target.Mark() then Marked += Format(Target.Id) + ','; until Target.Next() = 0;
                    Log.Put(Target.GetFilters() + '|' + Format(Target.MarkedOnly()) + '|' + Marked);
                end;
            }
            action(SelMarks)
            {
                trigger OnAction()
                var Target: Record "CV Row"; Marked: Text;
                begin
                    Target.Get(1); Target.Mark(true);
                    Target.Get(9); Target.Mark(true);
                    CurrPage.SetSelectionFilter(Target);
                    Target.SetRange(Id);
                    if Target.FindSet() then repeat if Target.Mark() then Marked += Format(Target.Id) + ','; until Target.Next() = 0;
                    Log.Put(Format(Target.MarkedOnly()) + '|' + Marked);
                end;
            }
        }
    }
    var Log: Codeunit "CV Log";
}
codeunit 70106 "CV Log"
{
    SingleInstance = true;
    var Last: Text;
    procedure Put(T: Text) begin Last := T; end;
    procedure Take(): Text begin exit(Last); end;
}
codeunit 70105 C
{
    local procedure Seed()
    var R: Record "CV Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.E := R.E::value1; R.Tier := R.Tier::Silver; R.C1 := 'N'; R.Insert();
        R.Id := 4; R.E := R.E::value4; R.Tier := R.Tier::Gold; R.Insert();
        R.Id := 9; R.E := R.E::" "; R.Insert();
        Commit();
    end;
    local procedure Shown(Id: Integer): Text
    var R: Record "CV Row"; P: TestPage "CV Card";
    begin
        R.Get(Id); P.OpenEdit(); P.GoToRecord(R);
        exit(P.E.Value());
    end;
    procedure SparseShown(): Text
    begin
        Seed();
        exit(Shown(1) + '|' + Shown(4) + '|[' + Shown(9) + ']');
    end;
    [TryFunction]
    local procedure TrySet(var P: TestPage "CV Card"; T: Text)
    begin
        P.E.SetValue(T);
    end;
    local procedure SetSparse(T: Text): Text
    var R: Record "CV Row"; P: TestPage "CV Card";
    begin
        R.Get(4); P.OpenEdit(); P.GoToRecord(R);
        if not TrySet(P, T) then exit(GetLastErrorText());
        P.Close(); R.Get(4);
        exit(Format(R.E.AsInteger()));
    end;
    procedure SparseSet(): Text
    begin
        Seed();
        exit(SetSparse('c1') + '#' + SetSparse(' ') + '#' + SetSparse('cap1') + '#' + SetSparse('c4') + '#' + SetSparse('4'));
    end;
    procedure AllowedOptionCount(): Integer
    var P: TestPage "CV Card";
    begin
        Seed(); P.OpenEdit();
        exit(P.EAllowed.OptionCount());
    end;
    procedure SparseOptionCount(): Integer
    var P: TestPage "CV Card";
    begin
        Seed(); P.OpenEdit();
        exit(P.E.OptionCount());
    end;
    [TryFunction]
    local procedure TryTier(var P: TestPage "CV Card"; T: Text)
    begin
        P.Tier.SetValue(T);
    end;
    local procedure SetTier(T: Text): Text
    var R: Record "CV Row"; P: TestPage "CV Card"; V: Text;
    begin
        R.Get(1); R.Tier := R.Tier::Silver; R.Modify();
        P.OpenEdit(); P.GoToRecord(R);
        if not TryTier(P, T) then exit(GetLastErrorText() + '/' + GetLastErrorCode() + '/' + P.Tier.Value());
        V := P.Tier.Value(); P.Close(); R.Get(1);
        exit(V + '/' + Format(R.Tier.AsInteger()));
    end;
    procedure CommaSet(): Text
    begin
        Seed();
        exit(SetTier('with bonus') + '#' + SetTier('Diamond Level') + '#' + SetTier('Plain') + '#' + SetTier('Diamond Level, with bonus') + '#' + SetTier('Diamond'));
    end;
    procedure CommaLongText()
    var R: Record "CV Row"; P: TestPage "CV Card";
    begin
        Seed(); R.Get(1); P.OpenEdit(); P.GoToRecord(R);
        P.Tier.SetValue('Diamond Level, with bonus2');
    end;
    procedure CommaVariable()
    var P: TestPage "CV Card";
    begin
        Seed(); P.OpenEdit();
        P.TierVar.SetValue('Plain');
    end;
    [TryFunction]
    local procedure TryDec(var P: TestPage "CV Card"; var D: Decimal)
    begin
        D := P.TxtCtl.AsDecimal();
    end;
    local procedure TxtDec(T: Text): Text
    var R: Record "CV Row"; P: TestPage "CV Card"; D: Decimal;
    begin
        R.Get(1); R.Txt := T; R.Modify();
        P.OpenEdit(); P.GoToRecord(R);
        if TryDec(P, D) then exit(Format(D));
        exit(GetLastErrorText() + '/' + GetLastErrorCode());
    end;
    procedure TextAsDecimal(): Text
    begin
        Seed();
        exit(TxtDec('12.5') + '#' + TxtDec('') + '#' + TxtDec(' ') + '#' + TxtDec('abc'));
    end;
    procedure TextAsDecimalDigits(): Text
    begin
        Seed();
        exit(TxtDec('1x2'));
    end;
    procedure ValuesAllowedSet(): Text
    var R: Record "CV Row"; P: TestPage "CV Card"; Res: Text;
    begin
        Seed(); R.Get(1); P.OpenEdit(); P.GoToRecord(R);
        P.C1Ctl.SetValue('A');
        Res := P.C1Ctl.Value();
        asserterror P.C1Ctl.SetValue('');
        Res += '#' + GetLastErrorText() + '/' + GetLastErrorCode() + '/' + P.C1Ctl.Value();
        asserterror P.C1Ctl.SetValue('X');
        Res += '#' + GetLastErrorText();
        exit(Res);
    end;
    procedure ValuesAllowedValidate(): Text
    var R: Record "CV Row";
    begin
        Seed(); R.Get(1);
        R.Validate(C1, 'X');
        exit(R.C1);
    end;
    procedure ValuesAllowedInteger()
    var R: Record "CV Row"; P: TestPage "CV Card";
    begin
        Seed(); R.Get(1); P.OpenEdit(); P.GoToRecord(R);
        P.NCtl.SetValue(1);
    end;
    procedure ValuesAllowedList()
    var P: TestPage "CV List";
    begin
        Seed(); P.OpenEdit(); P.First();
        P.C1Ctl.SetValue('A');
    end;
    procedure SelectionMarks(): Text
    var P: TestPage "CV List"; Log: Codeunit "CV Log"; Res: Text;
    begin
        Seed();
        P.OpenView(); P.GoToKey(4);
        P.Sel.Invoke();
        Res := Log.Take();
        P.SelMarks.Invoke();
        exit(Res + '#' + Log.Take());
    end;
}
"#;

#[test]
fn an_enum_control_shows_the_option_caption_entry_at_the_value_position() {
    // handlers/TestPageEnumField_Tests.al:197-205 (Enum_SparseOrdinals_PageOptionCaption): position 1 is c1, position 2 of
    // ' ,c1,,,c4,c5' is empty so ordinal 4 shows its number, position 0 is ' '.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "SparseShown")),
        "c1|4|[ ]"
    );
}

#[test]
fn set_value_on_an_enum_control_takes_the_option_caption_entry_at_the_value_position() {
    // handlers/TestPageEnumField_Tests.al:235-246 (Enum_SparseOrdinals_PageOptionCaption_SetValue).
    let refused = |t: &str| {
        format!(
            "Validation error for Field: E,  Message = 'Your entry of '{t}' is not an acceptable value for 'E'. (Select Refresh to discard errors)'"
        )
    };
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "SparseSet")),
        format!("1#0#{}#{}#{}", refused("cap1"), refused("c4"), refused("4"))
    );
}

// OptionCount of an Enum control with ValuesAllowed counts only the allowed values (misc/lt-y13-triage/Y13PageTriage.al:197-243,
// corpus master; option_count_values_allowed.rs).
#[test]
fn option_count_of_a_control_with_values_allowed_counts_the_allowed_values() {
    assert_eq!(al_testkit::int(al_testkit::run_in(SRC, "C", "AllowedOptionCount")), 2);
}

#[test]
fn option_count_of_an_enum_control_with_an_option_caption_is_refused() {
    refused(al_testkit::run_in(SRC, "C", "SparseOptionCount"), "OptionCaption");
}

#[test]
fn set_value_on_an_enum_control_splits_the_captions_at_every_comma() {
    // misc/lt-cv-odd-leads/CvEnumCaptionComma.al:150-174 (TestPageSetValueByCaption): the halves select other values, the whole
    // caption is refused and named shortened in the middle; a refused entry leaves the ordinal shown.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CommaSet")),
        " with bonus/2#Diamond Level/1#Plain/0#Validation error for Field: Tier,  Message = 'Your entry of 'Diamond ...th bonus' is not an acceptable value for 'Tier'. (Select Refresh to discard errors)'/TestValidation/3#Validation error for Field: Tier,  Message = 'Your entry of 'Diamond' is not an acceptable value for 'Tier'. (Select Refresh to discard errors)'/TestValidation/3"
    );
}

#[test]
fn a_refused_entry_of_an_unpinned_length_is_refused() {
    refused(al_testkit::run_in(SRC, "C", "CommaLongText"), "characters");
}

#[test]
fn an_enum_variable_control_with_a_comma_caption_is_refused() {
    refused(al_testkit::run_in(SRC, "C", "CommaVariable"), "comma");
}

#[test]
fn as_decimal_of_a_text_control_that_is_no_number_raises_the_format_error() {
    // pages/lt-ca-odd-667/CaTestPageTests.al:90-91, :130-135 (Ca01_AsDecimal_*).
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "TextAsDecimal")),
        "12.5#The input string '' was not in a correct format./AL:Format#The input string ' ' was not in a correct format./AL:Format#The input string 'abc' was not in a correct format./AL:Format"
    );
    refused(al_testkit::run_in(SRC, "C", "TextAsDecimalDigits"), "AsDecimal");
}

#[test]
fn set_value_checks_the_table_field_values_allowed() {
    // pages/lt-cd-testpage-leads/CdTestPageLeads.al:400-423 (Item56_ValuesAllowedEmpty_SetValueEmptyAfterA, _SetValueNotAllowed).
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ValuesAllowedSet")),
        "A#Validation error for Field: C1Ctl,  Message = 'Value '' is not a valid value. The following values can be used: ;A;N. (Select Refresh to discard errors)'/TestValidation/A#Validation error for Field: C1Ctl,  Message = 'Value 'X' is not a valid value. The following values can be used: ;A;N. (Select Refresh to discard errors)'"
    );
    // Validate in code does not check it (CdTestPageLeads.al:426-444, Item56_ValuesAllowed_ValidateInCode).
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ValuesAllowedValidate")),
        "X"
    );
    refused(al_testkit::run_in(SRC, "C", "ValuesAllowedInteger"), "ValuesAllowed");
    refused(al_testkit::run_in(SRC, "C", "ValuesAllowedList"), "ValuesAllowed");
}

#[test]
fn set_selection_filter_marks_the_selected_row_alone() {
    // pages/lt-cd-testpage-leads/CdTestPageLeads.al:676-708 (Item60_SelectionFilter_*): a fresh record gets the selected row
    // marked; pages/lt-ap-testpage-642/ApTestPageTests.al:866-882 (Ap15_SetSelectionFilter_IntoMarkedRecord, `marks`): the marks of
    // other rows are gone and MarkedOnly stays off.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "SelectionMarks")),
        "Id: 4|No|4,#No|4,"
    );
}
