//! ADR-069 conformance: a frozen store is shared by threads, and a write of one thread's store never reaches the frozen
//! tables or another thread's store.

use super::*;
use crate::Store;
use crate::contract::{cx, row};

/// The contract table without its secondary key: these tests put a handle (a List, a Blob, a RecordId) into field 2, which
/// AL does not allow in a key, and the store refuses a key value whose order is not pinned (al-runner issue 7644).
fn table() -> crate::TableDef {
    let mut t = crate::contract::table();
    t.keys.truncate(1);
    t
}

fn base() -> MemStore {
    let (c, t) = (cx("C"), table());
    let mut s = MemStore::default();
    for (no, amount) in [("A", 1), ("B", 2), ("C", 3)] {
        s.insert(&c, &t, row(no, amount, 1)).unwrap();
    }
    s.commit();
    s
}

fn nos(s: &MemStore) -> Vec<(String, String)> {
    s.stored_rows(&cx("C"), &table())
        .iter()
        .map(|r| (format!("{:?}", r[0]), format!("{:?}", r[1])))
        .collect()
}

#[test]
fn a_frozen_store_is_send_and_sync() {
    fn check<T: Send + Sync>() {}
    check::<FrozenStore>();
}

#[test]
fn freeze_refuses_a_row_with_a_handle() {
    let (c, t) = (cx("C"), table());
    let mut s = base();
    let list = Value::List(Default::default());
    s.insert(&c, &t, vec![Value::Code("D".into()), list, Value::Integer(1)])
        .unwrap();
    assert!(FrozenStore::freeze(&s).is_ok(), "an uncommitted row is not part of it");
    s.commit();
    assert!(FrozenStore::freeze(&s).is_err(), "a List is an Rc handle");
    assert!(shareable(&Value::Blob(al_builtins::stream::new_blob())));
    assert!(
        !shareable(&Value::TextConst("x".into())),
        "to_bytes does not write it either"
    );
}

#[test]
fn thaw_gives_the_committed_rows() {
    let f = FrozenStore::freeze(&base()).unwrap();
    assert_eq!(f.row_count(), 3);
    assert_eq!(nos(&f.thaw()), nos(&base()));
}

#[test]
fn a_write_in_one_thread_is_never_seen_by_another_or_the_base() {
    let f = std::sync::Arc::new(FrozenStore::freeze(&base()).unwrap());
    let before = nos(&f.thaw());
    let writers: Vec<_> = (0..4)
        .map(|k| {
            let f = f.clone();
            std::thread::spawn(move || {
                let (c, t) = (cx("C"), table());
                let mut s = f.thaw();
                for _ in 0..50 {
                    s.modify(&c, &t, row("A", 100 + k, 1)).unwrap();
                    s.delete(&c, &t, &[Value::Code("B".into())]).unwrap();
                    s.rename(&c, &t, &[Value::Code("C".into())], row("Z", 3, 1)).unwrap();
                    s.insert(&c, &t, row(&format!("N{k}"), k, 1)).unwrap();
                    s.commit();
                    assert_eq!(s.count(&c, &t, &[]).unwrap(), 3, "thread {k} sees only its own writes");
                    s.delete_all(&c, &t, &[]).unwrap();
                    s.commit();
                    s = f.thaw();
                }
                // A reader that started from the base sees the base while another thread writes.
                nos(&f.thaw())
            })
        })
        .collect();
    for w in writers {
        assert_eq!(w.join().unwrap(), before);
    }
    assert_eq!(nos(&f.thaw()), before, "the frozen tables are unchanged");
}

#[test]
fn rollback_drops_only_the_layer() {
    let (c, t) = (cx("C"), table());
    let f = FrozenStore::freeze(&base()).unwrap();
    let mut s = f.thaw();
    s.modify(&c, &t, row("A", 9, 1)).unwrap();
    s.insert(&c, &t, row("D", 4, 1)).unwrap();
    s.rollback();
    assert_eq!(nos(&s), nos(&base()));
    s.insert(&c, &t, row("D", 4, 1)).unwrap();
    s.commit();
    s.modify(&c, &t, row("A", 9, 1)).unwrap();
    s.rollback();
    assert_eq!(
        s.count(&c, &t, &[]).unwrap(),
        4,
        "the committed insert stays, the modify goes"
    );
    assert_eq!(nos(&f.thaw()), nos(&base()), "neither reached the frozen tables");
}

