use super::*;

// Review of al-runner PR 8119: the test runner asks which tables install code wrote since it started that a rollback
// now takes back: not a write before it started, nor one it committed.
#[test]
fn the_store_tells_which_tables_a_rollback_takes_back_of_the_writes_since_a_mark() {
    use crate::contract::{cx, row, table};
    let c = cx("C");
    let at = |object| {
        let mut t = table();
        t.object = object;
        t
    };
    let mut s = MemStore::default();
    s.insert(&c, &at(1), row("A", 1, 1)).unwrap();
    let mark = s.change_counter();
    s.insert(&c, &at(2), row("A", 1, 1)).unwrap();
    assert_eq!(
        s.tables_rolled_back_since(mark),
        [2].into(),
        "a write before the mark is not counted"
    );
    s.commit();
    assert!(
        s.tables_rolled_back_since(mark).is_empty(),
        "a committed write is not taken back"
    );
    s.insert(&c, &at(3), row("A", 1, 1)).unwrap();
    assert_eq!(s.tables_rolled_back_since(mark), [3].into());
    s.rollback();
    assert!(s.tables_rolled_back_since(mark).is_empty());
}

// al-runner issue 5641: the test runner asks which tables a Company Triggers subscriber wrote while it ran, and whether a
// test read or wrote one of them.
#[test]
fn the_store_tells_which_tables_changed_and_whether_a_watched_one_was_accessed() {
    use crate::contract::{cx, row, table};
    let (c, t) = (cx("C"), table());
    let mut other = table();
    other.object = 8;
    let mut s = MemStore::default();
    s.insert(&c, &other, row("A", 1, 1)).unwrap();
    s.commit();
    let before = s.change_counter();
    s.insert(&c, &t, row("A", 1, 1)).unwrap();
    assert_eq!(s.tables_changed_since(before), [7].into());
    s.rollback();
    assert!(
        s.tables_changed_since(before).is_empty(),
        "a rolled back write is not a change"
    );
    s.insert(&c, &t, row("A", 1, 1)).unwrap();
    s.commit();
    s.watch([7].into());
    let snap = s.clone();
    s.count(&c, &other, &[]).unwrap();
    assert_eq!(s.watch_hit(), None);
    s.count(&c, &t, &[]).unwrap();
    assert_eq!(s.watch_hit(), Some(7));
    assert_eq!(snap.watch_hit(), Some(7), "the clones of the store share the watch");
    s.watch(Default::default());
    s.count(&c, &t, &[]).unwrap();
    assert_eq!(s.watch_hit(), None);
    // A window on top of the watch (`watch_window`): it notes only what is accessed while it is open, whatever the
    // watch noted before, and the watch keeps its own.
    s.watch([7, 8].into());
    s.count(&c, &t, &[]).unwrap();
    s.watch_window([7].into());
    s.count(&c, &other, &[]).unwrap();
    assert_eq!(s.watch_window_end(), None, "7 was read before the window opened");
    s.watch_window([7].into());
    s.count(&c, &t, &[]).unwrap();
    assert_eq!(s.watch_window_end(), Some(7));
    assert_eq!(s.watch_hit(), Some(7), "the watch keeps its own");
    assert_eq!(s.watch_window_end(), None, "closed");
}

// ---- the key seek (ADR-035): a scan starts where the filters on the leading key fields start ----

use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::value::RtType;

fn seek_table(keys: Vec<Vec<usize>>) -> TableDef {
    let f = |id, name: &str, ty| FieldDef {
        id,
        name: name.into(),
        ty,
        stored: true,
        code_storage: Default::default(),
    };
    TableDef {
        object: 9,
        name: "Seek".into(),
        fields: vec![
            f(1, "A", RtType::Integer),
            f(2, "B", RtType::Integer),
            f(3, "C", RtType::Code(Some(10))),
            f(4, "D", RtType::Integer),
        ],
        keys,
        per_company: true,
    }
}

fn cx() -> AccessContext {
    AccessContext {
        company: "C".into(),
        user: "U".into(),
    }
}

fn range(field: usize, lo: Option<Value>, hi: Option<Value>) -> FieldFilter {
    FieldFilter {
        lang: al_builtins::filter::filter_language(),
        field,
        expr: FilterExpr::Range(lo, hi),
        group: 0,
    }
}

