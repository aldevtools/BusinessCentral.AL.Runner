//! Every way into the store records: the `Store` methods precisely, every other method through the catch-all or its own
//! note (ADR-079 decision 1, "every gap a conflict").

use super::MemStore;
use super::footprint::{Resource, StoreFootprint};
use crate::contract::{cx, row, table};
use crate::{SnapshotStore, Store};

/// A store with one committed row "B" in company "C", recording from now on.
fn recording() -> MemStore {
    let mut s = MemStore::default();
    s.insert(&cx("C"), &table(), row("B", 1, 1)).unwrap();
    s.commit();
    s.footprint_start();
    s
}

fn take(s: &mut MemStore) -> StoreFootprint {
    s.footprint_take().expect("a footprint was recorded")
}

fn extent() -> Resource {
    Resource::Extent("C".into(), table().object)
}

fn whole() -> Resource {
    Resource::Table(table().object)
}

#[test]
fn a_precise_get_notes_its_row_and_no_whole_table_read() {
    let mut s = recording();
    s.get(&cx("C"), &table(), &[row("B", 1, 1)[0].clone()]).unwrap();
    let f = take(&mut s);
    assert_eq!(f.reads.len(), 1, "{f:?}");
    assert!(matches!(f.reads.first(), Some(Resource::Row(..))));
}

#[test]
fn reads_outside_the_store_methods_read_the_whole_table() {
    let (c, t) = (cx("C"), table());
    let key = [row("B", 1, 1)[0].clone()];
    for (name, read) in [
        (
            "stored_rows",
            Box::new(|s: &MemStore| {
                let _ = s.stored_rows(&c, &t);
            }) as Box<dyn Fn(&MemStore)>,
        ),
        (
            "row_field_holds",
            Box::new(|s: &MemStore| {
                let _ = s.row_field_holds(&c, &t, &key, 1, |_| true);
            }),
        ),
        (
            "field_holders_from_last",
            Box::new(|s: &MemStore| {
                let _ = s.field_holders_from_last(&c, &t, &key, None, 1, 2, |_| true);
            }),
        ),
        (
            "count_field_holders_from_last",
            Box::new(|s: &MemStore| {
                let _ = s.count_field_holders_from_last(&c, &t, &key, None, 1, 2, |_| true);
            }),
        ),
        (
            "has_rows",
            Box::new(|s: &MemStore| {
                let _ = s.has_rows(t.object);
            }),
        ),
        (
            "companies_with_rows",
            Box::new(|s: &MemStore| {
                let _ = s.companies_with_rows(t.object);
            }),
        ),
        (
            "backed_rows",
            Box::new(|s: &MemStore| {
                let _ = s.backed_rows("C", t.object);
            }),
        ),
    ] {
        let mut s = recording();
        read(&s);
        let f = take(&mut s);
        assert!(f.reads.contains(&whole()), "{name}: {f:?}");
        assert!(f.writes.is_empty(), "{name}: {f:?}");
    }
}

#[test]
fn reads_of_whether_or_how_a_table_changed_read_its_extent_and_a_committed_row_its_row() {
    let (c, t) = (cx("C"), table());
    for (name, read) in [
        (
            "committed_stamp",
            Box::new(|s: &MemStore| {
                let _ = s.committed_stamp(&c, &t);
            }) as Box<dyn Fn(&MemStore)>,
        ),
        (
            "written_since_commit",
            Box::new(|s: &MemStore| {
                let _ = s.written_since_commit(&c, &t);
            }),
        ),
        (
            "table_changed",
            Box::new(|s: &MemStore| {
                let _ = s.table_changed(&c, &t);
            }),
        ),
        (
            "table_stamp",
            Box::new(|s: &MemStore| {
                let _ = s.table_stamp(&c, &t);
            }),
        ),
        (
            "emptied",
            Box::new(|s: &MemStore| {
                let _ = s.emptied(&c, &t);
            }),
        ),
    ] {
        let mut s = recording();
        read(&s);
        let f = take(&mut s);
        assert_eq!(f.reads.iter().collect::<Vec<_>>(), vec![&extent()], "{name}");
    }
    let mut s = recording();
    s.committed_get(&c, &t, &[row("B", 1, 1)[0].clone()]);
    let f = take(&mut s);
    assert!(matches!(f.reads.first(), Some(Resource::Row(..))), "{f:?}");
}

#[test]
fn replace_all_reads_and_writes_the_extent_and_writes_each_new_row() {
    let mut s = recording();
    s.replace_all(&cx("C"), &table(), vec![row("A", 1, 1)]).unwrap();
    let f = take(&mut s);
    assert!(f.reads.contains(&extent()) && f.writes.contains(&extent()), "{f:?}");
    assert_eq!(
        f.writes.iter().filter(|r| matches!(r, Resource::Row(..))).count(),
        1,
        "{f:?}"
    );
}

#[test]
fn a_copy_without_rows_reads_the_extent_and_writes_nothing() {
    let mut s = recording();
    let view = s
        .without_rows(&cx("C"), &table(), &[vec![row("B", 1, 1)[0].clone()]])
        .unwrap();
    let _ = view;
    let f = take(&mut s);
    assert!(f.writes.is_empty(), "{f:?}");
    assert!(f.reads.contains(&extent()), "{f:?}");
}

