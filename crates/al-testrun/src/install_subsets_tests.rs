//! The end of a round of the proof and the closure of the tables that differ (ADR-079 decisions 3 and 4).

use super::{next_round, plan, tainted};
use crate::install_proof::{Footprints, Graph};
use al_data::memory::Resource;
use al_vm::install_footprint::{InstallFootprint, Lasting};
use std::collections::BTreeSet;

fn graph(reads: &[(u32, u32)]) -> Graph {
    Graph {
        reads: reads.iter().copied().collect(),
        collide: BTreeSet::new(),
    }
}

#[test]
fn a_round_starts_again_only_when_the_graph_grew() {
    let before = graph(&[(1, 2)]);
    assert_eq!(next_round(&before, &graph(&[(1, 2), (1, 3)]), Some((1, 3))), Ok(true));
    assert_eq!(next_round(&before, &before, None), Ok(false));
}

/// A meet of an app of unknown order that adds no edge would repeat in every round: the proof stops and names
/// the pair (the review of PR 8865, finding H2).
#[test]
fn a_meet_that_adds_no_edge_stops_the_proof_with_the_pair() {
    let g = graph(&[(1, 2)]);
    assert_eq!(next_round(&g, &g, Some((1, 2))), Err((1, 2)));
}

fn rows(t: u32) -> Lasting {
    Lasting::Store(Resource::Extent("C".into(), t))
}

fn session(reads: &[Lasting], writes: &[Lasting]) -> InstallFootprint {
    InstallFootprint {
        reads: reads.iter().cloned().collect(),
        writes: writes.iter().cloned().collect(),
        ..Default::default()
    }
}

/// The 8056 shape with a copier: D (1) wrote table 10, which differs; X (2) depends on D, read table 10 and wrote table
/// 20; Z (3) read table 20 that X wrote and wrote table 30; Y (4) read nothing that differs.
#[test]
fn a_table_written_after_a_read_of_a_table_that_differs_joins_it_until_nothing_is_added() {
    let fp: Footprints = [
        (1, session(&[], &[rows(10)])),
        (2, session(&[rows(10)], &[rows(20)])),
        (3, session(&[rows(20)], &[rows(30)])),
        (4, session(&[rows(40)], &[rows(50)])),
    ]
    .into_iter()
    .collect();
    let (readers, tables) = tainted(&fp, &[(BTreeSet::from([10]), BTreeSet::new())], |_, _| false);
    assert_eq!(readers, BTreeSet::from([2, 3]));
    assert_eq!(tables, Some(BTreeSet::from([20, 30])));
}

/// The only writer of the table that differs is the reader itself (its own runs measured what it wrote), or installs
/// after the reader: no taint.
#[test]
fn a_read_of_rows_only_the_reader_or_a_later_install_wrote_does_not_taint() {
    let fp: Footprints = [
        (1, session(&[rows(10)], &[rows(10), rows(20)])),
        (2, session(&[], &[rows(10)])),
    ]
    .into_iter()
    .collect();
    // 2 depends on 1: it installs after 1.
    let (readers, tables) = tainted(&fp, &[(BTreeSet::from([10]), BTreeSet::new())], |r, w| (r, w) == (1, 2));
    assert!(readers.is_empty());
    assert_eq!(tables, Some(BTreeSet::new()));
}

/// A tainted install that wrote a number sequence or touches everything: every table.
#[test]
fn a_tainted_install_that_wrote_a_sequence_taints_every_table() {
    let seq = Lasting::Sequence(String::new(), "S".into());
    let fp: Footprints = [(1, session(&[], &[rows(10)])), (2, session(&[rows(10)], &[seq]))]
        .into_iter()
        .collect();
    assert_eq!(
        tainted(&fp, &[(BTreeSet::from([10]), BTreeSet::new())], |_, _| false).1,
        None
    );
}

/// An app of the unit whose runs measured the difference is exempt: its writes were compared in each of those runs.
#[test]
fn an_app_of_the_unit_that_measured_the_difference_is_not_tainted_by_it() {
    let fp: Footprints = [
        (1, session(&[rows(10)], &[rows(10), rows(20)])),
        (2, session(&[rows(10)], &[rows(10)])),
    ]
    .into_iter()
    .collect();
    let (readers, _) = tainted(&fp, &[(BTreeSet::from([10]), BTreeSet::from([1, 2]))], |_, _| false);
    assert!(readers.is_empty());
    // 1 is not of the unit: tainted, and 2 then reads the rows 1 wrote.
    let (readers, _) = tainted(&fp, &[(BTreeSet::from([10]), BTreeSet::from([2]))], |_, _| false);
    assert!(readers.contains(&1), "{readers:?}");
}

/// `needs` from a list of (app, app it depends on).
fn deps(pairs: &'static [(u32, u32)]) -> impl Fn(u32, u32) -> bool {
    move |a, b| pairs.contains(&(a, b))
}