/// The rows a scan returns, one `next` after the other.
fn scan_all(s: &MemStore, t: &TableDef, key: usize, filters: &[FieldFilter], ascending: bool) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::new();
    let mut after: Option<Vec<Value>> = None;
    while let Some(r) = s
        .next(
            &cx(),
            t,
            &Scan {
                key,
                filters,
                ascending,
                after: after.as_deref(),
            },
        )
        .unwrap()
    {
        after = Some(crate::position(t, key, &r));
        out.push(r);
    }
    out
}

/// What the scan has to return, without any seek: every row, in the order of the key, that passes.
fn reference(s: &MemStore, t: &TableDef, key: usize, filters: &[FieldFilter], ascending: bool) -> Vec<Row> {
    let mut rows: Vec<Row> = s.table(&cx(), t).unwrap().values().cloned().collect();
    rows.sort_by(|a, b| cmp_rows(t, key, a, b));
    if !ascending {
        rows.reverse();
    }
    rows.retain(|r| al_builtins::filter::row_passes_checked(r, filters));
    rows
}

mod count_tests;
mod seek_tests;

#[test]
fn passes_the_store_contract() {
    crate::contract::run_all(MemStore::default);
}

#[test]
fn dropping_a_company_removes_its_rows_and_a_rollback_restores_them() {
    let cx = |c: &str| AccessContext {
        company: c.into(),
        user: "U".into(),
    };
    let t = TableDef {
        object: 7,
        name: "T".into(),
        fields: vec![FieldDef {
            id: 1,
            name: "K".into(),
            ty: al_builtins::value::RtType::Integer,
            stored: true,
            code_storage: Default::default(),
        }],
        keys: vec![vec![0]],
        per_company: true,
    };
    let mut s = MemStore::default();
    s.insert(&cx("A"), &t, vec![Value::Integer(1)]).unwrap();
    s.insert(&cx("B"), &t, vec![Value::Integer(1)]).unwrap();
    s.commit();
    assert!(
        s.has_company_rows("A") && !s.has_company_rows("a"),
        "company names are case sensitive"
    );
    assert_eq!(s.companies_with_rows(7).len(), 2);
    let before = s.table_stamp(&cx("B"), &t);
    s.drop_company("B");
    assert!(!s.has_company_rows("B") && s.has_company_rows("A"));
    assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 0);
    assert_eq!(s.count(&cx("A"), &t, &[]).unwrap(), 1);
    assert_ne!(s.table_stamp(&cx("B"), &t), before, "queries see the change");
    s.rollback();
    assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 1);
    s.rename_company("B", "C");
    assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 0);
    assert_eq!(s.count(&cx("C"), &t, &[]).unwrap(), 1);
    s.rollback();
    assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 1);
}

#[test]
fn copying_a_company_copies_its_rows_as_they_are_and_a_rollback_removes_them() {
    let cx = |c: &str| AccessContext {
        company: c.into(),
        user: "U".into(),
    };
    let field = |id| FieldDef {
        id,
        name: "K".into(),
        ty: al_builtins::value::RtType::Integer,
        stored: true,
        code_storage: Default::default(),
    };
    let t = TableDef {
        object: 7,
        name: "T".into(),
        fields: vec![field(1), field(2)],
        keys: vec![vec![0]],
        per_company: true,
    };
    let mut s = MemStore::default();
    s.insert(&cx("A"), &t, vec![Value::Integer(1), Value::Integer(10)])
        .unwrap();
    s.insert(&cx("A"), &t, vec![Value::Integer(2), Value::Integer(20)])
        .unwrap();
    s.insert(&cx("B"), &t, vec![Value::Integer(9), Value::Integer(90)])
        .unwrap();
    s.commit();
    let before = s.table_stamp(&cx("C"), &t);
    s.copy_company("A", "C");
    assert_eq!(s.count(&cx("C"), &t, &[]).unwrap(), 2);
    let row = s.get(&cx("C"), &t, &[Value::Integer(2)]).unwrap().unwrap();
    assert!(matches!(row[1], Value::Integer(20)), "the row is copied as it is");
    assert_eq!(s.count(&cx("A"), &t, &[]).unwrap(), 2, "the source keeps its rows");
    assert_ne!(s.table_stamp(&cx("C"), &t), before, "queries see the change");
    s.insert(&cx("C"), &t, vec![Value::Integer(3), Value::Integer(30)])
        .unwrap();
    assert_eq!(
        s.count(&cx("A"), &t, &[]).unwrap(),
        2,
        "the copy is apart from the source"
    );
    s.rollback();
    assert_eq!(s.count(&cx("C"), &t, &[]).unwrap(), 0);
}

