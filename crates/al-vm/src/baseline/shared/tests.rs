//! ADR-069 conformance of the shared install baseline.

use super::*;
use al_builtins::value::Value;
use al_data::memory::ExportedTable;

fn sample() -> Baseline {
    crate::baseline::tests::sample()
}

#[test]
fn a_shared_baseline_is_send_and_sync() {
    fn check<T: Send + Sync>() {}
    check::<SharedBaseline>();
}

#[test]
fn a_thread_starts_from_what_a_copy_from_bytes_holds() {
    let b = sample();
    let shared = b.freeze().unwrap();
    let thawed = shared.thaw(None);
    let copy = Baseline::from_bytes(&b.to_bytes().unwrap()).unwrap();
    assert_eq!(thawed.to_bytes(), copy.to_bytes());
    assert_eq!(thawed.origin(), b.origin(), "it holds what the frozen baseline holds");
    assert_eq!(shared.rows(), 1);
    assert!(shared.unchanged());
}

#[test]
fn freeze_refuses_a_value_that_is_not_plain() {
    let mut b = sample();
    let list = Value::List(Default::default());
    b.store = al_data::memory::MemStore::from_exported(vec![ExportedTable {
        company: "CRONUS".into(),
        object: 311,
        rows: vec![(vec![Value::Code("".into())], vec![Value::Code("".into()), list])],
    }]);
    let Err(NotShared::Value(why)) = b.freeze() else {
        panic!("each thread then keeps a copy of its own")
    };
    assert_eq!(
        (
            why.company.as_str(),
            why.object,
            why.in_key,
            why.position,
            why.kind.as_str()
        ),
        ("CRONUS", 311, false, 1, "List of 0 elements")
    );
}

#[test]
fn a_write_on_one_thread_never_reaches_the_base_or_another_thread() {
    let shared = std::sync::Arc::new(sample().freeze().unwrap());
    let before = shared.thaw(None).to_bytes();
    let s = shared.clone();
    std::thread::spawn(move || {
        let mut b = s.thaw(None);
        let row = vec![Value::Code("".into()), Value::Integer(99)];
        let t = ExportedTable {
            company: "CRONUS".into(),
            object: 311,
            rows: vec![(vec![Value::Code("".into())], row)],
        };
        b.store = b.store.with_committed_tables(vec![(t, false)], &[("".into(), 7)]);
        b.next_guid += 1;
        assert_ne!(
            b.to_bytes(),
            s.thaw(None).to_bytes(),
            "the write is in this thread's store"
        );
    })
    .join()
    .unwrap();
    assert_eq!(shared.thaw(None).to_bytes(), before);
    assert!(shared.unchanged());
}

#[test]
fn a_baseline_that_cannot_be_written_as_bytes_names_the_value() {
    let mut b = sample();
    let rid = Value::Library(
        al_builtins::library_kind::LibraryKind::RecordId,
        std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::RecordId {
            table: 1,
            table_no: 18,
            name: "Customer".into(),
            key: vec![Value::Code("10000".into())],
        })),
    );
    let json = Value::Library("JsonObject".into(), Default::default());
    b.store = al_data::memory::MemStore::from_exported(vec![ExportedTable {
        company: "CRONUS".into(),
        object: 311,
        rows: vec![(vec![Value::Code("".into())], vec![Value::Code("".into()), rid, json])],
    }]);
    assert!(
        matches!(b.freeze(), Err(NotShared::Value(ref why)) if why.kind == "Library JsonObject holding Empty"),
        "the JsonObject has no plain form; the RecordId has one"
    );
    assert!(b.to_bytes().is_none());
    let why = b.first_without_bytes().expect("the JsonObject has no form");
    assert_eq!((why.position, why.kind.as_str()), (2, "JsonObject"));
}

/// The install baseline of a large partner app holds Job Queue Entry rows with a RecordId field and rows with a
/// TableFilter field. It is still shared: those tables are a copy per thread, read from their bytes.
#[test]
fn a_baseline_with_a_record_id_and_a_table_filter_is_shared() {
    let mut b = sample();
    let rid = Value::Library(
        al_builtins::library_kind::LibraryKind::RecordId,
        std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::RecordId {
            table: 1,
            table_no: 18,
            name: "Customer".into(),
            key: vec![Value::Code("10000".into())],
        })),
    );
    let filter = Value::Library(
        "TableFilter".into(),
        std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::Text(
            "Customer: No.=10000".into(),
        ))),
    );
    b.store = al_data::memory::MemStore::from_exported(vec![
        ExportedTable {
            company: "CRONUS".into(),
            object: 311,
            rows: vec![(
                vec![Value::Code("".into())],
                vec![Value::Code("".into()), Value::Integer(1)],
            )],
        },
        ExportedTable {
            company: "CRONUS".into(),
            object: 472,
            rows: vec![(vec![Value::Integer(1)], vec![Value::Integer(1), rid, filter])],
        },
    ]);
    let shared = b.freeze().expect("a RecordId and a TableFilter have a plain form");
    assert_eq!(shared.rows(), 2);
    assert_eq!(shared.held_tables(), vec![("CRONUS".to_string(), 472, 1)]);
    let copy = Baseline::from_bytes(&b.to_bytes().unwrap()).unwrap();
    assert_eq!(shared.thaw(None).to_bytes(), copy.to_bytes());
    assert!(shared.unchanged());
}
