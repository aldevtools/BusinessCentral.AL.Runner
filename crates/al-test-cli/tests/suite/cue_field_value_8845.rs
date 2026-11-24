//! Al-runner issue 8845: the Value of a field control in a cuegroup (al-vm pages/cue_value.rs, assumption cuegroup-field-value).
//! The reporter's repro, the cue cells of corpus pull request 1538 (testpage/GetFieldByControlId.al) and the neighbors past them,
//! as the neighbor apps tests/neighbors/8845-cue-field-value and 8832-getfield-control-id hold them.

use super::outside_fb_repros::assert_stub_outcomes as outcomes;
use al_builtins::assumptions::Assumption::{CuegroupFieldValue, TestPageGetFieldControlId};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8832-getfield-control-id/src/Probe.al");

/// The reporter's test passes: the Text field in the cuegroup on a Card reads `0`, the same field in a group its text. Main read
/// the text.
#[test]
fn issue_8845_repro_passes() {
    let src = format!(
        "{}\n{}",
        include_str!("../../../../tests/neighbors/8845-cue-field-value/src/Repro.al"),
        include_str!("../../../../tests/neighbors/8845-cue-field-value/src/ReproTests.al")
    );
    let o = outcomes(&src, "Repro Cue Tests");
    assert!(
        o.len() == 1 && o[0].1 == Outcome::Pass && o[0].2 == [CuegroupFieldValue],
        "{o:#?}"
    );
}

/// The cue cells pass under both entries: each type by its name and through GetField on a Card (:742), a CardPart (:856), a
/// ListPart (:923) and a List (:990), and the Text field of :653. On a RoleCenter (:1076), where BC finds no such field, the
/// read refuses.
#[test]
fn cue_cells_pass() {
    let o = outcomes(PROBE, "LT GFI Tests");
    let cue: Vec<_> = o
        .iter()
        .filter(|(n, ..)| n.starts_with("CueTypes") || n == "CueCard_CueField")
        .collect();
    let bad: Vec<_> = cue
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "CueTypesRoleCenter_ValueByNameAndById" => {
                !matches!(r, Outcome::NotRun(why) if why.starts_with("Value of a field control in a cuegroup on a RoleCenter"))
            }
            _ => {
                *r != Outcome::Pass
                    || !(marks.len() == 2
                        && marks.contains(&CuegroupFieldValue)
                        && marks.contains(&TestPageGetFieldControlId))
            }
        })
        .collect();
    assert!(cue.len() == 6 && bad.is_empty(), "{cue:#?}");
}

/// The neighbors past the cells, each measured on BC: an Integer in a wide cuegroup alone on the Card passes under the entry (BC
/// shows its value, :742 measures it); every other one refuses: a DateFormula field in a cuegroup, a Boolean field in a wide
/// one, a Code field in a cuegroup on a Document page and in a CardPart read through its host (BC shows `0` for each), and the
/// wide Integer there (BC shows its value; main refused it).
#[test]
fn neighbors_past_the_cells() {
    let src = include_str!("../../../../tests/neighbors/8845-cue-field-value/src/Neighbors.al");
    let mut o = outcomes(src, "LT CFN Tests");
    o.extend(outcomes(src, "LT CFN Wide Tests"));
    let bad: Vec<_> = o
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "WideCueInteger_Value" => *r != Outcome::Pass || *marks != [CuegroupFieldValue],
            _ => !matches!(r, Outcome::NotRun(why) if why.starts_with("Value of a field control in a cuegroup")),
        })
        .collect();
    assert!(o.len() == 7 && bad.is_empty(), "{o:#?}");
}
