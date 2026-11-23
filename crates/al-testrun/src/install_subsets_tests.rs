//! The end of a round of the proof and the closure of the tables that differ (ADR-079 decisions 3 and 4).

use super::{final_units, next_round, no_real_order, tainted};
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

/// Inputs 2 and 3 of one app each; 3 depends on 2 (review round 4, R1).
fn three_needs_two(a: u32, b: u32) -> bool {
    (a, b) == (3, 2)
}

#[test]
fn a_subset_that_puts_an_input_before_the_input_it_depends_on_is_skipped() {
    let (two, three) = (vec![2], vec![3]);
    let inputs = [&two, &three];
    // Bit k: input k installs before the unit. Only "3 before, 2 after" is no real order.
    assert!(no_real_order(&inputs, 0b10, three_needs_two));
    for mask in [0b00, 0b01, 0b11] {
        assert!(!no_real_order(&inputs, mask, three_needs_two), "{mask:b}");
    }
}

/// A multi-app input keeps the stop of [`super::Proof::order`]: the skip reads only inputs of one app.
#[test]
fn a_subset_with_a_multi_app_input_is_not_skipped() {
    let (two, three) = (vec![2, 4], vec![3]);
    assert!(!no_real_order(&[&two, &three], 0b10, three_needs_two));
}

/// Units in input order: A (1), C (3), B (2), B reads from C. A depends on B (review round 4, the final install).
#[test]
fn the_final_install_puts_a_unit_after_the_units_it_depends_on() {
    let units = vec![vec![1], vec![3], vec![2]];
    let inputs = |u: &[u32]| {
        units
            .iter()
            .filter(|v| u == [2] && v.as_slice() == [3])
            .collect::<Vec<_>>()
    };
    let order = final_units(&units, inputs, |a, b| (a, b) == (1, 2));
    assert_eq!(order, [&vec![3], &vec![2], &vec![1]]);
}

/// No dependency between the units: the order of `units` stays.
#[test]
fn the_final_install_keeps_the_order_of_the_units_without_a_dependency() {
    let units = vec![vec![1], vec![3], vec![2]];
    let order = final_units(&units, |_| Vec::new(), |_, _| false);
    assert_eq!(order, [&vec![1], &vec![3], &vec![2]]);
}
