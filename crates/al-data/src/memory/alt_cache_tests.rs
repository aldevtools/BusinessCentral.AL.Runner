//! The cache of `alt_next` (the rows a set of filters leaves, in the order of the scan's key) is kept per table and key,
//! a few sets of filters each, told apart by the filters themselves (`same_filters`), not by their text.

use super::alt_cache::ALT_SCANS_PER_KEY;
use super::*;
use crate::contract::{cx, row, table};
use al_builtins::filter::FilterExpr;

/// 200 rows of table 7 in company C (keys [No.], [Group, No.]): No. 0000..0199, Amount i, Group i % 7.
fn store() -> MemStore {
    let mut s = MemStore::default();
    for i in 0..200 {
        s.insert(&cx("C"), &table(), row(&format!("{i:04}"), i, i % 7)).unwrap();
    }
    s
}

/// Group = `g` and Amount >= `min`: the filters bound key 1 (Group leads it), not key 0.
fn group_from(g: i32, min: i32) -> Vec<FieldFilter> {
    vec![
        FieldFilter::new(2, FilterExpr::Eq(Value::Integer(g)), 0),
        FieldFilter::new(1, FilterExpr::Ge(Value::Decimal(min.into())), 0),
    ]
}

/// Every row in key 0 through `filters`, step by step (the second step builds the sorted list, later ones search it).
fn walk(s: &MemStore, filters: &[FieldFilter]) -> Vec<String> {
    let t = table();
    let (mut out, mut after) = (Vec::new(), None::<Vec<Value>>);
    while let Some(r) = s
        .next(
            &cx("C"),
            &t,
            &Scan {
                key: 0,
                filters,
                ascending: true,
                after: after.as_deref(),
            },
        )
        .unwrap()
    {
        out.push(format!("{:?}", r[0]));
        after = Some(crate::position(&t, 0, &r));
    }
    out
}

/// What the filters leave, worked out without the store.
fn expected(g: i32, min: i32) -> Vec<String> {
    (0..200)
        .filter(|i| i % 7 == g && *i >= min)
        .map(|i| format!("{:?}", Value::Code(format!("{i:04}").into())))
        .collect()
}

#[test]
fn a_table_and_key_keep_a_few_sets_of_filters_the_one_used_last_first() {
    let s = store();
    for min in 0..20 {
        assert_eq!(walk(&s, &group_from(3, min)), expected(3, min));
    }
    assert_eq!(s.alt_entries(&cx("C"), &table(), 0), ALT_SCANS_PER_KEY);
    assert!(s.alt_entry_kept(&cx("C"), &table(), 0, &group_from(3, 19)));
    assert!(!s.alt_entry_kept(&cx("C"), &table(), 0, &group_from(3, 0)));
}

#[test]
fn filters_built_apart_share_their_entry_and_other_filters_keep_their_own_rows() {
    let s = store();
    // Interleaved: each set's steps search its own sorted list, never the other's.
    for _ in 0..3 {
        assert_eq!(walk(&s, &group_from(3, 10)), expected(3, 10));
        assert_eq!(walk(&s, &group_from(4, 10)), expected(4, 10));
    }
    assert_eq!(s.alt_entries(&cx("C"), &table(), 0), 2);
    // A filter that differs only in its group or language is another scan.
    let mut grouped = group_from(3, 10);
    grouped[1].group = 2;
    assert_eq!(walk(&s, &grouped), expected(3, 10));
    assert_eq!(s.alt_entries(&cx("C"), &table(), 0), 3);
}
