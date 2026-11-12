//! The reporters' own repros of al-runner issues 7820 and 7821, unchanged but for Library Assert, which a test here cannot
//! load: `Assert.AreEqual` and `Assert.IsTrue` became the same comparisons with `Error`.
//! - 7820: a ListPart with OnFindRecord on a Document host reads its rows again after its own `CurrPage.Update(true)` in a
//!   field's OnValidate with `=>`, not `=><` (crates/al-vm/src/pages/part_refresh_find_forward.rs, a flagged pass).
//! - 7821: a Worksheet with the new-row line whose Post action emptied the view and left another primary key in Rec runs
//!   the blank line three times (crates/al-vm/src/pages/post_left_new_line.rs, a flagged pass); with two rows it stays refused.

use al_testrun::Outcome;

const R7820: &str = r#"
table 50130 "Repro Order Header"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50131 "Repro Order Line"
{
    fields
    {
        field(1; "Order No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Qty; Integer) { }
    }
    keys
    {
        key(PK; "Order No.", "Line No.") { Clustered = true; }
    }
}

codeunit 50130 "Repro Read Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Clear()
    begin
        Entries := '';
    end;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Joined(): Text
    begin
        exit(Entries);
    end;
}

page 50130 "Repro Order Card"
{
    PageType = Document;
    SourceTable = "Repro Order Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "Repro Order Subform")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}

page 50131 "Repro Order Subform"
{
    PageType = ListPart;
    SourceTable = "Repro Order Line";
    ApplicationArea = All;
    AutoSplitKey = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; Editable = false; }
                field(Qty; Rec.Qty)
                {
                    ApplicationArea = All;
                    trigger OnValidate()
                    begin
                        CurrPage.Update(true);
                    end;
                }
            }
        }
    }

    trigger OnFindRecord(Which: Text): Boolean
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('F' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('C' + Format(Rec."Line No." div 10000));
    end;
}
codeunit 50140 "Repro Refresh Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OneRowSetValueOnFirst()
    begin
        CheckReads(1, 'F=>;R1;F=;R1;C1;');
    end;

    [Test]
    procedure FiveRowsSetValueOnFirst()
    begin
        CheckReads(5, 'F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;');
    end;

    local procedure CheckReads(Rows: Integer; Expected: Text)
    var
        Header: Record "Repro Order Header";
        Line: Record "Repro Order Line";
        ReadLog: Codeunit "Repro Read Log";
        OrderCard: TestPage "Repro Order Card";
        i: Integer;
    begin
        Header.DeleteAll();
        Line.DeleteAll();
        Header."No." := 'O1';
        Header.Insert();
        for i := 1 to Rows do begin
            Line."Order No." := 'O1';
            Line."Line No." := i * 10000;
            Line.Qty := i;
            Line.Insert();
        end;
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        ReadLog.Clear();
        OrderCard.Lines.Qty.SetValue(100);
        if ReadLog.Joined() <> Expected then
            Error('The part read %1, expected %2.', ReadLog.Joined(), Expected);
        OrderCard.Lines."Line No.".AssertEquals(10000);
        OrderCard.Lines.Qty.AssertEquals(100);
    end;
}
"#;

