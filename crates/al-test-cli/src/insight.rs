//! Reading a run: the ranking of refusal reasons (`--top-refusals`). Pure functions over the run's outcomes.
//! `--baseline-diff` is `not_passing::render_diff`.

use crate::oracle::{CaseId, OracleCase};
use crate::report::Outcome;

pub use al_testrun::refusals::{Refusal, clip, normalise_reason, render_top};

/// The not-run cases of `cases`, grouped by normalised reason, most frequent first (ties by
/// reason). Cases real BC fails are not counted. `in_scope` narrows to an area.
pub fn top_refusals(
    cases: &[OracleCase],
    in_scope: &dyn Fn(&CaseId) -> bool,
    outcome: &dyn Fn(&CaseId) -> Outcome,
) -> Vec<Refusal> {
    al_testrun::refusals::rank(
        cases
            .iter()
            .filter(|c| !c.bc_failed && in_scope(&c.id))
            .filter_map(|c| match outcome(&c.id) {
                Outcome::NotRun(reason) => Some((c.id.clone(), reason)),
                _ => None,
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(cu: u32, n: &str) -> OracleCase {
        OracleCase {
            id: CaseId {
                codeunit: cu,
                name: n.into(),
            },
            bc_failed: false,
        }
    }

    #[test]
    fn top_refusals_rank_by_count_and_keep_the_first_example() {
        let cases = [case(2, "B"), case(1, "A"), case(3, "C"), case(4, "D")];
        let out = |id: &CaseId| match id.codeunit {
            1 | 2 => Outcome::NotRun(format!("X.Y: builtin Foo#{} is not implemented", id.codeunit)),
            3 => Outcome::NotRun("something else".into()),
            _ => Outcome::Pass,
        };
        let top = top_refusals(&cases, &|_| true, &out);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].key, "builtin Foo#N is not implemented");
        assert_eq!(top[0].count, 2);
        assert_eq!(top[0].example.codeunit, 1);
        assert_eq!(top[1].count, 1);
        let text = render_top("BC 1", &top, 1);
        assert!(
            text.contains("3 not run, 2 distinct reasons") && text.contains("e.g. 1 A"),
            "{text}"
        );
        assert!(text.contains("1 more reasons"), "{text}");
    }
}
