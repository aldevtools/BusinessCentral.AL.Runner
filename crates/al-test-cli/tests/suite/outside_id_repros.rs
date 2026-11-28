//! The reporter's repros of al-runner issues 8078 and 8339 to 8344 (green on real BC 29.1), with a neighbor shape of each that keeps
//! main's outcome: copies in fixtures/outside-id and fixtures/outside-pk, with the stand-in for Microsoft's Library Assert. A repro
//! whose behavior no corpus case pins is refused until a probe pins it. Delete a test when its pin lands in the corpus.

use al_test_cli::report::Outcome;

fn outcomes(file: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let src = format!("{}\n{}", file, include_str!("../fixtures/outside-fb/LibraryAssert.al"));
    let o = al_testkit::outcomes_in(&src, codeunit);
    assert!(!o.is_empty());
    o
}

/// `file` with each `(from, to)` replaced; every `from` must be in it.
fn with(file: &str, edits: &[(&str, &str)]) -> String {
    let mut s = file.replace('\r', "");
    for (from, to) in edits {
        assert!(s.contains(from), "{from}");
        s = s.replace(from, to);
    }
    s
}

fn outcome_of(o: &[(String, Outcome)], test: &str) -> Outcome {
    o.iter()
        .find(|(n, _)| n.contains(test))
        .unwrap_or_else(|| panic!("{test}: {o:#?}"))
        .1
        .clone()
}

fn refused(o: &[(String, Outcome)], test: &str, why: &str) {
    let r = outcome_of(o, test);
    assert!(matches!(&r, Outcome::NotRun(w) if w.contains(why)), "{test}: {r:?}");
}

/// Issue 8078: a page extension adds an action after a `separator` of the base page; BC compiles it and the action runs.
#[test]
fn issue_8078_action_added_after_a_separator() {
    let o = outcomes(
        include_str!("../fixtures/outside-pk/Issue8078.al"),
        "Repro Separator Tests",
    );
    assert!(o.iter().all(|(_, r)| *r == Outcome::Pass), "{o:#?}");
}

const RELATION_TEXT: &str = "SetValue of a text into a Text page variable with a TableRelation";

/// The refusal of a variable control on a Card without a source table or on a request page whose relation corpus PR 1646
/// does not measure (`pages/relation_text_entry.rs`): Company's primary key is a Text field, the probe's a Code field.
const UNMEASURED_RELATION: &str =
    "SetValue of a text into a page variable with a TableRelation, in a shape corpus PR 1646 does not measure";

/// Issue 8339: SetValue of a text into a Text page variable and a request page variable with `TableRelation = Company`:
/// refused. The reporter's answer (the stored name) fits corpus PR 1646's rule, but that probe measures Code keys only.
#[test]
fn issue_8339_text_entry_into_a_related_text_control_is_refused() {
    let o = outcomes(
        include_str!("../fixtures/outside-id/Issue8339.al"),
        "ALR P CompanyRelationSetValue",
    );
    refused(
        &o,
        "PageVariableWithCompanyRelationTakesStoredName",
        UNMEASURED_RELATION,
    );
    refused(
        &o,
        "RequestPageVariableWithCompanyRelationTakesStoredName",
        UNMEASURED_RELATION,
    );
}

/// Neighbors of issue 8339 keep main's result: a Text control without a relation and a Code control with one keep the text typed.
/// The Code control is related to Company, whose key is a Text field; corpus PR 1646 measures Code keys only, and a Code
/// variable related to a Text key keeps main's path (`pages/relation_text_entry.rs`).
#[test]
fn issue_8339_neighbors_keep_the_typed_text() {
    let file = include_str!("../fixtures/outside-id/Issue8339.al");
    let no_relation = with(
        file,
        &[
            (
                "field(CompanyNameField; CompName) { Caption = 'Company Name'; TableRelation = Company; }\n        }\n    }\n    var",
                "field(CompanyNameField; CompName) { Caption = 'Company Name'; }\n        }\n    }\n    var",
            ),
            (
                "SetValue(UpperCase(CompanyName()));\n        Assert.AreEqual(CompanyName(),",
                "SetValue('Typed Text');\n        Assert.AreEqual('Typed Text',",
            ),
        ],
    );
    let code = with(
        file,
        &[
            (
                "    var\n        CompName: Text[30];\n}\n\nreport",
                "    var\n        CompName: Code[30];\n}\n\nreport",
            ),
            (
                "SetValue(UpperCase(CompanyName()));\n        Assert.AreEqual(CompanyName(),",
                "SetValue(UpperCase(CompanyName()));\n        Assert.AreEqual(UpperCase(CompanyName()),",
            ),
        ],
    );
    for src in [no_relation, code] {
        let o = outcomes(&src, "ALR P CompanyRelationSetValue");
        assert_eq!(
            outcome_of(&o, "PageVariableWithCompanyRelationTakesStoredName"),
            Outcome::Pass,
            "{o:#?}"
        );
    }
}

