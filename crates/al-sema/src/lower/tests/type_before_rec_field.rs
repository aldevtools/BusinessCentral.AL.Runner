//! A built-in type name that is also a field of the implicit Rec of a codeunit with `TableNo` (al-runner issue 7962).
//! Measured with alc 18.0.39.10160 (probe apps `ver`, `ver2` and `ver3`, runtime 16.0, features `[]` and
//! `NoImplicitWith`): in a procedure (global, local or internal) of such a codeunit, `Version.Create(..)` (also quoted), `Guid.CreateGuid()`,
//! `Text.CopyStr(..)`, `Database.CompanyName()` and `Session.ApplicationIdentifier()` call the type's static method,
//! though the table has fields of those names. In OnRun and in a page on the table (without `NoImplicitWith`) the field
//! wins; a local variable of the name wins too.
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
    // A method the type does not have keeps main's result, the field's method.
    let field_method = codeunit(
        "begin\n    end;",
        "    procedure F2(): Text\n    begin\n        exit(Version.ToUpper());\n    end;",
    );
    let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &field_method)], false);
    assert!(al_codes(&l).is_empty(), "{:?}", al_codes(&l));
}