/// `reads` from a list of (unit app, input app it reads from).
fn reads(pairs: &'static [(u32, u32)]) -> impl Fn(u32, u32) -> bool {
    move |a, y| pairs.contains(&(a, y))
}

/// Inputs 2 and 3 of one app each; 3 depends on 2 (review round 4, R1). The subset "3 before the unit, 2 after it" is no
/// real order and is skipped; the other three run.
#[test]
fn a_subset_that_puts_an_input_before_the_input_it_depends_on_is_skipped() {
    let runs = plan(&[vec![1]], &[2, 3], reads(&[(1, 2), (1, 3)]), deps(&[(3, 2)]));
    assert_eq!(runs.ok(), Some(vec![vec![1, 2, 3], vec![2, 1, 3], vec![2, 3, 1]]));
}

/// The apps of an input of several apps are placed one by one (on BC they can install on both sides of the unit's app):
/// inputs 2, 4 (one unit) and 3, 3 depends on 2. Every placement but "3 before the unit, 2 after it" runs.
#[test]
fn the_apps_of_a_multi_app_input_are_placed_one_by_one() {
    let runs = plan(
        &[vec![1]],
        &[2, 4, 3],
        reads(&[(1, 2), (1, 4), (1, 3)]),
        deps(&[(3, 2)]),
    );
    let runs = runs.ok().unwrap_or_default();
    assert_eq!(runs.len(), 6, "{runs:?}");
    assert!(
        runs.contains(&vec![2, 3, 1, 4]) && runs.contains(&vec![4, 1, 2, 3]),
        "{runs:?}"
    );
    assert!(
        !runs
            .iter()
            .any(|r| r.iter().position(|&a| a == 3) < r.iter().position(|&a| a == 1)
                && r.iter().position(|&a| a == 2) > r.iter().position(|&a| a == 1))
    );
}

/// Issue 8907: tangle 1, 2 (2 reads from input 3), input 3 depends on 1. The runs are the three real orders; "3 before 2"
/// in the tangle order 2, 1 is no real order and is skipped.
#[test]
fn a_tangle_whose_input_depends_on_a_member_runs_every_real_order() {
    let runs = plan(&[vec![1, 2], vec![2, 1]], &[3], reads(&[(2, 3)]), deps(&[(3, 1)]));
    assert_eq!(runs.ok(), Some(vec![vec![1, 2, 3], vec![1, 3, 2], vec![2, 1, 3]]));
}

/// Issue 8907, the tangle of three: 1, 2, 3 with input 4 depending on 2 and read by 3. The order 1, 2, 4, 3 runs.
#[test]
fn a_tangle_of_three_places_its_input_after_the_member_it_depends_on() {
    let runs = plan(&[vec![1, 2, 3]], &[4], reads(&[(3, 4)]), deps(&[(4, 2)]));
    assert_eq!(runs.ok(), Some(vec![vec![1, 2, 3, 4], vec![1, 2, 4, 3]]));
}

/// An input both tangle apps read from also runs between them: T1, I, T2 gives T1 without I and T2 with it.
#[test]
fn an_input_two_members_read_from_also_installs_between_them() {
    let runs = plan(&[vec![1, 2], vec![2, 1]], &[3], reads(&[(1, 3), (2, 3)]), deps(&[]));
    let want = vec![
        vec![1, 2, 3],
        vec![1, 3, 2],
        vec![3, 1, 2],
        vec![2, 1, 3],
        vec![2, 3, 1],
        vec![3, 2, 1],
    ];
    assert_eq!(runs.ok(), Some(want));
}

/// A member that depends on the input: 2 depends on 3, 1 reads from 3. The runs are the three real orders.
#[test]
fn a_member_that_depends_on_the_input_keeps_it_before() {
    let runs = plan(&[vec![1, 2], vec![2, 1]], &[3], reads(&[(1, 3)]), deps(&[(2, 3)]));
    assert_eq!(runs.ok(), Some(vec![vec![1, 3, 2], vec![3, 1, 2], vec![3, 2, 1]]));
}

/// More runs than every order of a tangle of three times every subset of three inputs: refused.
#[test]
fn a_plan_beyond_the_runs_of_decision_5_is_refused() {
    let orders: Vec<Vec<u32>> = vec![
        vec![1, 2, 3],
        vec![1, 3, 2],
        vec![2, 1, 3],
        vec![2, 3, 1],
        vec![3, 1, 2],
        vec![3, 2, 1],
    ];
    let both = &[(1, 4), (2, 4), (1, 5), (2, 5), (1, 6), (2, 6)];
    let runs = plan(&orders, &[4, 5, 6], reads(both), deps(&[]));
    assert_eq!(runs, Err(162));
}
