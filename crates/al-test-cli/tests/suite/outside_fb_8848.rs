//! The corpus probes of al-runner issues 8848 and 8849 (corpus pull request 1542, merged, not pinned yet:
//! tests/al-language/pages/currpage/ListNewLineExtraCodeCodedFieldOnValidate.al:799-853 and
//! ListNewLineExtraCodeCodedRecModify.al:594-658, measured on BC 29.1, 11 of 11 green in 3 runs), run here until a bump pins
//! them: on the page of the page-with-code shape (`al-vm` pages/code_inserted_coded.rs `CodedObjects::Extra`), the later
//! SetValue on a field with its own OnValidate, and Rec modified after its save. The probes are read from their copies in the
//! neighbor app. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{ListNewLineExtraCode, ListNewLineExtraCodeCoded};
use al_test_cli::report::Outcome;

/// The field probe, whole, as the neighbor app holds it (its first line says so).
const FIELD_PROBE: &str =
    include_str!("../../../../tests/neighbors/8848-later-setvalue-field-code-rec-modify/src/ProbeFieldOnValidate.al");

/// The Rec probe, whole, as the neighbor app holds it.
const REC_PROBE: &str =
    include_str!("../../../../tests/neighbors/8848-later-setvalue-field-code-rec-modify/src/ProbeRecModify.al");

/// The neighbors just past the cells, each with BC's answer (3 runs of 3 on BC 29.1).
const NEIGHBORS: &str =
    include_str!("../../../../tests/neighbors/8848-later-setvalue-field-code-rec-modify/src/Neighbors.al");

fn assert_flagged_pass(name: &str, r: &Outcome, marks: &[al_builtins::assumptions::Assumption]) {
    assert_eq!(*r, Outcome::Pass, "{name}");
    assert!(
        marks.contains(&ListNewLineExtraCodeCoded) && marks.contains(&ListNewLineExtraCode),
        "{name}: {marks:?}"
    );
}

/// The field's own OnValidate runs before the control's: the repro of 8848 and the field that only sets a field pass,
/// flagged. A field that writes a row of the page's table (a row inserted, the row written through another record) and one
/// that raises an error keep a refusal.
#[test]
fn issue_8848_probe_cells() {
    let o = super::outside_fb_8712::outcomes(FIELD_PROBE, "LT NT6 Tests");
    assert_eq!(o.len(), 5);
    for (name, r, marks) in &o {
        match name.as_str() {
            "NewLine_LaterSetValueFieldOnValidateInsertsARowOfThePageTable_ShowsAndSavesTheRow"
            | "NewLine_LaterSetValueFieldOnValidateModifiesTheRowThroughAnotherRecord_ShowsAndSavesTheRow"
            | "NewLine_LaterSetValueFieldOnValidateRaisesAnError_TheSetValueFails" => {
                assert!(matches!(r, Outcome::NotRun(_)), "{name}: {r:?}")
            }
            _ => assert_flagged_pass(name, r, marks),
        }
    }
}

/// After the save the row and Rec may both change: the Update(false) shows the row as stored, in each of the six cells.
#[test]
fn issue_8849_probe_cells() {
    let o = super::outside_fb_8712::outcomes(REC_PROBE, "LT NY6 Tests");
    assert_eq!(o.len(), 6);
    for (name, r, marks) in &o {
        assert_flagged_pass(name, r, marks);
    }
}

/// A control without code over the field with its own OnValidate, and a second CurrPage.SaveRecord() after the codeunit
/// modified Rec, keep a refusal. The neighbors use both probes' tables and parts, so they compile with them.
#[test]
fn issue_8848_8849_neighbors_are_refused() {
    let o = super::outside_fb_8712::outcomes(&format!("{FIELD_PROBE}\n{REC_PROBE}\n{NEIGHBORS}"), "LT NT7 Tests");
    assert_eq!(o.len(), 2);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}

/// A later SetValue on a control with its own OnValidate and a TableRelation stays refused; the same page opened, read and
/// closed with OK keeps main's outcome.
#[test]
fn issue_8848_table_relation_neighbors_keep_main_or_refuse() {
    let o = super::outside_fb_8712::outcomes(&format!("{FIELD_PROBE}\n{REC_PROBE}\n{NEIGHBORS}"), "LT NT7 Rel Tests");
    assert_eq!(o.len(), 2);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r, _)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