const R7821: &str = r#"
// A journal line: primary key "Batch Name", "Line No.", as table 83 "Item Journal Line" has
// "Journal Template Name", "Journal Batch Name", "Line No.".
table 50760 "Journal Post Line"
{
    fields
    {
        field(1; "Batch Name"; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; Quantity; Decimal) { }
    }
    keys
    {
        key(PK; "Batch Name", "Line No.") { Clustered = true; }
    }
}
// Collects the page triggers that run after the action.
codeunit 50761 "Journal Post Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;
}
// A Worksheet with the new-row line (AutoSplitKey, DelayedInsert, insert allowed) over batch B1, as page 99000823
// "Output Journal". Its Post action mirrors what page 99000823's Post leaves behind: codeunit 241 "Item Jnl.-Post"
// copies the page's Rec, posts (every line is deleted), then, when Find('=><') answers false, resets the copy, sets
// the batch filters in FilterGroup(2) and assigns "Line No." := 1, a key no row has, and the OnRun trigger copies
// that back into the page's Rec (Base Application/Inventory/Posting/ItemJnlPost.Codeunit.al:22, :88-96). The action
// then calls CurrPage.Update(false) (Base Application/Manufacturing/Journal/OutputJournal.Page.al:684).
page 50762 "Journal Post New Row Wks"
{
    PageType = Worksheet;
    SourceTable = "Journal Post Line";
    ApplicationArea = All;
    UsageCategory = None;
    AutoSplitKey = true;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Quantity; Rec.Quantity) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Post)
            {
                ApplicationArea = All;
                Caption = 'Post';

                trigger OnAction()
                var
                    Posted: Record "Journal Post Line";
                begin
                    Posted.Copy(Rec);
                    Rec.DeleteAll();
                    if not Posted.Find('=><') then begin
                        Posted.Reset();
                        Posted.FilterGroup(2);
                        Posted.SetRange("Batch Name", 'B1');
                        Posted.FilterGroup(0);
                        Posted."Line No." := 1;
                    end;
                    Rec.Copy(Posted);
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange("Batch Name", 'B1');
        Rec.FilterGroup(0);
    end;

    trigger OnAfterGetRecord()
    var
        JournalPostLog: Codeunit "Journal Post Log";
    begin
        JournalPostLog.Add('R:' + Format(Rec."Line No."));
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        JournalPostLog: Codeunit "Journal Post Log";
    begin
        JournalPostLog.Add('N:' + Format(Rec."Line No."));
    end;

    trigger OnAfterGetCurrRecord()
    var
        JournalPostLog: Codeunit "Journal Post Log";
    begin
        JournalPostLog.Add('C:' + Format(Rec."Line No."));
    end;
}
// The same page without the new-row line (InsertAllowed = false, no AutoSplitKey), to tell the new-row line apart
// from the row the action leaves in Rec.
page 50763 "Journal Post No New Row Wks"
{
    PageType = Worksheet;
    SourceTable = "Journal Post Line";
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Quantity; Rec.Quantity) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Post)
            {
                ApplicationArea = All;
                Caption = 'Post';

                trigger OnAction()
                var
                    Posted: Record "Journal Post Line";
                begin
                    Posted.Copy(Rec);
                    Rec.DeleteAll();
                    if not Posted.Find('=><') then begin
                        Posted.Reset();
                        Posted.FilterGroup(2);
                        Posted.SetRange("Batch Name", 'B1');
                        Posted.FilterGroup(0);
                        Posted."Line No." := 1;
                    end;
                    Rec.Copy(Posted);
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange("Batch Name", 'B1');
        Rec.FilterGroup(0);
    end;

    trigger OnAfterGetRecord()
    var
        JournalPostLog: Codeunit "Journal Post Log";
    begin
        JournalPostLog.Add('R:' + Format(Rec."Line No."));
    end;

    trigger OnAfterGetCurrRecord()
    var
        JournalPostLog: Codeunit "Journal Post Log";
    begin
        JournalPostLog.Add('C:' + Format(Rec."Line No."));
    end;
}
// The worksheet opens on batch B1 with one line; Post deletes it and copies back a record that carries the batch
// filters in FilterGroup(2) and "Line No." = 1, a key no row has, then calls CurrPage.Update(false). The test reads
// the page triggers that ran after the action, on a page with the new-row line, opened with OpenEdit and with
// Page.Run on a filtered record.
codeunit 50770 "Journal Post Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        TriggersMsg: Label 'Triggers after the action', Locked = true;
        NoLineMsg: Label 'No line is left', Locked = true;

    [Test]
    procedure NewRowPageOpenedWithPageRun()
    begin
        CheckSeen('R:10000;N:0;C:0;N:0;C:0;N:0;C:0;', PostOnNewRowPage(true));
    end;

    [Test]
    procedure NewRowPageOpenedWithOpenEdit()
    begin
        CheckSeen('R:10000;N:0;C:0;N:0;C:0;N:0;C:0;', PostOnNewRowPage(false));
    end;

    local procedure PostOnNewRowPage(WithPageRun: Boolean) Seen: Text
    var
        Line: Record "Journal Post Line";
        JournalPostLog: Codeunit "Journal Post Log";
        Worksheet: TestPage "Journal Post New Row Wks";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Batch Name" := 'B1';
        Line."Line No." := 10000;
        Line.Quantity := 300;
        Line.Insert();
        Line.SetRange("Batch Name", 'B1');
        Line.FindFirst();
        Commit();

        if WithPageRun then begin
            Worksheet.Trap();
            Page.Run(Page::"Journal Post New Row Wks", Line);
        end else
            Worksheet.OpenEdit();
        JournalPostLog.Take();
        Worksheet.Post.Invoke();
        Seen := JournalPostLog.Take();
        Worksheet.Close();
        if not Line.IsEmpty() then Error(NoLineMsg);
    end;

    local procedure CheckSeen(Expected: Text; Seen: Text)
    begin
        if Seen <> Expected then
            Error('%1: %2, expected %3', TriggersMsg, Seen, Expected);
    end;
}
// The control: the same action on the same table, on a page without the new-row line. In its own codeunit, so that
// a refusal in codeunit 50770 does not reach it.
codeunit 50771 "Journal Post Control Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        TriggersMsg: Label 'Triggers after the action', Locked = true;
        NoLineMsg: Label 'No line is left', Locked = true;

    [Test]
    procedure NoNewRowPageOpenedWithPageRun()
    begin
        CheckSeen('R:10000;C:0;', PostOnNoNewRowPage(true));
    end;

    [Test]
    procedure NoNewRowPageOpenedWithOpenEdit()
    begin
        CheckSeen('R:10000;C:0;', PostOnNoNewRowPage(false));
    end;

    local procedure PostOnNoNewRowPage(WithPageRun: Boolean) Seen: Text
    var
        Line: Record "Journal Post Line";
        JournalPostLog: Codeunit "Journal Post Log";
        Worksheet: TestPage "Journal Post No New Row Wks";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Batch Name" := 'B1';
        Line."Line No." := 10000;
        Line.Quantity := 300;
        Line.Insert();
        Line.SetRange("Batch Name", 'B1');
        Line.FindFirst();
        Commit();

        if WithPageRun then begin
            Worksheet.Trap();
            Page.Run(Page::"Journal Post No New Row Wks", Line);
        end else
            Worksheet.OpenEdit();
        JournalPostLog.Take();
        Worksheet.Post.Invoke();
        Seen := JournalPostLog.Take();
        Worksheet.Close();
        if not Line.IsEmpty() then Error(NoLineMsg);
    end;

    local procedure CheckSeen(Expected: Text; Seen: Text)
    begin
        if Seen <> Expected then
            Error('%1: %2, expected %3', TriggersMsg, Seen, Expected);
    end;
}
"#;

