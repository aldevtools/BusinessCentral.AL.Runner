//! A Rename through a stale record (al-runner issue 7950, assumption stale-record-rename): another variable changed and
//! modified the row after this variable read it. Business Central raises DB:RecordChanged and writes nothing; the values
//! are those of corpus pull request 1370, record/table-relation/RenameBlankKeyRelation.al (`:2920`, `:2894`, probe runner
//! Mode 5 at `:887`). Only the measured shape raises it; another stale shape is refused, and a record that is not stale
//! keeps main's rename.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const MASTER: &str = "table 50120 \"S Master\" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; Code) { } } }";
const CHILD: &str = "table 50121 \"S Child\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\" where(Name = const('N')); } } keys { key(PK; Id) { } } }";

fn text(key: &str) -> String {
    format!(
        "The changes to the S Master record cannot be saved because some information on the page is not up-to-date. Close the page, reopen it, and try again.\\\\Identification fields and values:\\\\Code='{key}'|DB:RecordChanged"
    )
}

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

/// The probe's shape: the setup (`old` key, Name N, a child row relating to it) is committed; `M` reads the row by Get,
/// `{between}` runs (the probe: another variable changes Name and modifies the row), then `{rename}` runs.
/// Gives the last error's text and code, then the master rows.
fn cell_with(objects: &str, old: &str, between: &str, rename: &str) -> Result<String, Signal> {
    run_on(
        "29.0",
        objects,
        &format!(
            "var M: Record \"S Master\"; Other: Record \"S Master\"; C: Record \"S Child\"; RR: RecordRef; Rows: Text; begin
                M.Code := '{old}'; M.Name := 'N'; M.Insert(); C.Id := 1; C.Ref := '{old}'; C.Insert(); Commit();
                M.Get('{old}');
                {between}
                {rename};
                Other.Reset();
                if Other.FindSet() then repeat Rows += '[' + Other.Code + '][' + Other.Name + ']'; until Other.Next() = 0;
                exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Rows);
            end;"
        ),
    )
}

const STALE: &str = "Other.Get(M.Code); Other.Name := 'CHANGED'; Other.Modify();";

fn cell(old: &str, between: &str, rename: &str) -> Result<String, Signal> {
    cell_with(&format!("{MASTER}\n{CHILD}"), old, between, rename)
}

fn refused(r: Result<String, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// :2920 (X to Y) and :2894 (from the blank key: this error, not the blank-key error), the same on every major.
#[test]
fn a_rename_through_a_stale_record_raises_record_changed_and_writes_nothing() {
    for bc in ["27.0", "28.0", "29.0"] {
        for (old, new) in [("X", "Y"), ("", "X")] {
            take_hits();
            let body = format!(
                "var M: Record \"S Master\"; Other: Record \"S Master\"; C: Record \"S Child\"; begin
                    M.Code := '{old}'; M.Name := 'N'; M.Insert(); C.Id := 1; C.Ref := '{old}'; C.Insert(); Commit();
                    M.Get('{old}'); {STALE}
                    asserterror M.Rename('{new}');
                    exit(GetLastErrorText() + '|' + GetLastErrorCode());
                end;"
            );
            let r = run_on(bc, &format!("{MASTER}\n{CHILD}"), &body).unwrap();
            assert_eq!(r, text(old), "{bc} {old}");
            assert!(take_hits().contains(&Assumption::StaleRecordRename), "{bc} {old}");
        }
    }
    assert_eq!(
        cell("X", STALE, "asserterror M.Rename('Y')").unwrap(),
        format!("{}|[X][N]", text("X"))
    );
}

// Neighbors the probe does not measure: refused.
#[test]
fn a_stale_rename_outside_the_measured_shape_is_refused() {
    let subscriber = "codeunit 50161 Sub { [EventSubscriber(ObjectType::Table, Database::\"S Master\", 'OnBeforeRenameEvent', '', false, false)] local procedure B(var Rec: Record \"S Master\") begin end; }";
    let cases = [
        (
            "RecordRef.Rename",
            cell("X", &format!("{STALE} RR.GetTable(M);"), "asserterror RR.Rename('Y')"),
        ),
        ("return value", cell("X", STALE, "asserterror if M.Rename('Y') then")),
        (
            "Commit after the stale write",
            cell("X", &format!("{STALE} Commit();"), "asserterror M.Rename('Y')"),
        ),
        (
            "record changed since its Get",
            cell("X", &format!("{STALE} M.Name := 'Z';"), "asserterror M.Rename('Y')"),
        ),
        (
            "new key stored",
            cell(
                "X",
                &format!("{STALE} Other.Code := 'Y'; Other.Insert();"),
                "asserterror M.Rename('Y')",
            ),
        ),
        (
            "stale by Delete and Insert",
            cell(
                "X",
                "Other.Get(M.Code); Other.Delete(); Other.Insert();",
                "asserterror M.Rename('Y')",
            ),
        ),
        (
            "two Modifies",
            cell(
                "X",
                &format!("{STALE} Other.Name := 'AGAIN'; Other.Modify();"),
                "asserterror M.Rename('Y')",
            ),
        ),
        (
            "LockTable",
            cell("X", &format!("Other.LockTable(); {STALE}"), "asserterror M.Rename('Y')"),
        ),
        (
            "subscriber",
            cell_with(
                &format!("{MASTER}\n{CHILD}\n{subscriber}"),
                "X",
                STALE,
                "asserterror M.Rename('Y')",
            ),
        ),
    ];
    for (what, r) in cases {
        let why = refused(r);
        assert!(why.contains("through a stale record"), "{what}: {why}");
    }
}

// A record that is not stale keeps main's rename.
#[test]
fn a_record_that_is_not_stale_keeps_mains_rename() {
    take_hits();
    // A Modify that changes nothing leaves the row version.
    let unchanged = "Other.Get(M.Code); Other.Modify();";
    assert_eq!(cell("X", unchanged, "M.Rename('Y')").unwrap(), "||[Y][N]");
    // A record never read (no row version) renames the stored row.
    assert_eq!(
        cell("X", &format!("{STALE} M.Init(); M.Code := 'X';"), "M.Rename('Y')").unwrap(),
        "||[Y][]"
    );
    // A record that read a row inserted in the open transaction renames with its own values (corpus
    // record/filters/P68H_StaleModify.al:111-129).
    let uncommitted = format!(
        "var M: Record \"S Master\"; Other: Record \"S Master\"; begin
            M.Code := 'X'; M.Name := 'N'; M.Insert();
            M.Get('X'); {STALE}
            M.Rename('Y');
            Other.Get('Y');
            exit(Other.Name + '|' + Format(Other.Get('X')));
        end;"
    );
    assert_eq!(run_on("29.0", MASTER, &uncommitted).unwrap(), "N|No");
    assert!(!take_hits().contains(&Assumption::StaleRecordRename));
}
