//! Neighbor shapes of four pinned page cases (AL Runner issues 7770, 7788, 7789, 7771):
//! - testpage/TestPageBlankKeyInsert.al:256 and :272: SetValue on a variable control of an OpenNew Document inserts the
//!   header first (no part, or one empty linked ListPart);
//! - pages/currpage/SelectionFilterNoRow.al:372: Update(false) after an action fills an empty edit-mode Worksheet;
//! - pages/currpage/SelectionFilterNoRow.al:346: SetSelectionFilter on an empty modal Worksheet.
//!
//! Each shape here flips one fact of the pinned page: the new path must not run, and the case refuses or keeps main's outcome.

use super::user_reports::run_with_deps;
use al_test_cli::report::Outcome;

fn not_run_with(outcomes: &[(String, Outcome)], reason: &str, shape: &str) {
    assert!(
        matches!(&outcomes[0].1, Outcome::NotRun(why) if why.contains(reason)),
        "{shape}: {outcomes:?}"
    );
}

const NEW_DOCUMENT_REFUSAL: &str = "outside the measured OpenNew focus move";

/// The pinned Document of TestPageBlankKeyInsert.al:256 (page 67997, table 60572 "TPBK Row") with `page_code` added to the
/// page, `part` added to its content area, `table_code` added to the table and `extra` objects next to it.
fn new_document(page_code: &str, part: &str, table_code: &str, extra: &str, before: &str) -> String {
    format!(
        r#"
table 69500 Row {{
    fields {{ field(1; "No."; Code[20]) {{ }} field(2; Description; Text[100]) {{ }} }}
    keys {{ key(PK; "No.") {{ Clustered = true; }} }}
    trigger OnInsert() begin if "No." = '' then "No." := 'AUTO1'; end;
    {table_code}
}}
table 69504 Line {{
    fields {{ field(1; "Entry No."; Integer) {{ }} field(2; "Code Field"; Code[20]) {{ }} }}
    keys {{ key(PK; "Entry No.") {{ Clustered = true; }} }}
}}
page 69501 Doc {{
    PageType = Document; SourceTable = Row; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{
        group(General) {{
            field(No; Rec."No.") {{ ApplicationArea = All; }}
            field(Scan; InputText) {{ ApplicationArea = All;
                trigger OnValidate() begin Rec.Description := InputText; Rec.Modify(); end; }}
        }}
        {part}
    }} }}
    {page_code}
    var InputText: Text[20];
}}
page 69502 Lines {{
    PageType = ListPart; SourceTable = Line; ApplicationArea = All;
    layout {{ area(Content) {{ repeater(Rows) {{ field(EntryNo; Rec."Entry No.") {{ ApplicationArea = All; }} }} }} }}
}}
{extra}
codeunit 69503 T {{ Subtype = Test;
    [Test] procedure P() var Row: Record Row; Line: Record Line; Doc: TestPage Doc; begin
        {before}
        Doc.OpenNew(); Doc.Scan.Value('SCANNED');
        Row.Get('AUTO1');
        if Row.Description <> 'SCANNED' then Error('Got %1', Row.Description);
        Doc.Close();
    end;
}}
"#
    )
}

const LINKED_PART: &str = r#"part(Lines; Lines) { ApplicationArea = All; SubPageLink = "Code Field" = field("No."); }"#;

/// The pinned shapes themselves (no part, one empty linked ListPart) pass.
#[test]
fn pinned_new_document_shapes_pass() {
    for part in ["", LINKED_PART] {
        let outcomes = run_with_deps(&[], &new_document("", part, "", "", ""));
        assert_eq!(outcomes[0].1, Outcome::Pass, "{part}: {outcomes:?}");
    }
}

