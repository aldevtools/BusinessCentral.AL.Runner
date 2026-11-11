//! The conformance suite every `Store` must pass (ADR-003). Each function
//! takes a fresh, empty store.

use crate::*;
use al_builtins::filter::{FieldFilter, FilterExpr};

pub fn table() -> TableDef {
    TableDef {
        object: 7,
        name: "T".into(),
        fields: vec![
            FieldDef {
                id: 1,
                name: "No.".into(),
                ty: RtType::Code(Some(20)),
                stored: true,
                code_storage: Default::default(),
            },
            FieldDef {
                id: 2,
                name: "Amount".into(),
                ty: RtType::Decimal,
                stored: true,
                code_storage: Default::default(),
            },
            FieldDef {
                id: 3,
                name: "Group".into(),
                ty: RtType::Integer,
                stored: true,
                code_storage: Default::default(),
            },
        ],
        keys: vec![vec![0], vec![2, 1]],
        per_company: true,
    }
}

/// [`table`] without its secondary key: for a test that puts a handle (a List, a Blob) into field 2, which AL does not allow
/// in a key; the store refuses a key value whose order is not pinned (al-runner issue 7644).
pub fn table_without_secondary_key() -> TableDef {
    let mut t = table();
    t.keys.truncate(1);
    t
}

pub fn cx(company: &str) -> AccessContext {
    AccessContext {
        company: company.into(),
        user: "U".into(),
    }
}

pub fn row(no: &str, amount: i32, group: i32) -> Row {
    vec![
        Value::Code(no.into()),
        Value::Decimal(amount.into()),
        Value::Integer(group),
    ]
}

fn no(r: &Row) -> String {
    match &r[0] {
        Value::Code(s) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

/// All rows of a scan, in order.
pub fn scan_all<S: Store>(
    s: &S,
    c: &AccessContext,
    t: &TableDef,
    key: usize,
    filters: &[FieldFilter],
    ascending: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut after: Option<Vec<Value>> = None;
    while let Some(r) = s
        .next(
            c,
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
        after = Some(position(t, key, &r));
        out.push(no(&r));
    }
    out
}

pub fn insert_then_get<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    assert_eq!(s.insert(&c, &t, row("1", 10, 1)).unwrap(), InsertOutcome::Inserted);
    let got = s.get(&c, &t, &[Value::Code("1".into())]).unwrap().expect("row");
    assert_eq!(no(&got), "1");
    assert!(s.get(&c, &t, &[Value::Code("2".into())]).unwrap().is_none());
}

pub fn duplicate_insert_is_reported<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    s.insert(&c, &t, row("1", 10, 1)).unwrap();
    assert_eq!(s.insert(&c, &t, row("1", 99, 1)).unwrap(), InsertOutcome::AlreadyExists);
    let got = s.get(&c, &t, &[Value::Code("1".into())]).unwrap().unwrap();
    assert!(
        matches!(got[1], Value::Decimal(d) if d == 10.into()),
        "the original row is kept"
    );
}

pub fn modify_and_delete<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    assert_eq!(s.modify(&c, &t, row("1", 5, 1)).unwrap(), WriteOutcome::NotFound);
    s.insert(&c, &t, row("1", 10, 1)).unwrap();
    assert_eq!(s.modify(&c, &t, row("1", 5, 1)).unwrap(), WriteOutcome::Done);
    assert!(
        matches!(s.get(&c, &t, &[Value::Code("1".into())]).unwrap().unwrap()[1], Value::Decimal(d) if d == 5.into())
    );
    assert_eq!(
        s.delete(&c, &t, &[Value::Code("1".into())]).unwrap(),
        WriteOutcome::Done
    );
    assert_eq!(
        s.delete(&c, &t, &[Value::Code("1".into())]).unwrap(),
        WriteOutcome::NotFound
    );
}

pub fn rename<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    s.insert(&c, &t, row("1", 10, 1)).unwrap();
    s.insert(&c, &t, row("2", 20, 1)).unwrap();
    assert_eq!(
        s.rename(&c, &t, &[Value::Code("1".into())], row("2", 10, 1)).unwrap(),
        RenameOutcome::AlreadyExists
    );
    assert_eq!(
        s.rename(&c, &t, &[Value::Code("9".into())], row("8", 10, 1)).unwrap(),
        RenameOutcome::NotFound
    );
    assert_eq!(
        s.rename(&c, &t, &[Value::Code("1".into())], row("3", 10, 1)).unwrap(),
        RenameOutcome::Renamed
    );
    assert!(s.get(&c, &t, &[Value::Code("1".into())]).unwrap().is_none());
    assert!(s.get(&c, &t, &[Value::Code("3".into())]).unwrap().is_some());
}