fn outcome_of(src: &str, codeunit: &str, test: &str) -> Outcome {
    al_testkit::outcomes_in(src, codeunit)
        .into_iter()
        .find(|(n, _)| n == test)
        .map(|(_, o)| o)
        .unwrap_or_else(|| panic!("no test {test}"))
}

#[test]
fn r7820_part_refresh_asks_find_forward() {
    for test in ["OneRowSetValueOnFirst", "FiveRowsSetValueOnFirst"] {
        let o = outcome_of(R7820, "Repro Refresh Tests", test);
        assert!(matches!(o, Outcome::Pass), "{test}: {o:?}");
    }
}

#[test]
fn r7821_new_line_journal_after_post_left_other_key() {
    for (codeunit, test) in [
        ("Journal Post Test", "NewRowPageOpenedWithPageRun"),
        ("Journal Post Test", "NewRowPageOpenedWithOpenEdit"),
        ("Journal Post Control Test", "NoNewRowPageOpenedWithPageRun"),
        ("Journal Post Control Test", "NoNewRowPageOpenedWithOpenEdit"),
    ] {
        let o = outcome_of(R7821, codeunit, test);
        assert!(matches!(o, Outcome::Pass), "{test}: {o:?}");
    }
}

/// Two rows shown before the action on the page with the new-row line: nothing pins it, so it stays refused.
#[test]
fn r7821_two_rows_on_the_new_line_page_stay_refused() {
    let src = R7821.replace(
        "        Line.Insert();\n        Line.SetRange(\"Batch Name\", 'B1');",
        "        Line.Insert();\n        Line.\"Line No.\" := 20000;\n        Line.Insert();\n        Line.SetRange(\"Batch Name\", 'B1');",
    );
    assert_ne!(src, R7821);
    let o = outcome_of(&src, "Journal Post Test", "NewRowPageOpenedWithOpenEdit");
    assert!(
        matches!(&o, Outcome::NotRun(r) if r.contains("another primary key")),
        "{o:?}"
    );
}

/// `src` with the first `from` replaced by `to`; the text must be there.
fn variant(src: &str, from: &str, to: &str) -> String {
    assert!(src.contains(from), "not in the repro: {from}");
    src.replacen(from, to, 1)
}

/// `src` with each `(from, to)` of `edits` applied in turn (`variant`).
fn variants(src: &str, edits: &[(&str, &str)]) -> String {
    edits
        .iter()
        .fold(src.to_string(), |s, (from, to)| variant(&s, from, to))
}

