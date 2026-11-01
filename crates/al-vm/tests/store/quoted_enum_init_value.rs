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

/// Another case and a trailing space name the member too (record/lt-y47-initvalue-quoted/Y47EnumInitQuoted.al
/// Y47_InitValue_Lower and Y47_InitValue_Space, corpus branch al-runtime-rs-staging-y47). A number (whether it is the
/// ordinal or the position is not told apart by the probe) and an unknown name stay refused.
#[test]
fn a_single_quoted_name_in_another_case_or_with_a_space_is_the_member() {
    let src = r#"
enum 69989 "QEI Case" { value(0; " ") { } value(1; "G/L Account") { } }
table 69990 "QEI Case T" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "QEI Case") { InitValue = 'g/l account'; } } keys { key(PK; Id) { } } }
table 69992 "QEI Space T" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "QEI Case") { InitValue = 'G/L Account '; } } keys { key(PK; Id) { } } }
table 69993 "QEI Digit T" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "QEI Case") { InitValue = '1'; } } keys { key(PK; Id) { } } }
codeunit 69991 C {
    procedure F(): Text var R: Record "QEI Case T"; S: Record "QEI Space T"; begin R.Init(); S.Init(); exit(Format(R.Kind) + '|' + Format(S.Kind)); end;
    procedure D(): Text var R: Record "QEI Digit T"; begin R.Init(); exit(Format(R.Kind)); end;
}
"#;
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "G/L Account|G/L Account");
    let r = al_testkit::run_in(src, "C", "D");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(w)) if w.contains("InitValue")),
        "{r:?}"
    );
}
