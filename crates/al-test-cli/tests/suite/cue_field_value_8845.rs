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
/// ListPart (:923), a List (:990) and a Document (:2116), the Text field of :653, and a DateFormula and a wide Boolean on a Card
/// (:2216), also a blank formula and false (:3209); on the CardPart read through a Card by name (:2166) under the cue entry, and
/// by id through the TestPart (:3159) under both. On a RoleCenter (:1076), where BC finds no such field, the read refuses.
#[test]
fn cue_cells_pass() {
    let mut o = outcomes(PROBE, "LT GFI Tests");
    o.extend(outcomes(PROBE, "LT GFI Cue More Tests"));
    o.extend(outcomes(PROBE, "LT GFI Cue Review Tests"));
    let cue: Vec<_> = o
        .iter()
        .filter(|(n, ..)| {
            n.starts_with("CueTypes")
                || n.starts_with("CueDateFormula")
                || n.starts_with("CueBlankDateFormula")
                || n == "CueCard_CueField"
        })
        .collect();
    let bad: Vec<_> = cue
        .iter()
        .filter(|(n, r, marks)| match n.as_str() {
            "CueTypesRoleCenter_ValueByNameAndById" => {
                !matches!(r, Outcome::NotRun(why) if why.starts_with("Value of a field control in a cuegroup on a RoleCenter"))
            }
            "CueTypesPartThroughHost_ValueByName" => *r != Outcome::Pass || *marks != [CuegroupFieldValue],
            _ => {
                *r != Outcome::Pass
                    || !(marks.len() == 2
                        && marks.contains(&CuegroupFieldValue)
                        && marks.contains(&TestPageGetFieldControlId))
            }
        })
        .collect();
    assert!(cue.len() == 11 && bad.is_empty(), "{cue:#?}");
}

/// The neighbors past the cells, each measured on BC: the shapes main refused and the cells of :2116-:2216 measure pass under the
/// entry (a DateFormula field in a cuegroup, a Boolean field in a wide one, a Code field and a wide Integer on a Document page and
/// in a CardPart read through a Card, the wide Integer on the Card), and a Code field of a CardPart read by its id through the
/// Card's TestPart (:3159); just past them a CardPart a Document shows, and a DateFormula and a wide Boolean on a List, refuse
/// (BC shows `0` for each).
#[test]
fn neighbors_past_the_cells() {
    let src = include_str!("../../../../tests/neighbors/8845-cue-field-value/src/Neighbors.al");
    let more_src = include_str!("../../../../tests/neighbors/8845-cue-field-value/src/MoreNeighbors.al");
    let mut o = outcomes(src, "LT CFN Tests");
    o.extend(outcomes(src, "LT CFN Wide Tests"));
    let more = outcomes(&format!("{src}\n{more_src}"), "LT CFN More Tests");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != [CuegroupFieldValue])
        .chain(more.iter().filter(|(n, r, marks)| match n.as_str() {
            "PartThroughHostCueCode_ValueById" => {
                *r != Outcome::Pass
                    || !(marks.len() == 2
                        && marks.contains(&CuegroupFieldValue)
                        && marks.contains(&TestPageGetFieldControlId))
            }
            _ => !matches!(r, Outcome::NotRun(why) if why.starts_with("Value of a field control in a cuegroup")),
        }))
        .collect();
    assert!(o.len() == 7 && more.len() == 4 && bad.is_empty(), "{o:#?} {more:#?}");
}
