//! Al-runner issue 8832: `TestPage.GetField` and `TestPart.GetField` take the id the compiler gives a page control (al-vm
//! pages/get_field.rs, assumption testpage-getfield-control-id). The reporter's repro, the cells of corpus pull request 1538
//! (testpage/GetFieldByControlId.al) and the neighbors past them, as the neighbor app tests/neighbors/8832-getfield-control-id
//! holds them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::TestPageGetFieldControlId;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Neighbors.al");

/// The reporter's test passes: GetField with the Name control's id reads the row's name. Main answered "The field with ID =
/// 356257777 is not found on the page."
#[test]
fn issue_8832_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/ReproTests.al")
    );
    let o = outcomes(&src, "Repro GetField Tests");
    assert!(
        o.len() == 1 && o[0].1 == Outcome::Pass && o[0].2 == [TestPageGetFieldControlId],
        "{o:#?}"
    );
}

/// Every cell of the probe passes under the entry where GetField found a control, and without it where every id is one main
/// already did not find (:484 a name of no control; :589 a page extension's field, from BC 28). Caption of what a part control's
/// id gives (:692) refuses.
#[test]
fn probe_cells_pass_where_measured() {
    let o = outcomes(PROBE, "LT GFI Tests");
    assert_eq!(o.len(), 21);
    let bad: Vec<_> = o
        .iter()
        // A field in a cuegroup reads as by its name (`cue_field_value_8845.rs`).
        .filter(|(n, ..)| !(n.starts_with("CueCard_") || n.starts_with("CueTypes")))
        .filter(|(n, r, marks)| match n.as_str() {
            // GetField does not find a hidden field (:530, :1126 `hidden`), as main answered every id.
            "List_IdsOfNoField" | "Card_HiddenControls" => *r != Outcome::Pass || !marks.is_empty(),
            // Compiled without a BC version symbol, the cell expects BC 27's answer (`#else` of BC28PLUS), and the runtime
            // answers as BC 29 does without a target; the neighbor check runs it on each major.
            "Card_ExtensionControl" => !matches!(r, Outcome::Fail(m) if m.contains("Expected:<x1>")) || !marks.is_empty(),
            "Card_PartIdCaptionAndVisible" => {
                !matches!(r, Outcome::NotRun(why) if why.starts_with("TestField.Caption on what TestPage.GetField found"))
            }
            _ => *r != Outcome::Pass || *marks != [TestPageGetFieldControlId],
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The neighbors past the cells: Activate on what GetField found, which no cell calls, refuses, and so does Caption of what the
/// Card's part control id gives. GetField by a control's own id on a Worksheet, a ListPlus and a CardPart behind a TestPart reads
/// the field's Value, as the cells of :2676-:2702 measure.
#[test]
fn neighbors_past_the_cells() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT GFN Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "WorksheetGetField" | "ListPlusOwnId" | "CardPartOwnId" => {
                *r != Outcome::Pass || *marks != [TestPageGetFieldControlId]
            }
            _ => !matches!(r, Outcome::NotRun(why) if why.contains("on what TestPage.GetField found by a control id")),
        })
        .collect();
    assert!(o.len() == 5 && bad.is_empty(), "{o:#?}");
}

/// The shapes of the review of the 8902 pull request (:2559-:2702): the ids of an action and an action group and the base
/// page's id of an extension's field on a Worksheet, a host control's id through the TestPart of a CardPart a Card shows, an id
/// no control has on the CardPart a RoleCenter shows, on an API page, a PromptDialog, a ReportPreview and a ReportProcessingOnly
/// page are not found; a control's own id on a Worksheet, a ListPlus and a CardPart a TestPart shows reads its Value. The
/// extension's field by the extension's id on a Worksheet (:2576 `ext`) refuses, as only a Card's is measured. Just past the
/// cells an action's id on a NavigatePage and an id no control has on the CardPart a Document shows refuse. The cells run as
/// on BC 29: BC 27 and 28 raise on opening the ReportPreview page (:2643, `#if BC29PLUS`).
#[test]
fn review_shapes() {
    let src = format!("#define BC29PLUS\n{PROBE}\n{NEIGHBORS}");
    let kinds = outcomes(&src, "LT GFI Kinds Tests");
    let bad: Vec<_> = kinds
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "Worksheet_GetFieldExtensionFieldIds" => {
                !matches!(r, Outcome::NotRun(why) if why.starts_with("TestPage.GetField by a control id on a page kind"))
            }
            _ => *r != Outcome::Pass || *marks != [TestPageGetFieldControlId],
        })
        .collect();
    assert!(kinds.len() == 11 && bad.is_empty(), "{kinds:#?}");
    let past = outcomes(&src, "LT GFN Kind Tests");
    let bad: Vec<_> = past
        .iter()
        .filter(|(n, r, _)| match n.as_str() {
            "NavigatePageActionId" => {
                !matches!(r, Outcome::NotRun(why) if why.starts_with("TestPage.GetField by a control id on a page kind"))
            }
            _ => !matches!(r, Outcome::NotRun(why) if why.starts_with("TestPage.GetField with an id no control of the page has")),
        })
        .collect();
    assert!(past.len() == 2 && bad.is_empty(), "{past:#?}");
}

/// Al-runner issue 8887: an id no control has is not found on every page kind the cells open, a Worksheet, a ListPlus, a
/// CardPart behind a TestPart, a page without PageType, a RoleCenter, a NavigatePage, a StandardDialog, a ConfirmationDialog
/// and a HeadlinePart (:1631-:1797), and so is the id of another page's control on a Worksheet (the neighbor). Main refused
/// them: it checked the page kind before the lookup.
#[test]
fn an_id_no_control_has_is_not_found_on_every_page_kind_measured() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let mut o = outcomes(&src, "LT GFI Missing Tests");
    o.extend(outcomes(&src, "LT GFN Missing Tests"));
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [TestPageGetFieldControlId])
        .collect();
    assert!(o.len() == 10 && bad.is_empty(), "{o:#?}");
}