pub fn scans_follow_key_order_and_direction<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    for (n, g) in [("C", 1), ("A", 2), ("B", 1), ("D", 2)] {
        s.insert(&c, &t, row(n, 0, g)).unwrap();
    }
    assert_eq!(scan_all(&s, &c, &t, 0, &[], true), ["A", "B", "C", "D"]);
    assert_eq!(scan_all(&s, &c, &t, 0, &[], false), ["D", "C", "B", "A"]);
    // Secondary key (Group, No.).
    assert_eq!(scan_all(&s, &c, &t, 1, &[], true), ["B", "C", "A", "D"]);
}

/// A scan that continues after a position keeps going when the row at that position is gone (the code
/// deleted it, or changed the key it is positioned by), in both directions and in both keys, and rows with
/// equal secondary keys come in primary key order.
pub fn scans_continue_after_a_position_whose_row_is_gone<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    for (n, g) in [("A", 2), ("B", 1), ("C", 2), ("D", 1), ("E", 2)] {
        s.insert(&c, &t, row(n, 0, g)).unwrap();
    }
    for (key, ascending) in [(0usize, true), (0, false), (1, true), (1, false)] {
        let mut t_rows = Vec::new();
        // Position on "B", delete it, then continue.
        let b = s.get(&c, &t, &[Value::Code("B".into())]).unwrap();
        if let Some(b) = b {
            let at = position(&t, key, &b);
            s.delete(&c, &t, &[Value::Code("B".into())]).unwrap();
            let mut after = Some(at);
            while let Some(r) = s
                .next(
                    &c,
                    &t,
                    &Scan {
                        key,
                        filters: &[],
                        ascending,
                        after: after.as_deref(),
                    },
                )
                .unwrap()
            {
                after = Some(position(&t, key, &r));
                t_rows.push(no(&r));
            }
            s.insert(&c, &t, row("B", 0, 1)).unwrap();
        }
        let want: Vec<&str> = match (key, ascending) {
            (0, true) => vec!["C", "D", "E"],
            (0, false) => vec!["A"],
            (1, true) => vec!["D", "A", "C", "E"],
            (1, false) => vec![],
            _ => unreachable!(),
        };
        assert_eq!(t_rows, want, "key {key}, ascending {ascending}");
    }
    // Equal secondary keys: primary key order, and scanning from the middle of them.
    assert_eq!(scan_all(&s, &c, &t, 1, &[], true), ["B", "D", "A", "C", "E"]);
    assert_eq!(scan_all(&s, &c, &t, 1, &[], false), ["E", "C", "A", "D", "B"]);
}

/// Walks in the secondary key of a table big enough to be indexed keep their order while the table changes
/// underneath them: rows inserted, deleted, moved to another place in the key by a Modify, and a rollback.
pub fn secondary_key_walks_follow_the_table_as_it_changes<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    // The key is (Group, Amount, No.). Group = i % 5, Amount = i: it interleaves the primary key order.
    for i in 0..200 {
        s.insert(&c, &t, row(&format!("{i:03}"), i, i % 5)).unwrap();
    }
    s.commit();
    let by_key = |s: &S, ascending: bool| scan_all(s, &c, &t, 1, &[], ascending);
    // The model: (group, amount, id).
    let expect = |rows: &[(i32, i32, i32)], ascending: bool| -> Vec<String> {
        let mut v: Vec<(i32, i32, i32)> = rows.to_vec();
        v.sort();
        if !ascending {
            v.reverse();
        }
        v.iter().map(|(_, _, i)| format!("{i:03}")).collect()
    };
    let original: Vec<(i32, i32, i32)> = (0..200).map(|i| (i % 5, i, i)).collect();
    let mut model = original.clone();
    assert_eq!(by_key(&s, true), expect(&model, true));
    assert_eq!(by_key(&s, false), expect(&model, false));

    // A Modify that leaves the key alone, then one that changes the Amount, then one that moves the row.
    s.modify(&c, &t, row("010", 10, 0)).unwrap();
    assert_eq!(by_key(&s, true), expect(&model, true));
    s.modify(&c, &t, row("010", 999, 0)).unwrap();
    model.retain(|&(_, _, i)| i != 10);
    model.push((0, 999, 10));
    assert_eq!(by_key(&s, true), expect(&model, true));
    s.modify(&c, &t, row("010", 999, 4)).unwrap();
    model.retain(|&(_, _, i)| i != 10);
    model.push((4, 999, 10));
    assert_eq!(by_key(&s, true), expect(&model, true));
    // Insert, delete.
    s.insert(&c, &t, row("500", 0, 2)).unwrap();
    model.push((2, 0, 500));
    s.delete(&c, &t, &[Value::Code("003".into())]).unwrap();
    model.retain(|&(_, _, i)| i != 3);
    assert_eq!(by_key(&s, true), expect(&model, true));
    assert_eq!(by_key(&s, false), expect(&model, false));
    // A walk during which the current row leaves its place.
    let first = s
        .next(
            &c,
            &t,
            &Scan {
                key: 1,
                filters: &[],
                ascending: true,
                after: None,
            },
        )
        .unwrap()
        .unwrap();
    let at = position(&t, 1, &first);
    s.modify(&c, &t, row(&no(&first), 1_000, 4)).unwrap();
    let following = s
        .next(
            &c,
            &t,
            &Scan {
                key: 1,
                filters: &[],
                ascending: true,
                after: Some(&at),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        al_builtins::filter::key_cmp(&position(&t, 1, &following), &at),
        std::cmp::Ordering::Greater,
        "the walk goes on past the old position"
    );
    // Filters apply to the walk.
    let f = [FieldFilter {
        lang: al_builtins::filter::filter_language(),
        field: 2,
        expr: FilterExpr::Eq(Value::Integer(2)),
        group: 0,
    }];
    let seen = scan_all(&s, &c, &t, 1, &f, true);
    assert!(!seen.is_empty() && seen.iter().all(|n| n == "500" || n.parse::<i32>().unwrap() % 5 == 2));
    // A rollback puts the committed order back.
    s.rollback();
    assert_eq!(by_key(&s, true), expect(&original, true));
    assert_eq!(by_key(&s, false), expect(&original, false));
}