/// Code that the header insert would run and that the pinned page and table do not have: page triggers, page extension
/// triggers, page event subscribers, table insert subscribers and table extension triggers.
#[test]
fn new_document_insert_code_outside_the_pin_refuses() {
    let page_subscriber = r#"codeunit 69505 S {
    [EventSubscriber(ObjectType::Page, Page::Doc, 'OnInsertRecordEvent', '', false, false)]
    local procedure OnInsertRecord(var Rec: Record Row; BelowxRec: Boolean; var xRec: Record Row; var AllowInsert: Boolean) begin end;
}"#;
    let table_subscriber = r#"codeunit 69505 S {
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnBeforeInsertEvent', '', false, false)]
    local procedure OnBeforeInsert(var Rec: Record Row; RunTrigger: Boolean) begin end;
}"#;
    let page_extension = "pageextension 69506 DocExt extends Doc { trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin exit(true); end; }";
    let table_extension = "tableextension 69507 RowExt extends Row { trigger OnBeforeInsert() begin end; }";
    for (page_code, table_code, extra) in [
        (
            "trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin exit(false); end;",
            "",
            "",
        ),
        (
            "trigger OnNewRecord(BelowxRec: Boolean) begin Rec.Description := 'NEW'; end;",
            "",
            "",
        ),
        ("", "", page_extension),
        ("", "", page_subscriber),
        ("", "", table_subscriber),
        ("", "", table_extension),
        ("", "trigger OnModify() begin end;", ""),
    ] {
        for part in ["", LINKED_PART] {
            let outcomes = run_with_deps(&[], &new_document(page_code, part, table_code, extra, ""));
            not_run_with(
                &outcomes,
                NEW_DOCUMENT_REFUSAL,
                &format!("{page_code}{table_code}{extra} {part}"),
            );
        }
    }
}

/// Part shapes next to the pinned empty linked ListPart: part triggers of its own, of a page extension and of subscribers, a
/// FactBox part, a part with SourceTableView, a header with a composite key, and lines already stored for the new key.
#[test]
fn new_document_part_shapes_outside_the_pin_refuse() {
    let header_composite = r#"table 69500 Row {
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[100]) { } field(3; Kind; Integer) { } }
    keys { key(PK; "No.", Kind) { Clustered = true; } }
    trigger OnInsert() begin if "No." = '' then "No." := 'AUTO1'; end;
}"#;
    let pinned = new_document("", LINKED_PART, "", "", "");
    let shapes = [
        pinned.replace(
            "layout { area(Content) { repeater(Rows)",
            "trigger OnAfterGetRecord() begin end; layout { area(Content) { repeater(Rows)",
        ),
        new_document(
            "",
            LINKED_PART,
            "",
            "pageextension 69506 LinesExt extends Lines { trigger OnAfterGetCurrRecord() begin end; }",
            "",
        ),
        new_document(
            "",
            LINKED_PART,
            "",
            r#"codeunit 69505 S {
    [EventSubscriber(ObjectType::Page, Page::Lines, 'OnAfterGetCurrRecordEvent', '', false, false)]
    local procedure OnCurr(var Rec: Record Line) begin end;
}"#,
            "",
        ),
        pinned.replace(
            &format!("        {LINKED_PART}\n    }} }}"),
            &format!("    }} area(FactBoxes) {{ {LINKED_PART} }} }}"),
        ),
        pinned.replace(
            "PageType = ListPart; SourceTable = Line;",
            "PageType = ListPart; SourceTable = Line; SourceTableView = sorting(\"Entry No.\") order(descending);",
        ),
        pinned.replacen(
            &pinned[pinned.find("table 69500 Row {").unwrap()..pinned.find("table 69504 Line").unwrap()],
            &format!("{header_composite}\n"),
            1,
        ),
        new_document(
            "",
            LINKED_PART,
            "",
            "",
            "Line.\"Entry No.\" := 1; Line.\"Code Field\" := 'AUTO1'; Line.Insert();",
        ),
    ];
    for shape in shapes {
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, NEW_DOCUMENT_REFUSAL, &shape);
    }
}

