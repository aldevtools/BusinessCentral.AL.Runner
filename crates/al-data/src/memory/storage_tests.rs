//! A table whose primary key holds a Code field the database stores as a number keeps the number order of its keys on
//! every way a store is rebuilt: an export and import, a freeze and thaw, a row-change replay (al-runner issue 7618).
//! The order itself is pinned by Y82PkTests.al `PkInt_NumericOrder` (tests/al-language/record/system-fields/).

use super::*;
use crate::contract::{cx, row, table};
use crate::{Scan, Store};

/// The contract table with its key field `No.` stored as an Integer.
fn int_table() -> TableDef {
    let mut t = table();
    t.fields[0].code_storage = CodeStorage::Integer;
    t
}

fn code(s: &str) -> Vec<Value> {
    vec![Value::Code(s.into())]
}

/// A committed store holding the rows 1, 9 and 10 of `int_table`.
fn store() -> MemStore {
    let t = int_table();
    let mut s = MemStore::default();
    for no in ["10", "9", "1"] {
        s.insert(&cx("C"), &t, row(no, 1, 1)).unwrap();
    }
    s.commit();
    s
}

/// The `No.` of every row of the primary key, both ways.
fn order(s: &MemStore) -> (Vec<String>, Vec<String>) {
    let t = int_table();
    let scan = |ascending: bool| {
        let mut out = Vec::new();
        let mut after: Option<Vec<Value>> = None;
        while let Some(r) = s
            .next(
                &cx("C"),
                &t,
                &Scan {
                    key: 0,
                    filters: &[],
                    ascending,
                    after: after.as_deref(),
                },
            )
            .unwrap()
        {
            after = Some(crate::position(&t, 0, &r));
            out.push(match &r[0] {
                Value::Code(c) => c.as_str().to_string(),
                v => format!("{v:?}"),
            });
        }
        out
    };
    (scan(true), scan(false))
}

/// Every read of a key finds its row, and the scans give the number order.
fn assert_number_order(s: &MemStore, how: &str) {
    let t = int_table();
    for no in ["1", "9", "10"] {
        assert!(
            s.committed_get(&cx("C"), &t, &code(no)).is_some(),
            "{how}: committed_get misses {no}"
        );
        assert!(
            s.get(&cx("C"), &t, &code(no)).unwrap().is_some(),
            "{how}: get misses {no}"
        );
    }
    let (up, down) = order(s);
    assert_eq!(up, ["1", "9", "10"], "{how}: ascending");
    assert_eq!(down, ["10", "9", "1"], "{how}: descending");
}

#[test]
fn an_imported_table_keeps_the_number_order_of_its_key() {
    let s = store();
    assert_number_order(&s, "written");
    let back = MemStore::from_exported(s.export_committed().unwrap());
    assert_number_order(&back, "from_exported");
    let (one, emptied) = s.committed_table("C", 7).unwrap();
    let replaced = MemStore::default().with_committed_tables(vec![(one, emptied)], &[]);
    assert_number_order(&replaced, "with_committed_tables");
    let thawed = FrozenStore::freeze(&back).unwrap().thaw();
    assert_number_order(&thawed, "thaw");
}

#[test]
fn a_row_change_replay_keeps_the_number_order_of_the_key() {
    let t = int_table();
    let base = MemStore::from_exported(store().export_committed().unwrap());
    let mut next = base.clone();
    next.insert(&cx("C"), &t, row("2", 1, 1)).unwrap();
    next.delete(&cx("C"), &t, &code("9")).unwrap();
    next.commit();
    let changes = next
        .committed_row_changes_from(&base, &mut |a, b| Some(format!("{a:?}") == format!("{b:?}")))
        .unwrap();
    let replayed = base.with_committed_row_changes(changes).unwrap();
    for no in ["1", "2", "10"] {
        assert!(
            replayed.committed_get(&cx("C"), &t, &code(no)).is_some(),
            "committed_get misses {no}"
        );
    }
    assert!(replayed.committed_get(&cx("C"), &t, &code("9")).is_none());
    let (up, down) = order(&replayed);
    assert_eq!(up, ["1", "2", "10"]);
    assert_eq!(down, ["10", "2", "1"]);
}
