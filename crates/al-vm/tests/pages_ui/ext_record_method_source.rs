//! al-runner issue 7133: a pageextension control whose source names a parameterless method of the source table without
//! parentheses (`field(Total; Rec.CalculatedTotal)`) shows the method's value, as the form with parentheses does. Pinned by corpus
//! master pages/record-method-controls/TestRecordMethodControls.al:39 PageExtension_RecordMethod_NoParentheses_ReadsCalculatedValue
//! and :49 PageExtension_RecordMethod_Parentheses_ReadsCalculatedValue (corpus pull request 1152, BC 27.5, 28.5 and 29.0).

fn src(source: &str) -> String {
    format!(
        r#"
table 69028 "PAG Method Source Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; Amount; Decimal) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
    procedure CalculatedTotal(): Decimal
    begin
        exit(Amount + 1);
    end;
}}
page 69038 "PAG Method Host"
{{
    PageType = Card;
    SourceTable = "PAG Method Source Row";
    layout {{ area(Content) {{ group(Data) {{ field(Id; Rec.Id) {{ ApplicationArea = All; }} }} }} }}
}}
pageextension 69054 "PAG Method Extension" extends "PAG Method Host"
{{
    layout {{ addlast(Data) {{ field(Total; {source}) {{ ApplicationArea = All; Editable = false; }} field(IdAgain; Rec.Id) {{ ApplicationArea = All; }} }} }}
}}
codeunit 69056 C
{{
    procedure Run(): Text
    var
        Row: Record "PAG Method Source Row";
        View: TestPage "PAG Method Host";
        Result: Text;
    begin
        Row.DeleteAll(false);
        Row.Id := 77;
        Row.Amount := 41.5;
        Row.Insert(false);
        View.OpenView();
        View.Total.AssertEquals(42.5);
        Result := View.IdAgain.Value();
        View.Close();
        exit(Result);
    end;
}}
"#
    )
}

#[test]
fn an_extension_control_over_a_table_method_without_parentheses_shows_its_value() {
    let r = al_testkit::run_in(&src("Rec.CalculatedTotal"), "C", "Run");
    assert_eq!(al_testkit::text(r), "77");
}

#[test]
fn an_extension_control_over_a_table_method_with_parentheses_shows_its_value() {
    let r = al_testkit::run_in(&src("Rec.CalculatedTotal()"), "C", "Run");
    assert_eq!(al_testkit::text(r), "77");
}