/// Neighbor of issue 8339: a Text page variable whose TableRelation names a table with `TableType = Temporary` (no stored row
/// can be related) keeps the typed text, as on main; the same relation to a normal table stays refused (its key is an Integer,
/// which corpus PR 1646 does not measure).
#[test]
fn issue_8339_relation_to_a_temporary_table_keeps_the_typed_text() {
    let relation_to = |table_type: &str| {
        with(
            include_str!("../fixtures/outside-id/Issue8339.al"),
            &[
                (
                    "field(CompanyNameField; CompName) { Caption = 'Company Name'; TableRelation = Company; }\n        }\n    }\n    var",
                    "field(CompanyNameField; CompName) { Caption = 'Company Name'; TableRelation = \"ALR Pick Buffer\".\"Pick Caption\" where(\"Pick Kind\" = const(1)); }\n        }\n    }\n    var",
                ),
                (
                    "SetValue(UpperCase(CompanyName()));\n        Assert.AreEqual(CompanyName(),",
                    "SetValue('Typed Text');\n        Assert.AreEqual('Typed Text',",
                ),
                (
                    "codeunit 50202 \"ALR Probe Store\"",
                    &format!(
                        "table 50290 \"ALR Pick Buffer\"\n{{\n    {table_type}\n    fields {{ field(1; ID; Integer) {{ }} field(2; \"Pick Caption\"; Text[30]) {{ }} field(3; \"Pick Kind\"; Integer) {{ }} }}\n    keys {{ key(PK; ID) {{ Clustered = true; }} }}\n}}\n\ncodeunit 50202 \"ALR Probe Store\""
                    ),
                ),
            ],
        )
    };
    let o = outcomes(&relation_to("TableType = Temporary;"), "ALR P CompanyRelationSetValue");
    assert_eq!(
        outcome_of(&o, "PageVariableWithCompanyRelationTakesStoredName"),
        Outcome::Pass,
        "{o:#?}"
    );
    let o = outcomes(&relation_to(""), "ALR P CompanyRelationSetValue");
    refused(
        &o,
        "PageVariableWithCompanyRelationTakesStoredName",
        UNMEASURED_RELATION,
    );
}

/// Issue 8340: OnNewRecord raises an error at the OpenEdit of a Card over an empty table: refused.
#[test]
fn issue_8340_error_in_on_new_record_at_open_is_refused() {
    let o = outcomes(
        include_str!("../fixtures/outside-id/Issue8340.al"),
        "ALR P OpenEditOnEmptyCard",
    );
    refused(
        &o,
        "OpenEditOnEmptyCard",
        "an error raised by OnNewRecord or OnAfterGetCurrRecord at the open",
    );
}

/// Neighbor of issue 8340: an OnNewRecord that raises no error opens as on main.
#[test]
fn issue_8340_neighbor_without_error_opens() {
    let src = with(
        include_str!("../fixtures/outside-id/Issue8340.al"),
        &[("Error('OnNewRecord ran');", "Rec.Description := 'New';")],
    );
    let o = outcomes(&src, "ALR P OpenEditOnEmptyCard");
    assert_eq!(outcome_of(&o, "OpenEditOnEmptyCard"), Outcome::Pass);
}

/// Issue 8342: an action on a row edited through the TestPage of a Worksheet with OnAfterGetCurrRecord alone: refused.
#[test]
fn issue_8342_action_after_edit_on_page_with_row_trigger_is_refused() {
    let o = outcomes(
        include_str!("../fixtures/outside-id/Issue8342.al"),
        "ALR P SaveBeforeActionCurrRec",
    );
    refused(
        &o,
        "PageWithOnAfterGetCurrRecordSavesRowBeforeAction",
        "an action on a row edited through the TestPage",
    );
}

/// Neighbor of issue 8342: the same Worksheet without a row trigger saves the edit before the action, as on main.
#[test]
fn issue_8342_neighbor_without_row_trigger_saves() {
    let src = with(
        include_str!("../fixtures/outside-id/Issue8342.al"),
        &[("    trigger OnAfterGetCurrRecord()\n    begin\n    end;\n", "")],
    );
    let o = outcomes(&src, "ALR P SaveBeforeActionCurrRec");
    assert_eq!(
        outcome_of(&o, "PageWithOnAfterGetCurrRecordSavesRowBeforeAction"),
        Outcome::Pass
    );
}