#[test]
fn a_blob_write_in_one_thread_does_not_reach_the_base() {
    let (c, t) = (cx("C"), table());
    let mut s = MemStore::default();
    let blob = al_builtins::stream::new_blob();
    blob.borrow_mut().bytes = vec![1, 2, 3];
    s.insert(
        &c,
        &t,
        vec![Value::Code("A".into()), Value::Blob(blob), Value::Integer(1)],
    )
    .unwrap();
    s.commit();
    let f = std::sync::Arc::new(FrozenStore::freeze(&s).unwrap());
    let g = f.clone();
    std::thread::spawn(move || {
        let mut s = g.thaw();
        let fresh = al_builtins::stream::new_blob();
        fresh.borrow_mut().bytes = vec![9];
        s.modify(
            &c,
            &t,
            vec![Value::Code("A".into()), Value::Blob(fresh), Value::Integer(1)],
        )
        .unwrap();
        s.commit();
    })
    .join()
    .unwrap();
    let r = f.thaw().stored_rows(&cx("C"), &table());
    let Value::Blob(b) = &r[0][1] else { panic!() };
    assert_eq!(b.borrow().bytes, vec![1, 2, 3]);
}

#[test]
fn frozen_rows_are_shared_and_refuse_a_handle() {
    fn check<T: Send + Sync>() {}
    check::<FrozenRows>();
    let t = table();
    let rows = Rows::from_rows(&t, vec![row("A", 1, 1), row("B", 2, 1)]).unwrap();
    let f = std::sync::Arc::new(FrozenRows::freeze(rows).ok().unwrap());
    let g = f.clone();
    assert_eq!(std::thread::spawn(move || g.thaw().len()).join().unwrap(), 2);
    let with_list = vec![
        Value::Code("C".into()),
        Value::List(Default::default()),
        Value::Integer(1),
    ];
    assert!(FrozenRows::freeze(Rows::from_rows(&t, vec![with_list]).unwrap()).is_err());
}

#[test]
fn freeze_names_the_table_field_and_kind_that_block_it() {
    let (c, t) = (cx("C"), table());
    let mut s = base();
    s.insert(
        &c,
        &t,
        vec![
            Value::Code("D".into()),
            Value::Integer(1),
            Value::List(Default::default()),
        ],
    )
    .unwrap();
    s.commit();
    let why = FrozenStore::freeze(&s).err().expect("a List blocks it");
    assert_eq!(
        why,
        NotShareable {
            company: "C".into(),
            object: t.object,
            in_key: false,
            position: 2,
            kind: "List of 0 elements".into()
        }
    );
    assert_eq!(not_shareable(&Value::Integer(1)), None);
    assert_eq!(not_shareable(&Value::TextConst("x".into())), Some("TextConst"));
}

#[test]
fn a_frozen_store_and_frozen_rows_say_about_how_many_bytes_they_hold() {
    let small = FrozenStore::freeze(&base()).unwrap();
    let (c, t) = (cx("C"), table());
    let mut s = base();
    for k in 0..100 {
        s.insert(&c, &t, row(&format!("N{k:03}"), k, 1)).unwrap();
    }
    s.commit();
    let big = FrozenStore::freeze(&s).unwrap();
    let per_row = std::mem::size_of::<Value>() as u64 * 3;
    assert!(small.bytes() >= 3 * per_row, "{}", small.bytes());
    assert!(
        big.bytes() >= small.bytes() + 100 * per_row,
        "{} vs {}",
        big.bytes(),
        small.bytes()
    );
    let rows = Rows::from_rows(&t, (0..103).map(|k| row(&format!("N{k:03}"), k, 1)).collect()).unwrap();
    let frozen = FrozenRows::freeze(rows).ok().unwrap();
    assert!(frozen.bytes() >= 103 * per_row);
}

fn record_id(no: &str) -> Value {
    Value::Library(
        al_builtins::library_kind::LibraryKind::RecordId,
        std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::RecordId {
            table: 1,
            table_no: 18,
            name: "Customer".into(),
            key: vec![Value::Code(no.into())],
        })),
    )
}

/// A large partner app's install code leaves Job Queue Entry rows with a RecordId field and report selection rows with a
/// TableFilter field. The store is still frozen: those tables are held as their bytes and each thread reads its own copy.
#[test]
fn a_table_with_a_record_id_or_table_filter_is_frozen_and_each_thread_gets_its_own_copy() {
    let (c, t) = (cx("C"), table());
    let mut s = base();
    let filter = Value::Library(
        "TableFilter".into(),
        std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::Text(
            "Customer: No.=10000".into(),
        ))),
    );
    s.insert(&c, &t, vec![Value::Code("D".into()), record_id("10000"), filter])
        .unwrap();
    s.commit();
    let f = std::sync::Arc::new(FrozenStore::freeze(&s).expect("a RecordId and a TableFilter have a plain form"));
    assert_eq!(f.row_count(), 4);
    assert_eq!(nos(&f.thaw()), nos(&s));
    let handle = |s: &MemStore| match &s.stored_rows(&c, &t)[3][1] {
        Value::Library(_, cell) => cell.clone(),
        other => panic!("a RecordId, not {other:?}"),
    };
    let (a, b) = (f.thaw(), f.thaw());
    assert!(
        !std::rc::Rc::ptr_eq(&handle(&a), &handle(&b)),
        "a RecordId of each thread's own"
    );
    let other = f.clone();
    let seen = std::thread::spawn(move || nos(&other.thaw())).join().unwrap();
    assert_eq!(seen, nos(&s));
}