#[test]
fn tells_whether_a_company_changed_in_the_open_transaction() {
    let cx = |c: &str| AccessContext {
        company: c.into(),
        user: "U".into(),
    };
    let t = TableDef {
        object: 7,
        name: "T".into(),
        fields: vec![FieldDef {
            id: 1,
            name: "K".into(),
            ty: al_builtins::value::RtType::Integer,
            stored: true,
            code_storage: Default::default(),
        }],
        keys: vec![vec![0]],
        per_company: true,
    };
    let mut s = MemStore::default();
    s.insert(&cx("A"), &t, vec![Value::Integer(1)]).unwrap();
    assert!(s.company_changed("A") && !s.company_changed("B"));
    s.commit();
    assert!(!s.company_changed("A"));
    s.insert(&cx("B"), &t, vec![Value::Integer(1)]).unwrap();
    assert!(!s.company_changed("A") && s.company_changed("B"));
}

#[test]
fn reports_uncommitted_writes() {
    let cx = AccessContext {
        company: "C".into(),
        user: "U".into(),
    };
    let t = TableDef {
        object: 0,
        name: "T".into(),
        fields: vec![FieldDef {
            id: 1,
            name: "K".into(),
            ty: al_builtins::value::RtType::Integer,
            stored: true,
            code_storage: Default::default(),
        }],
        keys: vec![vec![0]],
        per_company: true,
    };
    let mut s = MemStore::default();
    assert!(!s.has_uncommitted());
    s.insert(&cx, &t, vec![Value::Integer(1)]).unwrap();
    assert!(s.has_uncommitted());
    s.commit();
    assert!(!s.has_uncommitted());
    s.insert(&cx, &t, vec![Value::Integer(2)]).unwrap();
    s.rollback();
    assert!(!s.has_uncommitted());
}

#[test]
fn table_stamp_changes_with_every_write_and_rollback() {
    let cx = AccessContext {
        company: "C".into(),
        user: "U".into(),
    };
    let t = TableDef {
        object: 0,
        name: "T".into(),
        fields: vec![FieldDef {
            id: 1,
            name: "K".into(),
            ty: al_builtins::value::RtType::Integer,
            stored: true,
            code_storage: Default::default(),
        }],
        keys: vec![vec![0]],
        per_company: true,
    };
    let mut s = MemStore::default();
    let s0 = s.table_stamp(&cx, &t);
    s.insert(&cx, &t, vec![Value::Integer(1)]).unwrap();
    let s1 = s.table_stamp(&cx, &t);
    assert_ne!(s0, s1, "an insert changes the stamp");
    s.insert(&cx, &t, vec![Value::Integer(1)]).unwrap();
    assert_eq!(s1, s.table_stamp(&cx, &t), "a refused insert does not");
    s.commit();
    s.insert(&cx, &t, vec![Value::Integer(2)]).unwrap();
    let s2 = s.table_stamp(&cx, &t);
    s.rollback();
    assert_eq!(s1, s.table_stamp(&cx, &t), "a rollback restores the committed stamp");
    s.insert(&cx, &t, vec![Value::Integer(3)]).unwrap();
    assert_ne!(s2, s.table_stamp(&cx, &t), "a stamp is never reused for other content");
}

