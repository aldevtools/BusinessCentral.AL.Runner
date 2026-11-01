//! A TableRelation condition `const(Type::"Item Attribute")` that qualifies the member with the condition's own field name
//! names the member as `const("Item Attribute")` does (al-runner issue 6677; assumption relation-const-field-qualified).
//! A rename of an unrelated table leaves the field alone; so does a rename of the related table, as the field says
//! ValidateTableRelation = false (record/TestRenamePropagation.al, `rename.rs`).

use al_testkit::text;

const SRC: &str = r#"
enum 69795 "Kind Enum" { Extensible = true; value(0; "Item Attribute") { } value(1; "Custom Attribute") { } }
table 69795 Base
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 69796 "Base Attr"
{
    fields { field(1; Name; Text[250]) { } }
    keys { key(PK; Name) { Clustered = true; } }
}
table 69797 Link
{
    fields
    {
        field(1; "Parent No."; Code[20]) { TableRelation = Base; }
        field(2; Type; Enum "Kind Enum") { }
        field(4; "Attribute Name"; Text[250])
        {
            TableRelation = if (Type = const(Type::"Item Attribute")) "Base Attr".Name;
            ValidateTableRelation = false;
        }
    }
    keys { key(PK; "Parent No.", "Attribute Name") { Clustered = true; } }
}
codeunit 69795 C
{
    procedure RenameParent(): Text
    var P: Record Base; L: Record Link;
    begin
        P."No." := 'OLD'; P.Insert();
        L."Parent No." := 'OLD'; L."Attribute Name" := 'Colour'; L.Insert();
        P.Rename('NEW');
        exit(Format(L.Get('NEW', 'Colour')));
    end;

    procedure RenameAttr(): Text
    var A: Record "Base Attr"; L: Record Link;
    begin
        A.Name := 'Colour'; A.Insert();
        L."Parent No." := 'P'; L."Attribute Name" := 'Colour'; L.Insert();
        A.Rename('Color');
        exit(Format(L.Get('P', 'Colour')) + Format(L.Get('P', 'Color')));
    end;
}
"#;

const OPTION_SRC: &str = r#"
table 69796 "Base Attr"
{
    fields { field(1; Name; Text[250]) { } }
    keys { key(PK; Name) { Clustered = true; } }
}
table 69798 OptLink
{
    fields
    {
        field(1; Id; Integer) { }
        field(3; Kind; Option "Item Attribute","Custom Attribute") { }
        field(5; "Option Name"; Text[250])
        {
            TableRelation = if (Kind = const(Kind::"Item Attribute")) "Base Attr".Name;
        }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69795 C
{
    procedure ValidateOption(): Text
    var A: Record "Base Attr"; L: Record OptLink;
    begin
        A.Name := 'Colour'; A.Insert();
        L.Validate("Option Name", 'Colour');
        exit(L."Option Name");
    end;
}
"#;

#[test]
fn a_field_qualified_const_condition_parses_and_a_rename_of_another_table_ignores_it() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "RenameParent")), "Yes");
}

#[test]
fn a_rename_of_the_related_table_parses_the_arm_and_keeps_a_field_without_validation() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "RenameAttr")), "YesNo");
}

// The same qualified form on an Option field is not pinned (whether BC compiles it is not known): it stays refused.
#[test]
fn an_option_field_qualified_const_arm_stays_refused() {
    assert!(matches!(
        al_testkit::run_in(OPTION_SRC, "C", "ValidateOption"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
