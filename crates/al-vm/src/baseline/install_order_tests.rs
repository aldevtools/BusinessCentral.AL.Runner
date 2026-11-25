//! The install-order comparison (`Baseline::install_order_differences`) compares what a read finds in each order: a table
//! the store holds no entry for reads the test data's rows there, and a company deleted (or copied) in one order only
//! changes what every untouched table of it answers (al-runner issue 9039).

use super::Baseline;
use super::tests::{NameRows, names, sample};
use crate::Engine;
use al_data::Store;
use al_data::memory::MemStore;

/// An engine with no table 11, so the comparison blanks no system fields of it.
fn engine() -> std::sync::Arc<Engine> {
    let l = al_testkit::compile("codeunit 50100 Empty { }");
    Engine::new(l.program.clone(), al_testkit::blocked(&l))
}

/// A baseline whose store has the test data rows of `NameRows` (table 11 in company `C`).
fn backed_start() -> Baseline {
    let mut b = sample();
    b.store = MemStore::default()
        .as_database()
        .with_backing(std::rc::Rc::new(NameRows));
    b.store.commit();
    b
}

/// The baseline `start` leaves after `install` changed its store and committed.
fn order(start: &Baseline, install: impl FnOnce(&mut MemStore)) -> Baseline {
    let mut b = start.clone();
    install(&mut b.store);
    b.store.commit();
    b
}

fn differences(a: &Baseline, b: &Baseline, start: &Baseline) -> Option<super::install_orders::OrderDifferences> {
    a.install_order_differences(b, start, &engine(), ((0, 0), (0, 0)))
}

// The issue's shape: one order deletes company C (its test data rows are gone), the other leaves it. A read of table 11 in
// C finds three rows in one order and none in the other, so the orders do not leave the same database.
#[test]
fn a_company_deleted_in_one_order_only_is_not_the_same_database() {
    let start = backed_start();
    let dropped = order(&start, |s| s.drop_company("C"));
    let kept = order(&start, |_| {});
    assert_eq!(differences(&dropped, &kept, &start), None);
    assert_eq!(differences(&kept, &dropped, &start), None);
}

// Both orders delete company C: every read of it finds the same rows in both, so the comparison still answers.
#[test]
fn a_company_deleted_in_both_orders_is_the_same_database() {
    let start = backed_start();
    let a = order(&start, |s| s.drop_company("C"));
    let b = order(&start, |s| s.drop_company("C"));
    assert_eq!(differences(&a, &b, &start), Some(Default::default()));
}

// One order empties table 11 in C with a DeleteAll, the other leaves it: the untouched order reads the three test data
// rows, the other none. The store of the untouched order holds no entry for the table; the comparison reads the test
// data's rows for it.
#[test]
fn a_backed_table_emptied_in_one_order_only_differs() {
    let start = backed_start();
    let emptied = order(&start, |s| {
        s.delete_all(&al_data::contract::cx("C"), &names(), &[]).unwrap();
    });
    let kept = order(&start, |_| {});
    let d = differences(&emptied, &kept, &start).expect("tables only");
    assert!(d.contains_key(&11), "table 11 differs: {d:?}");
}

// A DeleteAll without filters on a table with no rows to remove still stands as a mark (`MemStore::emptied`): a read of a
// platform table whose fresh-tenant rows the runner does not hold refuses in the order without it (al-vm `is_unseeded`).
#[test]
fn an_emptied_mark_in_one_order_only_differs() {
    let start = sample();
    let emptied = order(&start, |s| {
        s.delete_all(&al_data::contract::cx("D"), &names(), &[]).unwrap();
    });
    let kept = order(&start, |_| {});
    let d = differences(&emptied, &kept, &start).expect("tables only");
    assert!(d.contains_key(&11), "table 11 differs: {d:?}");
}

// Both orders copy C to E: an untouched table of E reads C's test data rows in both, so the comparison answers.
#[test]
fn a_company_copied_alike_in_both_orders_is_the_same_database() {
    let start = backed_start();
    let a = order(&start, |s| s.copy_company("C", "E"));
    let b = order(&start, |s| s.copy_company("C", "E"));
    assert_eq!(differences(&a, &b, &start), Some(Default::default()));
}
