//! al-runner issue 6924 (`Price Formula Test PFIFC` of an outside reporter): a TestPage on a List page with
//! DelayedInsert opens with OpenEdit, `New()`, a SetValue on a field that is not the key, then `Next()` while the
//! NotBlank primary key was never entered, then `GetValidationError()`. On BC the text contains
//! 'code must be filled in.' (the reporter's run). The pinned corpus does not measure leaving such a row, so the
//! runner refuses the step instead of answering it.

use al_test_cli::report::Outcome;
use al_testkit::outcomes_in;

fn src(key_type: &str, not_blank: bool, collect: bool, set_value: bool, step: &str) -> String {
    src_on(key_type, not_blank, true, collect, set_value, step)
}

fn src_on(key_type: &str, not_blank: bool, delayed: bool, collect: bool, set_value: bool, step: &str) -> String {
    src_with(
        key_type,
        not_blank,
        delayed,
        collect,
        set_value,
        step,
        &InsertCode::default(),
    )
}

/// AL that runs when the row is inserted: in the table, on the page, and in further objects (a table extension, a subscriber).
#[derive(Default)]
struct InsertCode {
    key: &'static str,
    table: &'static str,
    page: &'static str,
    objects: &'static str,
}

fn src_with(
    key_type: &str,
    not_blank: bool,
    delayed: bool,
    collect: bool,
    set_value: bool,
    step: &str,
    code: &InsertCode,
) -> String {
    let InsertCode {
        key,
        table,
        page,
        objects,
    } = code;
    let di = if delayed { "DelayedInsert = true;" } else { "" };
    let nb = if not_blank { "NotBlank = true;" } else { "" };
    let eb = if collect {
        "[ErrorBehavior(ErrorBehavior::Collect)]"
    } else {
        ""
    };
    let sv = if set_value { "P.DescF.SetValue('abc');" } else { "" };
    format!(
        r#"
table 99700 "LT NB Row"
{{
    fields {{ field(1; "Code"; {key_type}) {{ {nb} {key} }} field(2; Description; Text[50]) {{ }} }}
    keys {{ key(PK; "Code") {{ Clustered = true; }} }}
    {table}
}}
{objects}

page 99701 "LT NB List"
{{
    PageType = List;
    SourceTable = "LT NB Row";
    {di}
    layout {{ area(Content) {{ repeater(R) {{ field(CodeF; Rec."Code") {{ }} field(DescF; Rec.Description) {{ }} }} }} }}
    trigger OnOpenPage() begin CurrPage.Editable(true); end;
    {page}
}}

codeunit 99702 T
{{
    Subtype = Test;

    [Test]
    {eb}
    procedure LeaveNewRow()
    var
        P: TestPage "LT NB List";
        ErrorTxt: Text;
    begin
        P.OpenEdit();
        P.New();
        {sv}
        {step}
        ErrorTxt := P.GetValidationError();
        if not ErrorTxt.ToLower().Contains('code must be filled in.') then
            Error('Error is not correct: %1', ErrorTxt);
    end;
}}
"#
    )
}

fn outcome(src: &str) -> Outcome {
    let o = outcomes_in(src, "T");
    assert_eq!(o.len(), 1, "{o:?}");
    o.into_iter().next().unwrap().1
}

