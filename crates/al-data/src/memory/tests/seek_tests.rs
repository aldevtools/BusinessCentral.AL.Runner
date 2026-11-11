//! The key seek (ADR-035): a scan starts where the filters on the leading key fields start, and returns what a walk
//! returns.

use super::*;

#[test]
fn a_filter_on_the_leading_key_fields_makes_the_scan_read_only_the_rows_near_it() {
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
    let mut s = MemStore::default();
    for a in 0..200 {
        for b in 0..100 {
            let row = vec![
                Value::Integer(a),
                Value::Integer(b),
                Value::Code("X".into()),
                Value::Integer(b % 7),
            ];
            s.insert(&cx(), &t, row).unwrap();
        }
    }
    let looked = || ROWS_LOOKED_AT.with(|c| c.get());
    // Primary key: A = 150 is the 150th hundred of 20000 rows.
    let f = [range(0, Some(Value::Integer(150)), Some(Value::Integer(150)))];
    let before = looked();
    let first = s
        .next(
            &cx(),
            &t,
            &Scan {
                key: 0,
                filters: &f,
                ascending: true,
                after: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(format!("{:?}", first[0]), "Integer(150)");
    assert!(
        looked() - before <= 5,
        "primary key: {} rows looked at",
        looked() - before
    );
    // Another key (D, A, B): D = 6 holds one row in seven, and the scan has to find the first of them.
    let f = [
        range(3, Some(Value::Integer(6)), Some(Value::Integer(6))),
        range(0, Some(Value::Integer(190)), None),
    ];
    let before = looked();
    let first = s
        .next(
            &cx(),
            &t,
            &Scan {
                key: 1,
                filters: &f,
                ascending: true,
                after: None,
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(format!("{:?} {:?}", first[3], first[0]), "Integer(6) Integer(190)");
    assert!(
        looked() - before <= 5,
        "other key: {} rows looked at",
        looked() - before
    );
}

/// A point filter (`SetRange(F, X)`) on a Code key field whose values hold a hyphen ("No. Series Line" filtered on
/// "Series Code" = 'S-ORD', al-runner issue 6924): the rows that pass are those equal to X, which are next to each
/// other in the key's order whatever the characters, so the scan reads only the rows near them.
#[test]
fn a_point_filter_on_a_code_with_a_hyphen_makes_the_scan_read_only_the_rows_near_it() {
    let t = seek_table(vec![vec![0, 1], vec![2, 3, 0, 1]]);
    let mut s = MemStore::default();
    for a in 0..300 {
        let row = vec![
            Value::Integer(a),
            Value::Integer(0),
            Value::Code(format!("S-{:03}", a % 150).into()),
            Value::Integer(a / 150),
        ];
        s.insert(&cx(), &t, row).unwrap();
    }
    let looked = || ROWS_LOOKED_AT.with(|c| c.get());
    let f = [
        range(2, Some(Value::Code("S-077".into())), Some(Value::Code("S-077".into()))),
        range(3, Some(Value::Integer(0)), Some(Value::Integer(5))),
    ];
    for ascending in [true, false] {
        let before = looked();
        let got = scan_all(&s, &t, 1, &f, ascending);
        assert_eq!(format!("{got:?}"), format!("{:?}", reference(&s, &t, 1, &f, ascending)));
        assert_eq!(got.len(), 2);
        assert!(
            looked() - before <= 6,
            "ascending {ascending}: {} rows looked at",
            looked() - before
        );
    }
}

#[test]
fn a_key_slot_that_names_another_key_later_is_ordered_again() {
    // An ad-hoc key (SetCurrentKey on other fields) takes a slot of the key list; the same slot number then
    // stands for different fields, and the cached order of the first has to be left (found in Tests-ERM,
    // codeunit 138025: a seek on a stale order found no Value Entry).
    let by_b = seek_table(vec![vec![0], vec![1, 0]]);
    let by_d = seek_table(vec![vec![0], vec![3, 0]]);
    let mut s = MemStore::default();
    for a in 0..100 {
        let row = vec![
            Value::Integer(a),
            Value::Integer(a % 10),
            Value::Code("X".into()),
            Value::Integer(99 - a),
        ];
        s.insert(&cx(), &by_b, row).unwrap();
    }
    let f = [range(3, Some(Value::Integer(5)), Some(Value::Integer(5)))];
    assert_eq!(scan_all(&s, &by_b, 1, &[], true).len(), 100);
    assert_eq!(
        format!("{:?}", scan_all(&s, &by_d, 1, &f, true)),
        format!("{:?}", reference(&s, &by_d, 1, &f, true))
    );
    assert_eq!(scan_all(&s, &by_d, 1, &f, true).len(), 1);
    assert_eq!(
        format!("{:?}", scan_all(&s, &by_d, 1, &f, false)),
        format!("{:?}", reference(&s, &by_d, 1, &f, false))
    );
}

#[test]
fn a_scan_in_one_key_uses_the_band_of_another_key_the_filters_bound() {
    // Filters on D, the leading field of key 1, while the scan runs in the primary key (A, B): the walk reads
    // every row of the table for each step; the band of key 1 holds the 400 rows that can pass.
    let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
    let mut s = MemStore::default();
    for a in 0..200 {
        for b in 0..100 {
            let row = vec![
                Value::Integer(a),
                Value::Integer(b),
                Value::Code("X".into()),
                Value::Integer(b % 50),
            ];
            s.insert(&cx(), &t, row).unwrap();
        }
    }
    let f = [range(3, Some(Value::Integer(6)), Some(Value::Integer(6)))];
    let before = ROWS_LOOKED_AT.with(|c| c.get());
    let got = scan_all(&s, &t, 0, &f, true);
    let looked = ROWS_LOOKED_AT.with(|c| c.get()) - before;
    assert_eq!(got.len(), 400);
    assert!(looked <= 1500, "{looked} rows looked at for a scan of 400 rows");
    assert_eq!(format!("{got:?}"), format!("{:?}", reference(&s, &t, 0, &f, true)));
}

#[test]
fn a_scan_through_another_keys_band_follows_writes_between_the_steps() {
    let mut seed = 0xD1B5_4A32_D192_ED03u64;
    let mut rnd = move |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    for round in 0..30 {
        let t = seek_table(vec![vec![0, 1], vec![3, 0, 1], vec![2, 3, 0, 1]]);
        let mut s = MemStore::default();
        for _ in 0..400 {
            let row = vec![
                Value::Integer(rnd(30) as i32),
                Value::Integer(rnd(30) as i32),
                Value::Code(["A", "B", "ab", ""][rnd(4) as usize].into()),
                Value::Integer(rnd(40) as i32),
            ];
            let _ = s.insert(&cx(), &t, row);
        }
        let key = (round % 3) as usize;
        let ascending = round % 2 == 0;
        let lo = rnd(40) as i32;
        let filters = vec![
            range(3, Some(Value::Integer(lo)), Some(Value::Integer(lo + (round % 4)))),
            FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: 1,
                expr: FilterExpr::Ne(Value::Integer(7)),
                group: 0,
            },
        ];
        let mut after: Option<Vec<Value>> = None;
        for step in 0..300 {
            // A write between some steps; several reads in a row between others.
            if rnd(4) == 0 {
                let row = vec![
                    Value::Integer(rnd(30) as i32),
                    Value::Integer(rnd(30) as i32),
                    Value::Code(["A", "B", "ab", ""][rnd(4) as usize].into()),
                    Value::Integer(rnd(40) as i32),
                ];
                match rnd(3) {
                    0 => {
                        let _ = s.insert(&cx(), &t, row);
                    }
                    1 => {
                        let _ = s.modify(&cx(), &t, row);
                    }
                    _ => {
                        let _ = s.delete(&cx(), &t, &[row[0].clone(), row[1].clone()]);
                    }
                }
            }
            let got = s
                .next(
                    &cx(),
                    &t,
                    &Scan {
                        key,
                        filters: &filters,
                        ascending,
                        after: after.as_deref(),
                    },
                )
                .unwrap();
            let expect = reference(&s, &t, key, &filters, ascending).into_iter().find(|r| {
                after.as_ref().is_none_or(|a| {
                    let o = al_builtins::filter::key_cmp(&crate::position(&t, key, r), a);
                    if ascending {
                        o == Ordering::Greater
                    } else {
                        o == Ordering::Less
                    }
                })
            });
            assert_eq!(
                format!("{got:?}"),
                format!("{expect:?}"),
                "round {round} step {step} key {key} ascending {ascending}"
            );
            after = got.map(|r| crate::position(&t, key, &r));
        }
    }
}

#[test]
fn a_seek_returns_the_rows_a_walk_returns_in_the_same_order() {
    seek_matches_walk(&["", "A", "B", "ab", "C", " ", "  ", "Z"], 3);
}

/// The same with Codes that hold hyphens, plus signs and lower-case letters, whose key order is not the order a
/// filter compares them in (a point filter bounds them by the key order, `Seek::cmp_to`).
#[test]
fn a_seek_over_codes_with_hyphens_returns_the_rows_a_walk_returns() {
    seek_matches_walk(
        &["S-ORD", "S-INV", "-5", "5", "+5", "SORD", "s-ord", "A-", "A", "--5"],
        2,
    );
}

/// Random tables and filters: a scan in the primary key (A, C, B) and in a key that starts with `lead` returns what a
/// walk returns.
fn seek_matches_walk(codes: &[&str], lead: usize) {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut rnd = move |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    for round in 0..60 {
        let second = if lead == 2 { vec![2, 3, 0, 1] } else { vec![3, 2, 0, 1] };
        let t = seek_table(vec![vec![0, 2, 1], second]);
        let mut s = MemStore::default();
        let n = if round % 3 == 0 { 20 } else { 300 };
        for _ in 0..n {
            let row = vec![
                Value::Integer(rnd(6) as i32),
                Value::Integer(rnd(6) as i32),
                Value::Code(codes[rnd(codes.len() as u64) as usize].into()),
                Value::Integer(rnd(4) as i32),
            ];
            let _ = s.insert(&cx(), &t, row);
        }
        for _ in 0..40 {
            let mut filters = Vec::new();
            for _ in 0..rnd(4) {
                let field = rnd(4) as usize;
                let v = |r: u64| -> Value {
                    if field == 2 {
                        Value::Code(codes[(r % codes.len() as u64) as usize].into())
                    } else {
                        Value::Integer((r % 7) as i32)
                    }
                };
                let expr = match rnd(9) {
                    0 => FilterExpr::Eq(v(rnd(100))),
                    1 => FilterExpr::Range(Some(v(rnd(100))), None),
                    2 => FilterExpr::Range(None, Some(v(rnd(100)))),
                    3 => FilterExpr::Range(Some(v(rnd(100))), Some(v(rnd(100)))),
                    4 => FilterExpr::Gt(v(rnd(100))),
                    5 => FilterExpr::Lt(v(rnd(100))),
                    6 => FilterExpr::Ne(v(rnd(100))),
                    7 => FilterExpr::Or(vec![FilterExpr::Eq(v(rnd(100))), FilterExpr::Eq(v(rnd(100)))]),
                    _ => FilterExpr::Le(v(rnd(100))),
                };
                let group = [0, 0, 0, 2, -1][rnd(5) as usize];
                filters.push(FieldFilter {
                    lang: al_builtins::filter::filter_language(),
                    field,
                    expr,
                    group,
                });
            }
            for key in 0..2 {
                for ascending in [true, false] {
                    assert_eq!(
                        format!("{:?}", scan_all(&s, &t, key, &filters, ascending)),
                        format!("{:?}", reference(&s, &t, key, &filters, ascending)),
                        "key {key} ascending {ascending} filters {filters:?}"
                    );
                }
            }
            assert_eq!(
                s.count(&cx(), &t, &filters).unwrap(),
                reference(&s, &t, 0, &filters, true).len()
            );
        }
    }
}

/// A point filter on a blank Code key field (`SetRange("Lot No.", '')` on "Entry Summary" in the key "Lot No.",
/// "Serial No.", "Package No.", Tests-SCM codeunit 137380 FiveThousandSerialNos): the rows the filter finds ('' and texts
/// of spaces, `FilterExpr::matches`) are equal to '' in the key order, so they sit together and the filters on the
/// next key fields still bound the scan. Before, a blank value gave no bound and every step read the whole table.
#[test]
fn a_point_filter_on_a_blank_code_still_lets_the_next_key_fields_bound_the_scan() {
    let t = seek_table(vec![vec![0, 1], vec![2, 3, 0, 1]]);
    let mut s = MemStore::default();
    for a in 0..2000 {
        let lot = if a % 100 == 7 { "L1" } else { "" };
        let row = vec![
            Value::Integer(a),
            Value::Integer(0),
            Value::Code(lot.into()),
            Value::Integer(a),
        ];
        s.insert(&cx(), &t, row).unwrap();
    }
    let looked = || ROWS_LOOKED_AT.with(|c| c.get());
    let blank = || Some(Value::Code("".into()));
    for expr in [
        FilterExpr::Range(blank(), blank()),
        FilterExpr::Eq(Value::Code("".into())),
    ] {
        let f = [
            FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: 2,
                expr: expr.clone(),
                group: 0,
            },
            range(3, Some(Value::Integer(1500)), Some(Value::Integer(1500))),
        ];
        for ascending in [true, false] {
            let before = looked();
            let got = scan_all(&s, &t, 1, &f, ascending);
            assert_eq!(format!("{got:?}"), format!("{:?}", reference(&s, &t, 1, &f, ascending)));
            assert_eq!(got.len(), 1);
            assert!(
                looked() - before <= 6,
                "{expr:?} ascending {ascending}: {} rows looked at",
                looked() - before
            );
        }
    }
}

/// Random tables and filters with blank and space-only Codes leading the second key: the seek returns what a walk returns.
#[test]
fn a_seek_over_blank_codes_returns_the_rows_a_walk_returns() {
    seek_matches_walk(&["", " ", "  ", "", "A", "-", "a", "\u{200b}", "A B"], 2);
}

/// A scan in a key the filters bound only loosely, while another key's band holds one row ("Reservation Entry" in the
/// source key filtered on one source line and one "Serial No.", which leads another key; Tests-SCM codeunit 137380
/// TenThousandSerialNos: every serial number of the line sits in the band of the scan's key). The scan reads the
/// narrower band, and returns what a walk returns.
#[test]
fn a_scan_in_a_loosely_bounded_key_uses_a_much_narrower_band_of_another_key() {
    let t = seek_table(vec![vec![0, 1], vec![2, 0, 1], vec![3, 0, 1]]);
    let mut s = MemStore::default();
    for a in 0..2000 {
        let row = vec![
            Value::Integer(a),
            Value::Integer(0),
            Value::Code(if a < 1800 { "LINE1" } else { "LINE2" }.into()),
            Value::Integer(a),
        ];
        s.insert(&cx(), &t, row).unwrap();
    }
    let looked = || ROWS_LOOKED_AT.with(|c| c.get());
    for d in [1500, 1799, 1800, 3] {
        let f = [
            range(2, Some(Value::Code("LINE1".into())), Some(Value::Code("LINE1".into()))),
            range(3, Some(Value::Integer(d)), Some(Value::Integer(d))),
        ];
        for ascending in [true, false] {
            let before = looked();
            let got = scan_all(&s, &t, 1, &f, ascending);
            assert_eq!(format!("{got:?}"), format!("{:?}", reference(&s, &t, 1, &f, ascending)));
            assert_eq!(got.len(), usize::from(d < 1800));
            assert!(
                looked() - before <= 20,
                "D = {d} ascending {ascending}: {} rows looked at",
                looked() - before
            );
        }
    }
}
