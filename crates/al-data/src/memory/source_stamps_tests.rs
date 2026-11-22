//! A company copy's untouched tables read the rows of its source's backing. When the copy is made again from another
//! source (after a rollback, or after Company.Delete), the stamps the caches key on must change with the rows
//! (al-runner issue 8597; a copy holds its source's rows, session-system/company/Y22CopyCompany.al:65, and a rollback
//! takes the copy back, :262-288).

use super::*;
use crate::contract::{cx, row, table};
use al_builtins::filter::FilterExpr;

/// Companies A and B of table 7 (`contract::table`: keys [No.], [Group, No.]), 100 rows each, at least
/// `INDEX_MIN_ROWS`. `same_pks`: both use the keys 000..099, and B's Group is A's plus one; otherwise A's keys are
/// A000.. and B's B000.., with the same Groups.
struct TwoSources {
    same_pks: bool,
}

impl Backing for TwoSources {
    fn company(&self) -> &str {
        "A"
    }
    fn rows(&self, company: &str, object: u32) -> Option<Rows> {
        if object != 7 || (company != "A" && company != "B") {
            return None;
        }
        let shift = i32::from(self.same_pks && company == "B");
        let prefix = if self.same_pks { "" } else { company };
        let rows: Vec<Row> = (0..100)
            .map(|i| row(&format!("{prefix}{i:03}"), i, (i + shift) % 7))
            .collect();
        Rows::from_rows(&table(), rows).ok()
    }
}

fn store(same_pks: bool) -> MemStore {
    MemStore::default().with_backing(std::rc::Rc::new(TwoSources { same_pks }))
}

/// Every row of company `company` in key `key` (No. and Group of each), filtered on Group = 3 when `filtered`, step by step.
fn walk(s: &MemStore, company: &str, key: usize, filtered: bool) -> Vec<String> {
    let t = table();
    let group3 = [FieldFilter::new(2, FilterExpr::Eq(Value::Integer(3)), 0)];
    let filters: &[FieldFilter] = if filtered { &group3 } else { &[] };
    let (mut out, mut after) = (Vec::new(), None::<Vec<Value>>);
    while let Some(r) = s
        .next(
            &cx(company),
            &t,
            &Scan {
                key,
                filters,
                ascending: true,
                after: after.as_deref(),
            },
        )
        .unwrap()
    {
        out.push(format!("{:?}/{:?}", r[0], r[2]));
        after = Some(crate::position(&t, key, &r));
    }
    out
}

/// X copied from B in a store that never saw another copy.
fn fresh_copy_of_b(same_pks: bool, key: usize, filtered: bool) -> Vec<String> {
    let mut s = store(same_pks);
    s.copy_company("B", "X");
    walk(&s, "X", key, filtered)
}

/// X copied from A and read, the copy taken back by a rollback, then X copied from B and read again.
fn recopied_after_rollback(same_pks: bool, key: usize, filtered: bool) -> Vec<String> {
    let mut s = store(same_pks);
    s.commit();
    s.copy_company("A", "X");
    assert!(!walk(&s, "X", key, filtered).is_empty(), "X reads A's rows");
    s.rollback();
    s.copy_company("B", "X");
    walk(&s, "X", key, filtered)
}

#[test]
fn a_filtered_read_in_another_key_after_a_rollback_and_a_copy_from_another_source_reads_the_new_source() {
    let expected = fresh_copy_of_b(false, 0, true);
    assert_eq!(expected.len(), 14);
    assert_eq!(recopied_after_rollback(false, 0, true), expected);
}

#[test]
fn a_filtered_read_in_another_key_after_company_delete_and_a_copy_from_another_source_reads_the_new_source() {
    let mut s = store(false);
    s.copy_company("A", "X");
    assert!(!walk(&s, "X", 0, true).is_empty());
    s.drop_company("X");
    s.copy_company("B", "X");
    assert_eq!(walk(&s, "X", 0, true), fresh_copy_of_b(false, 0, true));
}

#[test]
fn a_recopied_source_with_the_same_keys_returns_only_rows_that_pass_the_filter() {
    let got = recopied_after_rollback(true, 0, true);
    assert!(got.iter().all(|r| r.ends_with("/Integer(3)")), "{got:?}");
    assert_eq!(got, fresh_copy_of_b(true, 0, true));
}

#[test]
fn a_recopied_source_with_the_same_keys_reads_in_its_own_order_of_a_secondary_key() {
    assert_eq!(recopied_after_rollback(true, 1, false), fresh_copy_of_b(true, 1, false));
}

#[test]
fn the_table_stamp_of_a_copy_changes_when_its_source_changes() {
    let t = table();
    let mut s = store(false);
    s.commit();
    let none = s.table_stamp(&cx("X"), &t);
    s.copy_company("A", "X");
    let from_a = s.table_stamp(&cx("X"), &t);
    assert_ne!(from_a, none, "queries see the copy");
    s.rollback();
    assert_eq!(
        s.table_stamp(&cx("X"), &t),
        none,
        "a rollback restores the committed stamp"
    );
    s.copy_company("B", "X");
    let from_b = s.table_stamp(&cx("X"), &t);
    assert_ne!(from_b, none);
    assert_ne!(from_b, from_a, "a stamp is never reused for other rows");
}

/// Table 11 with the Text key Name: A serves the row `Bæk`, B the row `Baek`.
struct NameSources;

impl Backing for NameSources {
    fn company(&self) -> &str {
        "A"
    }
    fn rows(&self, company: &str, object: u32) -> Option<Rows> {
        let name = match (company, object) {
            ("A", 11) => "Bæk",
            ("B", 11) => "Baek",
            _ => return None,
        };
        Rows::from_rows(&names(), vec![vec![Value::Text(name.into())]]).ok()
    }
}

fn names() -> TableDef {
    TableDef {
        object: 11,
        name: "Names".into(),
        fields: vec![FieldDef {
            id: 1,
            name: "Name".into(),
            ty: al_builtins::value::RtType::Text(Some(30)),
            stored: true,
            code_storage: Default::default(),
        }],
        keys: vec![vec![0]],
        per_company: true,
    }
}

#[test]
fn an_equality_on_a_recopied_source_checks_the_texts_the_new_source_holds() {
    // `Baek` and `Bæk` may be equal to BC: not measured (collation_read_tests.rs, an_equality_keeps_mains_answer_...).
    let t = names();
    let bk = [FieldFilter::new(0, FilterExpr::Eq(Value::Text("Bæk".into())), 0)];
    let mut s = MemStore::default()
        .as_database()
        .with_backing(std::rc::Rc::new(NameSources));
    s.commit();
    s.copy_company("A", "X");
    assert_eq!(s.count(&cx("X"), &t, &bk).unwrap(), 1);
    s.rollback();
    s.copy_company("B", "X");
    let got = s.count(&cx("X"), &t, &bk);
    assert!(matches!(got, Err(StoreError::Refused(_))), "{got:?}");
}
