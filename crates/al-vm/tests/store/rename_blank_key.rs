//! A Rename from a blank primary key of a table another table relates to (al-runner issue 7937, assumption
//! rename-blank-key-with-relation): only the reporter's shape raises BC's error; every other blank or zero key value of a
//! renamed table that other tables relate to is refused; a table nothing relates to keeps its rename.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

/// The reporter's tables (issue 7937, `app/Repro.al`) and the test body as AL source, plus a codeunit of procedures.
const TABLES: &str = r#"
table 50100 "RB Master"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 50101 "RB Child"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Master Code"; Code[20]) { TableRelation = "RB Master"; }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 50103 "RB Int Master"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { } }
}
table 50104 "RB Int Child"
{
    fields { field(1; Id; Integer) { } field(2; MasterNo; Integer) { TableRelation = "RB Int Master"; } }
    keys { key(PK; Id) { } }
}
table 50105 "RB Soft Master"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { } }
}
table 50106 "RB Soft Child"
{
    fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = "RB Soft Master"; ValidateTableRelation = false; } }
    keys { key(PK; Id) { } }
}
table 50102 "RB Lone"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { } }
}
"#;

fn run_on(bc: &str, body: &str) -> Result<String, Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let src = format!("{TABLES}\ncodeunit 50150 C {{ procedure F(): Text {body} }}");
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = al_vm::Engine::with_target(l.program.clone(), al_testkit::blocked(&l), Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    match s.call(oi, 0, inst, vec![])?.0 {
        Value::Text(t) => Ok(t.to_string()),
        other => panic!("{other:?}"),
    }
}

/// The reporter's test body (issue 7937, `test/ReproTest.Codeunit.al`), reading the error text instead of Library Assert.
const REPRO: &str = "var Master: Record \"RB Master\"; Child: Record \"RB Child\"; begin
    Master.Code := ''; Master.Insert();
    Child.\"Entry No.\" := 2; Child.\"Master Code\" := ''; Child.Insert();
    asserterror Master.Rename('X');
    exit(GetLastErrorText());
end;";

#[test]
fn the_reporters_rename_from_a_blank_key_raises_bcs_error() {
    take_hits();
    let r = run_on("29.0", REPRO).unwrap();
    assert_eq!(
        r,
        "The Code field on the RB Master table cannot be renamed from \"\" to \"X\".\n\nWhen a field has a relation defined, \
         the field cannot be renamed to and from zero or an empty value."
    );
    assert!(take_hits().contains(&Assumption::RenameBlankKeyWithRelation));
}

fn refused(r: Result<String, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// The reporter measured BC 29.1; the assumption takes BC 27 and 28 alike (no version gate has corpus evidence).
#[test]
fn the_same_rename_on_bc_27_and_28_takes_the_assumption() {
    for bc in ["27.0", "28.0"] {
        take_hits();
        assert!(
            run_on(bc, REPRO)
                .unwrap()
                .starts_with("The Code field on the RB Master table cannot be renamed")
        );
        assert!(take_hits().contains(&Assumption::RenameBlankKeyWithRelation));
    }
}

#[test]
fn a_rename_to_a_blank_key_is_refused() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Master\"; Child: Record \"RB Child\"; begin
            Master.Code := 'A'; Master.Insert();
            Child.\"Entry No.\" := 2; Child.\"Master Code\" := 'A'; Child.Insert();
            Master.Rename('');
            exit('renamed');
        end;",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_rename_from_a_blank_key_no_child_row_carries_is_refused() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Master\"; begin
            Master.Code := ''; Master.Insert();
            Master.Rename('X');
            exit('renamed');
        end;",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_rename_from_a_blank_key_two_child_rows_carry_is_refused() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Master\"; Child: Record \"RB Child\"; begin
            Master.Code := ''; Master.Insert();
            Child.\"Entry No.\" := 2; Child.\"Master Code\" := ''; Child.Insert();
            Child.\"Entry No.\" := 3; Child.Insert();
            Master.Rename('X');
            exit('renamed');
        end;",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_temporary_rename_from_a_blank_key_is_refused() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Master\" temporary; begin
            Master.Code := ''; Master.Insert();
            Master.Rename('X');
            exit(Master.Code);
        end;",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

// Main's outcome stays: a table nothing relates to renames from a blank key, and a non-blank rename follows the relation.
#[test]
fn a_rename_from_a_blank_key_of_a_table_nothing_relates_to_keeps_its_rename() {
    let r = run_on(
        "29.0",
        "var Lone: Record \"RB Lone\"; begin
            Lone.Code := ''; Lone.Insert();
            Lone.Rename('X');
            exit(Lone.Code);
        end;",
    );
    assert_eq!(r.unwrap(), "X");
}

#[test]
fn a_rename_between_non_blank_keys_still_follows_the_relation() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Master\"; Child: Record \"RB Child\"; begin
            Master.Code := 'A'; Master.Insert();
            Child.\"Entry No.\" := 2; Child.\"Master Code\" := 'A'; Child.Insert();
            Master.Rename('X');
            Child.Get(2);
            exit(Child.\"Master Code\");
        end;",
    );
    assert_eq!(r.unwrap(), "X");
}

#[test]
fn the_error_code_is_not_known() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Master\"; Child: Record \"RB Child\"; begin
            Master.Code := ''; Master.Insert();
            Child.\"Entry No.\" := 2; Child.\"Master Code\" := ''; Child.Insert();
            asserterror Master.Rename('X');
            exit(GetLastErrorCode());
        end;",
    );
    assert!(refused(r).contains("not pinned"));
}

#[test]
fn a_rename_of_an_integer_key_to_zero_is_refused() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Int Master\"; Child: Record \"RB Int Child\"; begin
            Master.No := 1; Master.Insert();
            Child.Id := 2; Child.MasterNo := 1; Child.Insert();
            Master.Rename(0);
            exit('renamed');
        end;",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_rename_from_a_blank_key_through_an_unvalidated_relation_is_refused() {
    let r = run_on(
        "29.0",
        "var Master: Record \"RB Soft Master\"; Child: Record \"RB Soft Child\"; begin
            Master.Code := ''; Master.Insert();
            Child.Id := 2; Child.Ref := ''; Child.Insert();
            Master.Rename('X');
            exit('renamed');
        end;",
    );
    assert!(refused(r).contains("blank or zero key value"));
}