// Gap cycle G8: a DeleteAll without filters marks the table emptied (its rows from before are known to be gone); the mark
// follows the rows through commit, rollback and a snapshot. A DeleteAll with filters does not mark it.
#[test]
fn a_delete_all_without_filters_marks_the_table_emptied_until_a_rollback() {
    let cx = AccessContext {
        company: "C".into(),
        user: "U".into(),
    };
    let t = TableDef {
        object: 0,
        name: "T".into(),
        fields: vec![FieldDef {
            id: 1,
            name: "K".into(),
            ty: al_builtins::value::RtType::Integer,
            stored: true,
            code_storage: Default::default(),
        }],
        keys: vec![vec![0]],
        per_company: true,
    };
    let mut s = MemStore::default();
    assert!(!s.emptied(&cx, &t));
    let filter = FieldFilter {
        lang: al_builtins::filter::filter_language(),
        field: 0,
        expr: al_builtins::filter::FilterExpr::Eq(Value::Integer(1)),
        group: 0,
    };
    s.delete_all(&cx, &t, &[filter]).unwrap();
    assert!(!s.emptied(&cx, &t), "a filtered DeleteAll");
    s.delete_all(&cx, &t, &[]).unwrap();
    assert!(s.emptied(&cx, &t));
    s.insert(&cx, &t, vec![Value::Integer(1)]).unwrap();
    assert!(s.emptied(&cx, &t), "rows written after it are known");
    s.rollback();
    assert!(!s.emptied(&cx, &t), "a rollback takes it back");
    s.delete_all(&cx, &t, &[]).unwrap();
    let snap = s.snapshot();
    s.commit();
    s.rollback();
    assert!(s.emptied(&cx, &t), "a commit keeps it");
    let mut fresh = MemStore::default();
    assert!(!fresh.emptied(&cx, &t));
    fresh.restore(snap);
    assert!(fresh.emptied(&cx, &t), "a snapshot holds it");
}

/// Rows a table holds before anything wrote to it: three rows for company `C`, table 7.
struct ThreeRows {
    reads: std::cell::Cell<usize>,
}

impl Backing for ThreeRows {
    fn company(&self) -> &str {
        "C"
    }
    fn rows(&self, company: &str, object: u32) -> Option<Rows> {
        if company != "C" || object != 7 {
            return None;
        }
        self.reads.set(self.reads.get() + 1);
        let t = crate::contract::table();
        Rows::from_rows(
            &t,
            vec![
                crate::contract::row("A", 1, 1),
                crate::contract::row("B", 2, 1),
                crate::contract::row("C", 3, 2),
            ],
        )
        .ok()
    }
}

fn backed() -> (MemStore, std::rc::Rc<ThreeRows>) {
    let b = std::rc::Rc::new(ThreeRows {
        reads: Default::default(),
    });
    (MemStore::default().with_backing(b.clone()), b)
}

#[test]
fn a_table_nothing_wrote_to_reads_the_backing_rows() {
    let (s, _) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
    assert!(s.get(&cx, &t, &[Value::Code("B".into())]).unwrap().is_some());
    // another company and another table see nothing
    assert_eq!(s.count(&crate::contract::cx("D"), &t, &[]).unwrap(), 0);
    let mut other = t.clone();
    other.object = 8;
    assert_eq!(s.count(&cx, &other, &[]).unwrap(), 0);
}

// al-runner issue 6507: a copy of the backed company holds the backing's rows of the tables nothing wrote to
// (session-system/company/Y22CopyCompany.al:Y22_CopyCompany_CopiesCompanyAndRows), apart from the source's.
#[test]
fn a_copy_of_the_backed_company_reads_the_backing_rows() {
    let (mut s, _) = backed();
    let (t, c, d) = (
        crate::contract::table(),
        crate::contract::cx("C"),
        crate::contract::cx("D"),
    );
    s.commit();
    s.copy_company("C", "D");
    assert_eq!(s.count(&d, &t, &[]).unwrap(), 3);
    assert!(s.has_company_rows("D"));
    assert!(s.companies_with_rows(7).contains(&"D".to_string()));
    s.delete(&d, &t, &[Value::Code("A".into())]).unwrap();
    assert_eq!(s.count(&d, &t, &[]).unwrap(), 2);
    assert_eq!(s.count(&c, &t, &[]).unwrap(), 3, "the source keeps its rows");
    // The copy is part of the transaction: a rollback removes it.
    s.rollback();
    assert_eq!(s.count(&d, &t, &[]).unwrap(), 0);
    s.copy_company("C", "D");
    s.commit();
    s.rename_company("D", "E");
    assert_eq!(s.count(&d, &t, &[]).unwrap(), 0);
    assert_eq!(s.count(&crate::contract::cx("E"), &t, &[]).unwrap(), 3);
    s.drop_company("E");
    assert_eq!(s.count(&crate::contract::cx("E"), &t, &[]).unwrap(), 0);
    assert!(!s.has_company_rows("E"));
}