/// Issue 8343: an action moves the row of a filtered Worksheet with OnAfterGetRecord out of the filter: refused.
#[test]
fn issue_8343_refresh_after_row_left_the_view_is_refused() {
    let o = outcomes(
        include_str!("../fixtures/outside-id/Issue8343.al"),
        "ALR P RefreshAfterAction",
    );
    refused(
        &o,
        "FirstAfterActionOnPageWithOnAfterGetRecord",
        "whose filters no longer hold the row it stands on",
    );
}

/// Neighbor of issue 8343: an action that keeps the row in the filter refreshes as on main.
#[test]
fn issue_8343_neighbor_row_stays_in_view() {
    let src = with(
        include_str!("../fixtures/outside-id/Issue8343.al"),
        &[
            (
                "AllLines.ModifyAll(Processed, true);",
                "AllLines.ModifyAll(Name, 'Changed');",
            ),
            (
                "Assert.IsFalse(Worksheet.First(), 'the processed line leaves the filtered view');",
                "Assert.IsTrue(Worksheet.First(), 'the line stays in the filtered view');",
            ),
        ],
    );
    let o = outcomes(&src, "ALR P RefreshAfterAction");
    assert_eq!(
        outcome_of(&o, "FirstAfterActionOnPageWithOnAfterGetRecord"),
        Outcome::Pass
    );
}

/// Issue 8344: New() after an action that stored rows into an empty AutoSplitKey Worksheet and put Rec on a stored row: refused.
#[test]
fn issue_8344_new_after_action_positioned_rec_is_refused() {
    let o = outcomes(
        include_str!("../fixtures/outside-id/Issue8344.al"),
        "ALR P AutoSplitKeyAfterFindSet",
    );
    refused(&o, "NewLineAfterActionPositionedOnFirstRow", "left Rec on a stored row");
}

/// Neighbor of issue 8344: the same action without the FindSet keeps main's refusal.
#[test]
fn issue_8344_neighbor_without_find_set_keeps_mains_refusal() {
    let src = with(
        include_str!("../fixtures/outside-id/Issue8344.al"),
        &[(
            "                    if not Rec.IsEmpty() then\n                        Rec.FindSet();\n",
            "",
        )],
    );
    let o = outcomes(&src, "ALR P AutoSplitKeyAfterFindSet");
    refused(
        &o,
        "NewLineAfterActionPositionedOnFirstRow",
        "TestPage.New after an action stored a row behind the new-row line",
    );
}

/// Issue 8343 with OnAfterGetCurrRecord in place of OnAfterGetRecord (the issue says it fails the same way): refused.
#[test]
fn issue_8343_with_on_after_get_curr_record_alone_is_refused() {
    let src = with(
        include_str!("../fixtures/outside-id/Issue8343.al"),
        &[("trigger OnAfterGetRecord()", "trigger OnAfterGetCurrRecord()")],
    );
    let o = outcomes(&src, "ALR P RefreshAfterAction");
    refused(
        &o,
        "FirstAfterActionOnPageWithOnAfterGetRecord",
        "whose filters no longer hold the row",
    );
}

/// Neighbor of issue 8343 with OnAfterGetCurrRecord alone: the action itself changes Rec's filters and the row leaves the new
/// view. Main already finds no row there; the page keeps main's result. With OnAfterGetRecord alone main keeps the client on
/// the row: refused.
#[test]
fn issue_8343_action_that_changes_the_view_keeps_mains_result_with_on_after_get_curr_record() {
    let changes_view = |trigger: &str| {
        with(
            include_str!("../fixtures/outside-id/Issue8343.al"),
            &[
                ("trigger OnAfterGetRecord()", trigger),
                (
                    "AllLines.ModifyAll(Processed, true);",
                    "AllLines.ModifyAll(Processed, true);\n                    Rec.Reset();\n                    Rec.SetRange(\"Line No.\", 0, 20000);\n                    Rec.SetRange(Processed, false);",
                ),
            ],
        )
    };
    let o = outcomes(
        &changes_view("trigger OnAfterGetCurrRecord()"),
        "ALR P RefreshAfterAction",
    );
    assert_eq!(
        outcome_of(&o, "FirstAfterActionOnPageWithOnAfterGetRecord"),
        Outcome::Pass,
        "{o:#?}"
    );
    let o = outcomes(&changes_view("trigger OnAfterGetRecord()"), "ALR P RefreshAfterAction");
    refused(
        &o,
        "FirstAfterActionOnPageWithOnAfterGetRecord",
        "whose filters no longer hold the row",
    );
}

