//! A value the order of a key cannot place never enters a key of the store (al-runner issue 7644): the store orders its rows
//! with `compare_key_value` and counts a comparison that fails as equal, so a row with such a value in a key field, or a
//! lookup by such a primary key, is refused instead of landing on another row.

use super::*;
use crate::contract::{cx, row, table};
use crate::{Scan, Store};

fn marked() -> Value {
    Value::Code(format!("Z{}", al_builtins::native::CALL_STACK_MARK).as_str().into())
}

fn refused<T: std::fmt::Debug>(r: Result<T, StoreError>) -> bool {
    matches!(r, Err(StoreError::Refused(_)))
}

#[test]
fn a_row_with_a_value_no_key_order_places_is_not_written() {
    let (c, t) = (cx("C"), table());
    let mut s = MemStore::default();
    s.insert(&c, &t, row("A", 1, 1)).unwrap();
    // Primary key field.
    let mut r = row("B", 1, 1);
    r[0] = marked();
    assert!(refused(s.insert(&c, &t, r.clone())));
    assert!(refused(s.rename(&c, &t, &[Value::Code("A".into())], r.clone())));
    // A field of the secondary key (Group, Amount).
    let mut r = row("A", 1, 1);
    r[2] = Value::Unknown("a column nothing pins".into());
    r[1] = Value::Null;
    assert!(refused(s.modify(&c, &t, r.clone())));
    r[0] = Value::Code("C".into());
    assert!(refused(s.insert(&c, &t, r)));
    // Nothing changed.
    let scan = Scan {
        key: 0,
        filters: &[],
        ascending: true,
        after: None,
    };
    let first = s.next(&c, &t, &scan).unwrap().expect("a row");
    assert!(matches!(&first[2], Value::Integer(1)), "{first:?}");
    assert_eq!(s.count(&c, &t, &[]).unwrap(), 1);
}

#[test]
fn a_lookup_by_a_key_no_order_places_is_refused() {
    let (c, t) = (cx("C"), table());
    let mut s = MemStore::default();
    s.insert(&c, &t, row("A", 1, 1)).unwrap();
    assert!(refused(s.get(&c, &t, &[marked()])));
    assert!(refused(s.delete(&c, &t, &[marked()])));
    let mut r = row("B", 1, 1);
    r[0] = Value::Code("B".into());
    assert!(refused(s.rename(&c, &t, &[marked()], r)));
}

#[test]
fn rows_with_a_value_no_key_order_places_make_no_table() {
    let t = table();
    let mut r = row("B", 1, 1);
    r[0] = marked();
    assert!(Rows::from_rows(&t, vec![row("A", 1, 1), r]).is_err());
    assert!(Rows::from_rows(&t, vec![row("A", 1, 1), row("B", 2, 1)]).is_ok());
}
