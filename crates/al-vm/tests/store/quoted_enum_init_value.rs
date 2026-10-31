//! An Enum field whose `InitValue` names the member in single quotes (`InitValue = 'G/L Account';`, al-runner issue 6585):
//! the member of that name, on a fresh variable, after Init and after Insert, as the double-quoted form gives
//! (types/TestBlankEnumInitTests.Codeunit.al). Not pinned yet: assumption quoted-enum-init-value.

use al_builtins::assumptions::{Assumption, take_hits};
use al_testkit::text;

const SRC: &str = r#"
enum 69985 "QEI Kind" { value(0; " ") { } value(1; "G/L Account") { } value(2; Item) { } }
table 69986 "QEI T"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Enum "QEI Kind") { InitValue = 'G/L Account'; }
    }
    keys { key(PK; Id) { } }
}
table 69987 "QEI Plain"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Enum "QEI Kind") { InitValue = "G/L Account"; }
    }
    keys { key(PK; Id) { } }
}
codeunit 69988 C
{
    procedure F(): Text
    var Rec: Record "QEI T"; Fresh: Record "QEI T"; s: Text;
    begin
        s := Format(Fresh.Kind);
        Rec.Kind := Rec.Kind::Item;
        Rec.Init();
        s += '|' + Format(Rec.Kind);
        Rec.Id := 1;
        Rec.Insert();
        Fresh.Get(1);
        s += '|' + Format(Fresh.Kind = Fresh.Kind::"G/L Account");
        exit(s);
    end;

    procedure Plain(): Text
    var R: Record "QEI Plain";
    begin
        exit(Format(R.Kind));
    end;
}
"#;

#[test]
fn a_single_quoted_enum_init_value_is_the_member_of_that_name_and_marks_its_assumption() {
    take_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "G/L Account|G/L Account|Yes");
    assert!(take_hits().contains(&Assumption::QuotedEnumInitValue));
}

#[test]
fn a_double_quoted_enum_init_value_does_not_mark_the_assumption() {
    take_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Plain")), "G/L Account");
    assert!(!take_hits().contains(&Assumption::QuotedEnumInitValue));
}