#[test]
fn copying_to_a_deleted_backed_company_restores_its_source_rows() {
    let (mut s, _) = backed();
    let (t, source, copy) = (
        crate::contract::table(),
        crate::contract::cx("C"),
        crate::contract::cx("D"),
    );

    s.drop_company("D");
    s.copy_company("C", "D");

    assert_eq!(s.count(&copy, &t, &[]).unwrap(), 3);
    assert_eq!(s.count(&source, &t, &[]).unwrap(), 3);
}

#[test]
fn noting_a_copy_to_a_deleted_backed_company_restores_its_source_rows() {
    let (mut s, _) = backed();
    let (t, copy) = (crate::contract::table(), crate::contract::cx("D"));

    s.drop_company("D");
    s.note_copy("D", "C");

    assert_eq!(s.count(&copy, &t, &[]).unwrap(), 3);
}

#[test]
fn renaming_to_a_deleted_backed_company_moves_its_source_rows() {
    let (mut s, _) = backed();
    let (t, old, new) = (
        crate::contract::table(),
        crate::contract::cx("C"),
        crate::contract::cx("D"),
    );

    s.drop_company("D");
    s.rename_company("C", "D");

    assert_eq!(s.count(&new, &t, &[]).unwrap(), 3);
    assert_eq!(s.count(&old, &t, &[]).unwrap(), 0);
}

#[test]
fn a_write_starts_from_the_backing_rows() {
    let (mut s, _) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    assert_eq!(
        s.insert(&cx, &t, crate::contract::row("A", 9, 9)).unwrap(),
        InsertOutcome::AlreadyExists,
        "a restored key cannot be inserted again"
    );
    assert_eq!(
        s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap(),
        InsertOutcome::Inserted
    );
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 4);
    assert_eq!(
        s.delete(&cx, &t, &[Value::Code("A".into())]).unwrap(),
        WriteOutcome::Done
    );
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
}

#[test]
fn deleting_every_restored_row_leaves_the_table_empty_not_restored_again() {
    let (mut s, _) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    assert_eq!(s.delete_all(&cx, &t, &[]).unwrap(), 3);
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
    s.commit();
    s.rollback();
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
}

#[test]
fn a_rollback_puts_the_backing_rows_back_and_a_commit_keeps_a_write() {
    let (mut s, _) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
    s.rollback();
    assert_eq!(
        s.count(&cx, &t, &[]).unwrap(),
        3,
        "the restored rows are committed data"
    );
    s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
    s.commit();
    s.rollback();
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 4);
}

#[test]
fn a_snapshot_taken_before_a_touch_still_reads_the_backing_rows() {
    let (s, _) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    let snap = s.snapshot();
    let mut s = s;
    s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
    s.restore(snap);
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
}

#[test]
fn the_company_and_table_queries_include_the_backing_rows() {
    let (s, _) = backed();
    assert!(s.has_rows(7) && !s.has_rows(8));
    assert!(s.has_company_rows("c") && !s.has_company_rows("D"));
    assert_eq!(s.companies_with_rows(7), vec!["C".to_string()]);
}

#[test]
fn dropping_a_backed_company_hides_rows_until_rollback() {
    let (mut s, _) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);

    s.drop_company("C");
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
    assert!(!s.has_company_rows("C"));

    s.rollback();
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
    assert!(s.has_company_rows("C"));

    s.drop_company("C");
    s.commit();
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
}

#[test]
fn changes_to_backing_rows_are_allowed_and_written_down_when_recording() {
    let (mut s, _) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    s.record_restored_changes(true);
    // the same values: allowed, and said to be the same
    let same = s.get(&cx, &t, &[Value::Code("A".into())]).unwrap().unwrap();
    assert_eq!(s.modify(&cx, &t, same).unwrap(), WriteOutcome::Done);
    // other values, a delete, a rename, a DeleteAll: allowed
    assert_eq!(
        s.modify(&cx, &t, crate::contract::row("A", 9, 9)).unwrap(),
        WriteOutcome::Done
    );
    assert_eq!(
        s.rename(&cx, &t, &[Value::Code("B".into())], crate::contract::row("Z", 1, 1))
            .unwrap(),
        RenameOutcome::Renamed
    );
    // a row the code made itself is not a restored row
    s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
    s.modify(&cx, &t, crate::contract::row("D", 5, 3)).unwrap();
    s.delete(&cx, &t, &[Value::Code("D".into())]).unwrap();
    let got: Vec<(&str, bool)> = s.restored_changes().iter().map(|c| (c.op, c.same)).collect();
    assert_eq!(got, vec![("Modify", true), ("Modify", false), ("Rename", false)]);
    assert!(s.restored_changes().iter().all(|c| c.table == t.name));
    s.delete_all(&cx, &t, &[]).unwrap();
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
    s.record_restored_changes(false);
    assert!(s.restored_changes().is_empty());
}

