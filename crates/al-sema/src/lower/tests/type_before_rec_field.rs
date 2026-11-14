//! The implicit Rec of a codeunit with `TableNo` exists only in its OnRun trigger (al-runner issues 7962 and 7974).
//! Corpus must-not-compile 3140 to 3157 (corpus PR 1386); the other shapes measured with alc 18.0.39.10160 (runtime 16.0,
//! features `[]`, some also `NoImplicitWith`; review of PR 7980): in a procedure of such a codeunit `Rec`, `xRec`, a bare field and a
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
    // al-runner issue 7974, corpus must-not-compile 3140 to 3152 (corpus PR 1386): in every procedure of a codeunit with `TableNo`
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
    // OnRun has the implicit Rec (bare field and `Rec.`, corpus must-not-compile 3153); a global variable named `Rec` or
    // `xRec` is that variable in a procedure (3141, 3142, their notes); a parameter `Rec` is the parameter (3152).
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
    // al-runner issue 7974 and the follow-up of 7962: `Version.ToUpper()` is AL0132 on the type (corpus must-not-compile
    // 3154, corpus PR 1386); `Database.CompanyName` and `Session.CurrentClientType` without parentheses compile (alc
    // 18.0.39.10160, no corpus case yet).
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
    // al-runner issue 7974, corpus must-not-compile 3153 (corpus PR 1386): `xRec` in OnRun is AL0161; `Rec` there compiles.
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

#[test]
fn a_member_the_version_type_has_keeps_main_result() {
    // Only a member `Version` lacks in every form is AL0132; `Version.Major()` (an instance method) and another type keep
    // main's result: no AL diagnostic.
    for p in [
        "procedure P(): Integer begin exit(Version.Major()); end;",
        "procedure P(): Text begin exit(Guid.ToUpper()); end;",
    ] {
        let src = codeunit("begin\n    end;", &format!("    {p}"));
        let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &src)], false);
        assert!(al_messages(&l).is_empty(), "{p}: {:?}", al_messages(&l));
    }
}

