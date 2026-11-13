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
table 50110 "RB Inv"
{
    fields { field(1; No; Code[20]) { } field(2; Posted; Boolean) { } field(3; Id; Guid) { } }
    keys { key(PK; No, Posted) { } }
}
table 50111 "RB Inv Ref"
{
    fields { field(1; Id; Integer) { } field(2; InvId; Guid) { TableRelation = "RB Inv".Id; } }
    keys { key(PK; Id) { } }
}
table 50112 "RB Two"
{
    fields { field(1; A; Code[20]) { } field(2; B; Code[20]) { } }
    keys { key(PK; A, B) { } }
}
table 50113 "RB Two Ref"
{
    fields { field(1; Id; Integer) { } field(2; RefA; Code[20]) { TableRelation = "RB Two".A; } }
    keys { key(PK; Id) { } }
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

fn run_src(bc: &str, extra: &str, body: &str) -> Result<String, Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let src = format!("{TABLES}\n{extra}\ncodeunit 50150 C {{ procedure F(): Text {body} }}");
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

// HIGH 1 of the review of PR 7943: only a relation whose related field is the key field that changes to or from blank enters.
// Base Application renames "Sales Invoice Entity Aggregate" ("No.", Posted) from Posted = false, while Gen. Journal Line
// relates to its Id (SalesInvoiceAggregator.Codeunit.al:195, GenJournalLine.Table.al:3806): main renames it, and so does this.
#[test]
fn a_boolean_key_field_leaving_false_with_a_relation_to_a_non_key_field_keeps_its_rename() {
    let r = run_src(
        "29.0",
        "",
        "var Inv: Record \"RB Inv\"; R: Record \"RB Inv Ref\"; begin
            Inv.No := 'A'; Inv.Id := CreateGuid(); Inv.Insert();
            R.Id := 1; R.InvId := Inv.Id; R.Insert();
            Inv.Rename('A', true);
            exit(Format(Inv.Posted));
        end;",
    );
    assert_eq!(r.unwrap(), "Yes");
}

#[test]
fn a_relation_to_key_field_a_while_key_field_b_leaves_blank_keeps_its_rename() {
    let r = run_src(
        "29.0",
        "",
        "var Two: Record \"RB Two\"; R: Record \"RB Two Ref\"; begin
            Two.A := 'A1'; Two.B := ''; Two.Insert();
            R.Id := 1; R.RefA := 'A1'; R.Insert();
            Two.Rename('A1', 'Y');
            R.Get(1);
            exit(Two.B + '/' + R.RefA);
        end;",
    );
    assert_eq!(r.unwrap(), "Y/A1");
}

/// The reporter's tables and rows, then `tail` (which renames and exits).
fn repro_with(extra: &str, tail: &str) -> Result<String, Signal> {
    run_src(
        "29.0",
        extra,
        &format!(
            "var Master: Record \"RB Master\"; Child: Record \"RB Child\"; M2: Record \"RB Master\"; RR: RecordRef; begin
                Master.Code := ''; Master.Insert();
                Child.\"Entry No.\" := 2; Child.\"Master Code\" := ''; Child.Insert();
                {tail}
            end;"
        ),
    )
}

// MED 2 of the review: each fact of the reporter's shape is required; a rename that differs in one of them is refused.
#[test]
fn a_rename_from_a_blank_key_onto_a_stored_key_is_refused() {
    let r = repro_with(
        "",
        "M2.Code := 'X'; M2.Insert(); asserterror Master.Rename('X'); exit(GetLastErrorText());",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn the_return_value_form_is_refused() {
    let r = repro_with("", "if Master.Rename('X') then exit('true'); exit('false');");
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn recordref_rename_is_refused() {
    let r = repro_with(
        "",
        "RR.GetTable(Master); asserterror RR.Rename('X'); exit(GetLastErrorText());",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_rename_with_the_database_rename_trigger_on_is_refused() {
    let subs = r#"codeunit 50160 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"RB Master" then OnDatabaseRename := true;
    end;
}"#;
    let r = repro_with(subs, "asserterror Master.Rename('X'); exit(GetLastErrorText());");
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_child_table_with_other_rows_beside_the_blank_one_is_refused() {
    let r = repro_with(
        "",
        "Child.\"Entry No.\" := 3; Child.\"Master Code\" := 'Q'; Child.Insert(); asserterror Master.Rename('X'); exit(GetLastErrorText());",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_buffer_read_under_another_key_is_refused() {
    let r = repro_with(
        "",
        "M2.Code := 'Q'; M2.Insert(); M2.Get('Q'); M2.Code := ''; asserterror M2.Rename('X'); exit(GetLastErrorText());",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

#[test]
fn a_rename_of_a_row_that_is_not_stored_is_refused() {
    let r = run_src(
        "29.0",
        "",
        "var Master: Record \"RB Master\"; Child: Record \"RB Child\"; begin
            Child.\"Entry No.\" := 2; Child.\"Master Code\" := ''; Child.Insert();
            Master.Code := '';
            asserterror Master.Rename('X');
            exit(GetLastErrorText());
        end;",
    );
    assert!(refused(r).contains("blank or zero key value"));
}

/// A table "S Master" renamed from its blank key to `new` while "S Child" (one row, its fields at their defaults) relates to it.
fn shape(master: &str, child: &str, extra: &str, new: &str) -> Result<String, Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse("29.0").unwrap());
    let src = format!(
        "{master}\n{child}\n{extra}\ncodeunit 50150 C {{ procedure F(): Text var M: Record \"S Master\"; C: Record \"S Child\"; begin
            M.Insert(); C.Id := 1; C.Insert();
            asserterror M.Rename({new});
            exit(GetLastErrorText());
        end; }}"
    );
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

const S_MASTER: &str = "table 50120 \"S Master\" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[20]) { } } keys { key(PK; Code) { } } }";
const S_CHILD: &str = "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\"; } } keys { key(PK; Id) { } } }";

#[test]
fn the_plain_shape_raises_bcs_error() {
    let r = shape(S_MASTER, S_CHILD, "", "'X'").unwrap();
    assert!(
        r.starts_with("The Code field on the S Master table cannot be renamed"),
        "{r}"
    );
}

// The facts of the reporter's shape one at a time (the PR's area table): each other value is refused.
#[test]
fn every_other_shape_of_master_or_child_is_refused() {
    let child = |rel: &str| {
        format!(
            "table 50121 \"S Child\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Ref; Code[20]) {{ {rel} }} field(3; Kind; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}"
        )
    };
    let master_with = |props: &str, field_props: &str, key_ty: &str| {
        format!(
            "table 50120 \"S Master\" {{ {props} fields {{ field(1; Code; {key_ty}) {{ {field_props} }} field(2; Name; Text[20]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}"
        )
    };
    let typed_child = |ty: &str| {
        format!(
            "table 50121 \"S Child\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Ref; {ty}) {{ TableRelation = \"S Master\"; }} }} keys {{ key(PK; Id) {{ }} }} }}"
        )
    };
    let plain = child("TableRelation = \"S Master\";");
    let sub = "codeunit 50161 Sub { [EventSubscriber(ObjectType::Table, Database::\"S Master\", 'OnBeforeInsertEvent', '', false, false)] local procedure B(var Rec: Record \"S Master\") begin end; }";
    let cases: Vec<(&str, String, String, &str, &str)> = vec![
        ("table caption", master_with("Caption = 'Cap';", "", "Code[20]"), plain.clone(), "", "'X'"),
        ("field caption", master_with("", "Caption = 'Cap';", "Code[20]"), plain.clone(), "", "'X'"),
        (
            "OnRename trigger",
            master_with("trigger OnRename() begin end;", "", "Code[20]"),
            plain.clone(),
            "",
            "'X'",
        ),
        (
            "table extension",
            S_MASTER.into(),
            plain.clone(),
            "tableextension 50130 SX extends \"S Master\" { fields { field(50; X; Integer) { } } }",
            "'X'",
        ),
        ("table event subscriber", S_MASTER.into(), plain.clone(), sub, "'X'"),
        ("not per company", master_with("DataPerCompany = false;", "", "Code[20]"), plain.clone(), "", "'X'"),
        ("Guid key", master_with("", "", "Guid"), typed_child("Guid"), "", "CreateGuid()"),
        ("Integer key", master_with("", "", "Integer"), typed_child("Integer"), "", "5"),
        ("Date key", master_with("", "", "Date"), typed_child("Date"), "", "20240101D"),
        ("shorter child field", S_MASTER.into(), typed_child("Code[10]"), "", "'X'"),
        (
            "condition",
            S_MASTER.into(),
            child("TableRelation = if (Kind = const(0)) \"S Master\";"),
            "",
            "'X'",
        ),
        ("where()", S_MASTER.into(), child("TableRelation = \"S Master\" where(Name = const(''));"), "", "'X'"),
        ("named related field", S_MASTER.into(), child("TableRelation = \"S Master\".Code;"), "", "'X'"),
        (
            "unvalidated",
            S_MASTER.into(),
            child("TableRelation = \"S Master\"; ValidateTableRelation = false;"),
            "",
            "'X'",
        ),
        (
            "FlowFilter relation",
            S_MASTER.into(),
            child("FieldClass = FlowFilter; TableRelation = \"S Master\";"),
            "",
            "'X'",
        ),
        (
            "child field in the child's key",
            S_MASTER.into(),
            "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\"; } } keys { key(PK; Id, Ref) { } } }".into(),
            "",
            "'X'",
        ),
        (
            "same-table relation",
            "table 50120 \"S Master\" { fields { field(1; Code; Code[20]) { } field(2; Parent; Code[20]) { TableRelation = \"S Master\"; } } keys { key(PK; Code) { } } }".into(),
            "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }".into(),
            "",
            "'X'",
        ),
        (
            "multi-field key",
            "table 50120 \"S Master\" { fields { field(1; Code; Code[20]) { } field(2; Sub; Code[20]) { } } keys { key(PK; Code, Sub) { } } }".into(),
            plain.clone(),
            "",
            "'X', ''",
        ),
    ];
    for (name, master, child, extra, new) in cases {
        match shape(&master, &child, extra, new) {
            Err(Signal::Unsupported(why)) => {
                assert!(why.to_string().contains("blank or zero key value"), "{name}: {why}")
            }
            other => panic!("{name}: expected a refusal, got {other:?}"),
        }
    }
}

// A temporary rename asks which fields relate to the table without following them: it takes none of the assumptions a
// rename that follows an arm takes (here rename-follows-filter-condition-arms), and with no relation to the key field that
// changes it keeps main's rename.
#[test]
fn a_temporary_rename_takes_no_assumption_of_the_arms_it_does_not_follow() {
    let master = "table 50120 \"S Master\" { fields { field(1; A; Code[20]) { } field(2; B; Code[20]) { } } keys { key(PK; A, B) { } } }";
    let child = "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Kind; Integer) { } field(3; Ref; Code[20]) { TableRelation = if (Kind = filter(1 | 2)) \"S Master\".A; } } keys { key(PK; Id) { } } }";
    let target = al_version::Target::for_bc(al_version::BcVersion::parse("29.0").unwrap());
    let src = format!(
        "{master}\n{child}\ncodeunit 50150 C {{ procedure F(): Text var M: Record \"S Master\" temporary; begin
            M.A := 'A1'; M.Insert();
            M.Rename('A1', 'Y');
            exit(M.B);
        end; }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = al_vm::Engine::with_target(l.program.clone(), al_testkit::blocked(&l), Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    take_hits();
    let r = s.call(oi, 0, inst, vec![]).unwrap().0;
    assert!(matches!(r, Value::Text(t) if t == "Y"));
    assert!(!take_hits().contains(&Assumption::RenameFollowsFilterConditionArms));
}