/// Walking a table row by row costs a few comparisons a row, not a pass over the table: 30,000 rows in a
/// second, where a scan that looks at every row for each step needs minutes. Release only
/// (`cargo test --release -p al-data -- --ignored`).
#[test]
#[ignore]
fn walking_a_big_table_row_by_row_is_not_quadratic() {
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    let mut s = MemStore::default();
    for i in 0..30_000 {
        s.insert(&cx, &t, crate::contract::row(&format!("{i:08}"), i, i % 7))
            .unwrap();
    }
    let started = std::time::Instant::now();
    for key in [0, 1] {
        let rows = crate::contract::scan_all(&s, &cx, &t, key, &[], true);
        assert_eq!(rows.len(), 30_000);
    }
    let took = started.elapsed();
    assert!(took < std::time::Duration::from_secs(5), "two walks took {took:?}");
}

#[test]
fn the_backing_is_asked_again_only_while_the_slot_is_absent() {
    let (mut s, b) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
    let after_write = b.reads.get();
    for _ in 0..5 {
        s.count(&cx, &t, &[]).unwrap();
    }
    assert_eq!(
        b.reads.get(),
        after_write,
        "a table the store holds is not read from the backing again"
    );
}

/// The last row in key 1 (`Group`, `Amount`), as a FindLast in a secondary key asks it.
fn last_in_key_1(s: &MemStore) -> Option<String> {
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    let scan = Scan {
        key: 1,
        filters: &[],
        ascending: false,
        after: None,
    };
    s.next(&cx, &t, &scan).unwrap().map(|r| format!("{:?}", r[0]))
}

fn big_store() -> MemStore {
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    let mut s = MemStore::default();
    for i in 0..200 {
        s.insert(&cx, &t, crate::contract::row(&format!("{i:04}"), i, i % 7))
            .unwrap();
    }
    s.commit();
    s
}

// Al-runner issue 5884: every test session starts from the same snapshot of the company and asks Base Application's
// login code for the last G/L entry by Posting Date. The order of the table in that key is built once, not once per
// session: a store restored to a snapshot keeps the orders its clones built for the same content.
#[test]
fn a_restored_snapshot_keeps_the_order_built_in_another_key() {
    let mut s = big_store();
    let snap = s.snapshot();
    let before = order_builds();
    assert_eq!(last_in_key_1(&s).as_deref(), Some("Code(\"0195\")"));
    s.restore(snap.clone());
    assert_eq!(last_in_key_1(&s).as_deref(), Some("Code(\"0195\")"));
    let mut other = snap.clone();
    assert_eq!(last_in_key_1(&other).as_deref(), Some("Code(\"0195\")"));
    assert_eq!(
        order_builds() - before,
        1,
        "the order of key 1 is built once for the same rows"
    );
    // A write in one clone changes its order and not the order of the snapshot or of the other clones.
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    other.insert(&cx, &t, crate::contract::row("X", 1000, 6)).unwrap();
    assert_eq!(last_in_key_1(&other).as_deref(), Some("Code(\"X\")"));
    assert_eq!(last_in_key_1(&s).as_deref(), Some("Code(\"0195\")"));
    s.restore(snap);
    assert_eq!(last_in_key_1(&s).as_deref(), Some("Code(\"0195\")"));
}

#[test]
fn sessions_that_write_between_the_reads_of_the_snapshot_keep_its_order() {
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    let snap = big_store();
    let before = order_builds();
    for i in 0..10 {
        // A session opens on the snapshot, reads, writes and reads again.
        let mut s = snap.clone();
        assert_eq!(last_in_key_1(&s).as_deref(), Some("Code(\"0195\")"));
        s.insert(&cx, &t, crate::contract::row(&format!("W{i}"), 1000, 6))
            .unwrap();
        assert_eq!(last_in_key_1(&s), Some(format!("Code(\"W{i}\")")));
    }
    assert_eq!(
        order_builds() - before,
        1,
        "the snapshot's order once; each session's write carries it (order_carry)"
    );
}

