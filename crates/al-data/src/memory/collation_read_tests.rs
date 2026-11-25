//! The store refuses a read whose answer rests on an order or a match of letters that no measurement covers (al-runner
//! issues 8195, 8196; review of al-runner PR 8220): `Æ` and `æ` in an ordered read, accented letters under `@`.

use super::*;
use crate::contract::cx;
use crate::{Scan, Store};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::value::RtType;

/// A table with the Text key `Name` and the Code field `Tag`, keyed by `Name` (0) and by `Tag` (1).
fn names() -> TableDef {
    let f = |id, name: &str, ty| FieldDef {
        id,
        name: name.into(),
        ty,
        stored: true,
        code_storage: Default::default(),
    };
    TableDef {
        object: 11,
        name: "Names".into(),
        fields: vec![
            f(1, "Name", RtType::Text(Some(30))),
            f(2, "Tag", RtType::Code(Some(20))),
        ],
        keys: vec![vec![0], vec![1, 0]],
        per_company: true,
    }
}

/// A store holding a row per (name, tag); `database`: the store of normal records, else of a temporary one.
fn store(rows: &[(&str, &str)], database: bool) -> MemStore {
    let mut s = if database {
        MemStore::default().as_database()
    } else {
        MemStore::default()
    };
    for (n, c) in rows {
        s.insert(
            &cx("C"),
            &names(),
            vec![Value::Text((*n).into()), Value::Code((*c).into())],
        )
        .unwrap();
    }
    s
}

fn filter(field: usize, expr: FilterExpr) -> FieldFilter {
    FieldFilter {
        lang: al_builtins::filter::filter_language(),
        field,
        expr,
        group: 0,
    }
}

fn first(s: &MemStore, key: usize, filters: &[FieldFilter]) -> Result<Option<Row>, StoreError> {
    s.next(
        &cx("C"),
        &names(),
        &Scan {
            key,
            filters,
            ascending: true,
            after: None,
        },
    )
}

fn text(t: &str) -> Value {
    Value::Text(t.into())
}

fn name(r: &Row) -> String {
    al_builtins::native::text_of(&r[0]).unwrap_or_default().to_string()
}

fn refused(r: Result<impl std::fmt::Debug, StoreError>) -> bool {
    matches!(r, Err(StoreError::Refused(_)))
}

#[test]
fn an_ordered_read_of_a_text_key_holding_a_small_ae_refuses() {
    // Review of PR 8220, coll-obs C30: main sorts `ærø` after `Bravo`; the capital `Æ` sorts as `AE` (issue 8195).
    let s = store(&[("Alpha", "A"), ("ærø", "B"), ("Bravo", "C")], true);
    assert!(refused(first(&s, 0, &[])));
    let range = filter(0, FilterExpr::Range(Some(text("A")), Some(text("B"))));
    assert!(refused(s.count(&cx("C"), &names(), &[range])));
}

#[test]
fn the_capital_ae_in_a_text_key_of_a_normal_record_keeps_the_assumed_order() {
    let s = store(&[("Alpha", "A"), ("Ærø", "B"), ("Bravo", "C")], true);
    let row = first(&s, 0, &[]).unwrap().unwrap();
    assert_eq!(name(&row), "Ærø", "Ærø sorts as AErø, before Alpha");
}

#[test]
fn a_temporary_record_holding_the_capital_ae_refuses_an_ordered_read() {
    // Review of PR 8220, coll-obs C35: the measurement of issue 8195 is of a normal table.
    let s = store(&[("Alpha", "A"), ("Ærø", "B")], false);
    assert!(refused(first(&s, 0, &[])));
    let s = store(&[("Alpha", "A"), ("Ærø", "B")], true);
    assert!(first(&s, 0, &[]).is_ok());
}

