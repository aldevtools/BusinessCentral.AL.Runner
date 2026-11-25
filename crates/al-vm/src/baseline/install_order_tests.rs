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
fn an_emptied_mark_in_one_order_only_differs_on_a_platform_unseeded_table() {
    let start = sample();
    let emptied = order(&start, |s| {
        s.delete_all(&al_data::contract::cx("D"), &names(), &[]).unwrap();
    });
    let kept = order(&start, |_| {});
    let mut e = engine();
    std::sync::Arc::get_mut(&mut e).unwrap().platform_unseeded.insert(11);
    let d = emptied
        .install_order_differences(&kept, &start, &e, ((0, 0), (0, 0)))
        .expect("tables only");
    assert!(d.contains_key(&11), "table 11 differs: {d:?}");
}

// On any other table the mark changes no read (`is_unseeded` reads it for platform unseeded tables only): the same rows
// in both orders are the same database, as on main.
#[test]
fn an_emptied_mark_in_one_order_only_is_the_same_database_on_another_table() {
    let start = sample();
    let emptied = order(&start, |s| {
        s.delete_all(&al_data::contract::cx("D"), &names(), &[]).unwrap();
    });
    let kept = order(&start, |_| {});
    assert_eq!(differences(&emptied, &kept, &start), Some(Default::default()));
}

// Both orders copy C to E: an untouched table of E reads C's test data rows in both, so the comparison answers.
#[test]
fn a_company_copied_alike_in_both_orders_is_the_same_database() {
    let start = backed_start();
    let a = order(&start, |s| s.copy_company("C", "E"));
    let b = order(&start, |s| s.copy_company("C", "E"));
    assert_eq!(differences(&a, &b, &start), Some(Default::default()));
}

// Without a backing a deleted company changes no read of an untouched table (`MemStore::has_backing`): the tables it held
// are gone in that order only, and the comparison names them.
#[test]
fn a_company_deleted_in_one_order_only_without_a_backing_names_its_tables() {
    let start = order(&sample(), |s| {
        let row = vec![
            al_builtins::value::Value::Text("Alpha".into()),
            al_builtins::value::Value::Code("A".into()),
        ];
        s.insert(&al_data::contract::cx("C"), &names(), row).unwrap();
    });
    let dropped = order(&start, |s| s.drop_company("C"));
    let kept = order(&start, |_| {});
    let d = differences(&dropped, &kept, &start).expect("tables only");
    assert!(d.contains_key(&11), "table 11 differs: {d:?}");
}

// Review of PR 9098 (round 1, MED-1): one order copies C to E after a per-company insert, the other before it. A copy
// carries the rows with their system fields (`MemStore::copy_company`), so in the first order E's row has the SystemId of
// C's row, in the second a SystemId of its own. The other fields match; the SystemId of the copy target still differs.
#[test]
fn a_company_copied_before_and_after_a_per_company_insert_differs() {
    let (engine, def) = system_engine();
    let start = backed_start();
    let insert = |s: &mut MemStore, company: &str, id: u8| {
        s.insert(&al_data::contract::cx(company), &def, setup_row(&def, id))
            .unwrap();
    };
    let copy_after = order(&start, |s| {
        insert(s, "C", 1);
        s.copy_company("C", "E");
    });
    let copy_before = order(&start, |s| {
        s.copy_company("C", "E");
        insert(s, "C", 1);
        insert(s, "E", 2);
    });
    let d = copy_after
        .install_order_differences(&copy_before, &start, &engine, ((0, 0), (0, 0)))
        .expect("tables only");
    assert!(d.contains_key(&def.object), "the setup table differs: {d:?}");
}