/// The pinned Worksheet of SelectionFilterNoRow.al:372 (page 67996) with `page_code` added to the page and `extra` objects next
/// to it. `properties` replaces the page's properties after SourceTable. The table has the extension and the manual
/// subscriber "ALT Universal" has in the corpus run (`REFRESH_TABLE_EXTENSION`, `REFRESH_TABLE_SUBSCRIBER`).
fn refresh_sheet(properties: &str, page_code: &str, extra: &str) -> String {
    format!(
        r#"
table 69510 Row {{
    fields {{ field(1; "Entry No."; Integer) {{ }} field(2; "Code Field"; Code[20]) {{ }} field(3; "Boolean Field"; Boolean) {{ }} }}
    keys {{ key(PK; "Entry No.") {{ Clustered = true; }} }}
}}
{REFRESH_TABLE_EXTENSION}
{REFRESH_TABLE_SUBSCRIBER}
page 69511 Sheet {{
    PageType = Worksheet; SourceTable = Row; {properties} ApplicationArea = All;
    layout {{ area(Content) {{ repeater(Rows) {{ field(Id; Rec."Entry No.") {{ ApplicationArea = All; }} }} }} }}
    actions {{ area(Processing) {{
        action(ImportRow) {{ ApplicationArea = All;
            trigger OnAction() var Row: Record Row; begin
                Row."Entry No." := 7; Row."Code Field" := 'SELECTED'; Row."Boolean Field" := true; Row.Insert();
                CurrPage.Update(false);
            end; }}
        action(CheckSelection) {{ ApplicationArea = All;
            trigger OnAction() var Selected: Record Row; begin
                CurrPage.SetSelectionFilter(Selected);
                if (Selected.Count() <> 1) or not Selected.FindFirst() then Error('Got %1', Selected.Count());
                Selected.TestField("Entry No.", 7);
            end; }}
    }} }}
    trigger OnOpenPage() begin
        Rec.FilterGroup(2); Rec.SetRange("Code Field", 'SELECTED'); Rec.FilterGroup(0);
        if Rec.FindFirst() then;
    end;
    {page_code}
}}
{extra}
codeunit 69512 T {{ Subtype = Test;
    [Test] procedure P() var Row: Record Row; Sheet: TestPage Sheet; begin
        Row."Entry No." := 1; Row."Code Field" := 'HIDDEN'; Row."Boolean Field" := true; Row.Insert();
        Sheet.OpenEdit(); Sheet.ImportRow.Invoke(); Sheet.CheckSelection.Invoke(); Sheet.Close();
    end;
}}
"#
    )
}

/// The table extension of "ALT Universal" in the corpus (_fixtures/tables/ALTUniversalValidated.TableExt.al:5): one field
/// with an OnValidate.
const REFRESH_TABLE_EXTENSION: &str = r#"tableextension 69516 RowValidated extends Row { fields { field(69516; "Ext Validated"; Text[50]) { trigger OnValidate() begin end; } } }"#;

/// The subscriber of "ALT Universal" in the corpus (_fixtures/events/ALTManualTableEventSubscriber.al:28): OnAfterInsertEvent
/// in a codeunit with manual binding, which no test binds.
const REFRESH_TABLE_SUBSCRIBER: &str = r#"codeunit 69517 ManualSub { EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnAfterInsertEvent', '', false, false)]
    local procedure OnIns(var Rec: Record Row; RunTrigger: Boolean) begin end;
}"#;

const PINNED_SHEET_PROPERTIES: &str =
    r#"SourceTableView = where("Boolean Field" = const(true)); DelayedInsert = true; InsertAllowed = false;"#;

/// The pinned Worksheet passes; each shape that flips one fact keeps main's refusal of SetSelectionFilter on no row.
#[test]
fn empty_worksheet_refresh_outside_the_pin_keeps_mains_refusal() {
    let pinned = run_with_deps(&[], &refresh_sheet(PINNED_SHEET_PROPERTIES, "", ""));
    assert_eq!(pinned[0].1, Outcome::Pass, "{pinned:?}");
    let shapes = [
        refresh_sheet(
            PINNED_SHEET_PROPERTIES,
            "",
            "pageextension 69513 SheetExt extends Sheet { trigger OnAfterGetRecord() begin end; trigger OnAfterGetCurrRecord() begin end; }",
        ),
        refresh_sheet(
            PINNED_SHEET_PROPERTIES,
            "",
            r#"codeunit 69514 S {
    [EventSubscriber(ObjectType::Page, Page::Sheet, 'OnAfterGetCurrRecordEvent', '', false, false)]
    local procedure OnCurr(var Rec: Record Row) begin end;
}"#,
        ),
        refresh_sheet(
            r#"SourceTableView = where("Boolean Field" = const(true), "Code Field" = const('SELECTED')); DelayedInsert = true; InsertAllowed = false;"#,
            "",
            "",
        )
        .replace("Rec.FilterGroup(2); Rec.SetRange(\"Code Field\", 'SELECTED'); Rec.FilterGroup(0);", ""),
        refresh_sheet(&format!("{PINNED_SHEET_PROPERTIES} Editable = false;"), "", ""),
        refresh_sheet(PINNED_SHEET_PROPERTIES, "trigger OnAfterGetRecord() begin end;", ""),
        refresh_sheet(PINNED_SHEET_PROPERTIES, "", "").replace(
            "Rec.SetRange(\"Code Field\", 'SELECTED');",
            "Rec.SetFilter(\"Code Field\", 'SEL*');",
        ),
    ];
    for shape in shapes {
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, "on no row", &shape);
    }
}