/// The assumption marks one test of `src` makes, and its outcome.
fn outcome_and_marks(src: &str, codeunit: &str, test: &str) -> (Outcome, Vec<al_builtins::assumptions::Assumption>) {
    let l = al_testkit::compile_apps(vec![al_testkit::app(src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    let o = outcomes
        .into_iter()
        .find(|(n, _)| n == test)
        .map(|(_, o)| o)
        .expect("the test");
    let marks = hits
        .into_iter()
        .find(|(n, _)| n == test)
        .map(|(_, m)| m)
        .unwrap_or_default();
    (o, marks)
}

#[test]
fn r7820_and_r7821_repros_are_flagged_passes() {
    use al_builtins::assumptions::Assumption::{PartUpdateFirstRowFindForward, PostLeftOtherKeyOnNewLine};
    let (o, marks) = outcome_and_marks(R7820, "Repro Refresh Tests", "FiveRowsSetValueOnFirst");
    assert!(matches!(o, Outcome::Pass), "{o:?}");
    assert!(marks.contains(&PartUpdateFirstRowFindForward), "{marks:?}");
    let (o, marks) = outcome_and_marks(R7821, "Journal Post Test", "NewRowPageOpenedWithPageRun");
    assert!(matches!(o, Outcome::Pass), "{o:?}");
    assert!(marks.contains(&PostLeftOtherKeyOnNewLine), "{marks:?}");
}

/// The 7820 repro's shape with one fact changed: the pin does not hold, the part keeps main's open read `=><` (the
/// repro's own check then fails with the log) and takes no assumption.
#[test]
fn r7820_neighbors_keep_mains_read() {
    use al_builtins::assumptions::Assumption::PartUpdateFirstRowFindForward;
    let after_get_record = "    trigger OnAfterGetRecord()\n    var\n        ReadLog: Codeunit \"Repro Read Log\";\n    begin\n        \
                            ReadLog.Add('R' + Format(Rec.\"Line No.\" div 10000));\n    end;\n";
    let after_get_curr = "    trigger OnAfterGetCurrRecord()\n    var\n        ReadLog: Codeunit \"Repro Read Log\";\n    begin\n        \
                          ReadLog.Add('C' + Format(Rec.\"Line No.\" div 10000));\n    end;\n";
    let line_table_end = "        key(PK; \"Order No.\", \"Line No.\") { Clustered = true; }\n    }\n";
    let qty_control = "                field(Qty; Rec.Qty)\n";
    let cases: Vec<(&str, String)> = vec![
        ("no OnAfterGetRecord", variant(R7820, after_get_record, "")),
        ("no OnAfterGetCurrRecord", variant(R7820, after_get_curr, "")),
        ("no AutoSplitKey", variant(R7820, "    AutoSplitKey = true;\n", "")),
        (
            "a table with OnModify",
            variant(
                R7820,
                line_table_end,
                &format!("{line_table_end}    trigger OnModify() begin end;\n"),
            ),
        ),
        (
            "a table with a procedure",
            variant(
                R7820,
                line_table_end,
                &format!("{line_table_end}    procedure Touch() begin end;\n"),
            ),
        ),
        (
            "a second field with the same OnValidate",
            variants(
                R7820,
                &[
                    (
                        "        field(3; Qty; Integer) { }\n",
                        "        field(3; Qty; Integer) { }\n        field(4; Qty2; Integer) { }\n",
                    ),
                    (
                        qty_control,
                        "                field(Qty2; Rec.Qty2)\n                {\n                    ApplicationArea = All;\n                    \
                         trigger OnValidate()\n                    begin\n                        CurrPage.Update(true);\n                    \
                         end;\n                }\n                field(Qty; Rec.Qty)\n",
                    ),
                ],
            ),
        ),
        (
            "the value typed is the one stored",
            variant(
                R7820,
                "OrderCard.Lines.Qty.SetValue(100);",
                "OrderCard.Lines.Qty.SetValue(1);",
            ),
        ),
        (
            "three rows (not measured)",
            variant(R7820, "CheckReads(5, ", "CheckReads(3, "),
        ),
        (
            "OnFindRecord logs another text",
            variant(R7820, "ReadLog.Add('F' + Which);", "ReadLog.Add('F' + Which + '');"),
        ),
    ];
    for (what, src) in cases {
        // The row count only changes the five-row test.
        let tests: &[&str] = if what.starts_with("three rows") {
            &["FiveRowsSetValueOnFirst"]
        } else {
            &["FiveRowsSetValueOnFirst", "OneRowSetValueOnFirst"]
        };
        for test in tests {
            let (o, marks) = outcome_and_marks(&src, "Repro Refresh Tests", test);
            assert!(
                matches!(&o, Outcome::Fail(m) if m.contains("read F=><")),
                "{what}, {test}: {o:?}"
            );
            assert!(!marks.contains(&PartUpdateFirstRowFindForward), "{what}, {test}");
        }
    }
}

/// A second SetValue after the pinned one: what the part asks then is not measured, so the call is refused (the first
/// one gave `=>`; main's `=><` there would mix two answers in one test).
#[test]
fn r7820_a_second_set_value_is_refused() {
    let src = variant(
        R7820,
        "        OrderCard.Lines.Qty.AssertEquals(100);\n",
        "        OrderCard.Lines.Qty.AssertEquals(100);\n        ReadLog.Clear();\n        OrderCard.Lines.Qty.SetValue(101);\n        \
         Error('second %1', ReadLog.Joined());\n",
    );
    let o = outcome_of(&src, "Repro Refresh Tests", "OneRowSetValueOnFirst");
    assert!(matches!(&o, Outcome::NotRun(_)), "{o:?}");
}

/// The 7821 repro's shape with one fact changed: the pin does not hold and the Post stays refused as on main.
#[test]
fn r7821_neighbors_stay_refused() {
    use al_builtins::assumptions::Assumption::PostLeftOtherKeyOnNewLine;
    let invoke = "        JournalPostLog.Take();\n        Worksheet.Post.Invoke();\n";
    let before_post = |call: &str| variant(R7821, invoke, &format!("        {call}\n{invoke}"));
    let cases: Vec<(&str, String)> = vec![
        (
            "no AutoSplitKey and DelayedInsert",
            variant(R7821, "    AutoSplitKey = true;\n    DelayedInsert = true;\n", ""),
        ),
        ("no DelayedInsert", variant(R7821, "    DelayedInsert = true;\n", "")),
        (
            "Quantity.SetValue before Post",
            before_post("Worksheet.Quantity.SetValue(5);"),
        ),
        ("First before Post", before_post("Worksheet.First();")),
        ("GoToRecord before Post", before_post("Worksheet.GoToRecord(Line);")),
        (
            "Filter.SetFilter before Post",
            before_post("Worksheet.Filter.SetFilter(\"Line No.\", '10000');"),
        ),
        (
            "Rec left on batch B2",
            variant(
                R7821,
                "                        Posted.SetRange(\"Batch Name\", 'B1');",
                "                        Posted.SetRange(\"Batch Name\", 'B2');",
            ),
        ),
        (
            "Rec left on line 20000",
            variant(R7821, "Posted.\"Line No.\" := 1;", "Posted.\"Line No.\" := 20000;"),
        ),
        (
            "OnOpenPage filters in group 0",
            variant(
                R7821,
                "        Rec.FilterGroup(2);\n        Rec.SetRange",
                "        Rec.FilterGroup(0);\n        Rec.SetRange",
            ),
        ),
        (
            "OnOpenPage filters B1|B3",
            variant(
                R7821,
                "        Rec.SetRange(\"Batch Name\", 'B1');",
                "        Rec.SetFilter(\"Batch Name\", 'B1|B3');",
            ),
        ),
        (
            "a second action with RunObject",
            variant(
                R7821,
                "            action(Post)\n",
                "            action(Other)\n            {\n                ApplicationArea = All;\n                RunObject = page \"Journal Post No \
                 New Row Wks\";\n            }\n            action(Post)\n",
            ),
        ),
        (
            "Post, a value on the blank line, Post again",
            variant(
                R7821,
                invoke,
                &format!("{invoke}        Worksheet.Quantity.SetValue(7);\n        Worksheet.Post.Invoke();\n"),
            ),
        ),
    ];
    for (what, src) in cases {
        for test in ["NewRowPageOpenedWithOpenEdit", "NewRowPageOpenedWithPageRun"] {
            let (o, marks) = outcome_and_marks(&src, "Journal Post Test", test);
            assert!(matches!(&o, Outcome::NotRun(_)), "{what}, {test}: {o:?}");
            let refused_before_post = matches!(&o, Outcome::NotRun(r) if r.contains("another primary key"));
            assert!(
                refused_before_post || what.starts_with("Post, a value"),
                "{what}, {test}: {o:?}"
            );
            assert!(
                !marks.contains(&PostLeftOtherKeyOnNewLine) || what.starts_with("Post, a value"),
                "{what}, {test}: {marks:?}"
            );
        }
    }
}
