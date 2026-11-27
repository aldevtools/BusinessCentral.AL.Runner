//! What a test leaves on its result beyond its outcome, taken from the session when the test ends and moved onto its
//! `TestResult`: the `--tdd` stubs it reached, the UI calls `--dev-endpoint-ui` answered for it, and whether an option of
//! the run left it out (`TestResult::skipped_by_option`).
use super::{Scratch, TestResult};
use al_vm::Session;

impl Scratch {
    /// Takes what the test `name` left in `session`; returns the `--tdd` stubs it reached and the UI calls answered.
    pub(super) fn take_marks(&mut self, name: &str, session: &mut Session) -> (Vec<String>, Vec<String>) {
        // The shadow run's notes of the test that just ended (ADR-062 stage 4).
        al_builtins::permission_shadow::drain(name);
        let reached = session.take_tdd_reached();
        if !reached.is_empty() {
            self.stubbed.push((name.to_string(), reached.clone()));
        }
        let answered = session.take_dev_endpoint_answers();
        if !answered.is_empty() {
            self.ui_answered.push((name.to_string(), answered.clone()));
        }
        if let Some(c) = session.take_cell_record() {
            self.cells.push((name.to_string(), c));
        }
        (reached, answered)
    }

    /// The outcome of the test `name` that `skip_after_timeout` or a stop (`stop.rs`) leaves out for `why`; the
    /// test is marked (`TestResult::skipped_by_option`) while its outcome is still that one.
    pub(super) fn skip(&mut self, name: &str, why: &str) -> crate::Outcome {
        self.skipped.push((name.to_string(), why.to_string()));
        crate::Outcome::NotRun(why.to_string())
    }

    /// Moves the order hint of test `name` (ADR-066 amendment 12, decision 2; set by the lane) onto its result.
    pub(super) fn move_order_hint(&mut self, name: &str, result: Option<&mut TestResult>) {
        if let (Some(i), Some(r)) = (self.order_hints.iter().position(|(n, _)| n == name), result) {
            r.order_hint = Some(self.order_hints.swap_remove(i).1);
        }
    }

    /// Moves what `take_marks` kept for the test `name` onto its result.
    pub(super) fn move_marks(&mut self, name: &str, result: Option<&mut TestResult>) {
        let Some(r) = result else {
            return;
        };
        if let Some(i) = self.stubbed.iter().position(|(n, _)| n == name) {
            r.stubs = std::mem::take(&mut self.stubbed[i].1);
        }
        if let Some(i) = self.cells.iter().position(|(n, _)| n == name) {
            r.cells = Some(self.cells.swap_remove(i).1);
        }
        if let Some(i) = self.ui_answered.iter().position(|(n, _)| n == name) {
            r.ui_answered = std::mem::take(&mut self.ui_answered[i].1);
        }
        // A later step may have put another outcome in place of the one the option gave (`corpus::refuse_all`).
        if let Some(i) = self.skipped.iter().position(|(n, _)| n == name) {
            let (_, why) = self.skipped.swap_remove(i);
            r.skipped_by_option = matches!(&r.outcome, crate::Outcome::NotRun(w) if *w == why);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Outcome;

    fn bare(outcome: Outcome) -> TestResult {
        TestResult::bare(0, 0, 50100, "C".into(), "T".into(), outcome)
    }

    #[test]
    fn a_left_out_test_a_later_step_refused_is_not_marked() {
        let mut scratch = Scratch::new(crate::host::RunClock::begin());
        let mut done = vec![("T".to_string(), scratch.skip("T", "stopped"))];
        super::super::corpus::refuse_all(&mut done, "refused after the pass");
        let mut r = bare(done[0].1.clone());
        scratch.move_marks("T", Some(&mut r));
        assert!(!r.skipped_by_option, "{:?}", r.outcome);
    }

    #[test]
    fn a_left_out_test_keeps_its_mark() {
        let mut scratch = Scratch::new(crate::host::RunClock::begin());
        let mut r = bare(scratch.skip("T", "stopped"));
        scratch.move_marks("T", Some(&mut r));
        assert!(r.skipped_by_option);
    }
}
