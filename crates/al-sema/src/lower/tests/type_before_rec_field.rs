//! The implicit Rec of a codeunit with `TableNo` exists only in its OnRun trigger (al-runner issues 7962 and 7974).
//! Measured with alc 18.0.39.10160 (probe apps `ver`, `ver2`, `ver3`, `n6`, `n9proc`, `n10procs`, `a1` to `a4`, runtime
//! 16.0, features `[]`, `ver*` also `NoImplicitWith`): in a procedure of such a codeunit `Rec`, `xRec`, a bare field and a
//! bare Record method are AL0118, so a built-in type name the table also has as a field (`Version.Create(..)`, also quoted,
//! `Guid.CreateGuid()`, `Text.CopyStr(..)`, `Database.CompanyName()`, `Session.ApplicationIdentifier()`) is the type.
//! In OnRun and in a page on the table (without `NoImplicitWith`) the field wins; a variable or procedure of the name wins.
use super::*;

const TABLE: &str = "table 50110 \"Probe Context\"
{
    fields
    {
        field(1; \"Entry No.\"; Integer) { }
        field(2; Version; Text[30]) { }
        field(3; Guid; Text[30]) { }
        field(4; Text; Text[30]) { }
        field(5; Database; Text[30]) { }
        field(6; Session; Text[30]) { }
    }
    keys
    {
        key(PK; \"Entry No.\") { }
    }
}";

fn codeunit(on_run: &str, procs: &str) -> String {
    format!(
        "codeunit 50111 \"Probe Handler\"\n{{\n    TableNo = \"Probe Context\";\n\n    trigger OnRun()\n    {on_run}\n\n{procs}\n}}"
    )
}

/// The reporter's procedure (issue 7962) and the other measured static calls.
const STATIC_CALLS: &str = "
    procedure GetVersionText(): Text
    var
        V: Version;
    begin
        V := Version.Create(2, 0, 0, 0);
        exit(V.ToText());
    end;

    local procedure L(): Text
    var
        V: Version;
    begin
        V := \"Version\".Create(2, 0, 0, 0);
        exit(V.ToText());
    end;

    internal procedure G(): Guid
    begin
        exit(Guid.CreateGuid());
    end;

    procedure T(): Text
    begin
        exit(Text.CopyStr('abc', 1, 2));
    end;

    procedure D(): Integer
    begin
        exit(StrLen(Database.CompanyName()) + StrLen(Session.ApplicationIdentifier()));
    end;";

fn al_codes(l: &Lowered) -> Vec<&str> {
    l.al_diagnostics.iter().map(|d| d.code).collect()
}

fn refusals(l: &Lowered) -> Vec<String> {
    l.unsupported
        .iter()
        .flatten()
        .flatten()
        .map(|r| r.to_string())
        .collect()
}

#[test]
fn the_type_wins_in_a_procedure_of_a_codeunit_with_table_no() {
    let src = codeunit("begin\n    end;", STATIC_CALLS);
    let (ws, l) = lower_files(&[("t.al", TABLE), ("c.al", &src)], false);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    assert!(al_codes(&l).is_empty(), "{:?}", al_codes(&l));
    // Lowered as in a codeunit without `TableNo`, where nothing hides the type: the same calls, the same refusals
    // (`Guid.CreateGuid` is not implemented in either).
    let plain = src.replace("    TableNo = \"Probe Context\";\n", "");
    let (_, without) = lower_files(&[("t.al", TABLE), ("c.al", &plain)], false);
    assert!(al_codes(&without).is_empty(), "{:?}", al_codes(&without));
    assert_eq!(refusals(&l), refusals(&without));
    assert_eq!(
        format!("{:?}", l.program.objects[1].routines),
        format!("{:?}", without.program.objects[1].routines)
    );
}

#[test]
fn the_field_keeps_winning_in_on_run_a_page_and_under_a_variable() {
    // OnRun: main's result, the field (`Text[30]` has no `Create`).
    let on_run = codeunit(
        "var\n        V: Version;\n    begin\n        V := Version.Create(2, 0, 0, 0);\n    end;",
        "",
    );
    let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &on_run)], false);
    assert_eq!(al_codes(&l), ["AL0132"]);
    // A page on the table: main's result, the field.
    let page = "page 50111 \"Probe Page\"
{
    SourceTable = \"Probe Context\";
    trigger OnOpenPage()
    var
        V: Version;
    begin
        V := Version.Create(2, 0, 0, 0);
    end;
}";
    let (_, l) = lower_files(&[("t.al", TABLE), ("p.al", page)], false);
    assert_eq!(al_codes(&l), ["AL0132"]);
    // A local variable of the name wins (alc and main).
    let local = codeunit(
        "begin\n    end;",
        "    procedure LV(): Text\n    var\n        Version: Text;\n    begin\n        exit(Version.ToUpper());\n    end;",
    );
    let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &local)], false);
    assert!(al_codes(&l).is_empty(), "{:?}", al_codes(&l));
}

