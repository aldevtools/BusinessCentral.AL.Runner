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

#[test]
fn other_accents_in_a_key_keep_mains_order() {
    let s = store(&[("Bravo", "A"), ("émile", "B")], false);
    assert_eq!(name(&first(&s, 0, &[]).unwrap().unwrap()), "Bravo");
}