#[test]
fn an_ordered_read_by_a_code_field_holding_ae_refuses_and_one_row_keeps_its_answer() {
    // Review of PR 8220, coll-obs C31: main sorts `ÆRØ` after `ZEBRA`.
    let s = store(&[("a", "ÆRØ"), ("b", "ZEBRA")], true);
    assert!(refused(first(&s, 1, &[])));
    assert!(first(&s, 0, &[]).is_ok(), "the order of Name does not read Tag");
    let s = store(&[("a", "ÆRØ")], true);
    assert!(
        first(&s, 1, &[]).unwrap().is_some(),
        "the order of one row is not observable"
    );
}

#[test]
fn a_range_on_a_code_field_holding_ae_refuses_and_equality_keeps_its_answer() {
    let s = store(&[("a", "ÆRØ"), ("b", "ZEBRA")], true);
    let range = filter(
        1,
        FilterExpr::Range(Some(Value::Code("A".into())), Some(Value::Code("B".into()))),
    );
    assert!(refused(s.count(&cx("C"), &names(), &[range])));
    let eq = filter(1, FilterExpr::Eq(Value::Code("ÆRØ".into())));
    assert_eq!(s.count(&cx("C"), &names(), &[eq]).unwrap(), 1);
}

#[test]
fn a_bound_holding_a_small_ae_refuses_whatever_the_rows_hold() {
    let s = store(&[("Alpha", "A"), ("Bravo", "B")], true);
    let range = filter(0, FilterExpr::Range(Some(text("A")), Some(text("æ"))));
    assert!(refused(s.count(&cx("C"), &names(), &[range])));
}

#[test]
fn an_at_filter_whose_answer_changes_with_the_accents_ignored_refuses() {
    // Review of PR 8220, at-obs D01, D04: `@emile` does not find `émile`, `@émile` does not find `Emile` on main.
    let at = |p: &str| filter(0, FilterExpr::Like(p.into(), true));
    let s = store(&[("émile", "A"), ("Emile", "B")], true);
    assert!(refused(s.count(&cx("C"), &names(), &[at("emile")])));
    let s = store(&[("Emile", "A"), ("emile", "B")], true);
    assert_eq!(
        s.count(&cx("C"), &names(), &[at("emile")]).unwrap(),
        2,
        "no accent: main's answer"
    );
    assert!(refused(s.count(&cx("C"), &names(), &[at("émile")])));
    let s = store(&[("Émile", "A"), ("Emile", "B")], true);
    assert_eq!(
        s.count(&cx("C"), &names(), &[at("emile")]).unwrap(),
        2,
        "the assumed É of issue 8196"
    );
}

#[test]
fn a_temporary_record_refuses_the_assumed_e_acute_under_at() {
    // Review of PR 8220, at-obs DX1: the measurement of issue 8196 is of a normal table.
    let s = store(&[("Émile", "A")], false);
    assert!(refused(s.count(
        &cx("C"),
        &names(),
        &[filter(0, FilterExpr::Like("emile".into(), true))]
    )));
}

/// The rows of `names` 0..n, each `Bæk <i>` with the tag `T<i>`, in a store of normal records.
fn many(n: usize) -> MemStore {
    let rows: Vec<(String, String)> = (0..n).map(|i| (format!("Bæk {i:05}"), format!("T{i:05}"))).collect();
    let refs: Vec<(&str, &str)> = rows.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    store(&refs, true)
}

/// Steps through every row in `key` through `filters`, as a FindSet/Next loop does; `each` runs after each step.
fn walk(s: &mut MemStore, key: usize, filters: &[FieldFilter], mut each: impl FnMut(&mut MemStore, &Row)) -> usize {
    let (mut n, mut after) = (0, None::<Vec<Value>>);
    loop {
        let scan = Scan {
            key,
            filters,
            ascending: true,
            after: after.as_deref(),
        };
        let Some(row) = s.next(&cx("C"), &names(), &scan).unwrap() else {
            return n;
        };
        n += 1;
        after = Some(
            names().keys[key]
                .iter()
                .chain(&names().keys[0])
                .map(|&i| row[i].clone())
                .collect(),
        );
        each(s, &row);
    }
}

