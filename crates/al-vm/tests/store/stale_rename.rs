//! A Rename through a stale record (al-runner issue 7950): another variable changed and modified the row after this
//! variable read it. Business Central raises DB:RecordChanged and writes nothing; the values are those of
//! record/table-relation/RenameBlankKeyRelation.al (`:2920`, `:2894`, probe runner Mode 5 at `:887`, run inside
//! `if Runner.Run()` at `:1646`). Only the measured shape raises it; another stale shape is refused, and a record that is
//! not stale keeps main's rename.

use al_builtins::assumptions::take_hits;
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

/// The probe's shape (:887, :1642-1651): the setup (`old` key, Name N, a child row relating to it) is committed; the runner
/// codeunit's OnRun, run as `if Runner.Run()`, reads the row into `M` by Get, runs `{between}` (the probe: another variable
/// changes Name and modifies the row), then `{rename}`. Gives the last error's text and code, then the master rows.
fn cell_on(bc: &str, objects: &str, old: &str, between: &str, rename: &str) -> Result<String, Signal> {
    let runner = format!(
        "codeunit 50151 R {{
            trigger OnRun() var M: Record \"S Master\"; Other: Record \"S Master\"; RR: RecordRef; begin
                M.Get(OldCode);
                {between}
                {rename};
            end;
            var OldCode: Code[20];
            procedure SetCode(c: Code[20]) begin OldCode := c; end;
        }}"
    );
    run_on(
        bc,
        &format!("{objects}\n{runner}"),
        &format!(
            "var M: Record \"S Master\"; Other: Record \"S Master\"; C: Record \"S Child\"; Runner: Codeunit R; Rows: Text;
                Res: Text; begin
                M.Code := '{old}'; M.Name := 'N'; M.Insert(); C.Id := 1; C.Ref := '{old}'; C.Insert(); Commit();
                ClearLastError();
                Runner.SetCode('{old}');
                if Runner.Run() then Res := 'ok' else Res := GetLastErrorText() + '|' + GetLastErrorCode();
                if Other.FindSet() then repeat Rows += '[' + Other.Code + '][' + Other.Name + ']'; until Other.Next() = 0;
                exit(Res + '|' + Rows);
            end;"
        ),
    )
}

fn cell_with(objects: &str, old: &str, between: &str, rename: &str) -> Result<String, Signal> {
    cell_on("29.0", objects, old, between, rename)
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
            let r = cell_on(
                bc,
                &format!("{MASTER}\n{CHILD}"),
                old,
                STALE,
                &format!("M.Rename('{new}')"),
            )
            .unwrap();
            assert_eq!(r, format!("{}|[{old}][N]", text(old)), "{bc} {old}");
            assert!(take_hits().is_empty(), "{bc} {old}");
        }
    }
}

// Neighbors the probe does not measure: refused.
#[test]
fn a_stale_rename_outside_the_measured_shape_is_refused() {
    let subscriber = "codeunit 50161 Sub { [EventSubscriber(ObjectType::Table, Database::\"S Master\", 'OnBeforeRenameEvent', '', false, false)] local procedure B(var Rec: Record \"S Master\") begin end; }";
    let cases = [
        (
            "RecordRef.Rename",
            cell("X", &format!("{STALE} RR.GetTable(M);"), "RR.Rename('Y')"),
        ),
        ("return value", cell("X", STALE, "if M.Rename('Y') then")),
        (
            "Commit after the stale write",
            cell("X", &format!("{STALE} Commit();"), "M.Rename('Y')"),
        ),
        (
            "filter",
            cell("X", &format!("{STALE} M.SetRange(Name, 'N');"), "M.Rename('Y')"),
        ),
        (
            "record changed since its Get",
            cell("X", &format!("{STALE} M.Name := 'Z';"), "M.Rename('Y')"),
        ),
        (
            "new key stored",
            cell(
                "X",
                &format!("{STALE} Other.Code := 'Y'; Other.Insert();"),
                "M.Rename('Y')",
            ),
        ),
        (
            "stale by Delete and Insert",
            cell(
                "X",
                "Other.Get(M.Code); Other.Delete(); Other.Insert();",
                "M.Rename('Y')",
            ),
        ),
        (
            "two Modifies",
            cell(
                "X",
                &format!("{STALE} Other.Name := 'AGAIN'; Other.Modify();"),
                "M.Rename('Y')",
            ),
        ),
        (
            "LockTable",
            cell("X", &format!("Other.LockTable(); {STALE}"), "M.Rename('Y')"),
        ),
        (
            "subscriber",
            cell_with(&format!("{MASTER}\n{CHILD}\n{subscriber}"), "X", STALE, "M.Rename('Y')"),
        ),
    ];
    for (what, r) in cases {
        let why = refused(r);
        assert!(why.contains("through a stale record"), "{what}: {why}");
    }
}