/// The pinned modal Worksheet of SelectionFilterNoRow.al:346 (page 67999) with `extra` objects next to it.
fn modal_worksheet(extra: &str) -> String {
    format!(
        r#"
table 69520 Row {{
    fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[20]) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
}}
page 69521 Worksheet {{
    PageType = Worksheet; InsertAllowed = false; DelayedInsert = true; SourceTable = Row; ApplicationArea = All;
    layout {{ area(Content) {{ repeater(Rows) {{ field(Id; Rec.Id) {{ ApplicationArea = All; }} }} }} }}
    actions {{ area(Processing) {{ action(Check) {{ ApplicationArea = All;
        trigger OnAction() var Selected: Record Row; Other: Record Row; begin
            CurrPage.SetSelectionFilter(Selected);
            if Selected.GetFilter(Id) <> '0' then Error('Got %1', Selected.GetFilters());
        end; }} }} }}
    trigger OnOpenPage() begin Rec.FilterGroup(2); Rec.SetRange(Name, 'VISIBLE'); Rec.FilterGroup(0); if Rec.FindFirst() then; end;
}}
{extra}
codeunit 69522 T {{ Subtype = Test;
    [Test] [HandlerFunctions('HandleWorksheet')]
    procedure P() var Row: Record Row; begin
        Row.Id := 1; Row.Name := 'HIDDEN'; Row.Insert(); Page.RunModal(Page::Worksheet);
    end;
    [ModalPageHandler] procedure HandleWorksheet(var Window: TestPage Worksheet) begin Window.Check.Invoke(); end;
}}
"#
    )
}

/// The pinned modal Worksheet with a page extension or a page subscriber keeps main's refusal.
#[test]
fn empty_modal_worksheet_with_extension_code_keeps_mains_refusal() {
    let pinned = run_with_deps(&[], &modal_worksheet(""));
    assert_eq!(pinned[0].1, Outcome::Pass, "{pinned:?}");
    for extra in [
        "pageextension 69523 WsExt extends Worksheet { trigger OnAfterGetCurrRecord() begin end; }",
        r#"codeunit 69524 S {
    [EventSubscriber(ObjectType::Page, Page::Worksheet, 'OnAfterGetCurrRecordEvent', '', false, false)]
    local procedure OnCurr(var Rec: Record Row) begin end;
}"#,
    ] {
        let outcomes = run_with_deps(&[], &modal_worksheet(extra));
        not_run_with(&outcomes, "on no row", extra);
    }
}

