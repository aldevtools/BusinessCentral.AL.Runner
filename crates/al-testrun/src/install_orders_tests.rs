use super::order_of;
use std::collections::BTreeSet;

/// Apps 1 (P), 2 (G), 3 (A); A depends on P.
fn needs(a: u32, b: u32) -> bool {
    (a, b) == (3, 1)
}

/// ADR-079 decision 3, clarification of 2026-11-23 (review round 3, N2): A before G and G before P are requested, and A
/// depends on P. No order holds all three, so none is given (before: P, G, A, the run "G before P" never made).
#[test]
fn a_requested_order_against_a_dependency_gives_no_order() {
    assert_eq!(order_of(&[1, 2, 3], needs, &[3, 2, 1]), Err(BTreeSet::from([1, 2, 3])));
}

/// For two apps with a dependency path the dependency decides, and the requested place of the others holds.
#[test]
fn a_dependency_decides_over_the_requested_place() {
    assert_eq!(order_of(&[1, 2, 3], needs, &[3, 1, 2]), Ok(vec![1, 3, 2]));
    assert_eq!(order_of(&[1, 2, 3], needs, &[2, 3, 1]), Ok(vec![2, 1, 3]));
}

/// The apps of the cycle only: an app the cycle blocks without being on it is not named.
#[test]
fn only_the_apps_of_the_cycle_are_named() {
    assert_eq!(
        order_of(&[4, 1, 2, 3], needs, &[3, 2, 1, 4]),
        Err(BTreeSet::from([1, 2, 3]))
    );
}