fn rows_looked_at() -> u64 {
    held_cols::COLLATION_ROWS.with(|n| n.get())
}

#[test]
fn a_find_set_loop_decides_the_read_once() {
    // Review of PR 8220 round 2 (HIGH-1, probe perf-de): the check ran in every step and looked at the whole table.
    let n = 2_000;
    let mut s = many(n);
    let range = filter(0, FilterExpr::Range(Some(text("A")), Some(text("C"))));
    let before = rows_looked_at();
    assert_eq!(walk(&mut s, 0, std::slice::from_ref(&range), |_, _| {}), n);
    let looked = rows_looked_at() - before;
    assert!(looked <= 2 * n as u64, "looked at {looked} rows for {n}");
}

#[test]
fn a_modify_loop_keeps_the_answer_until_a_read_column_changes() {
    let n = 2_000;
    let mut s = many(n);
    let range = filter(0, FilterExpr::Range(Some(text("A")), Some(text("C"))));
    let before = rows_looked_at();
    // Each step changes the tag, a column the read in the order of Name does not look at.
    let steps = walk(&mut s, 0, std::slice::from_ref(&range), |s, row| {
        let mut row = row.clone();
        row[1] = Value::Code(format!("U{}", al_builtins::native::text_of(&row[1]).unwrap_or_default()).into());
        s.modify(&cx("C"), &names(), row).unwrap();
    });
    assert_eq!(steps, n);
    let looked = rows_looked_at() - before;
    assert!(looked <= 2 * n as u64, "looked at {looked} rows for {n}");
    // A Modify of a column the read filters on is seen: the tag brings `Bak` next to `Bæk`, whose order rests on `æ`.
    let mut s = store(&[("Bæk", "A"), ("Bak", "B")], true);
    let tag = filter(1, FilterExpr::Eq(Value::Code("A".into())));
    assert_eq!(name(&first(&s, 0, std::slice::from_ref(&tag)).unwrap().unwrap()), "Bæk");
    let mut bak = first(&s, 0, &[filter(1, FilterExpr::Eq(Value::Code("B".into())))])
        .unwrap()
        .unwrap();
    bak[1] = Value::Code("A".into());
    assert!(matches!(
        s.modify(&cx("C"), &names(), bak).unwrap(),
        crate::WriteOutcome::Done
    ));
    assert!(refused(first(&s, 0, &[tag])));
}

#[test]
fn an_ordered_read_of_other_accents_looks_at_no_row() {
    // Probe perf-de: `Müller` in the key column; only `Æ` and `æ` can make an order open.
    let s = store(&[("Müller 1", "A"), ("Müller 2", "B")], true);
    let before = rows_looked_at();
    let range = filter(0, FilterExpr::Range(Some(text("A")), Some(text("N"))));
    assert_eq!(name(&first(&s, 0, &[range]).unwrap().unwrap()), "Müller 1");
    assert_eq!(rows_looked_at(), before);
}

#[test]
fn a_lookup_by_an_equality_looks_at_the_table_once() {
    // A loop that reads one row by name per step, the names holding `æ`: only rows that tie with the name are checked.
    let n = 2_000;
    let s = many(n);
    let before = rows_looked_at();
    for i in 0..n {
        let v = text(&format!("Bæk {i:05}"));
        let same = filter(0, FilterExpr::Range(Some(v.clone()), Some(v)));
        assert_eq!(s.count(&cx("C"), &names(), std::slice::from_ref(&same)).unwrap(), 1);
        assert!(first(&s, 1, &[same]).unwrap().is_some());
    }
    let looked = rows_looked_at() - before;
    assert!(looked <= 2 * n as u64, "looked at {looked} rows for {n}");
}