/// Facts of the pinned modal Worksheet flipped one at a time keep main's refusal: a row of the view stored before the
/// selection, a destination that holds a row or another key and order, a stored row with the blank key, an OnOpenPage
/// without its `FindFirst`, and handler calls the pin does not make (a move or a read before the action, the action twice).
#[test]
fn empty_modal_worksheet_facts_outside_the_pin_keep_mains_refusal() {
    let pinned = modal_worksheet("");
    let select = "CurrPage.SetSelectionFilter(Selected);";
    let invoke = "begin Window.Check.Invoke(); end;";
    let shapes = [
        pinned.replace(
            select,
            &format!("Other.Id := 5; Other.Name := 'VISIBLE'; Other.Insert(); {select}"),
        ),
        pinned.replace(select, &format!("Selected.Get(1); {select}")),
        pinned.replace(
            select,
            &format!("Selected.SetCurrentKey(Name); Selected.Ascending(false); {select}"),
        ),
        pinned.replace("Row.Id := 1;", "Row.Id := 0;"),
        pinned.replace("if Rec.FindFirst() then;", ""),
        pinned.replace(invoke, "begin Window.First(); Window.Check.Invoke(); end;"),
        pinned.replace(
            invoke,
            "begin if Window.Id.Value() = '' then; Window.Check.Invoke(); end;",
        ),
        pinned.replace(invoke, "begin Window.Check.Invoke(); Window.Check.Invoke(); end;"),
    ];
    for shape in shapes {
        assert_ne!(shape, pinned);
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, "on no row", &shape);
    }
}

/// A Document that does not insert as you go (InsertAllowed = false, DelayedInsert = true) or is not a Document never takes
/// the new path: main's outcome stays (the control's Modify finds no row).
#[test]
fn new_document_outside_the_gate_keeps_mains_outcome() {
    let pinned = new_document("", "", "", "", "");
    for shape in [
        pinned.replace("UsageCategory = None;", "UsageCategory = None; InsertAllowed = false;"),
        pinned.replace("UsageCategory = None;", "UsageCategory = None; DelayedInsert = true;"),
        pinned.replace("PageType = Document;", "PageType = Card;"),
    ] {
        let outcomes = run_with_deps(&[], &shape);
        assert!(
            matches!(&outcomes[0].1, Outcome::Fail(why) if why.contains("does not exist")),
            "{shape}: {outcomes:?}"
        );
    }
}

/// A Document like the pinned one with `controls` in its General group, `page_code` on the page and `fields` added to the
/// table; the test types into Scan on the OpenNew draft and ends with the stored row count: `n=0` is main's outcome.
fn scan_document(controls: &str, page_code: &str, fields: &str, part: &str) -> String {
    format!(
        r#"
table 69500 Row {{
    fields {{ field(1; "No."; Code[20]) {{ }} field(2; Description; Text[100]) {{ }} {fields} }}
    keys {{ key(PK; "No.") {{ Clustered = true; }} }}
    trigger OnInsert() begin if "No." = '' then "No." := 'AUTO1'; end;
}}
table 69504 Line {{
    fields {{ field(1; "Entry No."; Integer) {{ }} field(2; "Code Field"; Code[20]) {{ }} }}
    keys {{ key(PK; "Entry No.") {{ Clustered = true; }} }}
}}
page 69501 Doc {{
    PageType = Document; SourceTable = Row; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ group(General) {{ {controls} }} {part} }} }}
    {page_code}
    var InputText: Text[20]; Seen: Text;
}}
page 69502 Lines {{
    PageType = ListPart; SourceTable = Line; ApplicationArea = All;
    layout {{ area(Content) {{ repeater(Rows) {{ field(EntryNo; Rec."Entry No.") {{ ApplicationArea = All; }} }} }} }}
}}
codeunit 69503 T {{ Subtype = Test;
    [Test] procedure P() var Row: Record Row; Doc: TestPage Doc; begin
        Doc.OpenNew(); Doc.Scan.SetValue('SCANNED');
        Error('n=%1', Row.Count());
    end;
}}
"#
    )
}

const NO_CONTROL: &str = r#"field(No; Rec."No.") { ApplicationArea = All; }"#;
const PLAIN_SCAN: &str = "field(Scan; InputText) { ApplicationArea = All; }";