#[test]
fn a_procedure_of_the_name_keeps_main_result() {
    // alc 18.0.39.10160 (review probe apps n9proc, n10procs): a procedure of the codeunit named `Version`, `Guid`,
    // `Session` or `Database` wins over the type, so the call reaches the procedure's `Text` result: AL0132.
    for (name, call) in [
        ("Version", "Version.Create(1, 0, 0, 0).ToText()"),
        ("Guid", "Format(Guid.CreateGuid())"),
        ("Session", "Session.ApplicationIdentifier()"),
        ("Database", "Database.CompanyName()"),
    ] {
        let procs = format!(
            "    procedure {name}(): Text\n    begin\n        exit('x');\n    end;\n\n    procedure P(): Text\n    begin\n        exit({call});\n    end;"
        );
        let src = codeunit("begin\n    end;", &procs);
        let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &src)], false);
        assert_eq!(al_codes(&l), ["AL0132"], "{name}");
    }
}

fn al_messages(l: &Lowered) -> Vec<String> {
    l.al_diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn a_procedure_of_a_codeunit_with_table_no_has_no_rec() {
    // al-runner issue 7974, alc 18.0.39.10160 (probe apps a1 and a2): in every procedure of a codeunit with `TableNo`
    // (global, local, internal, an event subscriber, a test procedure) `Rec`, `xRec`, a bare field and a bare Record
    // method are AL0118, also next to a parameter named `Rec`.
    let cases = [
        (
            "procedure P(): Integer begin exit(\"Entry No.\"); end;",
            "\"Entry No.\"",
        ),
        ("procedure P(): Integer begin exit(Rec.\"Entry No.\"); end;", "Rec"),
        ("procedure P(): Integer begin exit(xRec.\"Entry No.\"); end;", "xRec"),
        ("procedure P() begin Find(); end;", "Find"),
        ("procedure P() begin Rec.Find(); end;", "Rec"),
        ("procedure P() begin Modify(); end;", "Modify"),
        (
            "local procedure P(): Integer begin exit(\"Entry No.\"); end;",
            "\"Entry No.\"",
        ),
        (
            "internal procedure P(): Integer begin exit(\"Entry No.\"); end;",
            "\"Entry No.\"",
        ),
        (
            "[EventSubscriber(ObjectType::Codeunit, Codeunit::\"Probe Handler\", 'OnE', '', false, false)]\n    local procedure S() begin \"Entry No.\" := 3; end;\n    [IntegrationEvent(false, false)]\n    procedure OnE() begin end;",
            "\"Entry No.\"",
        ),
        (
            "procedure P(var Rec: Record \"Probe Context\") begin Rec.\"Entry No.\" := 1; \"Entry No.\" := 2; end;",
            "\"Entry No.\"",
        ),
    ];
    for (procs, name) in cases {
        let src = codeunit("begin\n    end;", &format!("    {procs}"));
        let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &src)], false);
        assert_eq!(
            al_messages(&l),
            [format!(
                "AL0118 The name '{name}' does not exist in the current context."
            )],
            "{procs}"
        );
    }
    let test_codeunit = "codeunit 50113 \"Probe Test\"\n{\n    Subtype = Test;\n    TableNo = \"Probe Context\";\n    trigger OnRun() begin \"Entry No.\" := 1; end;\n    [Test]\n    procedure T1() begin \"Entry No.\" := 1; end;\n}";
    let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", test_codeunit)], false);
    assert_eq!(
        al_messages(&l),
        ["AL0118 The name '\"Entry No.\"' does not exist in the current context."]
    );
}

