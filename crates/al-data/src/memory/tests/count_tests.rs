//! Count and existence checks: an existence check stops at the first row that passes, and a count reads the band of the
//! key the filters narrow most, as a scan does (ADR-035); both answer what a walk of every row answers.

use super::*;

/// 20 000 rows: A 0..200, B 0..100, C "X" everywhere, D = B mod 7; keys (A, B) and (D, A, B).
fn big_store(t: &TableDef) -> MemStore {
    let mut s = MemStore::default();
    for a in 0..200 {
        for b in 0..100 {
            let row = vec![
                Value::Integer(a),
                Value::Integer(b),
                Value::Code("X".into()),
                Value::Integer(b % 7),
            ];
            s.insert(&cx(), t, row).unwrap();
        }
    }
    s
}

fn looked() -> usize {
    ROWS_LOOKED_AT.with(|c| c.get())
}

fn eq(field: usize, v: Value) -> FieldFilter {
    range(field, Some(v.clone()), Some(v))
}

#[test]
fn an_existence_check_stops_at_the_first_row_that_passes() {
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
    let s = big_store(&t);
    // No key starts with C, and every row holds "X" there: every row passes.
    let f = [eq(2, Value::Code("X".into()))];
    let before = looked();
    assert!(s.exists(&cx(), &t, &f).unwrap());
    assert_eq!(looked() - before, 1, "rows looked at by exists");
    let before = looked();
    assert_eq!(s.count_up_to(&cx(), &t, &f, 2).unwrap(), 2);
    assert_eq!(looked() - before, 2, "rows looked at by a count up to 2");
}

#[test]
fn a_count_reads_the_band_of_a_secondary_key_the_filters_bound() {
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
    let s = big_store(&t);
    // D = 3 holds B = 3, 10, .., 94: 14 rows in each of the 200 hundreds.
    let f = [eq(3, Value::Integer(3))];
    let before = looked();
    assert_eq!(s.count(&cx(), &t, &f).unwrap(), 2800);
    assert!(looked() - before <= 2800, "count: {} rows looked at", looked() - before);
    // No row holds D = 9: the band is empty.
    let f = [eq(3, Value::Integer(9))];
    let before = looked();
    assert!(!s.exists(&cx(), &t, &f).unwrap());
    assert_eq!(looked() - before, 0, "exists over an empty band");
}

#[test]
fn count_and_exists_answer_what_a_walk_of_every_row_answers() {
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
    let mut s = big_store(&t);
    let int = Value::Integer;
    let cases: Vec<Vec<FieldFilter>> = vec![
        vec![],
        vec![eq(3, int(3))],
        vec![range(3, Some(int(2)), Some(int(4)))],
        vec![range(3, None, Some(int(0)))],
        vec![eq(0, int(150)), eq(3, int(3))],
        vec![range(1, Some(int(10)), Some(int(20))), eq(3, int(5))],
        vec![eq(2, Value::Code("X".into())), eq(3, int(6))],
        vec![eq(2, Value::Code("Y".into())), eq(3, int(6))],
        vec![eq(3, int(9))],
        vec![range(0, Some(int(199)), None)],
    ];
    let check = |s: &MemStore, when: &str| {
        for f in &cases {
            let n = reference(s, &t, 0, f, true).len();
            assert_eq!(s.count(&cx(), &t, f).unwrap(), n, "{when}: count of {f:?}");
            assert_eq!(s.exists(&cx(), &t, f).unwrap(), n > 0, "{when}: exists of {f:?}");
            assert_eq!(
                s.count_up_to(&cx(), &t, f, 2).unwrap(),
                n.min(2),
                "{when}: count up to 2 of {f:?}"
            );
        }
    };
    check(&s, "as inserted");
    // Writes the cached order of (D, A, B) is carried through: a new row, a Modify that moves a row, a Delete.
    s.insert(&cx(), &t, vec![int(500), int(3), Value::Code("Y".into()), int(6)])
        .unwrap();
    s.modify(&cx(), &t, vec![int(7), int(3), Value::Code("X".into()), int(9)])
        .unwrap();
    s.delete(&cx(), &t, &[int(150), int(3)]).unwrap();
    check(&s, "after writes");
}