/// A variable control without OnValidate, or one the test types into with no focus move from another control, keeps main's
/// outcome: no header is stored before it validates.
#[test]
fn new_document_without_the_validating_focus_move_keeps_mains_outcome() {
    let harmless_scan =
        "field(Scan; InputText) { ApplicationArea = All; trigger OnValidate() begin Seen := InputText; end; }";
    let shapes = [
        scan_document(&format!("{harmless_scan}{NO_CONTROL}"), "", "", ""),
        scan_document(&format!("{PLAIN_SCAN}{NO_CONTROL}"), "", "", ""),
        scan_document(
            &format!("{NO_CONTROL}{PLAIN_SCAN}"),
            "trigger OnOpenPage() begin end; trigger OnAfterGetCurrRecord() begin end;",
            "",
            "",
        ),
        scan_document(
            &format!("{NO_CONTROL}{PLAIN_SCAN}"),
            "",
            "field(3; Kind; Integer) { trigger OnValidate() begin end; }",
            "",
        ),
        scan_document(&format!("{NO_CONTROL}{PLAIN_SCAN}"), "", "", ""),
        scan_document(&format!("{NO_CONTROL}{PLAIN_SCAN}"), "", "", LINKED_PART),
    ];
    for shape in shapes {
        let outcomes = run_with_deps(&[], &shape);
        assert!(
            matches!(&outcomes[0].1, Outcome::Fail(why) if why.contains("n=0")),
            "{shape}: {outcomes:?}"
        );
    }
}

/// The header's primary key of two fields without a part, and rows stored before OpenNew: the pinned table has the one key
/// field "No." (TpbkRow.Table.al:16) and no row (TestPageBlankKeyInsert.al:40 `Row.DeleteAll()`). Main fails there; the
/// focus move refuses.
#[test]
fn new_document_key_and_rows_outside_the_pin_refuse() {
    let pinned = new_document("", "", "", "", "");
    let shapes = [
        pinned.replace(
            r#"field(2; Description; Text[100]) { } }
    keys { key(PK; "No.") { Clustered = true; } }"#,
            r#"field(2; Description; Text[100]) { } field(3; Kind; Integer) { } }
    keys { key(PK; "No.", Kind) { Clustered = true; } }"#,
        ),
        new_document("", "", "", "", "Row.\"No.\" := 'OLD'; Row.Insert();"),
    ];
    for shape in shapes {
        assert_ne!(shape, pinned);
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, NEW_DOCUMENT_REFUSAL, &shape);
    }
}

/// Facts of the pinned Worksheet refresh (SelectionFilterNoRow.al:372, page 67996, table "ALT Universal") flipped one at a
/// time keep main's refusal of SetSelectionFilter on no row: the table empty before the action (the pin holds the HIDDEN
/// row), an action that changes the range value, OnOpenPage left in filter group 2, a primary key of two fields, an action
/// that changes Rec's key after Update(false), and an OnOpenPage without its `FindFirst`.
#[test]
fn empty_worksheet_refresh_facts_outside_the_pin_keep_mains_refusal() {
    let pinned = refresh_sheet(PINNED_SHEET_PROPERTIES, "", "");
    let shapes = [
        pinned.replace(
            "Row.\"Entry No.\" := 1; Row.\"Code Field\" := 'HIDDEN'; Row.\"Boolean Field\" := true; Row.Insert();",
            "",
        ),
        pinned.replace(
            "Row.\"Code Field\" := 'SELECTED'; Row.\"Boolean Field\" := true; Row.Insert();",
            "Row.\"Code Field\" := 'OTHER'; Row.\"Boolean Field\" := true; Row.Insert(); Rec.FilterGroup(2); Rec.SetRange(\"Code Field\", 'OTHER'); Rec.FilterGroup(0);",
        ),
        pinned.replace(
            "Rec.SetRange(\"Code Field\", 'SELECTED'); Rec.FilterGroup(0);",
            "Rec.SetRange(\"Code Field\", 'SELECTED');",
        ),
        pinned.replace(
            "key(PK; \"Entry No.\")",
            "key(PK; \"Entry No.\", \"Boolean Field\")",
        ),
        pinned.replace(
            "CurrPage.Update(false);",
            "CurrPage.Update(false); Rec.\"Entry No.\" := 99;",
        ),
        pinned.replace("if Rec.FindFirst() then;", ""),
    ];
    for shape in shapes {
        assert_ne!(shape, pinned);
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, "on no row", &shape);
    }
}