#[test]
fn whole_store_reads_and_company_changes_touch_everything() {
    type Op = Box<dyn Fn(&mut MemStore)>;
    let ops: Vec<(&str, Op)> = vec![
        (
            "nonempty_tables",
            Box::new(|s| {
                let _ = s.nonempty_tables();
            }),
        ),
        (
            "has_company_rows",
            Box::new(|s| {
                let _ = s.has_company_rows("C");
            }),
        ),
        (
            "company_changed",
            Box::new(|s| {
                let _ = s.company_changed("C");
            }),
        ),
        (
            "export_current",
            Box::new(|s| {
                let _ = s.export_current();
            }),
        ),
        (
            "export_committed",
            Box::new(|s| {
                let _ = s.export_committed();
            }),
        ),
        (
            "committed_table",
            Box::new(|s| {
                let _ = s.committed_table("C", 7);
            }),
        ),
        (
            "committed_row_count",
            Box::new(|s| {
                let _ = s.committed_row_count();
            }),
        ),
        (
            "committed_marks",
            Box::new(|s| {
                let _ = s.committed_marks();
            }),
        ),
        (
            "first_value_where",
            Box::new(|s| {
                let _ = s.first_value_where(|_| false);
            }),
        ),
        (
            "committed_changes_from",
            Box::new(|s| {
                let _ = s.committed_changes_from(&MemStore::default());
            }),
        ),
        ("copy_company", Box::new(|s| s.copy_company("C", "D"))),
        ("rename_company", Box::new(|s| s.rename_company("C", "D"))),
        ("drop_company", Box::new(|s| s.drop_company("C"))),
        ("note_copy", Box::new(|s| s.note_copy("D", "C"))),
    ];
    for (name, op) in ops {
        let mut s = recording();
        op(&mut s);
        assert!(!take(&mut s).everything.is_empty(), "{name}");
    }
}

#[test]
fn reads_of_changes_count_only_the_session_s_own_writes() {
    let mut s = MemStore::default();
    let before = s.change_counter();
    s.insert(&cx("C"), &table(), row("B", 1, 1)).unwrap();
    s.footprint_start();
    let at_start = s.change_counter();
    s.tables_changed_since(at_start);
    s.tables_rolled_back_since(at_start);
    s.has_uncommitted();
    let f = take(&mut s);
    assert_eq!(
        f.everything.len(),
        1,
        "only the pending write from before counts: {f:?}"
    );
    s.footprint_start();
    s.tables_changed_since(before);
    assert!(!take(&mut s).everything.is_empty(), "changes since before the session");
    s.footprint_start();
    s.tables_rolled_back_since(before);
    assert!(
        !take(&mut s).everything.is_empty(),
        "rolled back since before the session"
    );
}

#[test]
fn a_whole_table_resource_meets_every_resource_of_its_table() {
    let row = Resource::Row("C".into(), 7, vec![1]);
    assert!(whole().writes_collide(&row) && row.writes_collide(&whole()));
    assert!(whole().read_sees_write(&row) && row.read_sees_write(&whole()));
    assert!(extent().read_sees_write(&whole()));
    assert!(!whole().writes_collide(&Resource::Row("C".into(), 8, vec![1])));
    assert!(
        !Resource::Extent("D".into(), 7).read_sees_write(&extent()),
        "another company"
    );
}

#[test]
fn a_snapshot_put_back_keeps_the_recording() {
    let mut s = recording();
    let snap = s.snapshot();
    s.stored_rows(&cx("C"), &table());
    s.restore(snap);
    assert!(take(&mut s).reads.contains(&whole()));
}

#[test]
fn the_safety_net_finds_a_committed_row_change_no_write_covers() {
    let mut before = MemStore::default();
    before.insert(&cx("C"), &table(), row("B", 1, 1)).unwrap();
    before.commit();
    let mut after = before.clone();
    after.modify(&cx("C"), &table(), row("B", 2, 1)).unwrap();
    after.insert(&cx("C"), &table(), row("D", 1, 1)).unwrap();
    after.commit();
    let key = |k: &str| after.footprint_key(&[row(k, 1, 1)[0].clone()]);
    let object = table().object;
    let row_of = |k: &str| Resource::Row("C".into(), object, key(k));
    // Nothing recorded: both rows are unrecorded writes.
    assert!(after.unrecorded_writes(&before, &Default::default()).is_err());
    // One row recorded: the other is still found.
    let one = [row_of("B")].into_iter().collect();
    assert!(after.unrecorded_writes(&before, &one).is_err());
    // Both rows recorded, or the table written whole: nothing is left.
    let both = [row_of("B"), row_of("D")].into_iter().collect();
    assert_eq!(after.unrecorded_writes(&before, &both), Ok(()));
    let all = [whole()].into_iter().collect();
    assert_eq!(after.unrecorded_writes(&before, &all), Ok(()));
    // No change: nothing to cover.
    assert_eq!(before.unrecorded_writes(&before.clone(), &Default::default()), Ok(()));
}