// The same shape without a backing: the copy records no link to a source (`MemStore::copy_company`), and E is still a
// company the copy made, so its SystemIds count.
#[test]
fn a_company_copied_before_and_after_a_per_company_insert_differs_without_a_backing() {
    let (engine, def) = system_engine();
    let start = sample();
    let insert = |s: &mut MemStore, company: &str, id: u8| {
        s.insert(&al_data::contract::cx(company), &def, setup_row(&def, id))
            .unwrap();
    };
    let copy_after = order(&start, |s| {
        insert(s, "C", 1);
        s.copy_company("C", "E");
    });
    let copy_before = order(&start, |s| {
        s.copy_company("C", "E");
        insert(s, "C", 1);
        insert(s, "E", 2);
    });
    let d = copy_after
        .install_order_differences(&copy_before, &start, &engine, ((0, 0), (0, 0)))
        .expect("tables only");
    assert!(d.contains_key(&def.object), "the setup table differs: {d:?}");
}

// Outside a company a copy made the SystemIds still do not count: BC gives them new values on every install.
#[test]
fn a_per_company_insert_with_other_system_ids_outside_a_copy_is_the_same_database() {
    let (engine, def) = system_engine();
    let start = sample();
    let a = order(&start, |s| {
        s.insert(&al_data::contract::cx("C"), &def, setup_row(&def, 1)).unwrap();
    });
    let b = order(&start, |s| {
        s.insert(&al_data::contract::cx("C"), &def, setup_row(&def, 2)).unwrap();
    });
    let d = a.install_order_differences(&b, &start, &engine, ((0, 0), (0, 0)));
    assert_eq!(d, Some(Default::default()));
}

// Review of PR 9098 (round 1, LOW-1): one order copies C to E before a write to C's backed table 11, the other after it.
// E's table 11 reads C's test data rows in the first order and holds the changed rows in the second.
#[test]
fn a_company_copied_before_and_after_a_write_to_the_source_differs() {
    let start = backed_start();
    let change = |s: &mut MemStore| {
        let row = vec![
            al_builtins::value::Value::Text("Alpha".into()),
            al_builtins::value::Value::Code("Z".into()),
        ];
        s.modify(&al_data::contract::cx("C"), &names(), row).unwrap();
    };
    let copy_first = order(&start, |s| {
        s.copy_company("C", "E");
        change(s);
    });
    let write_first = order(&start, |s| {
        change(s);
        s.copy_company("C", "E");
    });
    let d = differences(&copy_first, &write_first, &start).expect("tables only");
    assert!(d.contains_key(&11), "table 11 differs: {d:?}");
}

// Review of PR 9098 (round 1, LOW-2): one order copies C to E, the other does not. With a backing, every untouched table of
// E reads C's test data rows in one order only; the comparison cannot name them by table.
#[test]
fn a_company_copied_in_one_order_only_is_not_the_same_database() {
    let start = backed_start();
    let copied = order(&start, |s| s.copy_company("C", "E"));
    let kept = order(&start, |_| {});
    assert_eq!(differences(&copied, &kept, &start), None);
    assert_eq!(differences(&kept, &copied, &start), None);
}

/// An engine with table 50110 `Setup` (the key `Code`, with the system fields BC adds), and its definition.
fn system_engine() -> (std::sync::Arc<Engine>, al_data::TableDef) {
    let l =
        al_testkit::compile("table 50110 Setup { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }");
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let def = engine
        .tables
        .iter()
        .flatten()
        .find(|d| d.name == "Setup")
        .cloned()
        .expect("table 50110");
    (engine, (*def).clone())
}

/// A row of `def` with the key `SETUP` and a SystemId ending in `id`; the other system fields left at null.
fn setup_row(def: &al_data::TableDef, id: u8) -> Vec<al_builtins::value::Value> {
    use al_builtins::value::Value;
    def.fields
        .iter()
        .map(|f| match f.id {
            1 => Value::Code("SETUP".into()),
            2_000_000_000 => {
                let mut g = [0; 16];
                g[15] = id;
                Value::Guid(g)
            }
            _ => Value::Null,
        })
        .collect()
}