#[test]
fn rec_in_on_run_and_a_declared_rec_keep_main_result() {
    // alc 18.0.39.10160 (probe apps a1, a3, a4): OnRun has the implicit Rec (bare field and `Rec.`); a global variable
    // named `Rec` or `xRec` is that variable in a procedure; a parameter `Rec` is the parameter.
    let shapes = [
        codeunit(
            "begin\n        \"Entry No.\" := 1;\n        Rec.\"Entry No.\" := 2;\n    end;",
            "",
        ),
        "codeunit 50111 \"Probe Handler\"\n{\n    TableNo = \"Probe Context\";\n    var\n        Rec: Record \"Probe Context\";\n    trigger OnRun() begin end;\n    procedure P1(): Integer begin exit(Rec.\"Entry No.\"); end;\n}".to_string(),
        "codeunit 50111 \"Probe Handler\"\n{\n    TableNo = \"Probe Context\";\n    var\n        xRec: Record \"Probe Context\";\n    trigger OnRun() begin end;\n    procedure P1(): Integer begin exit(xRec.\"Entry No.\"); end;\n}".to_string(),
        codeunit(
            "begin\n    end;",
            "    procedure P(var Rec: Record \"Probe Context\") begin Rec.\"Entry No.\" := 1; end;",
        ),
    ];
    for src in shapes {
        let (ws, l) = lower_files(&[("t.al", TABLE), ("c.al", &src)], false);
        assert!(ws.diagnostics.is_empty(), "{src}: {:?}", ws.diagnostics);
        assert!(al_messages(&l).is_empty(), "{src}: {:?}", al_messages(&l));
    }
}

#[test]
fn the_type_and_its_members_in_a_procedure_of_a_codeunit_with_table_no() {
    // al-runner issue 7974 and the follow-up of 7962, alc 18.0.39.10160: `Version.ToUpper()` is AL0132 on the type
    // (probe app a1); `Database.CompanyName` and `Session.CurrentClientType` without parentheses compile (probe app n6).
    let upper = codeunit(
        "begin\n    end;",
        "    local procedure P(): Text begin exit(Version.ToUpper()); end;",
    );
    let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &upper)], false);
    assert_eq!(
        al_messages(&l),
        ["AL0132 'Version' does not contain a definition for 'ToUpper'"]
    );
    let no_paren = codeunit(
        "begin\n    end;",
        "    procedure P(): Text begin exit(Database.CompanyName + Format(Session.CurrentClientType)); end;",
    );
    let (ws, l) = lower_files(&[("t.al", TABLE), ("c.al", &no_paren)], false);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    assert!(al_messages(&l).is_empty(), "{:?}", al_messages(&l));
}

#[test]
fn x_rec_in_on_run_of_a_codeunit_with_table_no_is_inaccessible() {
    // al-runner issue 7974, alc 18.0.39.10160 (probe app a1): `xRec` in OnRun is AL0161; `Rec` there compiles.
    let src = codeunit(
        "begin\n        if xRec.\"Entry No.\" = 0 then;\n        Rec.\"Entry No.\" := 2;\n    end;",
        "",
    );
    let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &src)], false);
    assert_eq!(
        al_messages(&l),
        ["AL0161 'xRec' is inaccessible due to its protection level"]
    );
}
