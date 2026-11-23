//! The influence graph and its units (ADR-079 decision 2).

use super::{Footprints, Graph, tie};
use al_vm::install_footprint::{InstallFootprint, Lasting};
use std::collections::BTreeSet;

fn meets(apps: &[u32]) -> InstallFootprint {
    InstallFootprint {
        apps_met: apps.iter().copied().collect(),
        ..Default::default()
    }
}

fn writes(seq: &str) -> InstallFootprint {
    InstallFootprint {
        writes: [Lasting::Sequence(String::new(), seq.into())].into_iter().collect(),
        ..Default::default()
    }
}

fn all(apps: &[u32]) -> BTreeSet<u32> {
    apps.iter().copied().collect()
}

#[test]
fn the_8056_shape_has_a_unit_per_app_and_each_install_reads_from_both_subscriber_apps() {
    // A (1) and A2 (2) meet B (3) and B2 (4); B and B2 have no install code.
    let fp: Footprints = [(1, meets(&[3, 4])), (2, meets(&[3, 4]))].into_iter().collect();
    let apps = all(&[1, 2, 3, 4]);
    let g = Graph::of(&apps, &fp, |x, y| x != y);
    let units = g.units(&apps);
    assert_eq!(units.len(), 4, "{units:?}");
    let a = units.iter().position(|u| u == &vec![1]).unwrap();
    let b = units.iter().position(|u| u == &vec![3]).unwrap();
    assert!(b < a, "an input before the unit that reads from it: {units:?}");
    let inputs: Vec<_> = g.inputs(&units, &[1]).into_iter().cloned().collect();
    assert_eq!(inputs, vec![vec![3], vec![4]]);
    assert!(g.inputs(&units, &[3]).is_empty());
}

#[test]
fn two_installs_that_write_the_same_key_are_one_unit() {
    let fp: Footprints = [(1, writes("S")), (2, writes("S")), (3, writes("T"))]
        .into_iter()
        .collect();
    let apps = all(&[1, 2, 3]);
    let units = Graph::of(&apps, &fp, |x, y| x != y).units(&apps);
    assert!(units.contains(&vec![1, 2]) && units.contains(&vec![3]), "{units:?}");
    assert!(tie(&fp, 1, 2, |a| a.to_string(), |t| t.to_string()).contains("both write"));
}

#[test]
fn a_cycle_of_four_installs_is_one_unit_and_ordered_apps_have_no_edge() {
    let fp: Footprints = [(1, meets(&[2])), (2, meets(&[3])), (3, meets(&[4])), (4, meets(&[1]))]
        .into_iter()
        .collect();
    let apps = all(&[1, 2, 3, 4]);
    assert_eq!(
        Graph::of(&apps, &fp, |x, y| x != y).units(&apps),
        vec![vec![1, 2, 3, 4]]
    );
    // 4 depends on 1: their order is known, so the cycle breaks there.
    let ordered = |x: u32, y: u32| x != y && (x, y) != (4, 1) && (x, y) != (1, 4);
    assert_eq!(Graph::of(&apps, &fp, ordered).units(&apps).len(), 4);
}

/// ADR-079 decision 1, clarification of 2026-11-23: a session that touches everything ties every other install session;
/// app 3 has no install code and is not met, so nothing ties it.
#[test]
fn an_install_that_touches_everything_ties_every_unordered_install_session() {
    let mut page = InstallFootprint::default();
    page.everything.insert("a page".into());
    let fp: Footprints = [(1, page), (2, InstallFootprint::default())].into_iter().collect();
    let apps = all(&[1, 2, 3]);
    assert_eq!(
        Graph::of(&apps, &fp, |x, y| x != y).units(&apps),
        vec![vec![1, 2], vec![3]]
    );
    assert!(tie(&fp, 2, 1, |a| a.to_string(), |t| t.to_string()).contains("a page"));
}

#[test]
fn ancestors_follow_inputs_of_inputs() {
    // 1 reads from 2, 2 reads from 3.
    let fp: Footprints = [(1, meets(&[2])), (2, meets(&[3]))].into_iter().collect();
    let apps = all(&[1, 2, 3]);
    let g = Graph::of(&apps, &fp, |x, y| x != y);
    let units = g.units(&apps);
    assert_eq!(units, vec![vec![3], vec![2], vec![1]]);
    let anc: Vec<_> = g.ancestors(&units, &[1]).into_iter().cloned().collect();
    assert_eq!(anc, vec![vec![3], vec![2]]);
}

#[test]
fn the_tie_names_the_table_and_the_kind_of_resource_not_its_debug_form() {
    use al_data::memory::Resource;
    let extent = Lasting::Store(Resource::Extent("C".into(), 7));
    let fp: Footprints = [
        (
            1,
            InstallFootprint {
                reads: [extent.clone()].into_iter().collect(),
                ..Default::default()
            },
        ),
        (
            2,
            InstallFootprint {
                writes: [Lasting::AutoIncrement("C".into(), 7), extent].into_iter().collect(),
                ..Default::default()
            },
        ),
    ]
    .into_iter()
    .collect();
    let t = tie(&fp, 1, 2, |a| format!("app {a}"), |t| format!("T{t}"));
    assert!(t.contains("the rows of table T7 in company C"), "{t}");
    assert!(!t.contains("Extent("), "{t}");
    let both = tie(&fp, 2, 2, |a| format!("app {a}"), |t| format!("T{t}"));
    assert!(both.contains("the AutoIncrement counter of table T7"), "{both}");
}

/// An app without install code is reached only by a meet: a session that touches everything but does not meet it has
/// no edge to it; one that meets it reads from it.
#[test]
fn an_app_without_install_code_is_reached_only_by_a_meet() {
    let mut page = InstallFootprint::default();
    page.everything.insert("a page".into());
    let apps = all(&[1, 2]);
    let alone: Footprints = [(1, page.clone())].into_iter().collect();
    assert_eq!(Graph::of(&apps, &alone, |x, y| x != y), Graph::default());
    page.apps_met.insert(2);
    let met: Footprints = [(1, page)].into_iter().collect();
    let g = Graph::of(&apps, &met, |x, y| x != y);
    assert_eq!(g.reads, BTreeSet::from([(1, 2)]));
    assert!(g.collide.is_empty());
}
