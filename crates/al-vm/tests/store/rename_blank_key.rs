//! A Rename that changes a primary key field to or from its blank or zero value while a validated relation targets that key
//! field (al-runner issue 7937, assumption rename-blank-key-with-relation). Business Central raises an error after the
//! triggers; the values are those of corpus pull request 1370, record/table-relation/RenameBlankKeyRelation.al (the `:line`
//! citations). Only the measured cells raise it: the base cell and each cell one fact away from it. Any other such rename is
//! refused, and a rename outside the rule keeps main's result.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const MASTER: &str = "table 50120 \"S Master\" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[20]) { } } keys { key(PK; Code) { } } }";
const CHILD: &str = "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\"; } } keys { key(PK; Id) { } } }";
const SECOND: &str = "The Code field on the S Master table cannot be renamed from \"\" to \"X\".\\\\When a field has a relation defined, the field cannot be renamed to and from zero or an empty value.";

/// Runs `procedure F(): Text {body}` of a codeunit beside the objects `objects`, on BC `bc`.
fn run_on(bc: &str, objects: &str, body: &str) -> Result<String, Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let src = format!("{objects}\ncodeunit 50150 C {{ procedure F(): Text {body} }}");
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

fn run(objects: &str, body: &str) -> Result<String, Signal> {
    run_on("29.0", objects, body)
}

/// The probe's form (:1239): the setup is committed, another variable reads the row by Get and renames it to `new`. Gives
/// the error text and code.
fn cell(master: &str, child: &str, extra: &str, setup: &str, new: &str) -> Result<String, Signal> {
    run(
        &format!("{master}\n{child}\n{extra}"),
        &format!(
            "var M: Record \"S Master\"; M2: Record \"S Master\"; C: Record \"S Child\"; RR: RecordRef; Ok: Boolean; begin
                {setup} M.Insert(); C.Id := 1; C.Insert(); Commit();
                M2.Get(M.Code);
                asserterror M2.Rename({new});
                exit(GetLastErrorText() + '|' + GetLastErrorCode());
            end;"
        ),
    )
}

fn refused(r: Result<String, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The reporter's test body (issue 7937, `test/ReproTest.Codeunit.al`; the probe's :3007), reading the error instead of
/// Library Assert.
const REPRO: &str = "var Master: Record \"S Master\"; Child: Record \"S Child\"; begin
    Master.Code := ''; Master.Insert();
    Child.Id := 2; Child.Ref := ''; Child.Insert();
    asserterror Master.Rename('X');
    exit(GetLastErrorText() + '|' + GetLastErrorCode());
end;";

// The values are the same on BC 27.5, 28.5 and 29.0 (:1239, :3007): two backslash characters, no CR or LF, code DB:Ok.
#[test]
fn the_reporters_rename_from_a_blank_key_raises_bcs_error_on_every_major() {
    for bc in ["27.0", "28.0", "29.0"] {
        take_hits();
        let r = run_on(bc, &format!("{MASTER}\n{CHILD}"), REPRO).unwrap();
        assert_eq!(r, format!("{SECOND}|DB:Ok"), "{bc}");
        assert!(take_hits().contains(&Assumption::RenameBlankKeyWithRelation));
    }
}

#[test]
fn the_base_cell_raises_from_and_to_blank() {
    assert_eq!(cell(MASTER, CHILD, "", "", "'X'").unwrap(), format!("{SECOND}|DB:Ok"));
    let r = cell(MASTER, CHILD, "", "M.Code := 'x\"y';", "''").unwrap();
    assert!(
        r.starts_with("The Code field on the S Master table cannot be renamed from \"X\"Y\" to \"\".\\\\When"),
        "{r}"
    );
}

fn master_with(props: &str, field_props: &str, key_ty: &str) -> String {
    format!(
        "table 50120 \"S Master\" {{ {props} fields {{ field(1; Code; {key_ty}) {{ {field_props} }} field(2; Name; Text[20]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}"
    )
}

fn child_with(ty: &str, rel: &str, rest: &str) -> String {
    format!(
        "table 50121 \"S Child\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Ref; {ty}) {{ {rel} }} field(3; Kind; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} {rest} }}"
    )
}

