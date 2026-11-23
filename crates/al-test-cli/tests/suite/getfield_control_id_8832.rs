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
/// page are not found; a control's own id on a Worksheet, a ListPlus and a CardPart a TestPart shows reads its Value, and so
/// does the extension's field with ApplicationArea = All by the extension's id (:2576 `ext`, al-runner issue 8944). Just past the
/// cells an action's id on a NavigatePage and an id no control has on the CardPart a Document shows refuse. Without a version
/// symbol the ReportPreview cell takes BC 29's branch (:2643; BC 27 and 28 raise on the page's open, `#if BC27PLUS and not
/// BC29PLUS`).
#[test]
fn review_shapes() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let kinds = outcomes(&src, "LT GFI Kinds Tests");
    let bad: Vec<_> = kinds
        .iter()
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [TestPageGetFieldControlId])
        .collect();
    assert!(kinds.len() == 11 && bad.is_empty(), "{kinds:#?}");
    let past = outcomes(&src, "LT GFN Kind Tests");
    let bad: Vec<_> = past
        .iter()
        .filter(|(n, r, _)| match n.as_str() {
            "NavigatePageActionId" => {
                !matches!(r, Outcome::NotRun(why) if why.starts_with("TestPage.GetField by a control id on a page kind"))
            }
            "WorksheetExtActionRefId" | "CardPartHostExtFieldId" => !matches!(r, Outcome::NotRun(why) if why.starts_with(EXT_ID)),
            _ => !matches!(r, Outcome::NotRun(why) if why.starts_with("TestPage.GetField with an id no control of the page has")),
        })
        .collect();
    assert!(past.len() == 4 && bad.is_empty(), "{past:#?}");
}

/// How the refusal of an id a page extension's control has, on the page, the host or a part's page, starts.
const EXT_ID: &str = "TestPage.GetField with the id of a page extension's control";

/// The review of the 8902 pull request at 22:31Z (:3015-:3051): on a Worksheet an extension's actionref by the extension's id
/// (BC finds it, an empty field), a field the host's extension adds read through the CardPart and a field a part page's
/// extension adds read on the RoleCenter (BC: not found) refuse, as the entry says; the previous head of this pull request
/// answered "not found" for the first and the third, and main refuses both.
#[test]
fn extension_control_ids_refuse() {
    let o = outcomes(PROBE, "LT GFI Owner Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(why) if why.starts_with(EXT_ID)))
        .collect();
    assert!(o.len() == 3 && bad.is_empty(), "{o:#?}");
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

/// Al-runner issue 8944: GetField with the extension's control id of a field a page extension adds with ApplicationArea = All
/// finds it on a Card (:2804) and on a Worksheet (:2576 `ext`), on every version measured; main answered "not found" from BC
/// 28. The reporter's test passes, and so does the cell; the cell of the field without ApplicationArea (:2822) is compiled
/// without a version symbol and expects BC 27's answer, as at :589.
#[test]
fn an_extension_field_with_an_area_is_found_by_the_extensions_id() {
    let repro = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8944-getfield-extension-field/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8944-getfield-extension-field/src/ReproTests.al")
    );
    let o = outcomes(&repro, "Repro ExtField Tests");
    assert!(
        o.len() == 1 && o[0].1 == Outcome::Pass && o[0].2 == [TestPageGetFieldControlId],
        "{o:#?}"
    );
    let cells = outcomes(PROBE, "LT GFI Ext Area Tests");
    let bad: Vec<_> = cells
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "Card_ExtensionFieldWithoutArea_ByEitherId" => {
                !matches!(r, Outcome::Fail(m) if m.contains("Expected:<found x1>")) || !marks.is_empty()
            }
            _ => *r != Outcome::Pass || *marks != [TestPageGetFieldControlId],
        })
        .collect();
    assert!(cells.len() == 2 && bad.is_empty(), "{cells:#?}");
}

/// Just past the cells of 8944: the same field on a List, a field with ApplicationArea = Suite, a hidden one, and Caption
/// through GetField refuse (BC: `Gamma`, `Gamma`, not found, `Note`).
#[test]
fn extension_field_neighbors_refuse() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8944-getfield-extension-field/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8944-getfield-extension-field/src/Neighbors.al")
    );
    let o = outcomes(&src, "Repro ExtField Neighbors");
    let bad: Vec<_> = o.iter().filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_))).collect();
    assert!(o.len() == 4 && bad.is_empty(), "{o:#?}");
}