#[test]
fn an_equality_keeps_mains_answer_unless_the_texts_tie_without_ae() {
    // Review of PR 8220 round 2, partner T12 and T13: another row holding `æ` made SetRange(Name, 'Adatum A/S') refuse.
    let s = store(&[("Adatum A/S", "A"), ("Bæk", "B"), ("Kjær", "C")], true);
    let same = |v: &str| filter(0, FilterExpr::Range(Some(text(v)), Some(text(v))));
    assert_eq!(s.count(&cx("C"), &names(), &[same("Adatum A/S")]).unwrap(), 1);
    assert_eq!(s.count(&cx("C"), &names(), &[same("Bæk")]).unwrap(), 1);
    assert_eq!(
        first(&s, 1, &[same("Kjær")]).unwrap().map(|r| name(&r)),
        Some("Kjær".into())
    );
    // `Baek` and `Bæk` may be equal to BC: not measured.
    assert!(refused(s.count(&cx("C"), &names(), &[same("Baek")])));
    assert!(refused(s.count(
        &cx("C"),
        &names(),
        &[filter(0, FilterExpr::Eq(text("BAEK")))]
    )));
}

#[test]
fn an_order_refuses_only_where_ae_decides_it() {
    // Partner T07 and T14: a filter that leaves one row needs no order.
    let s = store(&[("Alpha", "A"), ("ærø", "B"), ("Bravo", "C")], true);
    let one = filter(1, FilterExpr::Eq(Value::Code("B".into())));
    assert_eq!(name(&first(&s, 0, &[one]).unwrap().unwrap()), "ærø");
    // Two names that differ after the same `Bæk`: main's order.
    let s = store(&[("Bæk 2", "A"), ("Alpha", "B"), ("Bæk 1", "C"), ("Cæsar", "D")], true);
    assert_eq!(name(&first(&s, 0, &[]).unwrap().unwrap()), "Alpha");
    let names_in_order: Vec<String> = {
        let mut s = s.clone();
        let mut out = Vec::new();
        walk(&mut s, 0, &[], |_, r| out.push(name(r)));
        out
    };
    assert_eq!(names_in_order, ["Alpha", "Bæk 1", "Bæk 2", "Cæsar"]);
    // `Bak` against `Bæk`: the order rests on `æ`.
    let s = store(&[("Bæk", "A"), ("Bak", "B")], true);
    assert!(refused(first(&s, 0, &[])));
}

#[test]
fn a_row_a_measured_filter_keeps_out_does_not_refuse() {
    let s = store(&[("Alpha", "A"), ("ærø", "X")], true);
    let range = filter(0, FilterExpr::Range(Some(text("A")), Some(text("B"))));
    let tag = filter(1, FilterExpr::Eq(Value::Code("A".into())));
    assert_eq!(s.count(&cx("C"), &names(), &[range.clone(), tag]).unwrap(), 1);
    assert!(refused(s.count(&cx("C"), &names(), &[range])));
}

#[test]
fn other_accents_in_a_key_keep_mains_order() {
    let s = store(&[("Bravo", "A"), ("émile", "B")], false);
    assert_eq!(name(&first(&s, 0, &[]).unwrap().unwrap()), "Bravo");
}

/// A store that `with_committed_tables` builds refuses what the store its tables came from refuses: it notes the held
/// columns of the rows it takes in, as `from_exported` does. The table put on an empty store, and the table put in place
/// of a base's own table, which held only a row that refuses nothing.
#[test]
fn a_store_built_from_committed_tables_refuses_an_order_its_source_refuses() {
    let mut source = store(&[("Alpha", "A"), ("ærø", "B"), ("Bravo", "C")], true);
    source.commit();
    assert!(refused(first(&source, 0, &[])), "the source");
    let table = source.committed_table("C", names().object).unwrap();
    let built = MemStore::default()
        .as_database()
        .with_committed_tables(vec![table.clone()], &[]);
    assert!(refused(first(&built, 0, &[])), "the table put on an empty store");
    let mut base = store(&[("Alpha", "A")], true);
    base.commit();
    assert!(!refused(first(&base, 0, &[])), "the base");
    let replaced = base.with_committed_tables(vec![table], &[]);
    assert!(
        refused(first(&replaced, 0, &[])),
        "the table put in place of the base's"
    );
}
