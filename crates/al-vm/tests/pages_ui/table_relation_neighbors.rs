//! Neighbors of the 8848/8849 fixes: on the table-relation variant of the same page, open/read/OK keeps main's outcome and a
//! later SetValue is refused.

use al_testrun::Outcome;

const FIELD_PROBE: &str =
    include_str!("../../../../tests/neighbors/8848-later-setvalue-field-code-rec-modify/src/ProbeFieldOnValidate.al");
const REC_PROBE: &str =
    include_str!("../../../../tests/neighbors/8848-later-setvalue-field-code-rec-modify/src/ProbeRecModify.al");
const NEIGHBORS: &str =
    include_str!("../../../../tests/neighbors/8848-later-setvalue-field-code-rec-modify/src/Neighbors.al");

fn outcomes() -> Vec<(String, Outcome)> {
    let src = format!("{FIELD_PROBE}\n{REC_PROBE}\n{NEIGHBORS}");
    al_testkit::outcomes_in(&src, "LT NT7 Rel Tests")
}

#[test]
fn table_relation_open_read_ok_and_later_setvalue_are_refused() {
    let o = outcomes();
    assert_eq!(o.len(), 2);
    let ran: Vec<_> = o
        .iter()
        .filter(|(_, r)| !matches!(r, Outcome::NotRun(_)))
        .map(|(name, r)| format!("{name}: {r:?}"))
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
