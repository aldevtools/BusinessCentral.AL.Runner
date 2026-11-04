//! An Option field whose `InitValue` is a number (`InitValue = 1`, Power BI Report embeddings' PowerBI Reports
//! Setup."First Day Of Week"): the member at that ordinal, on a fresh variable, after Init and after Insert. Windows BC
//! 29.0 probe (corpus branch winprobe-wc6, tests/wc6-probe/ContosoProbe.al, cu 61320 CS_OptionInitValueOrdinal);
//! assumption option-init-value-ordinal.

use al_builtins::assumptions::take_hits;
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
fn a_numeric_option_init_value_is_the_member_at_that_ordinal() {
    // record/Y70Probes.al:715-748 OptionInitValue_Ordinal.
    take_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "Monday/1|Monday/1|Monday/1");
    assert!(take_hits().is_empty());
}

#[test]
fn an_option_init_value_by_name_is_that_member() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Named")), "B");
}

/// A member named by digits wins over the ordinal, for the quoted and the bare form alike (al-runner issue 6859; corpus
/// record/crud/R6859NumericOptionInit.al, measured on Windows BC 27.5).
#[test]
fn a_numeric_init_value_that_is_a_member_name_is_that_member() {
    let src = r#"
table 69983 "OIO Amb" { fields { field(1; Id; Integer) { }
  field(2; Q; Option) { OptionMembers = "3","2","1","0"; InitValue = "1"; }
  field(3; B; Option) { OptionMembers = "3","2","1","0"; InitValue = 1; }
  field(4; H; Option) { OptionMembers = "10","20","90","100","110"; InitValue = "100"; }
  field(5; L; Option) { OptionMembers = " ","Net 30","Net 60"; InitValue = "net 30"; } } keys { key(PK; Id) { } } }
codeunit 69984 C { procedure F(): Text var R: Record "OIO Amb"; begin R.Init();
  exit(Format(R.Q) + '/' + Format(R.Q, 0, 2) + ';' + Format(R.B) + '/' + Format(R.B, 0, 2) + ';' + Format(R.H) + '/' + Format(R.H, 0, 2) + ';' + Format(R.L) + '/' + Format(R.L, 0, 2)); end; }
"#;
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "1/2;1/2;100/3;Net 30/1");
}

/// An Enum field whose member names are numbers: the name is taken, not the ordinal (corpus
/// record/crud/R6859NumericOptionInit.al R6859_NumEnum, Windows BC 27.5 and 28.5).
#[test]
fn an_enum_init_value_that_is_a_number_is_the_member_of_that_name() {
    let src = r#"
enum 69985 NumE { Extensible = true; value(0; "10") { } value(1; "20") { } value(3; "5") { } value(7; "7") { } value(100; "1") { } }
table 69986 "OIO En" { fields { field(1; Id; Integer) { }
  field(2; A; Enum NumE) { InitValue = "20"; } field(3; B; Enum NumE) { InitValue = "7"; }
  field(4; C; Enum NumE) { InitValue = "5"; } field(5; D; Enum NumE) { InitValue = "1"; } } keys { key(PK; Id) { } } }
codeunit 69987 C { procedure F(): Text var R: Record "OIO En"; begin R.Init();
  exit(Format(R.A) + '/' + Format(R.A.AsInteger()) + ';' + Format(R.B) + '/' + Format(R.B.AsInteger()) + ';' + Format(R.C) + '/' + Format(R.C.AsInteger()) + ';' + Format(R.D) + '/' + Format(R.D.AsInteger())); end; }
"#;
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "20/1;7/7;5/3;1/100");
}
