//! SetView of a view whose WHERE conditions are joined by a dot (al-runner issue 6969). Measured on BC 29 by corpus branch
//! al-runtime-rs-staging-r6961, record/views/R6969SetViewDot.al: BC refuses every such view, in the numeric and the named form,
//! with or without a space, with a trailing or a double dot, with "Invalid expression of type: table view." and the view, code
//! DB:Ok; RecordRef.SetView too. The record keeps its filters.

use al_testkit::text;

const SRC: &str = r#"
table 50871 "VD Head"
{
    fields
    {
        field(1; "Document Type"; Option) { OptionMembers = Quote,"Order",Invoice; }
        field(3; "No."; Code[20]) { }
        field(120; Status; Option) { OptionMembers = Open,Released; }
        field(130; Amount; Decimal) { }
    }
    keys { key(PK; "Document Type", "No.") { } }
}
codeunit 50872 C
{
    [TryFunction]
    local procedure TrySetView(var H: Record "VD Head"; View: Text)
    begin
        H.SetView(View);
    end;

    local procedure Apply(View: Text): Text
    var H: Record "VD Head";
    begin
        H.SetRange(Amount, 5);
        if TrySetView(H, View) then
            exit('ok ' + H.GetFilters());
        exit(GetLastErrorText() + ' ' + GetLastErrorCode() + ' [' + H.GetFilters() + ']');
    end;

    procedure Numeric(): Text
    begin
        exit(Apply('VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2).Field120=1(0))'));
    end;

    procedure Variants(): Text
    begin
        exit(Apply('VERSION(1) SORTING(Field1,Field3) WHERE(Field130=1(10..20).Field120=1(1))') + '|' +
            Apply('VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2) .Field120=1(1))') + '|' +
            Apply('VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2).)') + '|' +
            Apply('SORTING(Document Type,No.) WHERE(Status=FILTER(Open).No.=FILTER(A*))'));
    end;

    procedure CommaStillWorks(): Text
    begin
        exit(Apply('SORTING(Document Type,No.) WHERE(Status=FILTER(Open),No.=FILTER(A*))'));
    end;

    procedure ViaRecordRef(): Text
    var RecRef: RecordRef;
    begin
        RecRef.Open(Database::"VD Head");
        asserterror RecRef.SetView('VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2).Field120=1(0))');
        exit(GetLastErrorText() + ' ' + GetLastErrorCode());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

const BAD: &str = "Invalid expression of type: table view.\\";

// R6969SetViewDot.al SetView_Numeric_CommaAndDot `dot`.
#[test]
fn set_view_with_numeric_conditions_joined_by_a_dot_is_an_invalid_view() {
    assert_eq!(
        run("Numeric"),
        format!("{BAD}VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2).Field120=1(0)) DB:Ok [Amount: 5]")
    );
}

// SetView_Numeric_DotInsideAValue `range`, SetView_Numeric_DotVariants `space dot` and `trailing dot`, SetView_Named_Dot
// `named field with dot second`, and `named field with dot comma` (accepted).
#[test]
fn set_view_dot_variants_are_invalid_and_a_comma_is_not() {
    let want = [
        "VERSION(1) SORTING(Field1,Field3) WHERE(Field130=1(10..20).Field120=1(1))",
        "VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2) .Field120=1(1))",
        "VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2).)",
        "SORTING(Document Type,No.) WHERE(Status=FILTER(Open).No.=FILTER(A*))",
    ]
    .map(|v| format!("{BAD}{v} DB:Ok [Amount: 5]"))
    .join("|");
    assert_eq!(run("Variants"), want);
    assert_eq!(run("CommaStillWorks"), "ok No.: A*, Status: Open");
}

// RecordRef_SetView_Numeric_Dot.
#[test]
fn recordref_set_view_with_a_dot_is_an_invalid_view() {
    assert_eq!(
        run("ViaRecordRef"),
        format!("{BAD}VERSION(1) SORTING(Field1,Field3) WHERE(Field1=1(2).Field120=1(0)) DB:Ok")
    );
}