const PLAIN: &str = "TableRelation = \"S Master\";";
const LOG: &str = "codeunit 50162 Log { SingleInstance = true; var T: Text; procedure Add(S: Text) begin T += S; end; procedure Get(): Text begin exit(T); end; }";
const SUBS: &str = "codeunit 50161 Sub {
    [EventSubscriber(ObjectType::Table, Database::\"S Master\", 'OnBeforeRenameEvent', '', false, false)] local procedure B(var Rec: Record \"S Master\") var L: Codeunit Log; begin L.Add('before;'); end;
    [EventSubscriber(ObjectType::Table, Database::\"S Master\", 'OnAfterRenameEvent', '', false, false)] local procedure A(var Rec: Record \"S Master\") var L: Codeunit Log; begin L.Add('after;'); end; }";

// Each cell that differs from the base cell in one fact raises the error with its own text.
#[test]
fn each_measured_cell_raises_the_error() {
    let from = |field: &str, table: &str, old: &str, new: &str| {
        format!(
            "The {field} field on the {table} table cannot be renamed from \"{old}\" to \"{new}\".\\\\When a field has a relation defined, the field cannot be renamed to and from zero or an empty value.|DB:Ok"
        )
    };
    let plain = child_with("Code[20]", PLAIN, "");
    let x = from("Code", "S Master", "", "X");
    let typed = |ty: &str| (master_with("", "", ty), child_with(ty, PLAIN, ""));
    let (int_m, int_c) = typed("Integer");
    let (guid_m, guid_c) = typed("Guid");
    let (date_m, date_c) = typed("Date");
    let (opt_m, opt_c) = typed("Option");
    let opt_m = opt_m.replace("Code; Option) {", "Code; Option) { OptionMembers = \" \",Alpha,Beta;");
    let opt_c = opt_c.replace("Ref; Option) {", "Ref; Option) { OptionMembers = \" \",Alpha,Beta;");
    let cases: Vec<(&str, String, String, &str, &str, &str, String)> = vec![
        // :2034
        ("captions", master_with("Caption = 'T Cap';", "Caption = 'F Cap';", "Code[20]"), plain.clone(), "", "", "'X'", from("F Cap", "T Cap", "", "X")),
        // :1333, :1346
        ("Integer key from zero", int_m.clone(), int_c.clone(), "", "", "5", from("Code", "S Master", "0", "5")),
        ("Integer key to zero", int_m, int_c, "", "M.Code := 5;", "0", from("Code", "S Master", "5", "0")),
        // :1441
        ("Guid key", guid_m, guid_c, "", "", "'{11111111-2222-3333-4444-555555555555}'", from("Code", "S Master", "{00000000-0000-0000-0000-000000000000}", "{11111111-2222-3333-4444-555555555555}")),
        // :1533
        ("Date key", date_m, date_c, "", "", "DMY2Date(1, 2, 2024)", from("Code", "S Master", "", "02/01/24")),
        // :1489
        ("Option key", opt_m, opt_c, "", "", "1", from("Code", "S Master", " ", "Alpha")),
        // :2647
        ("shorter related field", MASTER.into(), child_with("Code[10]", PLAIN, ""), "", "", "'X'", x.clone()),
        // :1676
        ("condition", MASTER.into(), child_with("Code[20]", "TableRelation = if (Kind = const(1)) \"S Master\";", ""), "", "", "'X'", x.clone()),
        // :1738
        ("where()", MASTER.into(), child_with("Code[20]", "TableRelation = \"S Master\" where(Name = const('N'));", ""), "", "", "'X'", x.clone()),
        // :1780
        ("named key field", MASTER.into(), child_with("Code[20]", "TableRelation = \"S Master\".Code;", ""), "", "", "'X'", x.clone()),
        // :2767
        ("ObsoleteState = Pending", MASTER.into(), child_with("Code[20]", "TableRelation = \"S Master\"; ObsoleteState = Pending; ObsoleteReason = 'r';", ""), "", "", "'X'", x.clone()),
        // :2793
        ("TestTableRelation = false", MASTER.into(), child_with("Code[20]", "TableRelation = \"S Master\"; TestTableRelation = false;", ""), "", "", "'X'", x.clone()),
        // :2819
        ("NotBlank key", master_with("", "NotBlank = true;", "Code[20]"), plain.clone(), "", "", "'X'", x.clone()),
        // :2845
        ("key with its own relation", master_with("", "TableRelation = \"S Target\";", "Code[20]"), plain.clone(), "table 50122 \"S Target\" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }", "", "'X'", x.clone()),
        // :2871
        ("OnModify on the related table", MASTER.into(), child_with("Code[20]", PLAIN, "trigger OnModify() begin Error('ran'); end;"), "", "", "'X'", x.clone()),
        // :1937
        ("same-table relation", "table 50120 \"S Master\" { fields { field(1; Code; Code[20]) { } field(2; Parent; Code[20]) { TableRelation = \"S Master\"; } } keys { key(PK; Code) { } } }".into(), "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }".into(), "", "", "'X'", x.clone()),
        // :1895
        ("table extension field", MASTER.into(), "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }".into(), "tableextension 50130 CX extends \"S Child\" { fields { field(50; Ref; Code[20]) { TableRelation = \"S Master\"; } } }", "", "'X'", x.clone()),
    ];
    for (name, master, child, extra, setup, new, want) in cases {
        take_hits();
        match cell(&master, &child, extra, setup, new) {
            Ok(got) => assert_eq!(got, want, "{name}"),
            other => panic!("{name}: {other:?}"),
        }
        assert!(take_hits().contains(&Assumption::RenameBlankKeyWithRelation), "{name}");
    }
}

// :1979: OnBeforeRenameEvent and the OnRename trigger run, then the error; OnAfterRenameEvent does not run.
#[test]
fn the_error_comes_after_the_trigger_and_before_the_after_event() {
    let master = master_with(
        "trigger OnRename() var L: Codeunit Log; begin L.Add('trigger;'); end;",
        "",
        "Code[20]",
    );
    let r = run(
        &format!("{master}\n{CHILD}\n{LOG}\n{SUBS}"),
        "var M: Record \"S Master\"; M2: Record \"S Master\"; C: Record \"S Child\"; L: Codeunit Log; begin
            M.Insert(); C.Id := 1; C.Insert(); Commit();
            M2.Get('');
            asserterror M2.Rename('X');
            exit(L.Get() + GetLastErrorCode() + ';' + M2.Code + ';' + Format(M.Get('')));
        end;",
    );
    assert_eq!(r.unwrap(), "before;trigger;DB:Ok;;Yes");
}

// The return-value form raises too (:2281), and so does RecordRef.Rename (:2321).
#[test]
fn the_return_value_form_and_recordref_rename_raise() {
    for call in ["Ok := M2.Rename('X')", "RR.GetTable(M2); RR.Rename('X')"] {
        let r = run(
            &format!("{MASTER}\n{CHILD}"),
            &format!(
                "var M: Record \"S Master\"; M2: Record \"S Master\"; C: Record \"S Child\"; RR: RecordRef; Ok: Boolean; begin
                    M.Insert(); C.Id := 1; C.Insert(); Commit();
                    M2.Get('');
                    asserterror begin {call}; end;
                    exit(GetLastErrorText() + '|' + GetLastErrorCode());
                end;"
            ),
        );
        assert_eq!(r.unwrap(), format!("{SECOND}|DB:Ok"), "{call}");
    }
}

// :1400: key (A, B), the relation targets B with where(A = field(RefA)), B changes from blank.
#[test]
fn the_two_field_cell_raises() {
    let objects = "table 50120 \"S Master\" { fields { field(1; A; Code[10]) { } field(2; B; Code[10]) { } } keys { key(PK; A, B) { } } }
        table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; RefA; Code[10]) { } field(3; RefB; Code[10]) { TableRelation = \"S Master\".B where(A = field(RefA)); } } keys { key(PK; Id) { } } }";
    let body = |new: &str| {
        format!(
            "var M: Record \"S Master\"; M2: Record \"S Master\"; begin
                M.A := 'A1'; M.Insert(); Commit();
                M2.Get('A1', '');
                asserterror M2.Rename({new});
                exit(GetLastErrorText());
            end;"
        )
    };
    let r = run(objects, &body("'A1', 'X'")).unwrap();
    assert!(
        r.starts_with("The B field on the S Master table cannot be renamed from \"\" to \"X\".\\\\"),
        "{r}"
    );
    // Both key fields change: not the measured cell.
    assert!(refused(run(objects, &body("'A2', 'X'"))).contains("two-field key"));
    // Both key fields change from blank.
    let both = "var M: Record \"S Master\"; M2: Record \"S Master\"; begin
        M.Insert(); Commit();
        M2.Get('', '');
        M2.Rename('A2', 'X');
        exit('renamed');
    end;";
    assert!(refused(run(objects, both)).contains("more than one key field changes"));
}

// One fact outside the measured cells, or two measured facts at once: refused.
#[test]
fn a_rename_outside_the_measured_cells_is_refused() {
    let plain = child_with("Code[20]", PLAIN, "");
    let cases: Vec<(&str, String, String, &str, &str, &str)> = vec![
        ("two measured facts", master_with("Caption = 'T';", "Caption = 'F';", "Code[20]"), child_with("Code[10]", PLAIN, ""), "", "", "'X'"),
        ("table caption only", master_with("Caption = 'T';", "", "Code[20]"), plain.clone(), "", "", "'X'"),
        ("table extension of the renamed table", MASTER.into(), plain.clone(), "tableextension 50130 SX extends \"S Master\" { fields { field(50; X; Integer) { } } }", "", "'X'"),
        ("not per company", master_with("DataPerCompany = false;", "", "Code[20]"), plain.clone(), "", "", "'X'"),
        ("Code[10] key", master_with("", "", "Code[10]"), child_with("Code[10]", PLAIN, ""), "", "", "'X'"),
        ("Boolean key", master_with("", "", "Boolean"), child_with("Boolean", PLAIN, ""), "", "", "true"),
        ("Integer of four digits", master_with("", "", "Integer"), child_with("Integer", PLAIN, ""), "", "", "1000"),
        ("Guid with letters", master_with("", "", "Guid"), child_with("Guid", PLAIN, ""), "", "", "'{AAAAAAAA-2222-3333-4444-555555555555}'"),
        ("related field in its table's key", MASTER.into(), "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\"; } } keys { key(PK; Id, Ref) { } } }".into(), "", "", "'X'"),
        ("two arms", MASTER.into(), child_with("Code[20]", "TableRelation = if (Kind = const(1)) \"S Master\" else if (Kind = const(2)) \"S Master\";", ""), "", "", "'X'"),
        ("filter condition", MASTER.into(), child_with("Code[20]", "TableRelation = if (Kind = filter(1 | 2)) \"S Master\";", ""), "", "", "'X'"),
        ("two relations", MASTER.into(), plain.clone(), "table 50123 \"S Other\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\"; } } keys { key(PK; Id) { } } }", "", "'X'"),
        ("OnDelete on the related table", MASTER.into(), child_with("Code[20]", PLAIN, "trigger OnDelete() begin end;"), "", "", "'X'"),
        ("OnModify on the related table, to blank", MASTER.into(), child_with("Code[20]", PLAIN, "trigger OnModify() begin end;"), "", "M.Code := 'A';", "''"),
        ("a trigger that writes", master_with("trigger OnRename() var C: Record \"S Child\"; begin C.Id := 9; C.Insert(); end;", "", "Code[20]"), plain.clone(), "", "", "'X'"),
    ];
    for (name, master, child, extra, setup, new) in cases {
        let why = refused(cell(&master, &child, extra, setup, new));
        assert!(why.contains("blank or zero key value"), "{name}: {why}");
    }
}

// The record's own state outside the measured cells: refused.
#[test]
fn a_record_outside_the_measured_states_is_refused() {
    let objects = format!("{MASTER}\n{CHILD}");
    let body = |steps: &str| {
        format!(
            "var M: Record \"S Master\"; M2: Record \"S Master\"; M3: Record \"S Master\"; C: Record \"S Child\"; RR: RecordRef; Ok: Boolean; begin
                {steps}
                exit('renamed');
            end;"
        )
    };
    let cases = [
        (
            "Get of a row the open transaction wrote",
            "M.Insert(); M2.Get(''); M2.Rename('X');",
        ),
        ("own Insert, to blank", "M.Code := 'A'; M.Insert(); M.Rename('');"),
        (
            "a changed buffer",
            "M.Insert(); Commit(); M2.Get(''); M2.Name := 'n'; M2.Rename('X');",
        ),
        (
            "a stale record",
            "M.Insert(); Commit(); M2.Get(''); M3.Get(''); M3.Name := 'n'; M3.Modify(); M2.Rename('X');",
        ),
        (
            "a filter",
            "M.Insert(); Commit(); M2.Get(''); M2.SetRange(Name, 'n'); M2.Rename('X');",
        ),
        (
            "RecordRef with the return value",
            "M.Insert(); Commit(); M2.Get(''); RR.GetTable(M2); Ok := RR.Rename('X');",
        ),
    ];
    for (name, steps) in cases {
        let why = refused(run(&objects, &body(steps)));
        assert!(why.contains("blank or zero key value"), "{name}: {why}");
    }
}

// Outside the rule main's rename and main's errors stay.
#[test]
fn a_rename_outside_the_rule_keeps_mains_result() {
    let plain = format!("{MASTER}\n{CHILD}");
    let lone = MASTER.to_string();
    let soft = format!(
        "{MASTER}\n{}",
        child_with(
            "Code[20]",
            "TableRelation = \"S Master\"; ValidateTableRelation = false;",
            ""
        )
    );
    let flow = format!(
        "{MASTER}\n{}",
        child_with("Code[20]", "FieldClass = FlowFilter; TableRelation = \"S Master\";", "")
    );
    // Base Application's "Sales Invoice Entity Aggregate" shape (:2495): the relation targets a non-key field.
    let inv = "table 50120 \"S Master\" { fields { field(1; No; Code[20]) { } field(2; Posted; Boolean) { } field(3; Id; Code[20]) { } } keys { key(PK; No, Posted) { } } }
        table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\".Id; } } keys { key(PK; Id) { } } }";
    // :2610: the relation targets key field A while B changes from blank.
    let pair = "table 50120 \"S Master\" { fields { field(1; A; Code[10]) { } field(2; B; Code[10]) { } } keys { key(PK; A, B) { } } }
        table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[10]) { TableRelation = \"S Master\".A; } } keys { key(PK; Id) { } } }";
    let cases: [(&str, &str, &str, &str); 9] = [
        // :1572
        (
            "a temporary record",
            &plain,
            "var T: Record \"S Master\" temporary; C: Record \"S Child\"; begin C.Id := 1; C.Insert(); T.Insert(); T.Rename('X'); exit(T.Code); end;",
            "X",
        ),
        // :1607
        (
            "no relation",
            &lone,
            "var M: Record \"S Master\"; begin M.Insert(); M.Rename('X'); exit(M.Code); end;",
            "X",
        ),
        // :1305
        (
            "non-blank to non-blank",
            &plain,
            "var M: Record \"S Master\"; C: Record \"S Child\"; begin M.Code := 'A'; M.Insert(); C.Id := 1; C.Ref := 'A'; C.Insert(); M.Rename('X'); C.Get(1); exit(C.Ref); end;",
            "X",
        ),
        // :1859
        (
            "ValidateTableRelation = false",
            &soft,
            "var M: Record \"S Master\"; C: Record \"S Child\"; begin M.Insert(); C.Id := 1; C.Insert(); M.Rename('X'); exit(M.Code); end;",
            "X",
        ),
        // :2422
        (
            "a FlowFilter relation",
            &flow,
            "var M: Record \"S Master\"; C: Record \"S Child\"; begin M.Insert(); C.Id := 1; C.Insert(); M.Rename('X'); exit(M.Code); end;",
            "X",
        ),
        (
            "a Boolean key field beside a relation to a non-key field",
            inv,
            "var M: Record \"S Master\"; begin M.No := 'A'; M.Insert(); M.Rename('A', true); exit(Format(M.Posted)); end;",
            "Yes",
        ),
        (
            "a relation to key field A while B leaves blank",
            pair,
            "var M: Record \"S Master\"; begin M.A := 'A1'; M.Insert(); M.Rename('A1', 'X'); exit(M.B); end;",
            "X",
        ),
        // :2178
        (
            "the new key is stored",
            &plain,
            "var M: Record \"S Master\"; N: Record \"S Master\"; C: Record \"S Child\"; begin M.Insert(); N.Code := 'X'; N.Insert(); C.Id := 1; C.Insert(); asserterror M.Rename('X'); exit(GetLastErrorCode()); end;",
            "DB:RecordExists",
        ),
        // :2229
        (
            "the old row is not stored",
            &plain,
            "var M: Record \"S Master\"; C: Record \"S Child\"; begin C.Id := 1; C.Insert(); asserterror M.Rename('X'); exit(GetLastErrorCode()); end;",
            "DB:RecordNotFound",
        ),
    ];
    for (name, objects, body, want) in cases {
        take_hits();
        match run(objects, body) {
            Ok(got) => assert_eq!(got, want, "{name}"),
            other => panic!("{name}: {other:?}"),
        }
        assert!(!take_hits().contains(&Assumption::RenameBlankKeyWithRelation), "{name}");
    }
}