/// The pinned Document with an OnValidate that does not modify Rec (main stores no row and answers `n=0`) keeps main's
/// outcome: the pin's OnValidate is `Rec.Description := InputText; Rec.Modify();`.
#[test]
fn new_document_onvalidate_outside_the_pin_keeps_mains_outcome() {
    let harmless_scan =
        "field(Scan; InputText) { ApplicationArea = All; trigger OnValidate() begin Seen := InputText; end; }";
    let shape = scan_document(&format!("{NO_CONTROL}{harmless_scan}"), "", "", "");
    let outcomes = run_with_deps(&[], &shape);
    assert!(
        matches!(&outcomes[0].1, Outcome::Fail(why) if why.contains("n=0")),
        "{shape}: {outcomes:?}"
    );
}

/// Calls and controls the pin does not have: a read of No before the SetValue, a second SetValue after the insert, and a
/// second field control in the General group. Main fails each (the control's Modify finds no row); the new path refuses.
#[test]
fn new_document_calls_and_controls_outside_the_pin_refuse() {
    let pinned = new_document("", "", "", "", "");
    let typed = "Doc.OpenNew(); Doc.Scan.Value('SCANNED');";
    for (shape, reason) in [
        (
            pinned.replace(typed, "Doc.OpenNew(); if Doc.No.Value() = '' then; Doc.Scan.Value('SCANNED');"),
            NEW_DOCUMENT_REFUSAL,
        ),
        (
            pinned.replace(typed, "Doc.OpenNew(); Doc.Scan.Value('SCANNED'); Doc.Scan.Value('AGAIN');"),
            "after the header of an OpenNew Document was stored",
        ),
        (
            pinned.replace(
                "field(No; Rec.\"No.\") { ApplicationArea = All; }",
                "field(No; Rec.\"No.\") { ApplicationArea = All; } field(Descr; Rec.Description) { ApplicationArea = All; }",
            ),
            NEW_DOCUMENT_REFUSAL,
        ),
    ] {
        assert_ne!(shape, pinned);
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, reason, &shape);
    }
}

/// The pinned Worksheet refresh with First before the selection (SelectionFilterNoRow.al:385) passes. A SetValue or a second
/// action after the refresh refuses (main fails or refuses there: the page shows no row). A range value with a wildcard
/// character, a table extension field without its OnValidate, a subscriber with automatic binding and a bound manual one
/// keep main's refusal of SetSelectionFilter on no row.
#[test]
fn empty_worksheet_refresh_calls_and_tables_outside_the_pin() {
    let pinned = refresh_sheet(PINNED_SHEET_PROPERTIES, "", "");
    let select = "Sheet.CheckSelection.Invoke();";
    let first = pinned.replace(select, &format!("if not Sheet.First() then Error('none'); {select}"));
    assert_eq!(run_with_deps(&[], &first)[0].1, Outcome::Pass, "{first}");
    for shape in [
        pinned.replace(select, "Sheet.Id.SetValue(7);"),
        pinned.replace(select, &format!("{select} {select}")),
    ] {
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, "after the refresh of an empty Worksheet", &shape);
    }
    for shape in [
        pinned.replace("'SELECTED'", "'SEL*'"),
        pinned.replace(" trigger OnValidate() begin end; ", " "),
        pinned.replace("EventSubscriberInstance = Manual;", ""),
        pinned
            .replace("Sheet.OpenEdit();", "BindSubscription(Sub); Sheet.OpenEdit();")
            .replace(
                "Sheet: TestPage Sheet;",
                "Sheet: TestPage Sheet; Sub: Codeunit ManualSub;",
            ),
    ] {
        assert_ne!(shape, pinned);
        let outcomes = run_with_deps(&[], &shape);
        not_run_with(&outcomes, "on no row", &shape);
    }
}

/// Two selections in the one action of the pinned modal Worksheet: the pin makes one, so the second keeps main's refusal.
#[test]
fn empty_modal_worksheet_second_selection_keeps_mains_refusal() {
    let pinned = modal_worksheet("");
    let select = "CurrPage.SetSelectionFilter(Selected);";
    let shape = pinned.replace(select, &format!("CurrPage.SetSelectionFilter(Other); {select}"));
    let outcomes = run_with_deps(&[], &shape);
    not_run_with(&outcomes, "on no row", &shape);
}
