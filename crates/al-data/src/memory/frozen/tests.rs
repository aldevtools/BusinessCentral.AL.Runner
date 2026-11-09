//! ADR-069 conformance: a frozen store is shared by threads, and a write of one thread's store never reaches the frozen
//! tables or another thread's store.

use super::*;
use crate::Store;
use crate::contract::{cx, row, table};

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
    assert!(
        FrozenStore::freeze(&s).is_some(),
        "an uncommitted row is not part of it"
    );
    s.commit();
    assert!(FrozenStore::freeze(&s).is_none(), "a List is an Rc handle");
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