#[test]
fn two_clones_that_write_different_rows_each_see_their_own_order() {
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    let snap = big_store();
    let mut a = snap.clone();
    let mut b = snap.clone();
    a.insert(&cx, &t, crate::contract::row("A", 1000, 6)).unwrap();
    b.insert(&cx, &t, crate::contract::row("B", 1000, 6)).unwrap();
    assert_eq!(last_in_key_1(&a).as_deref(), Some("Code(\"A\")"));
    assert_eq!(last_in_key_1(&b).as_deref(), Some("Code(\"B\")"));
    // A modify that moves a row in key 1, in each clone the other way.
    let mut a2 = snap.clone();
    let mut b2 = snap.clone();
    a2.modify(&cx, &t, crate::contract::row("0001", 2000, 6)).unwrap();
    b2.modify(&cx, &t, crate::contract::row("0002", 2000, 6)).unwrap();
    assert_eq!(last_in_key_1(&a2).as_deref(), Some("Code(\"0001\")"));
    assert_eq!(last_in_key_1(&b2).as_deref(), Some("Code(\"0002\")"));
}

// Al-runner issue 6924: No. Series Line has the keys ("Series Code", "Starting Date", ...) and ("Last Date Used").
// Each number taken sets "Last Date Used", which moves the row in the second key only; the next read in the first
// key must not sort the table again.
#[test]
fn a_modify_that_moves_a_row_in_one_key_keeps_the_order_of_the_others() {
    let mut t = crate::contract::table();
    t.keys = vec![vec![0], vec![2], vec![1]];
    let cx = crate::contract::cx("C");
    let mut s = MemStore::default();
    for i in 0..200 {
        s.insert(&cx, &t, crate::contract::row(&format!("{i:04}"), i, i % 7))
            .unwrap();
    }
    s.commit();
    let last = |s: &MemStore, key: usize| {
        let scan = Scan {
            key,
            filters: &[],
            ascending: false,
            after: None,
        };
        s.next(&cx, &t, &scan).unwrap().map(|r| format!("{:?}", r[0]))
    };
    assert_eq!(last(&s, 1).as_deref(), Some("Code(\"0195\")"));
    assert_eq!(last(&s, 2).as_deref(), Some("Code(\"0199\")"));
    // Amount (key 2) changes, Group (key 1) stays.
    s.modify(&cx, &t, crate::contract::row("0003", 5000, 3)).unwrap();
    let before = order_builds();
    assert_eq!(last(&s, 1).as_deref(), Some("Code(\"0195\")"));
    assert_eq!(order_builds() - before, 0, "key 1 keeps its order");
    assert_eq!(last(&s, 2).as_deref(), Some("Code(\"0003\")"));
    assert_eq!(
        order_builds() - before,
        0,
        "key 2 carries its order over the move (order_carry)"
    );
    // A rollback brings back the order of key 2 from before the Modify.
    s.rollback();
    assert_eq!(last(&s, 2).as_deref(), Some("Code(\"0199\")"));
}

/// 201 rows of table 7 in company `C`, the last in key 1 named `last`.
struct ManyRows {
    last: &'static str,
}

impl Backing for ManyRows {
    fn company(&self) -> &str {
        "C"
    }
    fn rows(&self, company: &str, object: u32) -> Option<Rows> {
        if company != "C" || object != 7 {
            return None;
        }
        let t = crate::contract::table();
        let mut rows: Vec<Row> = (0..200)
            .map(|i| crate::contract::row(&format!("{i:04}"), i, i % 7))
            .collect();
        rows.push(crate::contract::row(self.last, 1000, 6));
        Rows::from_rows(&t, rows).ok()
    }
}