pub fn filters_count_and_delete_all<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    for (n, a) in [("A", 5), ("B", 15), ("C", 25)] {
        s.insert(&c, &t, row(n, a, 1)).unwrap();
    }
    let f = [FieldFilter {
        lang: al_builtins::filter::filter_language(),
        field: 1,
        expr: FilterExpr::Range(Some(Value::Integer(10)), None),
        group: 0,
    }];
    assert_eq!(scan_all(&s, &c, &t, 0, &f, true), ["B", "C"]);
    assert_eq!(s.count(&c, &t, &f).unwrap(), 2);
    assert_eq!(s.count(&c, &t, &[]).unwrap(), 3);
    assert_eq!(s.delete_all(&c, &t, &f).unwrap(), 2);
    assert_eq!(scan_all(&s, &c, &t, 0, &[], true), ["A"]);
}

pub fn companies_are_separate<S: Store>(mut s: S) {
    let t = table();
    s.insert(&cx("A"), &t, row("1", 1, 1)).unwrap();
    assert!(s.get(&cx("B"), &t, &[Value::Code("1".into())]).unwrap().is_none());
    s.insert(&cx("B"), &t, row("1", 2, 1)).unwrap();
    assert_eq!(s.count(&cx("A"), &t, &[]).unwrap(), 1);
    let mut shared = table();
    shared.per_company = false;
    s.insert(&cx("A"), &shared, row("S", 1, 1)).unwrap();
    assert!(
        s.get(&cx("B"), &shared, &[Value::Code("S".into())]).unwrap().is_some(),
        "DataPerCompany = false is shared"
    );
}

pub fn rollback_discards_uncommitted_changes<S: Store>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    s.insert(&c, &t, row("1", 1, 1)).unwrap();
    s.commit();
    s.insert(&c, &t, row("2", 1, 1)).unwrap();
    s.delete(&c, &t, &[Value::Code("1".into())]).unwrap();
    s.rollback();
    assert_eq!(scan_all(&s, &c, &t, 0, &[], true), ["1"]);
}

/// Every contract test, for one store type.
pub fn run_all<S: Store>(new: impl Fn() -> S) {
    insert_then_get(new());
    duplicate_insert_is_reported(new());
    modify_and_delete(new());
    rename(new());
    scans_follow_key_order_and_direction(new());
    scans_continue_after_a_position_whose_row_is_gone(new());
    secondary_key_walks_follow_the_table_as_it_changes(new());
    filters_count_and_delete_all(new());
    companies_are_separate(new());
    rollback_discards_uncommitted_changes(new());
}

/// The snapshot section, only for `SnapshotStore`s (ADR-004).
pub fn snapshot_restores_everything<S: SnapshotStore>(mut s: S) {
    let (t, c) = (table(), cx("A"));
    s.insert(&c, &t, row("1", 1, 1)).unwrap();
    let snap = s.snapshot();
    s.insert(&c, &t, row("2", 1, 1)).unwrap();
    s.commit();
    s.restore(snap);
    assert_eq!(scan_all(&s, &c, &t, 0, &[], true), ["1"]);
}