fn assert_refused(o: &Outcome) {
    match o {
        Outcome::NotRun(r) => assert!(r.contains("DelayedInsert"), "{r}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The reporter's shape: refused, not failed.
#[test]
fn reporter_shape_is_refused() {
    assert_refused(&outcome(&src("Code[20]", true, true, true, "P.Next();")));
}

/// The same without ErrorBehavior::Collect: refused too.
#[test]
fn without_collect_is_refused() {
    assert_refused(&outcome(&src("Code[20]", true, false, true, "P.Next();")));
}

/// Previous and Close off the same row: refused too.
#[test]
fn previous_and_close_are_refused() {
    assert_refused(&outcome(&src("Code[20]", true, true, true, "P.Previous();")));
    assert_refused(&outcome(&src("Code[20]", true, true, true, "P.Close(); exit;")));
}

/// An Integer key with NotBlank: refused as the Code key is.
#[test]
fn integer_key_is_refused() {
    assert_refused(&outcome(&src("Integer", true, true, true, "P.Next();")));
}

fn assert_main_kept(o: &Outcome) {
    // Main inserts the row and GetValidationError reads an empty text: the check in the test fails, nothing refuses.
    assert!(
        matches!(o, Outcome::Fail(m) if m.contains("Error is not correct")),
        "{o:?}"
    );
}

/// Neighbors outside the refused shape keep main's outcome: a key field without NotBlank, a page without DelayedInsert (the
/// row is inserted at the SetValue), a key that was typed, and a new row nothing was typed on.
#[test]
fn neighbors_keep_main_outcome() {
    assert_main_kept(&outcome(&src("Code[20]", false, true, true, "P.Next();")));
    assert_main_kept(&outcome(&src_on("Code[20]", true, false, true, true, "P.Next();")));
    assert_main_kept(&outcome(&src(
        "Code[20]",
        true,
        true,
        true,
        "P.CodeF.SetValue('A'); P.Next();",
    )));
    assert_main_kept(&outcome(&src("Code[20]", true, true, false, "P.Next();")));
}

fn with_code(key_type: &str, code: InsertCode) -> Outcome {
    outcome(&src_with(key_type, true, true, true, true, "P.Next();", &code))
}

/// Insert-time code that can fill the key keeps main's outcome: the table's OnInsert, a table extension's OnBeforeInsert or
/// OnInsert, a subscriber to the table's OnBeforeInsertEvent or OnAfterInsertEvent, the page's OnInsertRecord, and a database
/// insert trigger the setup turns on.
#[test]
fn insert_time_code_keeps_main_outcome() {
    assert_main_kept(&with_code(
        "Code[20]",
        InsertCode {
            table: "trigger OnInsert() begin if \"Code\" = '' then \"Code\" := 'AUTO'; end;",
            ..Default::default()
        },
    ));
    for trigger in ["OnBeforeInsert", "OnInsert"] {
        let objects: &'static str = Box::leak(
            format!(
                "tableextension 99703 \"LT NB Ext\" extends \"LT NB Row\" {{ trigger {trigger}() begin if Rec.\"Code\" = '' then Rec.\"Code\" := 'AUTO'; end; }}"
            )
            .into_boxed_str(),
        );
        assert_main_kept(&with_code(
            "Code[20]",
            InsertCode {
                objects,
                ..Default::default()
            },
        ));
    }
    for event in ["OnBeforeInsertEvent", "OnAfterInsertEvent"] {
        let objects: &'static str = Box::leak(
            format!(
                "codeunit 99704 \"LT NB Sub\" {{ [EventSubscriber(ObjectType::Table, Database::\"LT NB Row\", '{event}', '', false, false)] \
                 local procedure Fill(var Rec: Record \"LT NB Row\"; RunTrigger: Boolean) begin if Rec.\"Code\" = '' then Rec.\"Code\" := 'AUTO'; end; }}"
            )
            .into_boxed_str(),
        );
        assert_main_kept(&with_code(
            "Code[20]",
            InsertCode {
                objects,
                ..Default::default()
            },
        ));
    }
    assert_main_kept(&with_code(
        "Code[20]",
        InsertCode {
            page: "trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin if Rec.\"Code\" = '' then Rec.\"Code\" := 'AUTO'; end;",
            ..Default::default()
        },
    ));
    assert_main_kept(&with_code(
        "Code[20]",
        InsertCode {
            objects: "codeunit 99705 \"LT NB Setup\" { \
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)] \
                local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) \
                begin if TableId = Database::\"LT NB Row\" then OnDatabaseInsert := true; end; }",
            ..Default::default()
        },
    ));
}

/// An AutoIncrement key the platform fills keeps main's outcome.
#[test]
fn auto_increment_key_keeps_main_outcome() {
    assert_main_kept(&with_code(
        "Integer",
        InsertCode {
            key: "AutoIncrement = true;",
            ..Default::default()
        },
    ));
}

/// A database insert trigger the setup leaves off for this table does not keep main: still refused.
#[test]
fn database_trigger_off_is_refused() {
    assert_refused(&with_code(
        "Code[20]",
        InsertCode {
            objects: "codeunit 99705 \"LT NB Setup\" { \
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)] \
                local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) \
                begin if TableId = 18 then OnDatabaseInsert := true; end; }",
            ..Default::default()
        },
    ));
}