#[test]
fn stores_over_different_backings_do_not_share_an_order() {
    let a = MemStore::default().with_backing(std::rc::Rc::new(ManyRows { last: "A" }));
    let b = a.clone().with_backing(std::rc::Rc::new(ManyRows { last: "B" }));
    let before = order_builds();
    assert_eq!(last_in_key_1(&a).as_deref(), Some("Code(\"A\")"));
    assert_eq!(last_in_key_1(&b).as_deref(), Some("Code(\"B\")"));
    let mut again = a.clone();
    again.restore(a.snapshot());
    assert_eq!(last_in_key_1(&again).as_deref(), Some("Code(\"A\")"));
    assert_eq!(order_builds() - before, 2, "one order per backing");
}

#[test]
fn passes_the_snapshot_contract() {
    crate::contract::snapshot_restores_everything(MemStore::default());
}

// ---- export and import of the committed rows (the install baseline's snapshot on disk) ----

#[test]
fn exported_rows_import_into_a_store_that_reads_and_writes_like_the_original() {
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
    let mut s = MemStore::default();
    for a in 0..70 {
        s.insert(
            &cx(),
            &t,
            vec![
                Value::Integer(a),
                Value::Integer(a % 3),
                Value::Code("X".into()),
                Value::Integer(a % 5),
            ],
        )
        .unwrap();
    }
    s.commit();
    // A change that is not committed is not part of the export.
    s.insert(
        &cx(),
        &t,
        vec![
            Value::Integer(999),
            Value::Integer(0),
            Value::Code("X".into()),
            Value::Integer(0),
        ],
    )
    .unwrap();
    let exported = s.export_committed().expect("nothing backs this store");
    assert_eq!(exported.len(), 1);
    assert_eq!(
        (exported[0].company.as_str(), exported[0].object, exported[0].rows.len()),
        ("C", 9, 70)
    );
    let mut back = MemStore::from_exported(exported);
    s.rollback();
    for key in 0..2 {
        assert_eq!(
            format!("{:?}", scan_all(&back, &t, key, &[], true)),
            format!("{:?}", scan_all(&s, &t, key, &[], true))
        );
    }
    assert_eq!(back.count(&cx(), &t, &[]).unwrap(), 70);
    // It is a store like any other: a write is seen, a rollback takes it away, a commit keeps it.
    back.insert(
        &cx(),
        &t,
        vec![
            Value::Integer(500),
            Value::Integer(0),
            Value::Code("X".into()),
            Value::Integer(0),
        ],
    )
    .unwrap();
    assert_eq!(back.count(&cx(), &t, &[]).unwrap(), 71);
    back.rollback();
    assert_eq!(back.count(&cx(), &t, &[]).unwrap(), 70);
    let before = back.table_stamp(&cx(), &t);
    back.insert(
        &cx(),
        &t,
        vec![
            Value::Integer(501),
            Value::Integer(0),
            Value::Code("X".into()),
            Value::Integer(0),
        ],
    )
    .unwrap();
    assert_ne!(back.table_stamp(&cx(), &t), before, "a write changes the table's stamp");
}

#[test]
fn a_backed_store_exports_the_tables_it_holds_and_not_the_backing() {
    let (mut s, b) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    let mut other = t.clone();
    other.object = 8;
    // Nothing wrote to table 9: it reads the backing and is not part of the export.
    assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
    assert!(s.export_committed().expect("a backing is no obstacle").is_empty());
    // A write copies the backing's rows into the store; deleting them all leaves an empty held table.
    s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
    s.delete(&cx, &t, &[Value::Code("A".into())]).unwrap();
    s.commit();
    let exported = s.export_committed().unwrap();
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].rows.len(), 3, "B, C and D");
    // The way back: the same backing, and the held table now says what it said.
    let back = MemStore::from_exported(exported).with_backing(b);
    assert_eq!(back.count(&cx, &t, &[]).unwrap(), 3);
    assert!(back.get(&cx, &t, &[Value::Code("A".into())]).unwrap().is_none());
    assert!(back.get(&cx, &t, &[Value::Code("D".into())]).unwrap().is_some());
    assert_eq!(back.count(&cx, &other, &[]).unwrap(), 0);
}

#[test]
fn a_deleted_out_backed_table_stays_empty_after_the_way_back() {
    let (mut s, b) = backed();
    let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
    s.delete_all(&cx, &t, &[]).unwrap();
    s.commit();
    let back = MemStore::from_exported(s.export_committed().unwrap()).with_backing(b);
    assert_eq!(back.count(&cx, &t, &[]).unwrap(), 0, "not restored again");
}
