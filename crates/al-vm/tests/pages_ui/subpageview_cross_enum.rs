//! `SubPageView = where("Document Type" = const("Purchase Document Type"::Order), ...)` on a part whose field is of another enum
//! that has a member Order with the same ordinal shows the Order row (al-runner issue 7265). Pinned by corpus pull request 1199
//! (measured on real BC 27.5, 28.5 and 29.0): record/qualified-enum-filters/TestQualifiedEnumFilters.al:162
//! SubPageView_CrossEnumQualifier_SelectsPurchaseOrderComment (host page :202). There the field is Base Application's
//! "Purch. Comment Line"."Document Type"; these local enums repeat its first members. A member of the other enum whose name or
//! ordinal the field's enum does not share stays refused.

const SRC: &str = r#"
enum 69970 "XE Purchase Document Type" { value(0; Quote) { } value(1; Order) { } value(2; Invoice) { } value(3; Other) { } }
enum 69971 "XE Comment Document Type" { value(0; Quote) { } value(1; Order) { } value(2; Invoice) { } value(4; Other) { } }
table 69972 "XE Comment Line"
{
    fields
    {
        field(1; "Document Type"; Enum "XE Comment Document Type") { }
        field(2; "No."; Code[20]) { }
        field(3; "Line No."; Integer) { }
        field(4; Comment; Text[80]) { }
    }
    keys { key(PK; "Document Type", "No.", "Line No.") { Clustered = true; } }
}
table 69973 "XE Host" { fields { field(1; "Entry No."; Integer) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
page 69974 "XE Part"
{
    PageType = ListPart;
    SourceTable = "XE Comment Line";
    layout { area(Content) { repeater(Rows) { field(Value; Rec.Comment) { ApplicationArea = All; } } } }
}
page 69975 "XE Host Page"
{
    PageType = Card;
    SourceTable = "XE Host";
    layout
    {
        area(Content) { field(EntryNo; Rec."Entry No.") { ApplicationArea = All; } }
        area(FactBoxes)
        {
            part(Lines; "XE Part")
            {
                ApplicationArea = All;
                SubPageView = where("Document Type" = const("XE Purchase Document Type"::Order), "No." = const('NEUTRAL-ENUM'));
            }
        }
    }
}
page 69976 "XE Other Host Page"
{
    PageType = Card;
    SourceTable = "XE Host";
    layout
    {
        area(FactBoxes)
        {
            part(Lines; "XE Part")
            {
                ApplicationArea = All;
                SubPageView = where("Document Type" = const("XE Purchase Document Type"::Other));
            }
        }
    }
}
codeunit 69977 C
{
    local procedure Seed()
    var
        Row: Record "XE Comment Line";
    begin
        Row."Document Type" := Row."Document Type"::Order;
        Row."No." := 'NEUTRAL-ENUM';
        Row."Line No." := 101;
        Row.Comment := 'ORDER-COMMENT';
        Row.Insert(false);
        Row."Document Type" := Row."Document Type"::Invoice;
        Row.Comment := 'INVOICE-COMMENT';
        Row.Insert(false);
    end;

    procedure F(): Text
    var
        View: TestPage "XE Host Page";
        s: Text;
    begin
        Seed();
        View.OpenView();
        View.Lines.First();
        s := View.Lines.Value.Value();
        if View.Lines.Next() then
            s += '|more:' + View.Lines.Value.Value();
        View.Close();
        exit(s);
    end;

    procedure G(): Text
    var
        View: TestPage "XE Other Host Page";
    begin
        Seed();
        View.OpenView();
        exit(View.Lines.Value.Value());
    end;
}
"#;

#[test]
fn a_member_of_another_enum_with_the_same_name_and_ordinal_selects_the_row() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "F")), "ORDER-COMMENT");
}

#[test]
fn a_member_whose_ordinal_differs_stays_refused() {
    let r = al_testkit::run_in(SRC, "C", "G");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