/// The objects of the review probes of PR 7980: a table with a field `Version`, and another table.
const VERSION_TABLES: &str = "table 50110 \"RV Ctx\"
{
    fields
    {
        field(1; \"Entry No.\"; Integer) { }
        field(2; Version; Text[30]) { }
    }
    keys { key(PK; \"Entry No.\") { } }
}
table 50126 \"RV Dir T\" { fields { field(1; \"No.\"; Integer) { } } keys { key(PK; \"No.\") { } } }";

#[test]
fn version_member_in_a_report_or_xmlport_keeps_main_result() {
    // alc 18.0.39.10160 and 17.0.34.45391 compile each of these (review of PR 7980, shapes r01, r02, r07, r08, r09, r10): the
    // Rec of a request page SourceTable supplies `Version` where the checker does not model it. Main gives no AL diagnostic.
    let shapes = [
        "report 50123 \"RV R\"\n{\n    dataset { dataitem(O; \"RV Dir T\") { column(N; \"No.\") { } } }\n    requestpage { SourceTable = \"RV Ctx\"; trigger OnOpenPage() begin if Version.ToUpper() = '' then; end; }\n}",
        "report 50123 \"RV R\"\n{\n    dataset { dataitem(O; \"RV Dir T\") { column(N; \"No.\") { } } }\n    requestpage { SourceTable = \"RV Ctx\"; layout { area(Content) { field(V; Rec.Version) { trigger OnValidate() begin if Version.ToUpper() = '' then; end; } } } }\n}",
        "xmlport 50128 \"RV X\"\n{\n    schema { textelement(Root) { } }\n    requestpage { SourceTable = \"RV Ctx\"; trigger OnOpenPage() begin if Version.ToUpper() = '' then; end; }\n}",
        "xmlport 50128 \"RV X\"\n{\n    schema { textelement(Root) { } }\n    requestpage { SourceTable = \"RV Ctx\"; }\n    trigger OnPreXmlPort() begin if Version.ToUpper() = '' then; end;\n}",
        "report 50123 \"RV R\"\n{\n    dataset { dataitem(O; \"RV Dir T\") { column(N; \"No.\") { } } }\n    requestpage { SourceTable = \"RV Ctx\"; }\n    trigger OnPreReport() begin if Version.ToUpper() = '' then; end;\n}",
        "report 50123 \"RV R\"\n{\n    dataset { dataitem(O; \"RV Dir T\") { column(N; \"No.\") { } trigger OnAfterGetRecord() begin if Version.ToUpper() = '' then; end; } }\n    requestpage { SourceTable = \"RV Ctx\"; }\n}",
    ];
    for src in shapes {
        let (ws, l) = lower_files(&[("t.al", VERSION_TABLES), ("o.al", src)], false);
        assert!(ws.diagnostics.is_empty(), "{src}: {:?}", ws.diagnostics);
        assert!(al_messages(&l).is_empty(), "{src}: {:?}", al_messages(&l));
    }
}

#[test]
fn version_member_in_a_codeunit_is_al0132() {
    // Corpus must-not-compile 3155 (a codeunit without TableNo; corpus PR 1386). The same under a `with` whose record lacks
    // the field (alc 18.0.39.10160, review of PR 7980, shape d11).
    for body in [
        "procedure P(): Text begin exit(Version.ToUpper()); end;",
        "procedure P(var O: Record \"RV Dir T\"): Text begin with O do exit(Version.ToUpper()); end;",
    ] {
        let src = format!("codeunit 50132 \"RV W\"\n{{\n    {body}\n}}");
        let (_, l) = lower_files(&[("t.al", VERSION_TABLES), ("c.al", &src)], false);
        assert_eq!(
            al_messages(&l),
            ["AL0132 'Version' does not contain a definition for 'ToUpper'"],
            "{body}"
        );
    }
}

#[test]
fn other_triggers_of_a_codeunit_with_table_no_have_no_rec() {
    // alc 18.0.39.10160 and 17.0.34.45391 (review of PR 7980, shapes t11, t11b, t12, t12b): `Rec`, a bare field and `xRec`
    // in the triggers of an Install or a TestRunner codeunit with TableNo are AL0118. Only OnRun has the implicit Rec.
    let install = |body: &str| {
        format!(
            "codeunit 50111 \"Probe Handler\"\n{{\n    Subtype = Install;\n    TableNo = \"Probe Context\";\n    trigger OnInstallAppPerCompany() begin {body} end;\n}}"
        )
    };
    let runner = |body: &str| {
        format!(
            "codeunit 50111 \"Probe Handler\"\n{{\n    Subtype = TestRunner;\n    TableNo = \"Probe Context\";\n    trigger OnRun() begin end;\n    trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; Success: Boolean) begin {body} end;\n}}"
        )
    };
    for (body, name) in [
        ("Rec.\"Entry No.\" := 1;", "Rec"),
        ("\"Entry No.\" := 1;", "\"Entry No.\""),
        ("xRec.\"Entry No.\" := 1;", "xRec"),
    ] {
        for src in [install(body), runner(body)] {
            let (_, l) = lower_files(&[("t.al", TABLE), ("c.al", &src)], false);
            assert_eq!(
                al_messages(&l),
                [format!(
                    "AL0118 The name '{name}' does not exist in the current context."
                )],
                "{src}"
            );
        }
    }
}

#[test]
fn a_declared_global_rec_of_another_type_refuses() {
    // A procedure of a codeunit with TableNo names a declared global `Rec`; lowering has one slot for it and the implicit
    // Rec of OnRun. Only a non-temporary record of the TableNo table runs there (main's shape); whether BC keeps the
    // declared variable apart from OnRun's Rec is not measured. Every other declared type refuses (review of PR 7980,
    // shape run2: a Record "RV Other" read `TableName()` = 'RV Ctx').
    let other = "table 50112 \"Probe Other\" { fields { field(1; \"Other No.\"; Integer) { } } keys { key(PK; \"Other No.\") { } } }";
    for decl in [
        "Rec: Record \"Probe Other\";",
        "Rec: Record \"Probe Context\" temporary;",
        "Rec: Integer;",
    ] {
        let src = format!(
            "codeunit 50111 \"Probe Handler\"\n{{\n    TableNo = \"Probe Context\";\n    var\n        {decl}\n    trigger OnRun() begin end;\n    procedure P(): Text begin exit(Format(Rec)); end;\n}}"
        );
        let (ws, l) = lower_files(&[("t.al", TABLE), ("o.al", other), ("c.al", &src)], false);
        assert!(ws.diagnostics.is_empty(), "{decl}: {:?}", ws.diagnostics);
        assert!(al_messages(&l).is_empty(), "{decl}: {:?}", al_messages(&l));
        let r = refusals(&l);
        assert!(r.iter().any(|r| r.contains("a global `Rec`")), "{decl}: {r:?}");
    }
    // The measured shape (corpus must-not-compile 3141, its note: a global `Rec` compiles in the procedure) keeps main's
    // result: no refusal.
    let same = "codeunit 50111 \"Probe Handler\"\n{\n    TableNo = \"Probe Context\";\n    var\n        Rec: Record \"Probe Context\";\n    trigger OnRun() begin end;\n    procedure P1(): Integer begin exit(Rec.\"Entry No.\"); end;\n}";
    let (_, l) = lower_files(&[("t.al", TABLE), ("o.al", other), ("c.al", same)], false);
    assert!(refusals(&l).is_empty(), "{:?}", refusals(&l));
}

const SYSTEM_NAMES_TABLE: &str = "table 50110 \"Probe Context\"
{
    fields
    {
        field(1; \"Entry No.\"; Integer) { }
        field(3; UserId; Code[50]) { }
        field(4; Today; Date) { }
        field(5; WorkDate; Date) { }
        field(6; CompanyName; Text[30]) { }
    }
    keys { key(PK; \"Entry No.\") { } }
}";

/// The debug form of routine `name` of the object `object` in `l`.
fn routine_ir(l: &Lowered, object: &str, name: &str) -> String {
    let o = l.program.objects.iter().find(|o| o.name == object).expect("object");
    let r = o
        .routines
        .iter()
        .find(|r| r.name.eq_ignore_ascii_case(name))
        .expect("routine");
    format!("{r:?}")
}

#[test]
fn a_field_named_like_a_system_function_read_bare_in_a_procedure_runs_under_the_assumption() {
    // In a procedure of a codeunit (no Subtype) with TableNo the bare or quoted name is the system function. Not pinned
    // yet: it runs under the assumption `tableno-procedure-bare-system-function` (corpus PR 1393,
    // codeunits/TableNoBareSystemFunction.al:337-417, :435-456) and binds the builtin, not the field.
    for (body, builtin) in [
        ("procedure P(): Text begin exit(\"UserId\"); end;", "UserId"),
        ("procedure P(): Date begin exit(\"Today\"); end;", "Today"),
        ("procedure P(): Date begin exit(\"WorkDate\"); end;", "WorkDate"),
        ("procedure P(): Text begin exit(\"CompanyName\"); end;", "CompanyName"),
        ("internal procedure P(): Text begin exit(UserId); end;", "UserId"),
        ("procedure P(): Text begin exit(UserId); end;", "UserId"),
        ("procedure P(): Text begin exit(UserId()); end;", "UserId"),
        ("procedure P(): Date begin exit(Today); end;", "Today"),
        ("procedure P(): Date begin exit(Today()); end;", "Today"),
        ("procedure P(): Date begin exit(WorkDate); end;", "WorkDate"),
        ("procedure P(): Text begin exit(CompanyName); end;", "CompanyName"),
        ("local procedure P(): Text begin exit(UserId); end;", "UserId"),
        (
            "procedure P(var Rec: Record \"Probe Context\"): Text begin exit(UserId); end;",
            "UserId",
        ),
        (
            "[EventSubscriber(ObjectType::Codeunit, Codeunit::\"Probe Handler\", 'OnProbe', '', false, false)]\n    local procedure P() begin Message(UserId); end;\n    [IntegrationEvent(false, false)]\n    procedure OnProbe() begin end;",
            "UserId",
        ),
    ] {
        let src = codeunit("begin\n    end;", &format!("    {body}"));
        let (ws, l) = lower_files(&[("t.al", SYSTEM_NAMES_TABLE), ("c.al", &src)], false);
        assert!(ws.diagnostics.is_empty(), "{body}: {:?}", ws.diagnostics);
        assert!(al_messages(&l).is_empty(), "{body}: {:?}", al_messages(&l));
        assert!(refusals(&l).is_empty(), "{body}: {:?}", refusals(&l));
        let ir = routine_ir(&l, "Probe Handler", "P");
        assert!(
            ir.contains("Assume(TableNoProcedureBareSystemFunction)"),
            "{body}: {ir}"
        );
        assert!(
            ir.contains(&format!(".{builtin}#")),
            "{body}: binds the builtin {builtin}: {ir}"
        );
        assert!(!ir.contains("Field"), "{body}: no field read: {ir}");
    }
    // OnRun keeps the field and marks nothing.
    let src = codeunit("var\n        T: Text;\n    begin\n        T := UserId;\n    end;", "");
    let (_, l) = lower_files(&[("t.al", SYSTEM_NAMES_TABLE), ("c.al", &src)], false);
    assert!(refusals(&l).is_empty(), "{:?}", refusals(&l));
    assert!(!routine_ir(&l, "Probe Handler", "OnRun").contains("TableNoProcedureBareSystemFunction"));
    // A codeunit that also declares a global `Rec` (TableNoBareSystemFunction.al:459-482): the bare and quoted names in a
    // procedure run under the assumption too.
    for body in [
        "procedure P(): Text begin exit(UserId); end;",
        "procedure P(): Text begin exit(\"UserId\"); end;",
    ] {
        let src = format!(
            "codeunit 50111 \"Probe Handler\"\n{{\n    TableNo = \"Probe Context\";\n    var\n        Rec: Record \"Probe Context\";\n    trigger OnRun() begin end;\n    {body}\n}}"
        );
        let (_, l) = lower_files(&[("t.al", SYSTEM_NAMES_TABLE), ("c.al", &src)], false);
        assert!(refusals(&l).is_empty(), "{body}: {:?}", refusals(&l));
        let ir = routine_ir(&l, "Probe Handler", "P");
        assert!(
            ir.contains("Assume(TableNoProcedureBareSystemFunction)"),
            "{body}: {ir}"
        );
        assert!(ir.contains(".UserId#"), "{body}: {ir}");
    }
}

#[test]
fn a_bare_system_function_named_like_a_field_outside_the_assumed_shapes_keeps_main_refusal() {
    // Not measured (TableNoBareSystemFunction.al:20-23 compiles them but does not run them): a trigger other than OnRun
    // (Install, TestRunner) and every routine of a codeunit with a Subtype (a test procedure of a test codeunit, a
    // procedure of an Install codeunit). Main's refusal stays.
    let install = "codeunit 50111 \"Probe Handler\"\n{\n    Subtype = Install;\n    TableNo = \"Probe Context\";\n    trigger OnInstallAppPerCompany()\n    var\n        T: Text;\n    begin\n        T := UserId;\n    end;\n}";
    let install_procedure = "codeunit 50111 \"Probe Handler\"\n{\n    Subtype = Install;\n    TableNo = \"Probe Context\";\n    procedure P(): Text begin exit(UserId); end;\n}";
    let test_procedure = "codeunit 50111 \"Probe Handler\"\n{\n    Subtype = Test;\n    TableNo = \"Probe Context\";\n    [Test]\n    procedure P()\n    var\n        T: Text;\n    begin\n        T := UserId;\n    end;\n}";
    let test_runner = "codeunit 50111 \"Probe Handler\"\n{\n    Subtype = TestRunner;\n    TableNo = \"Probe Context\";\n    trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean\n    var\n        T: Text;\n    begin\n        T := UserId;\n        exit(true);\n    end;\n}";
    for src in [install, install_procedure, test_procedure, test_runner] {
        let (_, l) = lower_files(&[("t.al", SYSTEM_NAMES_TABLE), ("c.al", src)], false);
        let r = refusals(&l);
        assert!(r.iter().any(|r| r.contains("system function `UserId`")), "{src}: {r:?}");
    }
    // A codeunit without TableNo runs the system function as on main, unmarked.
    let plain = "codeunit 50111 \"Probe Handler\"\n{\n    procedure P(): Text begin exit(UserId); end;\n}";
    let (_, l) = lower_files(&[("t.al", SYSTEM_NAMES_TABLE), ("c.al", plain)], false);
    assert!(refusals(&l).is_empty(), "{:?}", refusals(&l));
    assert!(!routine_ir(&l, "Probe Handler", "P").contains("TableNoProcedureBareSystemFunction"));
}

#[test]
fn a_declared_global_rec_is_its_own_variable_in_a_procedure_of_a_codeunit_without_subtype() {
    // TableNoBareSystemFunction.al:459-482 (corpus PR 1393, not pinned): a procedure reads the declared `Rec`, OnRun the
    // record given to Codeunit.Run. The procedure runs under `tableno-codeunit-global-rec-own-variable`.
    let src = "codeunit 50111 \"Probe Handler\"\n{\n    TableNo = \"Probe Context\";\n    trigger OnRun() begin Rec.\"Entry No.\" := 1; end;\n    var\n        Rec: Record \"Probe Context\";\n    procedure P() begin Rec.\"Entry No.\" := 2; end;\n}";
    let (_, l) = lower_files(&[("t.al", SYSTEM_NAMES_TABLE), ("c.al", src)], false);
    assert!(refusals(&l).is_empty(), "{:?}", refusals(&l));
    let o = l
        .program
        .objects
        .iter()
        .find(|o| o.name == "Probe Handler")
        .expect("object");
    let implicit = o.rec_slot.expect("Rec slot");
    let proc_ir = routine_ir(&l, "Probe Handler", "P");
    let run_ir = routine_ir(&l, "Probe Handler", "OnRun");
    assert!(
        proc_ir.contains("Assume(TableNoCodeunitGlobalRecOwnVariable)"),
        "{proc_ir}"
    );
    assert!(!proc_ir.contains(&format!("Global({implicit})")), "{proc_ir}");
    assert!(run_ir.contains(&format!("Global({implicit})")), "{run_ir}");
    assert!(!run_ir.contains("TableNoCodeunitGlobalRecOwnVariable"), "{run_ir}");
    // A codeunit with a Subtype keeps main's one slot, unmarked.
    let test_cu = src.replace("TableNo =", "Subtype = Test;\n    TableNo =");
    let (_, l) = lower_files(&[("t.al", SYSTEM_NAMES_TABLE), ("c.al", &test_cu)], false);
    let o = l
        .program
        .objects
        .iter()
        .find(|o| o.name == "Probe Handler")
        .expect("object");
    let implicit = o.rec_slot.expect("Rec slot");
    let proc_ir = routine_ir(&l, "Probe Handler", "P");
    assert!(proc_ir.contains(&format!("Global({implicit})")), "{proc_ir}");
    assert!(!proc_ir.contains("TableNoCodeunitGlobalRecOwnVariable"), "{proc_ir}");
}
