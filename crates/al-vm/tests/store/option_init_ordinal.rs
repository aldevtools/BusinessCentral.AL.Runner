//! An Option field whose `InitValue` is a number (`InitValue = 1`, Power BI Report embeddings' PowerBI Reports
//! Setup."First Day Of Week"): the member at that ordinal, on a fresh variable, after Init and after Insert. Windows BC
//! 29.0 probe (corpus branch winprobe-wc6, tests/wc6-probe/ContosoProbe.al, cu 61320 CS_OptionInitValueOrdinal);
//! assumption option-init-value-ordinal.

use al_builtins::assumptions::{Assumption, take_hits};
use al_testkit::text;

const SRC: &str = r#"
table 69980 "OIO T"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Dow; Option) { OptionMembers = Sunday,Monday,Tuesday,Wednesday,Thursday,Friday,Saturday; InitValue = 1; }
    }
    keys { key(PK; Id) { } }
}
table 69981 "OIO Named"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = A,B,C; InitValue = B; }
    }
    keys { key(PK; Id) { } }
}
codeunit 69982 C
{
    procedure F(): Text
    var Rec: Record "OIO T"; Fresh: Record "OIO T"; s: Text;
    begin
        s := Format(Fresh.Dow) + '/' + Format(Fresh.Dow, 0, 2);
        Rec.Dow := Rec.Dow::Friday;
        Rec.Init();
        s += '|' + Format(Rec.Dow) + '/' + Format(Rec.Dow, 0, 2);
        Rec.Id := 1;
        Rec.Insert();
        Fresh.Get(1);
        s += '|' + Format(Fresh.Dow) + '/' + Format(Fresh.Dow, 0, 2);
        exit(s);
    end;

    procedure Named(): Text
    var R: Record "OIO Named";
    begin
        exit(Format(R.Opt));
    end;
}
"#;

#[test]
fn a_numeric_option_init_value_is_the_member_at_that_ordinal_and_marks_its_assumption() {
    take_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "Monday/1|Monday/1|Monday/1");
    assert!(take_hits().contains(&Assumption::OptionInitValueOrdinal));
}

#[test]
fn an_option_init_value_by_name_does_not_mark_the_assumption() {
    take_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Named")), "B");
    assert!(!take_hits().contains(&Assumption::OptionInitValueOrdinal));
}

/// A member named by digits that sits at another ordinal makes `InitValue = 1` ambiguous between the name and the
/// ordinal; nothing pins which BC takes, so the InitValue stays unresolved and Init refuses.
#[test]
fn a_numeric_init_value_that_is_also_a_member_name_elsewhere_is_refused() {
    let src = r#"
table 69983 "OIO Amb" { fields { field(1; Id; Integer) { } field(2; Opt; Option) { OptionMembers = " ",X,"1"; InitValue = 1; } } keys { key(PK; Id) { } } }
codeunit 69984 C { procedure F(): Text var R: Record "OIO Amb"; begin R.Init(); exit(Format(R.Opt)); end; }
"#;
    let r = al_testkit::run_in(src, "C", "F");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(w)) if w.contains("InitValue")),
        "{r:?}"
    );
}
