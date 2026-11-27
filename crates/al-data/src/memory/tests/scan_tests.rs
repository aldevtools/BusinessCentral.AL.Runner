//! Every way a scan steps returns what a walk of every row returns (issue 7671: one scan path): the walk in the primary
//! key, the band of the scan's own key, the band of another key the filters bound, and the sorted list `alt_next` keeps
//! for later steps at the same table version. Both directions, every key and an ad hoc key, writes between the steps
//! (a Modify that moves a row in a key among them), and several scans at one table version, so the kept lists are read.

use super::*;

/// A small xorshift generator: the same rounds on every run.
fn rng(mut seed: u64) -> impl FnMut(u64) -> u64 {
    move |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    }
}

const CODES: [&str; 6] = ["A", "B", "AB", "C", "", "ab"];

/// A random row: A and B the primary key, C a Code, D an Integer.
fn random_row(rnd: &mut impl FnMut(u64) -> u64) -> Row {
    vec![
        Value::Integer(rnd(40) as i32),
        Value::Integer(rnd(20) as i32),
        Value::Code(CODES[rnd(CODES.len() as u64) as usize].into()),
        Value::Integer(rnd(30) as i32),
    ]
}

/// One to three filters that bound the primary key, key (D, A, B), key (C, D, A, B), or no key.
fn random_filters(rnd: &mut impl FnMut(u64) -> u64) -> Vec<FieldFilter> {
    let mut filters = Vec::new();
    for _ in 0..1 + rnd(3) {
        let lo = rnd(30) as i32;
        filters.push(match rnd(6) {
            0 => range(3, Some(Value::Integer(lo)), Some(Value::Integer(lo + rnd(4) as i32))),
            1 => range(3, Some(Value::Integer(lo)), Some(Value::Integer(lo))),
            2 => {
                let c = Value::Code(CODES[rnd(4) as usize].into());
                range(2, Some(c.clone()), Some(c))
            }
            3 => range(0, Some(Value::Integer(lo)), None),
            4 => FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: 1,
                expr: FilterExpr::Ne(Value::Integer(7)),
                group: 0,
            },
            _ => range(2, Some(Value::Code("A".into())), Some(Value::Code("B".into()))),
        });
    }
    filters
}

/// The row a scan in `key` returns after the position `after`, worked out from every row.
fn reference_next(
    s: &MemStore,
    t: &TableDef,
    key: usize,
    filters: &[FieldFilter],
    ascending: bool,
    after: Option<&[Value]>,
) -> Option<Row> {
    reference(s, t, key, filters, ascending).into_iter().find(|r| {
        after.is_none_or(|a| {
            let o = al_builtins::filter::key_cmp(&crate::position(t, key, r), a);
            if ascending {
                o == Ordering::Greater
            } else {
                o == Ordering::Less
            }
        })
    })
}

/// A write between two steps: an Insert, a Modify (which may move the row in keys 1, 2 and the ad hoc key) or a Delete.
fn random_write(s: &mut MemStore, t: &TableDef, rnd: &mut impl FnMut(u64) -> u64) {
    let row = random_row(rnd);
    match rnd(3) {
        0 => {
            let _ = s.insert(&cx(), t, row);
        }
        1 => {
            let _ = s.modify(&cx(), t, row);
        }
        _ => {
            let _ = s.delete(&cx(), t, &[row[0].clone(), row[1].clone()]);
        }
    }
}

#[test]
fn every_scan_path_returns_what_a_walk_returns_through_writes() {
    let mut rnd = rng(0x2545_F491_4F6C_DD1D);
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1], vec![2, 3, 0, 1]]);
    // An ad hoc key (SetCurrentKey on fields no key starts with) takes the next key slot (al-vm `Session::table`).
    let mut adhoc = t.clone();
    adhoc.keys.push(vec![1, 2]);
    for round in 0..24 {
        let mut s = MemStore::default();
        for _ in 0..150 + rnd(250) {
            let _ = s.insert(&cx(), &t, random_row(&mut rnd));
        }
        let filters = random_filters(&mut rnd);
        // Several scans at one table version read the lists the first ones kept; a write now and then starts again.
        for scan_no in 0..8 {
            let key = rnd(4) as usize;
            let def = if key == 3 { &adhoc } else { &t };
            let ascending = rnd(2) == 0;
            let writes = round % 2 == 1;
            let mut after: Option<Vec<Value>> = None;
            for step in 0..60 {
                if writes && rnd(8) == 0 {
                    random_write(&mut s, &t, &mut rnd);
                }
                let scan = Scan {
                    key,
                    filters: &filters,
                    ascending,
                    after: after.as_deref(),
                };
                let got = s.next(&cx(), def, &scan).unwrap();
                let expect = reference_next(&s, def, key, &filters, ascending, after.as_deref());
                assert_eq!(
                    format!("{got:?}"),
                    format!("{expect:?}"),
                    "round {round} scan {scan_no} step {step} key {key} ascending {ascending} filters {filters:?}"
                );
                let Some(row) = got else { break };
                after = Some(crate::position(def, key, &row));
            }
            assert_eq!(
                s.count(&cx(), &t, &filters).unwrap(),
                reference(&s, &t, 0, &filters, true).len(),
                "round {round} count"
            );
        }
    }
}

/// Two sets of filters and both directions, interleaved step by step at one table version: each scan keeps to its
/// own list and its own direction.
#[test]
fn interleaved_scans_at_one_table_version_each_return_what_a_walk_returns() {
    let mut rnd = rng(0x9E37_79B9_7F4A_7C15);
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1], vec![2, 3, 0, 1]]);
    let mut s = MemStore::default();
    for _ in 0..400 {
        let _ = s.insert(&cx(), &t, random_row(&mut rnd));
    }
    let sets = [
        vec![range(3, Some(Value::Integer(4)), Some(Value::Integer(6)))],
        vec![
            range(3, Some(Value::Integer(4)), Some(Value::Integer(6))),
            range(0, Some(Value::Integer(10)), None),
        ],
    ];
    for key in 0..3 {
        // (filters, ascending, position, done)
        let mut scans: Vec<(usize, bool, Option<Vec<Value>>, bool)> = vec![
            (0, true, None, false),
            (0, false, None, false),
            (1, true, None, false),
            (1, false, None, false),
        ];
        while scans.iter().any(|x| !x.3) {
            for (f, ascending, after, done) in scans.iter_mut().filter(|x| !x.3) {
                let scan = Scan {
                    key,
                    filters: &sets[*f],
                    ascending: *ascending,
                    after: after.as_deref(),
                };
                let got = s.next(&cx(), &t, &scan).unwrap();
                let expect = reference_next(&s, &t, key, &sets[*f], *ascending, after.as_deref());
                assert_eq!(
                    format!("{got:?}"),
                    format!("{expect:?}"),
                    "key {key} filters {f} ascending {ascending}"
                );
                match got {
                    Some(row) => *after = Some(crate::position(&t, key, &row)),
                    None => *done = true,
                }
            }
        }
    }
}
