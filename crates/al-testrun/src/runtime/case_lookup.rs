//! Looking up the outcome of an oracle case by name.
//!
//! A data-driven procedure's cases are named `Procedure[id]` (BiDataDriven.al:19). When the runner refuses a whole test
//! codeunit before it lists the cases (a test handler it cannot run, an unknown transaction model, a refused OnRun), it
//! can only record one `NotRun` row named after the procedure. That row covers every case of the procedure: none of them
//! ran, and the reason is the same for each. A `Fail` or `Pass` row named after the procedure does not: it is one result
//! of its own (BiDataDriven.al:636), so it never stands in for a case.

use super::Outcome;
use std::collections::HashMap;

/// The outcome recorded for `name` (lower case) of codeunit `cu` in run `si`, or the `NotRun` of its data-driven procedure.
pub(super) fn find(
    outcomes: &HashMap<(usize, u32, String), Outcome>,
    si: usize,
    cu: u32,
    name: &str,
) -> Option<Outcome> {
    if let Some(o) = outcomes.get(&(si, cu, name.to_string())) {
        return Some(o.clone());
    }
    let procedure = name.strip_suffix(']')?.split_once('[')?.0;
    match outcomes.get(&(si, cu, procedure.to_string())) {
        Some(o @ Outcome::NotRun(_)) => Some(o.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(rows: &[(&str, Outcome)]) -> HashMap<(usize, u32, String), Outcome> {
        rows.iter().map(|(n, o)| ((0, 7, n.to_string()), o.clone())).collect()
    }

    #[test]
    fn a_refused_procedure_row_covers_each_of_its_cases() {
        let m = map(&[("rows", Outcome::NotRun("why".into()))]);
        assert!(matches!(find(&m, 0, 7, "rows[alpha]"), Some(Outcome::NotRun(w)) if w == "why"));
        assert!(matches!(find(&m, 0, 7, "rows[ b ]"), Some(Outcome::NotRun(_))));
    }

    #[test]
    fn an_exact_case_row_wins() {
        let m = map(&[("rows", Outcome::NotRun("why".into())), ("rows[alpha]", Outcome::Pass)]);
        assert!(matches!(find(&m, 0, 7, "rows[alpha]"), Some(Outcome::Pass)));
    }

    #[test]
    fn a_failed_or_passed_procedure_row_is_no_case() {
        let m = map(&[("rows", Outcome::Fail("x".into())), ("other", Outcome::Pass)]);
        assert!(find(&m, 0, 7, "rows[alpha]").is_none());
        assert!(find(&m, 0, 7, "other[a]").is_none());
        assert!(find(&m, 0, 7, "missing").is_none());
        assert!(find(&m, 1, 7, "rows[alpha]").is_none());
    }
}
