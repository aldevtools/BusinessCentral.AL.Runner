//! A Code field with `SQLDataType = Integer` in the primary key, or with another SQLDataType (`Variant`), is refused only
//! where its sort order or comparison is observed, not at every touch of the table (runner issues on the pattern of
//! refusals at the table definition). Pinned: Integer sorts by number under a secondary key
//! (corpus record/lt-x-table-props/XTablePropsTests.al:322-325). Not pinned: the order of a primary key holding such a
//! field, anything about Variant.

use al_builtins::native::Signal;
use al_testkit::{int, text};

const SRC: &str = r#"
table 69980 "SDS Pk"
{
    fields
    {
        field(1; "Entry no."; Code[10]) { SqlDataType = Integer; }
        field(2; Note; Text[20]) { }
        field(3; Qty; Integer) { }
    }
    keys { key(PK; "Entry no.") { Clustered = true; } key(K1; Note) { } }
}
table 69981 "SDS Variant"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Silo sort"; Code[6]) { SQLDataType = Variant; }
        field(3; Note; Text[20]) { }
    }
    keys { key(PK; Id) { } key(K3; Note, "Silo sort") { } key(K4; "Silo sort") { } }
}
codeunit 69982 C
{
    local procedure SeedPk()
    var R: Record "SDS Pk";
    begin
        R.Init(); R."Entry no." := '10'; R.Note := 'b'; R.Insert();
        R.Init(); R."Entry no." := '9'; R.Note := 'a'; R.Insert();
    end;

    procedure PkInsertGetCount(): Text
    var R: Record "SDS Pk";
    begin
        SeedPk();
        R.Get('9');
        R.Qty := 5;
        R.Modify();
        R.Get('10');
        R.SetRange("Entry no.", '10');
        exit(R.Note + Format(R.Count()) + Format(R.Qty));
    end;

    procedure PkCountAll(): Integer
    var R: Record "SDS Pk";
    begin
        SeedPk();
        exit(R.Count());
    end;

    procedure PkDelete(): Integer
    var R: Record "SDS Pk";
    begin
        SeedPk();
        R.Get('9');
        R.Delete();
        exit(R.Count());
    end;

    procedure OtherKeyFirst(): Text
    var R: Record "SDS Pk";
    begin
        SeedPk();
        R.SetCurrentKey(Note);
        R.FindFirst();
        exit(R."Entry no.");
    end;

    procedure PkFindFirst(): Text
    var R: Record "SDS Pk";
    begin
        SeedPk();
        R.FindFirst();
        exit(R."Entry no.");
    end;

    procedure PkFindSet(): Integer
    var R: Record "SDS Pk"; n: Integer;
    begin
        SeedPk();
        if R.FindSet() then repeat n += 1; until R.Next() = 0;
        exit(n);
    end;

    procedure PkRange(): Integer
    var R: Record "SDS Pk";
    begin
        SeedPk();
        R.SetRange("Entry no.", '9', '10');
        exit(R.Count());
    end;

    local procedure SeedVariant()
    var R: Record "SDS Variant";
    begin
        R.Init(); R.Id := 1; R.Note := 'x'; R.Insert();
        R.Init(); R.Id := 2; R.Note := 'y'; R.Insert();
    end;

    procedure VariantRows(): Text
    var R: Record "SDS Variant";
    begin
        SeedVariant();
        R.Get(2);
        R.Note := 'z';
        R.Modify();
        R.SetRange(Id, 1, 2);
        R.FindSet();
        R.Next();
        exit(Format(R.Id) + R.Note + Format(R.Count()));
    end;

    procedure VariantOtherKey(): Text
    var R: Record "SDS Variant";
    begin
        SeedVariant();
        R.SetCurrentKey(Note);
        R.FindLast();
        exit(Format(R.Id));
    end;

    procedure VariantSetValue()
    var R: Record "SDS Variant";
    begin
        R.Init(); R.Id := 1; R."Silo sort" := 'A'; R.Insert();
    end;

    procedure VariantKeyOrder(): Integer
    var R: Record "SDS Variant";
    begin
        SeedVariant();
        R.SetCurrentKey("Silo sort");
        R.FindFirst();
        exit(R.Id);
    end;

    procedure VariantFilter(): Integer
    var R: Record "SDS Variant";
    begin
        SeedVariant();
        R.SetRange("Silo sort", 'A');
        exit(R.Count());
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

fn refused(routine: &str, needle: &str) {
    match run(routine) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{routine}: {why}"),
        other => panic!("{routine}: {other:?}"),
    }
}

#[test]
fn an_integer_code_primary_key_runs_without_observing_its_order() {
    assert_eq!(text(run("PkInsertGetCount")), "b10");
    assert_eq!(int(run("PkCountAll")), 2);
    assert_eq!(int(run("PkDelete")), 1);
    // XTablePropsTests.al:322-325: a secondary key on another field is not affected.
    assert_eq!(text(run("OtherKeyFirst")), "9");
}

#[test]
fn reading_in_the_order_of_an_integer_code_primary_key_is_refused() {
    for routine in ["PkFindFirst", "PkFindSet"] {
        refused(routine, "primary key");
    }
    // A range filter on the field compares numbers or text: not pinned.
    refused("PkRange", "SQLDataType");
}

#[test]
fn a_variant_code_field_nobody_sets_does_not_stop_the_table() {
    assert_eq!(text(run("VariantRows")), "2z2");
    assert_eq!(text(run("VariantOtherKey")), "2");
}

#[test]
fn observing_a_variant_code_field_is_refused() {
    refused("VariantSetValue", "Variant");
    refused("VariantKeyOrder", "Variant");
    refused("VariantFilter", "SQLDataType");
}