/// Neighbor of issue 8343: the same Worksheet without a row trigger finds no row after the action, as on main (the reporter:
/// it passes on BC 29.1).
#[test]
fn issue_8343_neighbor_without_row_trigger_keeps_mains_result() {
    let src = with(
        include_str!("../fixtures/outside-id/Issue8343.al"),
        &[("    trigger OnAfterGetRecord()\n    begin\n    end;\n", "")],
    );
    let o = outcomes(&src, "ALR P RefreshAfterAction");
    assert_eq!(
        outcome_of(&o, "FirstAfterActionOnPageWithOnAfterGetRecord"),
        Outcome::Pass
    );
}

/// Issue 8344 on a List page: New() takes a stored key there too: refused.
#[test]
fn issue_8344_on_a_list_is_refused() {
    let src = with(
        include_str!("../fixtures/outside-id/Issue8344.al"),
        &[("PageType = Worksheet;", "PageType = List;")],
    );
    let o = outcomes(&src, "ALR P AutoSplitKeyAfterFindSet");
    refused(&o, "NewLineAfterActionPositionedOnFirstRow", "left Rec on a stored row");
}

/// Issue 8339: a page extension field bound to the base page's protected Text variable (the old refusal), and SetValue of an
/// Integer into the Text variable (a value corpus PR 1646 does not measure): refused.
#[test]
fn issue_8339_base_variable_and_non_text_value_are_refused() {
    let src = format!(
        "{}\n{}",
        with(
            include_str!("../fixtures/outside-id/Issue8339.al"),
            &[(
                "    }\n    var\n        CompName: Text[30];\n}\n\nreport",
                "    }\n    protected var\n        CompName: Text[30];\n}\n\nreport"
            )],
        ),
        r#"
pageextension 50203 "ALR Probe Company Pick Ext" extends "ALR Probe Company Pick"
{
    layout { addlast(Content) { field(ExtCompanyField; CompName) { TableRelation = Company; } } }
}

codeunit 50219 "ALR P CompanyRelationMore"
{
    Subtype = Test;

    [Test]
    procedure ExtensionFieldOnBaseVariable()
    var
        CompanyPick: TestPage "ALR Probe Company Pick";
    begin
        CompanyPick.OpenEdit();
        CompanyPick.ExtCompanyField.SetValue(UpperCase(CompanyName()));
        CompanyPick.Close();
    end;

    [Test]
    procedure IntegerIntoTextVariable()
    var
        CompanyPick: TestPage "ALR Probe Company Pick";
    begin
        CompanyPick.OpenEdit();
        CompanyPick.CompanyNameField.SetValue(42);
        CompanyPick.Close();
    end;
}
"#
    );
    let o = outcomes(&src, "ALR P CompanyRelationMore");
    refused(&o, "ExtensionFieldOnBaseVariable", RELATION_TEXT);
    refused(&o, "IntegerIntoTextVariable", UNMEASURED_RELATION);
}

const OPEN_ERROR: &str = "an error raised by OnNewRecord or OnAfterGetCurrRecord at the open";

/// Issue 8340 on a List, a Worksheet and a Document page: the same error at the open over an empty table: refused.
#[test]
fn issue_8340_on_list_worksheet_and_document_is_refused() {
    let repeater = (
        "            field(\"Code\"; Rec.\"Code\") { }\n            field(Description; Rec.Description) { }\n",
        "            repeater(R)\n            {\n                field(\"Code\"; Rec.\"Code\") { }\n                field(Description; Rec.Description) { }\n            }\n",
    );
    for ty in ["List", "Worksheet"] {
        let to = format!("PageType = {ty};");
        let src = with(
            include_str!("../fixtures/outside-id/Issue8340.al"),
            &[("PageType = Card;", to.as_str()), repeater],
        );
        let o = outcomes(&src, "ALR P OpenEditOnEmptyCard");
        refused(&o, "OpenEditOnEmptyCard", OPEN_ERROR);
    }
    let src = with(
        include_str!("../fixtures/outside-id/Issue8340.al"),
        &[("PageType = Card;", "PageType = Document;")],
    );
    let o = outcomes(&src, "ALR P OpenEditOnEmptyCard");
    refused(&o, "OpenEditOnEmptyCard", OPEN_ERROR);
}
