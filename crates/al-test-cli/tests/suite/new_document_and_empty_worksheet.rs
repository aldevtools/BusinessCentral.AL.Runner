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
/// to it. `properties` replaces the page's properties after SourceTable.
fn refresh_sheet(properties: &str, page_code: &str, extra: &str) -> String {
    format!(
        r#"
table 69510 Row {{
    fields {{ field(1; "Entry No."; Integer) {{ }} field(2; "Code Field"; Code[20]) {{ }} field(3; "Boolean Field"; Boolean) {{ }} }}
    keys {{ key(PK; "Entry No.") {{ Clustered = true; }} }}
}}
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

/// The pinned modal Worksheet of SelectionFilterNoRow.al:346 (page 67999) with a page extension or a page subscriber keeps
/// main's refusal.
#[test]
fn empty_modal_worksheet_with_extension_code_keeps_mains_refusal() {
    let base = |extra: &str| {
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
        trigger OnAction() var Selected: Record Row; begin
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
    };
    let pinned = run_with_deps(&[], &base(""));
    assert_eq!(pinned[0].1, Outcome::Pass, "{pinned:?}");
    for extra in [
        "pageextension 69523 WsExt extends Worksheet { trigger OnAfterGetCurrRecord() begin end; }",
        r#"codeunit 69524 S {
    [EventSubscriber(ObjectType::Page, Page::Worksheet, 'OnAfterGetCurrRecordEvent', '', false, false)]
    local procedure OnCurr(var Rec: Record Row) begin end;
}"#,
    ] {
        let outcomes = run_with_deps(&[], &base(extra));
        not_run_with(&outcomes, "on no row", extra);
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
