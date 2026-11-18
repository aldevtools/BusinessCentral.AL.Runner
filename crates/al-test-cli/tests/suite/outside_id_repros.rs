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

/// Issue 8339: SetValue of a text into a Text page variable and a request page variable with `TableRelation = Company`: refused.
#[test]
fn issue_8339_text_entry_into_a_related_text_control_is_refused() {
    let o = outcomes(
        include_str!("../fixtures/outside-id/Issue8339.al"),
        "ALR P CompanyRelationSetValue",
    );
    refused(&o, "PageVariableWithCompanyRelationTakesStoredName", RELATION_TEXT);
    refused(
        &o,
        "RequestPageVariableWithCompanyRelationTakesStoredName",
        RELATION_TEXT,
    );
}

/// Neighbors of issue 8339 keep main's result: a Text control without a relation and a Code control with one keep the text typed.
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