// The widths the probe holds fixed: the key (one Code[20] field), no Caption, the one relation to the table (a const
// where() on a field outside the key, from a per-company table of the same app with its primary key only), a Record
// Modify, and the trap (`if Runner.Run()` around the OnRun that renames, :1646). Each other value is refused.
#[test]
fn a_stale_rename_outside_the_measured_widths_is_refused() {
    let master_with = |extra: &str, field: &str| {
        format!(
            "table 50120 \"S Master\" {{ {extra} fields {{ {field} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}"
        )
    };
    let code_key = "field(1; Code; Code[20]) { }";
    let child_with = |relation: &str, keys: &str| {
        format!(
            "table 50121 \"S Child\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Ref; Code[20]) {{ TableRelation = {relation}; }} }} keys {{ key(PK; Id) {{ }} {keys} }} }}"
        )
    };
    let measured_relation = "\"S Master\" where(Name = const('N'))";
    let second_child = "table 50122 \"S Child2\" { fields { field(1; Id; Integer) { } field(2; Ref; Code[20]) { TableRelation = \"S Master\" where(Name = const('N')); } } keys { key(PK; Id) { } } }";
    let try_runner = "codeunit 50152 T { trigger OnRun() begin Try(); end;
        [TryFunction] local procedure Try() var M: Record \"S Master\"; Other: Record \"S Master\"; begin
            M.Get('X'); Other.Get('X'); Other.Name := 'CHANGED'; Other.Modify(); M.Rename('Y'); end; }";
    let cases = [
        (
            "a Code[10] key",
            cell_with(
                &format!(
                    "{}\n{}",
                    master_with("", "field(1; Code; Code[10]) { }"),
                    child_with(measured_relation, "").replace("Code[20]", "Code[10]")
                ),
                "X",
                STALE,
                "M.Rename('Y')",
            ),
        ),
        (
            "a Caption on the table",
            cell_with(
                &format!("{}\n{CHILD}", master_with("Caption = 'Cap';", code_key)),
                "X",
                STALE,
                "M.Rename('Y')",
            ),
        ),
        (
            "a Caption on the key field",
            cell_with(
                &format!(
                    "{}\n{CHILD}",
                    master_with("", "field(1; Code; Code[20]) { Caption = 'Cap'; }")
                ),
                "X",
                STALE,
                "M.Rename('Y')",
            ),
        ),
        (
            "no relation to the table",
            cell_with(
                &format!(
                    "{MASTER}\n{}",
                    CHILD.replace(" TableRelation = \"S Master\" where(Name = const('N'));", "")
                ),
                "X",
                STALE,
                "M.Rename('Y')",
            ),
        ),
        (
            "two relations to the table",
            cell_with(
                &format!("{MASTER}\n{CHILD}\n{second_child}"),
                "X",
                STALE,
                "M.Rename('Y')",
            ),
        ),
        (
            "a plain relation",
            cell_with(
                &format!("{MASTER}\n{}", child_with("\"S Master\"", "")),
                "X",
                STALE,
                "M.Rename('Y')",
            ),
        ),
        (
            "a related table with a secondary key",
            cell_with(
                &format!("{MASTER}\n{}", child_with(measured_relation, "key(K2; Ref) { }")),
                "X",
                STALE,
                "M.Rename('Y')",
            ),
        ),
        (
            "a Modify through a RecordRef",
            cell(
                "X",
                "RR.Open(Database::\"S Master\"); RR.Field(1).SetRange('X'); RR.FindFirst(); RR.Field(2).Value := 'CHANGED'; RR.Modify();",
                "M.Rename('Y')",
            ),
        ),
        (
            "asserterror around the Rename",
            run_on(
                "29.0",
                &format!("{MASTER}\n{CHILD}"),
                "var M: Record \"S Master\"; Other: Record \"S Master\"; C: Record \"S Child\"; begin
                    M.Code := 'X'; M.Name := 'N'; M.Insert(); C.Id := 1; C.Ref := 'X'; C.Insert(); Commit();
                    M.Get('X'); Other.Get('X'); Other.Name := 'CHANGED'; Other.Modify();
                    asserterror M.Rename('Y');
                    exit(GetLastErrorText());
                end;",
            ),
        ),
        (
            "a TryFunction inside the run",
            run_on(
                "29.0",
                &format!("{MASTER}\n{CHILD}\n{try_runner}"),
                "var M: Record \"S Master\"; C: Record \"S Child\"; begin
                    M.Code := 'X'; M.Name := 'N'; M.Insert(); C.Id := 1; C.Ref := 'X'; C.Insert(); Commit();
                    if Codeunit.Run(Codeunit::T) then;
                    exit(GetLastErrorText());
                end;",
            ),
        ),
    ];
    let mut bad = Vec::new();
    for (what, r) in cases {
        match r {
            Err(Signal::Unsupported(w)) if w.contains("through a stale record") => {}
            other => bad.push(format!("{what}: {other:?}")),
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

// A record that is not stale keeps main's rename.
#[test]
fn a_record_that_is_not_stale_keeps_mains_rename() {
    take_hits();
    // A Modify that changes nothing leaves the row version.
    let unchanged = "Other.Get(M.Code); Other.Modify();";
    assert_eq!(cell("X", unchanged, "M.Rename('Y')").unwrap(), "ok|[Y][N]");
    // A record never read (no row version) renames the stored row.
    assert_eq!(
        cell("X", &format!("{STALE} M.Init(); M.Code := 'X';"), "M.Rename('Y')").unwrap(),
        "ok|[Y][]"
    );
    // A record that read a row inserted in the open transaction renames with its own values (corpus
    // record/filters/P68H_StaleModify.al:108-132).
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
    assert!(take_hits().is_empty());
}
